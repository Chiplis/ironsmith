//! The readings of one "activate only ..." condition: the typed activation
//! conditions (once each turn, timing, graveyard, total power, damage
//! sources, the source's attack history, an "only if" predicate, ...) read
//! before the control-condition fallback. Formerly a first-match ladder in
//! `activation_conditions`; every reading runs, resolved by rank while the
//! overlaps are measured.

use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct ActivationCondition<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl ActivationCondition<'_> {
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
    admits: fn(&ActivationCondition<'_>) -> bool,
    read: fn(&ActivationCondition<'_>) -> ParseOutcome<PredicateAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("activation-condition-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("repeated-or-if-activation-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_repeated_or_if_activation_condition(input)),
    },
    Reading {
        id: RuleId::new("once-each-turn-and-if-activation-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_once_each_turn_and_if_activation_condition(input)),
    },
    Reading {
        id: RuleId::new("combined-once-and-timing-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_combined_once_and_timing_condition(input)),
    },
    Reading {
        id: RuleId::new("activate-count-each-turn-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activate_count_each_turn_condition(input)),
    },
    Reading {
        id: RuleId::new("activate-only-count-per-turn-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activate_only_count_per_turn_condition(input)),
    },
    Reading {
        id: RuleId::new("activate-only-instant-timing"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activate_only_instant_timing(input)),
    },
    Reading {
        id: RuleId::new("graveyard-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_graveyard_condition(input)),
    },
    Reading {
        id: RuleId::new("max-speed-status"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_max_speed_status(input)),
    },
    Reading {
        id: RuleId::new("total-power-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_total_power_condition(input)),
    },
    Reading {
        id: RuleId::new("sources-damage-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sources_damage_condition(input)),
    },
    Reading {
        id: RuleId::new("controlled-creature-power-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_controlled_creature_power_condition(input)),
    },
    Reading {
        id: RuleId::new("source-entered-this-turn-condition"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_entered_this_turn_condition(input)),
    },
    Reading {
        id: RuleId::new("text-only-activation-restriction"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("once-each-turn-and-if-activation-condition")
        },
        read: |input| input.outcome(read_text_only_activation_restriction(input)),
    },
    Reading {
        id: RuleId::new("activate-only-if-predicate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activate_only_if_predicate(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &ActivationCondition<'_>) -> ParseOutcome<RuleMatch<PredicateAst>> {
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

fn read_repeated_or_if_activation_condition(
    input: &ActivationCondition<'_>,
) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_repeated_or_if_activation_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_once_each_turn_and_if_activation_condition(
    input: &ActivationCondition<'_>,
) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_once_each_turn_and_if_activation_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_combined_once_and_timing_condition(
    input: &ActivationCondition<'_>,
) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_combined_once_and_timing_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_activate_count_each_turn_condition(
    input: &ActivationCondition<'_>,
) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_activate_count_each_turn_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_activate_only_count_per_turn_condition(
    input: &ActivationCondition<'_>,
) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_activate_only_count_per_turn_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_activate_only_instant_timing(input: &ActivationCondition<'_>) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if matches_any_prefix_tokens(tokens, ACTIVATE_ONLY_INSTANT_PREFIXES) {
        return Some(PredicateAst::ActivationTiming(ActivationTiming::AnyTime));
    }
    None
}
fn read_graveyard_condition(input: &ActivationCondition<'_>) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_graveyard_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_max_speed_status(input: &ActivationCondition<'_>) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(status_tokens) = tokens.get(3..)
        && let Some(condition) =
            super::super::super::conditions::parse_player_status_condition(status_tokens)
        && condition.status == super::super::super::conditions::PlayerStatusAst::MaxSpeed
    {
        return condition.condition_expr();
    }
    None
}
fn read_total_power_condition(input: &ActivationCondition<'_>) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_total_power_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_sources_damage_condition(input: &ActivationCondition<'_>) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_sources_damage_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_controlled_creature_power_condition(
    input: &ActivationCondition<'_>,
) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_controlled_creature_power_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_source_entered_this_turn_condition(
    input: &ActivationCondition<'_>,
) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) = parse_source_entered_this_turn_condition(tokens) {
        return Some(condition);
    }
    None
}
fn read_text_only_activation_restriction(input: &ActivationCondition<'_>) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition) =
            super::super::super::restriction_normalization::parse_text_only_activation_restriction_tokens(
                tokens,
            )
        {
            return Some(match condition {
                super::super::super::restriction_normalization::TextOnlyActivationRestriction::SourceDidNotAttackThisTurn => {
                    PredicateAst::Not(Box::new(PredicateAst::SourceAttackedThisTurn))
                }
                super::super::super::restriction_normalization::TextOnlyActivationRestriction::SourceAttackedThisTurn => {
                    PredicateAst::SourceAttackedThisTurn
                }
            });
        }
    None
}
fn read_activate_only_if_predicate(input: &ActivationCondition<'_>) -> Option<PredicateAst> {
    let tokens = input.tokens;
    if let Some(condition_tokens) = parse_activate_only_if_tail_tokens(tokens)
        && let Ok(predicate) = super::super::super::filters::parse_predicate(condition_tokens)
    {
        match predicate {
            crate::cards::builders::PredicateAst::SourceHasCounterAtLeast {
                counter_type,
                count,
                surface,
            } => {
                return Some(PredicateAst::SourceHasCounterAtLeast {
                    counter_type,
                    count,
                    surface,
                });
            }
            crate::cards::builders::PredicateAst::SourceMatches(filter) => {
                return Some(PredicateAst::SourceMatches(filter));
            }
            _ => {}
        }
    }
    None
}
