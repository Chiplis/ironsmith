//! The readings of one "as long as ..." static condition clause: the typed
//! condition shapes (life totals, player counters, attachment counts,
//! devotion, subject and player status, ...) read before the existential
//! count grammar. Formerly a first-match ladder in `anthem_grant_lines`; every
//! reading runs; two different readings of one input are an ambiguity error.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct ConditionClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) display: &'a String,
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl ConditionClause<'_> {
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
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<PredicateAst>, CardTextError>,
    ) -> ParseOutcome<PredicateAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("static-condition-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&ConditionClause<'_>) -> bool,
    read: fn(&ConditionClause<'_>) -> ParseOutcome<PredicateAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("static-condition-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("mana-from-source-spent-comparison"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_mana_from_source_spent_comparison(input)),
    },
    Reading {
        id: RuleId::new("removed-from-draft"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_removed_from_draft(input)),
    },
    Reading {
        id: RuleId::new("negated-subject-descriptor"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_negated_subject_descriptor(input)),
    },
    Reading {
        id: RuleId::new("cards-in-hand"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cards_in_hand(input)),
    },
    Reading {
        id: RuleId::new("life-total"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_life_total(input)),
    },
    Reading {
        id: RuleId::new("source-keyword-filter"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_keyword_filter(input)),
    },
    Reading {
        id: RuleId::new("fixed-static-condition-kind"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_fixed_static_condition_kind(input)),
    },
    Reading {
        id: RuleId::new("life-total-or-less-condition"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("life-total")
        },
        read: |input| input.outcome(read_life_total_or_less_condition(input)),
    },
    Reading {
        id: RuleId::new("player-counter-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_counter_condition(input)),
    },
    Reading {
        id: RuleId::new("object-attached-to-object-condition"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("fixed-static-condition-kind")
        },
        read: |input| input.outcome(read_object_attached_to_object_condition(input)),
    },
    Reading {
        id: RuleId::new("devotion-static-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_devotion_static_condition(input)),
    },
    Reading {
        id: RuleId::new("subject-status-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_subject_status_condition(input)),
    },
    Reading {
        id: RuleId::new("subject-descriptor-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_subject_descriptor_condition(input)),
    },
    Reading {
        id: RuleId::new("player-status-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_status_condition(input)),
    },
    Reading {
        id: RuleId::new("x-value-at-least-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_x_value_at_least_condition(input)),
    },
    Reading {
        id: RuleId::new("player-achievement-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_achievement_condition(input)),
    },
    Reading {
        id: RuleId::new("cards-drawn-this-turn-static-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cards_drawn_this_turn_static_condition(input)),
    },
    Reading {
        id: RuleId::new("blocking-source-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_blocking_source_condition(input)),
    },
    Reading {
        id: RuleId::new("dice-rolled-this-turn-static-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_dice_rolled_this_turn_static_condition(input)),
    },
    Reading {
        id: RuleId::new("source-in-graveyard-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_in_graveyard_condition(input)),
    },
    Reading {
        id: RuleId::new("independently-articled-graveyard-cards-static-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_independently_articled_graveyard_cards_static_condition(input))
        },
    },
    Reading {
        id: RuleId::new("conjoined-static-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conjoined_static_condition(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &ConditionClause<'_>) -> ParseOutcome<RuleMatch<PredicateAst>> {
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
    let mut distinct: Vec<RegistryCandidate<PredicateAst>> = Vec::new();
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

fn read_life_total_or_less_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(life) = anthem_grant_grammar::parse_life_total_or_less_condition(&tokens) {
        return Ok(Some(PredicateAst::LifeTotalOrLess(life as i32)));
    }
    Ok(None)
}
fn read_player_counter_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    let display = input.display;
    if let Some(counter) = crate::grammar::conditions::parse_player_counter_condition(&tokens) {
        let Some((operator, value)) =
            crate::util::comparison_to_value_comparison_operator(counter.comparison)
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player-counter comparison (clause: '{display}')"
            )))
            .map(Some);
        };
        return Ok(Some(PredicateAst::ValueComparison {
            left: crate::effect::Value::PlayerCounters(counter.player, counter.counter_type),
            operator,
            right: crate::effect::Value::Fixed(value),
        }));
    }
    Ok(None)
}
fn read_object_attached_to_object_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(attachment) =
        crate::grammar::conditions::parse_object_attached_to_object_condition(&tokens)
    {
        return Ok(Some(PredicateAst::AttachmentCount {
            attachment: attachment.attachment_filter,
            host: ironsmith_core::AttachmentConditionHost::Matching(attachment.attached_to_filter),
            comparison: attachment.comparison,
            display: attachment.display,
        }));
    }
    Ok(None)
}
fn read_devotion_static_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = parse_devotion_static_condition(&tokens)? {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_subject_status_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = crate::grammar::conditions::parse_subject_status_condition(&tokens)
        .and_then(|condition| condition.condition_expr())
    {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_subject_descriptor_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    let display = input.display;
    if let Some(condition) = crate::grammar::conditions::parse_subject_descriptor_condition(&tokens)
    {
        return Ok(Some(condition.condition_expr(display.clone())));
    }
    Ok(None)
}
fn read_player_status_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = crate::grammar::conditions::parse_player_status_condition(&tokens)
        .and_then(|condition| condition.condition_expr())
    {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_x_value_at_least_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(count) = anthem_grant_grammar::parse_x_value_at_least_condition(&tokens) {
        return Ok(Some(PredicateAst::XValueAtLeast(count)));
    }
    Ok(None)
}
fn read_player_achievement_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = crate::grammar::conditions::parse_player_achievement_condition(&tokens)
        .and_then(|condition| condition.condition_expr())
    {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_cards_drawn_this_turn_static_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = parse_cards_drawn_this_turn_static_condition(&tokens) {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_blocking_source_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    let display = input.display;
    if let Some(shape) = anthem_grant_grammar::parse_blocking_source_condition(&tokens) {
        return Ok(Some(PredicateAst::CountComparison {
            count: AnthemCountExpression::BlockingSource,
            comparison: shape.comparison,
            display: Some(display.clone()),
        }));
    }
    Ok(None)
}
fn read_dice_rolled_this_turn_static_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = parse_dice_rolled_this_turn_static_condition(&tokens) {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_source_in_graveyard_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    let display = input.display;
    if anthem_grant_grammar::parse_source_in_graveyard_condition(&tokens) {
        let mut filter = ObjectFilter::source();
        filter.zone = Some(Zone::Graveyard);
        return Ok(Some(PredicateAst::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
            display: Some(display.clone()),
        }));
    }
    Ok(None)
}
fn read_independently_articled_graveyard_cards_static_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = parse_independently_articled_graveyard_cards_static_condition(&tokens)
    {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_conjoined_static_condition(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(conjoined) = parse_conjoined_static_condition_clause(&tokens) {
        return Ok(Some(conjoined));
    }
    Ok(None)
}
fn read_mana_from_source_spent_comparison(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Ok(PredicateAst::ValueComparison {
        left:
            crate::effect::Value::ManaFromSourceSpentToCastThisSpell {
                source_filter,
                include_source_noun: false,
                ..
            },
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(1),
    }) = crate::grammar::filters::parse_condition_predicate_lexed(&tokens)
    {
        return Ok(Some(PredicateAst::ValueComparison {
            left: crate::effect::Value::ManaFromSourceSpentToCastThisSpell {
                source_filter,
                include_source_noun: false,
                reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
            },
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(1),
        }));
    }
    Ok(None)
}
fn read_removed_from_draft(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = crate::grammar::conditions::parse_removed_from_draft_condition(&tokens)
    {
        return Ok(Some(PredicateAst::Player(PlayerPredicateAst::PlayerRemovedDraftCardMatching {
            player: condition.player,
            filter: condition.filter,
            with_cards_named: condition.with_cards_named,
        })));
    }
    Ok(None)
}
fn read_negated_subject_descriptor(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = parse_negated_subject_descriptor_condition(&tokens) {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_cards_in_hand(input: &ConditionClause<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = parse_cards_in_hand_static_condition(&tokens) {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_life_total(input: &ConditionClause<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(condition) = parse_life_total_static_condition(&tokens) {
        return Ok(Some(condition));
    }
    Ok(None)
}
fn read_source_keyword_filter(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    let fixed_kind = anthem_grant_grammar::parse_fixed_static_condition_kind(&tokens);
    // Attack history and source characteristics are disjoint condition
    // domains. A complete `attacked a battle this turn` fact is temporal, so
    // it never enters the source-characteristic filter grammar.
    if !matches!(
        fixed_kind,
        Some(anthem_grant_grammar::FixedStaticConditionKind::SourceAttackedBattleThisTurn)
    ) && let Some(filter) =
        crate::grammar::filters::parse_source_keyword_condition_filter_lexed(&tokens)
    {
        return Ok(Some(PredicateAst::Source(SourcePredicateAst::SourceMatches(filter))));
    }
    Ok(None)
}
fn read_fixed_static_condition_kind(
    input: &ConditionClause<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    let display = input.display;
    let fixed_kind = anthem_grant_grammar::parse_fixed_static_condition_kind(&tokens);
    if let Some(kind) = fixed_kind {
        use anthem_grant_grammar::FixedStaticConditionKind;
        return (match kind {
            FixedStaticConditionKind::SourceEquipmentAttachedToCreature => Ok(
                PredicateAst::AttachedToSourceMatches(ObjectFilter::creature()),
            ),
            FixedStaticConditionKind::SourceSpellWasKicked => Ok(PredicateAst::ThisSpellWasKicked),
            FixedStaticConditionKind::OpponentLostLifeThisTurn => {
                Ok(PredicateAst::TurnEvents(TurnEventPredicateAst::OpponentLostLifeThisTurn))
            }
            FixedStaticConditionKind::YouDidNotCastSpellThisTurn => Ok(PredicateAst::Not(
                Box::new(PredicateAst::Player(PlayerPredicateAst::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerAst::You,
                    count: 1,
                })),
            )),
            FixedStaticConditionKind::YouCastSpellThisTurn => {
                Ok(PredicateAst::Player(PlayerPredicateAst::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerAst::You,
                    count: 1,
                }))
            }
            FixedStaticConditionKind::NoCardsInYourLibrary => Ok(PredicateAst::CountComparison {
                count: AnthemCountExpression::MatchingFilter(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .owned_by(PlayerFilter::You),
                ),
                comparison: crate::effect::Comparison::Equal(0),
                display: Some("there are no cards in your library".to_string()),
            }),
            FixedStaticConditionKind::SourceIsOnBattlefield => {
                Ok(PredicateAst::Source(SourcePredicateAst::SourceIsInZone(Zone::Battlefield)))
            }
            FixedStaticConditionKind::SourceIsNotOnBattlefield => Ok(PredicateAst::Not(Box::new(
                PredicateAst::Source(SourcePredicateAst::SourceIsInZone(Zone::Battlefield)),
            ))),
            FixedStaticConditionKind::SourceDevouredCreature => {
                Ok(PredicateAst::Source(SourcePredicateAst::SourceDevouredCreaturesOrMore(1)))
            }
            FixedStaticConditionKind::SourceIsSoulbondPaired => {
                Ok(PredicateAst::Source(SourcePredicateAst::SourceIsSoulbondPaired))
            }
            FixedStaticConditionKind::SourceAttackedThisTurn => {
                Ok(PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn))
            }
            FixedStaticConditionKind::SourceAttackedBattleThisTurn => {
                Ok(PredicateAst::Source(SourcePredicateAst::SourceAttackedBattleThisTurn))
            }
            FixedStaticConditionKind::YouAttackedThisTurn => Ok(PredicateAst::TurnEvents(TurnEventPredicateAst::AttackedThisTurn)),
            FixedStaticConditionKind::SourceEnteredThisTurn => {
                let mut filter = ObjectFilter::source();
                filter.entered_battlefield_this_turn = true;
                Ok(PredicateAst::CountComparison {
                    count: AnthemCountExpression::MatchingFilter(filter),
                    comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                    display: Some(display.clone()),
                })
            }
            FixedStaticConditionKind::YourTurn => Ok(PredicateAst::YourTurn),
            FixedStaticConditionKind::SourcePowerEven => Err(CardTextError::ParseError(
                "unsupported source power parity condition (clause: 'this power is even')"
                    .to_string(),
            )),
            FixedStaticConditionKind::SourcePowerOdd => Err(CardTextError::ParseError(
                "unsupported source power parity condition (clause: 'this power is odd')"
                    .to_string(),
            )),
            FixedStaticConditionKind::NotYourTurn => {
                Ok(PredicateAst::Not(Box::new(PredicateAst::YourTurn)))
            }
            FixedStaticConditionKind::YourLifeAtMostHalfStarting => {
                Ok(PredicateAst::Player(PlayerPredicateAst::PlayerLifeAtMostHalfStartingLifeTotal {
                    player: PlayerAst::You,
                }))
            }
            FixedStaticConditionKind::YouCommittedCrimeThisTurn => {
                Ok(PredicateAst::Player(PlayerPredicateAst::PlayerCommittedCrimeThisTurn {
                    player: PlayerAst::You,
                }))
            }
        })
        .map(Some);
    }
    Ok(None)
}
