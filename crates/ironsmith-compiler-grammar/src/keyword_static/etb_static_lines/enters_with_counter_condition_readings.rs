//! The readings of the condition of one "enters with ... counters if ..."
//! line: the fixed condition shapes, an X threshold, spells cast this turn,
//! colors of mana spent, same-color mana spent, read before the general
//! static condition clause. Formerly a first-match ladder in
//! `etb_static_lines`; every reading runs, resolved by rank while the
//! overlaps are measured.

use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct CounterCondition<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl CounterCondition<'_> {
    /// A reading's outcome.
    fn outcome(&self, read: Option<PredicateAst>) -> ParseOutcome<PredicateAst> {
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
    admits: fn(&CounterCondition<'_>) -> bool,
    read: fn(&CounterCondition<'_>) -> ParseOutcome<PredicateAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("enters-with-counter-condition-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("fixed-condition-shape"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_fixed_condition_shape(input)),
    },
    Reading {
        id: RuleId::new("x-value-threshold"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_x_value_threshold(input)),
    },
    Reading {
        id: RuleId::new("you-cast-spells-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_you_cast_spells_this_turn(input)),
    },
    Reading {
        id: RuleId::new("colors-of-mana-spent"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_colors_of_mana_spent(input)),
    },
    Reading {
        id: RuleId::new("same-color-mana-spent"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_same_color_mana_spent(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &CounterCondition<'_>) -> ParseOutcome<RuleMatch<PredicateAst>> {
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

fn read_fixed_condition_shape(input: &CounterCondition<'_>) -> Option<PredicateAst> {
    let condition_tokens = input.tokens;
    if let Some(shape) =
        etb_grammar::parse_enters_with_counter_condition_shape_tokens(&condition_tokens)
    {
        match shape {
            EntersWithCounterConditionShape::AttackedThisTurn => {
                return Some(PredicateAst::TurnEvents(TurnEventPredicateAst::AttackedThisTurn));
            }
            EntersWithCounterConditionShape::SourceWasCast => {
                return Some(PredicateAst::Source(SourcePredicateAst::SourceWasCast));
            }
            EntersWithCounterConditionShape::ThisSpellWasKicked => {
                return Some(PredicateAst::ThisSpellWasKicked);
            }
            EntersWithCounterConditionShape::ThisSpellEscaped => {
                return Some(PredicateAst::ThisSpellEscaped);
            }
            EntersWithCounterConditionShape::CreatureDiedThisTurn => {
                return Some(PredicateAst::TurnEvents(TurnEventPredicateAst::CreatureDiedThisTurn));
            }
            EntersWithCounterConditionShape::OpponentLostLifeThisTurn => {
                return Some(PredicateAst::TurnEvents(TurnEventPredicateAst::OpponentLostLifeThisTurn));
            }
            EntersWithCounterConditionShape::PermanentLeftUnderYourControl => {
                return Some(
                    PredicateAst::TurnEvents(TurnEventPredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn {
                        surface:
                            crate::PermanentLeftBattlefieldControlSurface::LeftUnderYourControl,
                    }),
                );
            }
            EntersWithCounterConditionShape::NotCastOrNoManaSpent => {
                return Some(PredicateAst::Or(
                    Box::new(PredicateAst::Not(Box::new(PredicateAst::Source(SourcePredicateAst::SourceWasCast)))),
                    Box::new(PredicateAst::Not(Box::new(
                        PredicateAst::ManaSpentToCastThisSpellAtLeast {
                            amount: 1,
                            symbol: None,
                        },
                    ))),
                ));
            }
            EntersWithCounterConditionShape::XValueAtLeast(_)
            | EntersWithCounterConditionShape::YouCastSpellsThisTurn(_)
            | EntersWithCounterConditionShape::ColorsOfManaSpent(_) => {}
        }
    }
    None
}
fn read_x_value_threshold(input: &CounterCondition<'_>) -> Option<PredicateAst> {
    let condition_tokens = input.tokens;
    if let Some(amount) =
        parse_enters_with_counter_x_value_threshold_condition_tokens(&condition_tokens)
    {
        return Some(PredicateAst::XValueAtLeast(amount));
    }
    None
}
fn read_you_cast_spells_this_turn(input: &CounterCondition<'_>) -> Option<PredicateAst> {
    let condition_tokens = input.tokens;
    if let Some(amount) =
        parse_enters_with_counter_you_cast_spells_this_turn_condition_tokens(&condition_tokens)
    {
        return Some(PredicateAst::Player(PlayerPredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: PlayerAst::You,
            count: amount,
        }));
    }
    None
}
fn read_colors_of_mana_spent(input: &CounterCondition<'_>) -> Option<PredicateAst> {
    let condition_tokens = input.tokens;
    if let Some(amount) =
        parse_enters_with_counter_colors_mana_spent_condition_tokens(&condition_tokens)
    {
        return Some(PredicateAst::ColorsOfManaSpentToCastThisSpellOrMore(amount));
    }
    None
}
fn read_same_color_mana_spent(input: &CounterCondition<'_>) -> Option<PredicateAst> {
    let condition_tokens = input.tokens;
    if let Some(amount) =
        crate::grammar::filters::parse_same_color_mana_spent_to_cast_predicate(&condition_tokens)
    {
        return Some(PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(
            amount,
        ));
    }
    None
}
