//! The readings of one "this spell costs ... if ..." condition: a life total
//! or less, life change this turn, a target controller's graveyard, the known
//! spell-cost facts, a bound predicate, a bound static condition. Formerly a
//! first-match ladder in `keyword_static`; every reading runs; two different
//! readings of one input are an ambiguity error.

use super::static_mid_facts::KnownSpellCostConditionFact as Fact;
use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};
use crate::static_abilities::ThisSpellCostCondition;

/// The input the readings read.
pub(super) struct SpellCostCondition<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) words: &'a [&'a str],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl SpellCostCondition<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                crate::word_claims::probe_only(|| {
                    (reading.admits)(self)
                        && matches!((reading.read)(self), ParseOutcome::Match(_))
                })
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome.
    fn outcome(
        &self,
        read: Option<crate::static_abilities::ThisSpellCostCondition>,
    ) -> ParseOutcome<crate::static_abilities::ThisSpellCostCondition> {
        match read {
            Some(value) => ParseOutcome::matched(value, crate::util::span_from_tokens(self.tokens)),
            None => ParseOutcome::NoMatch,
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&SpellCostCondition<'_>) -> bool,
    read: fn(
        &SpellCostCondition<'_>,
    ) -> ParseOutcome<crate::static_abilities::ThisSpellCostCondition>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("spell-cost-condition-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("combat-and-completed-action-cost-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(super::spell_cost_event_conditions::read(input.tokens)),
    },
    Reading {
        id: RuleId::new("life-total-or-less"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_life_total_or_less(input)),
    },
    Reading {
        id: RuleId::new("life-change-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_life_change_this_turn(input)),
    },
    Reading {
        id: RuleId::new("target-controller-graveyard-cards"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_controller_graveyard_cards(input)),
    },
    Reading {
        id: RuleId::new("known-spell-cost-fact"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("target-controller-graveyard-cards")
        },
        read: |input| input.outcome(read_known_spell_cost_fact(input)),
    },
    Reading {
        // "if a player attacked you during their last turn" (Avenge).
        id: RuleId::new("player-attacked-you-during-their-last-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(
                matches!(
                    input.words,
                    [
                        "a" | "an",
                        "player" | "opponent",
                        "attacked",
                        "you",
                        "during",
                        "their",
                        "last",
                        "turn"
                    ]
                )
                .then(|| ThisSpellCostCondition::ConditionExpr {
                    condition: crate::ConditionExpr::TurnHistory(
                        ironsmith_core::TurnHistoryCondition::OpponentAttackedControllerDuringTheirLastTurn,
                    ),
                    display: input.words.join(" "),
                }),
            )
        },
    },
    Reading {
        id: RuleId::new("bound-condition-predicate"),
        head: HeadDiscriminator::Any,
        admits: |input| !input.read_by("combat-and-completed-action-cost-condition"),
        read: |input| input.outcome(read_bound_condition_predicate(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(
    input: &SpellCostCondition<'_>,
) -> ParseOutcome<RuleMatch<crate::static_abilities::ThisSpellCostCondition>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !crate::word_claims::probe_only(|| (reading.admits)(input)) {
            continue;
        }
        let (registry_outcome, registry_claims) =
            crate::word_claims::scoped(|| (reading.read)(input).within(reading.id));
        match registry_outcome {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            ).with_claims(registry_claims)),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<crate::static_abilities::ThisSpellCostCondition>> =
        Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_life_total_or_less(
    input: &SpellCostCondition<'_>,
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    let tokens = input.tokens;
    if let Some(condition) = parse_life_total_or_less_spell_cost_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_life_change_this_turn(
    input: &SpellCostCondition<'_>,
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    let tokens = input.tokens;
    if let Some(condition) = parse_player_life_change_this_turn_condition(tokens)
        .and_then(this_spell_cost_condition_from_life_change_this_turn)
    {
        return Some(condition);
    }
    None
}
fn read_target_controller_graveyard_cards(
    input: &SpellCostCondition<'_>,
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    let tokens = input.tokens;
    if let Some(condition) =
        parse_target_whose_controller_has_cards_in_graveyard_cost_condition(tokens)
    {
        return Some(condition);
    }
    None
}
fn read_known_spell_cost_fact(
    input: &SpellCostCondition<'_>,
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    let tokens = input.tokens;
    let words = input.words;
    if let Some(fact) = static_mid_facts::parse_known_spell_cost_condition(tokens) {
        let descriptor_filter = matches!(fact, Fact::CastThisTurn { .. })
            .then(|| cast_this_turn_descriptor_filter(tokens))
            .flatten();
        let condition = match fact {
            Fact::LifeTotalLessThanStarting => ThisSpellCostCondition::LifeTotalLessThanStarting,
            Fact::AttackedThisTurn => ThisSpellCostCondition::ConditionExpr {
                condition: crate::ConditionExpr::AttackedThisTurn,
                display: words.join(" "),
            },
            Fact::CreatureDiedThisTurn => ThisSpellCostCondition::ConditionExpr {
                condition: crate::ConditionExpr::CreatureDiedThisTurn,
                display: words.join(" "),
            },
            Fact::Night => ThisSpellCostCondition::IsNight,
            Fact::Bargained => ThisSpellCostCondition::ConditionExpr {
                condition: crate::ConditionExpr::ThisSpellPaidLabel("Bargain".into()),
                display: words.join(" "),
            },
            Fact::SacrificedArtifactThisTurn => {
                ThisSpellCostCondition::YouSacrificedArtifactThisTurn
            }
            Fact::CommittedCrimeThisTurn => ThisSpellCostCondition::YouCommittedCrimeThisTurn,
            Fact::CreatureLeftBattlefieldUnderYourControlThisTurn => {
                ThisSpellCostCondition::CreatureLeftBattlefieldUnderYourControlThisTurn
            }
            // "if you've cast another creature spell this turn": any other
            // descriptor is a full spell filter over the turn's cast history.
            Fact::CastThisTurn { another, .. } if descriptor_filter.is_some() => {
                let filter = descriptor_filter.clone().expect("guarded descriptor filter");
                ThisSpellCostCondition::ConditionExpr {
                    condition: crate::ConditionExpr::ValueComparison {
                        left: Value::SpellsCastThisTurnMatching {
                            player: PlayerFilter::You,
                            filter,
                            exclude_source: another,
                        },
                        operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(1),
                    },
                    display: crate::lexer::render_token_slice(tokens)
                        .trim()
                        .trim_end_matches(['.', ','])
                        .to_string(),
                }
            }
            // "If you've cast another white spell this turn" (Patrician's
            // Scorn): the color restricts which earlier spell counts, and
            // "another" excludes this spell itself.
            Fact::CastThisTurn { another, card_types }
                if words
                    .iter()
                    .any(|word| crate::util::parse_color(word).is_some()) =>
            {
                claim_plain_cast_this_turn_descriptor(tokens);
                let mut filter = ObjectFilter::spell();
                for word in words.iter() {
                    if let Some(color) = crate::util::parse_color(word) {
                        filter.colors = Some(filter.colors.unwrap_or_default().union(color));
                    }
                }
                filter.card_types = card_types;
                ThisSpellCostCondition::ConditionExpr {
                    condition: crate::ConditionExpr::ValueComparison {
                        left: Value::SpellsCastThisTurnMatching {
                            player: PlayerFilter::You,
                            filter,
                            exclude_source: another,
                        },
                        operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(1),
                    },
                    display: crate::lexer::render_token_slice(tokens)
                        .trim()
                        .trim_end_matches(['.', ','])
                        .to_string(),
                }
            }
            Fact::CastThisTurn { card_types, .. } => {
                // "another spell" / "an instant or sorcery spell": the typed
                // count of spells cast; those words are this reading's.
                claim_plain_cast_this_turn_descriptor(tokens);
                ThisSpellCostCondition::YouCastSpellsThisTurnOrMore {
                    count: 1,
                    card_types,
                }
            }
            Fact::NotStartingPlayer => ThisSpellCostCondition::NotStartingPlayer,
            Fact::CreatureIsAttackingYou => ThisSpellCostCondition::CreatureIsAttackingYou,
            Fact::CreatureCardPutIntoYourGraveyardThisTurn => {
                ThisSpellCostCondition::CreatureCardPutIntoYourGraveyardThisTurn
            }
            Fact::DistinctCardTypesInYourGraveyardOrMore(count) => {
                ThisSpellCostCondition::DistinctCardTypesInYourGraveyardOrMore(count)
            }
            Fact::CardsInYourGraveyardOrMore { count, card_types } => {
                if card_types.is_empty() {
                    ThisSpellCostCondition::YouHaveCardsInYourGraveyardOrMore(count)
                } else {
                    ThisSpellCostCondition::YouHaveCardsOfTypesInYourGraveyardOrMore {
                        count,
                        card_types,
                    }
                }
            }
            Fact::OpponentHasPoisonCountersOrMore(count) => {
                ThisSpellCostCondition::OpponentHasPoisonCountersOrMore(count)
            }
            Fact::OpponentHasCardsInGraveyardOrMore(count) => {
                ThisSpellCostCondition::OpponentHasCardsInGraveyardOrMore(count)
            }
            Fact::NoCardsInHandMatching(filter) => ThisSpellCostCondition::NoCardsInHandMatching {
                filter,
                display: words.join(" "),
            },
            Fact::OnlyCreatureCardsInHandNamed(name) => {
                ThisSpellCostCondition::OnlyCreatureCardsInHandNamed(name)
            }
            Fact::CardInYourGraveyardMatching(filter) => {
                ThisSpellCostCondition::CardInYourGraveyardMatching {
                    filter,
                    display: words.join(" "),
                }
            }
            Fact::TargetsLargeControlledCreature => {
                let mut protected = ObjectFilter::creature().you_control();
                protected.power = Some(crate::filter::Comparison::GreaterThanOrEqual(7));
                let mut stack_target = ObjectFilter::default();
                stack_target.zone = Some(Zone::Stack);
                stack_target.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
                stack_target.targets_object = Some(Box::new(protected));
                ThisSpellCostCondition::TargetsObject(stack_target)
            }
            Fact::Target(target) => match target {
                static_mid_facts::CostTargetFact::You => {
                    ThisSpellCostCondition::TargetsPlayer(PlayerFilter::You)
                }
                static_mid_facts::CostTargetFact::Opponent => {
                    ThisSpellCostCondition::TargetsPlayer(PlayerFilter::Opponent)
                }
                static_mid_facts::CostTargetFact::AnyPlayer => {
                    ThisSpellCostCondition::TargetsPlayer(PlayerFilter::Any)
                }
                static_mid_facts::CostTargetFact::Object(filter) => {
                    ThisSpellCostCondition::TargetsObject(filter)
                }
            },
            Fact::OpponentHasNoCardsInHand => ThisSpellCostCondition::OpponentHasNoCardsInHand,
            Fact::OpponentControlsLandsOrMore(count) => {
                ThisSpellCostCondition::OpponentControlsLandsOrMore(count)
            }
            Fact::OpponentControlsMoreCreaturesThanYou(count) => {
                ThisSpellCostCondition::OpponentControlsAtLeastNMoreCreaturesThanYou(count)
            }
            Fact::TotalCreatureCardsInAllGraveyardsOrMore(count) => {
                ThisSpellCostCondition::TotalCreatureCardsInAllGraveyardsOrMore(count)
            }
            Fact::OpponentCastSpellsThisTurnOrMore(count) => {
                ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(count)
            }
            Fact::OpponentDrewCardsThisTurnOrMore(count) => {
                ThisSpellCostCondition::OpponentDrewCardsThisTurnOrMore(count)
            }
            Fact::OpponentHadCardsPutIntoGraveyardThisTurnOrMore(count) => {
                ThisSpellCostCondition::OpponentHadCardsPutIntoGraveyardThisTurnOrMore(count)
            }
            Fact::YouWereDealtDamageByCreaturesThisTurnOrMore(count) => {
                ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(count)
            }
            Fact::AssassinOrCommanderDealtCombatDamage => {
                ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                    Subtype::Assassin,
                )
            }
        };
        return Some(condition);
    }
    None
}
fn read_bound_condition_predicate(
    input: &SpellCostCondition<'_>,
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    let tokens = input.tokens;
    let words = input.words;
    // The spell-cost model states its conditions bound, so a recognized
    // predicate is bound here at that boundary.
    if let Some(condition_expr) =
        parse_conjoined_this_spell_cost_condition(tokens).and_then(bind_static_condition_predicate)
    {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: condition_expr,
            display: words.join(" "),
        });
    }
    None
}
pub(super) fn read_bound_static_condition(
    input: &SpellCostCondition<'_>,
) -> Option<crate::static_abilities::ThisSpellCostCondition> {
    let tokens = input.tokens;
    let words = input.words;
    if let Some(condition_expr) = static_condition_clause_bound(tokens) {
        return Some(ThisSpellCostCondition::ConditionExpr {
            condition: condition_expr,
            display: words.join(" "),
        });
    }
    None
}


/// The descriptor tokens of "you('ve) cast [another | a | an] <descriptor>
/// this turn" and whether "another" introduced them.
fn cast_this_turn_descriptor(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], Option<&OwnedLexToken>)> {
    let cast = tokens.iter().position(|token| token.is_word("cast"))?;
    let this = tokens
        .windows(2)
        .rposition(|pair| pair[0].is_word("this") && pair[1].is_word("turn"))?;
    let mut start = cast + 1;
    let another = tokens.get(start).filter(|token| token.is_word("another"));
    if another.is_some() || tokens.get(start).is_some_and(|token| token.is_any_word(&["a", "an"])) {
        start += 1;
    }
    (start < this).then(|| (&tokens[start..this], another))
}

/// A descriptor of only "spell(s)", instant/sorcery and connectives, or
/// colors: the typed readings above already model these.
fn is_plain_cast_this_turn_descriptor(descriptor: &[OwnedLexToken]) -> bool {
    descriptor.iter().filter_map(OwnedLexToken::as_word).all(|word| {
        matches!(word, "spell" | "spells" | "instant" | "sorcery" | "or" | "and")
            || crate::util::parse_color(word).is_some()
    })
}

/// Any other spell descriptor, read by the ordinary spell-filter grammar
/// ("creature spell", "Dragon spell", "spell with mana value 4 or greater").
fn cast_this_turn_descriptor_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let (descriptor, another) = cast_this_turn_descriptor(tokens)?;
    let ends_in_spell = descriptor
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .last()
        .is_some_and(|word| matches!(word, "spell" | "spells"));
    if !ends_in_spell || is_plain_cast_this_turn_descriptor(descriptor) {
        return None;
    }
    let mut filter =
        crate::grammar::filters::parse_spell_filter_with_grammar_entrypoint(descriptor);
    // "another" excludes this spell through `exclude_source`, not `other`.
    filter.other = false;
    if let Some(another) = another {
        crate::word_claims::claim_tokens(std::slice::from_ref(another));
    }
    Some(filter)
}

/// Claim the typed plain descriptor ("another [instant or sorcery] spell").
fn claim_plain_cast_this_turn_descriptor(tokens: &[OwnedLexToken]) {
    if let Some((descriptor, another)) = cast_this_turn_descriptor(tokens)
        && is_plain_cast_this_turn_descriptor(descriptor)
    {
        crate::word_claims::claim_tokens(descriptor);
        if let Some(another) = another {
            crate::word_claims::claim_tokens(std::slice::from_ref(another));
        }
    }
}
