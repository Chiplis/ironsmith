//! The readings of the filter of one looked-card reveal ("reveal a
//! noncreature, nonland permanent card"): the typed compound filters read
//! before the generic disjunction and object filter. Formerly a first-match
//! ladder in `looked_card_shapes`; every reading runs, resolved by rank while
//! the overlaps are measured.

use super::*;
use crate::recognition::{ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct LookedCardFilter<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) words: &'a [&'a str],
    pub(super) same_name: bool,
}

impl LookedCardFilter<'_> {
    /// A reading's outcome.
    fn outcome(&self, read: Option<ObjectFilter>) -> ParseOutcome<ObjectFilter> {
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
    admits: fn(&LookedCardFilter<'_>) -> bool,
    read: fn(&LookedCardFilter<'_>) -> ParseOutcome<ObjectFilter>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("looked-card-filter-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("noncreature-nonland-permanent"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_noncreature_nonland_permanent(input)),
    },
    Reading {
        id: RuleId::new("conjunctive-negated-card"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conjunctive_negated_card(input)),
    },
    Reading {
        id: RuleId::new("land-or-legendary-permanent"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_land_or_legendary_permanent(input)),
    },
    Reading {
        id: RuleId::new("modified-permanent-cards"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_modified_permanent_cards(input)),
    },
    Reading {
        id: RuleId::new("filter-disjunction"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_filter_disjunction(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &LookedCardFilter<'_>) -> ParseOutcome<RuleMatch<ObjectFilter>> {
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
    let mut distinct: Vec<RegistryCandidate<ObjectFilter>> = Vec::new();
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

fn read_noncreature_nonland_permanent(input: &LookedCardFilter<'_>) -> Option<ObjectFilter> {
    let words = input.words;
    let same_name = input.same_name;
    let filter_tokens = input.tokens;
    if let Some(filter) = parse_noncreature_nonland_permanent(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    None
}
fn read_conjunctive_negated_card(input: &LookedCardFilter<'_>) -> Option<ObjectFilter> {
    let words = input.words;
    let same_name = input.same_name;
    let filter_tokens = input.tokens;
    if let Some(filter) = parse_conjunctive_negated_card_filter(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    None
}
fn read_land_or_legendary_permanent(input: &LookedCardFilter<'_>) -> Option<ObjectFilter> {
    let words = input.words;
    let same_name = input.same_name;
    let filter_tokens = input.tokens;
    if let Some(filter) = parse_land_or_legendary_permanent(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    None
}
fn read_modified_permanent_cards(input: &LookedCardFilter<'_>) -> Option<ObjectFilter> {
    let words = input.words;
    let same_name = input.same_name;
    let filter_tokens = input.tokens;
    if let Some(filter) = parse_modified_permanent_cards(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    None
}
fn read_filter_disjunction(input: &LookedCardFilter<'_>) -> Option<ObjectFilter> {
    let words = input.words;
    let same_name = input.same_name;
    let filter_tokens = input.tokens;
    if let Some(filter) = parse_filter_disjunction(filter_tokens, &words) {
        return Some(apply_same_name(filter, same_name));
    }
    None
}
