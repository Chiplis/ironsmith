//! The readings of one phase or step gate predicate ("if you created a token
//! this turn", control, attachment, empty-battlefield, source-state, zone
//! history, player-counter and world-status gates). Formerly a first-match
//! ladder in `phase_step_gates`; every reading runs, resolved by rank while the
//! overlaps are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct Gate<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl Gate<'_> {
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
                RuleId::new("phase-step-gate-registry-reading"),
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
    admits: fn(&Gate<'_>) -> bool,
    read: fn(&Gate<'_>) -> ParseOutcome<PredicateAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("phase-step-gate-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("existing-value-gate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_existing_value_gate(input)),
    },
    Reading {
        id: RuleId::new("control-gate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_control_gate(input)),
    },
    Reading {
        id: RuleId::new("attachment-gate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_attachment_gate(input)),
    },
    Reading {
        id: RuleId::new("empty-battlefield-gate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_empty_battlefield_gate(input)),
    },
    Reading {
        id: RuleId::new("source-state-gate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_state_gate(input)),
    },
    Reading {
        id: RuleId::new("existing-zone-history-gate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_existing_zone_history_gate(input)),
    },
    Reading {
        id: RuleId::new("player-counter-gate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_player_counter_gate(input)),
    },
    Reading {
        id: RuleId::new("world-status-gate"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_world_status_gate(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &Gate<'_>) -> ParseOutcome<RuleMatch<PredicateAst>> {
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

fn read_existing_value_gate(input: &Gate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_existing_value_gate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
fn read_control_gate(input: &Gate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_control_gate(tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
fn read_attachment_gate(input: &Gate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_attachment_gate(tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
fn read_empty_battlefield_gate(input: &Gate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_empty_battlefield_gate(tokens)? {
        return Ok(Some(predicate));
    }
    Ok(None)
}
fn read_source_state_gate(input: &Gate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_source_state_gate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
fn read_existing_zone_history_gate(
    input: &Gate<'_>,
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_existing_zone_history_gate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
fn read_player_counter_gate(input: &Gate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_player_counter_gate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
fn read_world_status_gate(input: &Gate<'_>) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(predicate) = parse_world_status_gate(tokens) {
        return Ok(Some(predicate));
    }
    Ok(None)
}
