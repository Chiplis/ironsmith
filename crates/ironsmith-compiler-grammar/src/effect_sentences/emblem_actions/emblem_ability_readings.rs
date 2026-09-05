//! The readings of one emblem ability: the no-maximum-hand-size static, a
//! complete typed emblem trigger, a triggered line, an activated line, read
//! before the general static ability line. Formerly a first-match ladder in
//! `emblem_actions`; every reading runs, resolved by rank while the overlaps
//! are measured.

use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct EmblemAbility<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl EmblemAbility<'_> {
    /// A reading's outcome.
    fn outcome(&self, read: Option<EmblemAbilityAst>) -> ParseOutcome<EmblemAbilityAst> {
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
    admits: fn(&EmblemAbility<'_>) -> bool,
    read: fn(&EmblemAbility<'_>) -> ParseOutcome<EmblemAbilityAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("emblem-ability-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("no-maximum-hand-size"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_no_maximum_hand_size(input)),
    },
    Reading {
        id: RuleId::new("complete-typed-emblem-trigger"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_complete_typed_emblem_trigger(input)),
    },
    Reading {
        id: RuleId::new("triggered-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_triggered_line(input)),
    },
    Reading {
        id: RuleId::new("activated-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activated_line(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &EmblemAbility<'_>) -> ParseOutcome<RuleMatch<EmblemAbilityAst>> {
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
    let mut distinct: Vec<RegistryCandidate<EmblemAbilityAst>> = Vec::new();
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

fn read_no_maximum_hand_size(input: &EmblemAbility<'_>) -> Option<EmblemAbilityAst> {
    let tokens = input.tokens;
    if let Ok(Some(ability)) = crate::keyword_static::parse_no_maximum_hand_size_line(tokens) {
        return Some(EmblemAbilityAst::Static(vec![StaticAbilityAst::Static(
            ability,
        )]));
    }
    None
}
fn read_complete_typed_emblem_trigger(input: &EmblemAbility<'_>) -> Option<EmblemAbilityAst> {
    let tokens = input.tokens;
    if let Some(ability) = parse_complete_typed_emblem_trigger(tokens) {
        return Some(ability);
    }
    None
}
fn read_triggered_line(input: &EmblemAbility<'_>) -> Option<EmblemAbilityAst> {
    let tokens = input.tokens;
    if clause_grammar::parse_trigger_intro_tokens(tokens).body_first > 0
        && let Ok(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        }) = parse_triggered_line_lexed(tokens)
    {
        return Some(EmblemAbilityAst::Triggered {
            trigger,
            effects,
            trigger_limit_condition: trigger_surface::parse_trigger_frequency_condition_tokens(
                tokens,
                max_triggers_per_turn,
            ),
        });
    }
    None
}
fn read_activated_line(input: &EmblemAbility<'_>) -> Option<EmblemAbilityAst> {
    let tokens = input.tokens;
    if activated_lines::parse_activated_line_split_tokens(tokens).is_some()
        && let Ok(Some(ability)) = parse_activated_line(tokens)
    {
        return Some(EmblemAbilityAst::Activated(ability));
    }
    None
}
