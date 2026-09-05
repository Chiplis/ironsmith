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
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
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
        id: RuleId::new("bound-condition-predicate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
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
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
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
            Fact::CastThisTurn { card_types, .. } => {
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
