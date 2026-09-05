//! The readings of one keyword line as a whole: a flashback line, dynamic
//! firebending, dynamic soulshift, cumulative upkeep. Formerly a first-match
//! ladder in `clause_support`; every reading runs, resolved by rank while the
//! overlaps are measured. The comma-separated keyword segments are the
//! fallback.

use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct KeywordLine<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl KeywordLine<'_> {
    /// A reading's outcome.
    fn outcome(&self, read: Option<Vec<KeywordAction>>) -> ParseOutcome<Vec<KeywordAction>> {
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
    admits: fn(&KeywordLine<'_>) -> bool,
    read: fn(&KeywordLine<'_>) -> ParseOutcome<Vec<KeywordAction>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("keyword-line-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("flashback-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_flashback_line(input)),
    },
    Reading {
        id: RuleId::new("dynamic-firebending"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_dynamic_firebending(input)),
    },
    Reading {
        id: RuleId::new("dynamic-soulshift"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_dynamic_soulshift(input)),
    },
    Reading {
        id: RuleId::new("cumulative-upkeep"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cumulative_upkeep(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &KeywordLine<'_>) -> ParseOutcome<RuleMatch<Vec<KeywordAction>>> {
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
    let mut distinct: Vec<RegistryCandidate<Vec<KeywordAction>>> = Vec::new();
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

fn read_flashback_line(input: &KeywordLine<'_>) -> Option<Vec<KeywordAction>> {
    let tokens = input.tokens;
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }
    None
}
fn read_dynamic_firebending(input: &KeywordLine<'_>) -> Option<Vec<KeywordAction>> {
    let tokens = input.tokens;
    if let Some(action) = super::super::keyword_static::parse_dynamic_firebending(tokens) {
        return Some(vec![action]);
    }
    None
}
fn read_dynamic_soulshift(input: &KeywordLine<'_>) -> Option<Vec<KeywordAction>> {
    let tokens = input.tokens;
    let words = TokenWordView::new(tokens).word_refs();
    if let Some(action) =
            super::super::activation_and_restrictions::keyword_action_costs::parse_dynamic_soulshift_keyword_action(&words)
        {
            return Some(vec![action]);
        }
    None
}
fn read_cumulative_upkeep(input: &KeywordLine<'_>) -> Option<Vec<KeywordAction>> {
    let tokens = input.tokens;
    if let Some(action @ KeywordAction::CumulativeUpkeep { .. }) = parse_ability_phrase(tokens) {
        return Some(vec![action]);
    }
    None
}
