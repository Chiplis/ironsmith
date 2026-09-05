//! The readings of one ward cost: a fixed mana cost, a compact sacrifice, a
//! discarded card type, a payment clause. Formerly a first-match ladder in
//! `keyword_static`; every reading runs, resolved by rank while the overlaps
//! are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct WardCost<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl WardCost<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<StaticAbility>, CardTextError>,
    ) -> ParseOutcome<StaticAbility> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("ward-cost-registry-reading"),
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
    admits: fn(&WardCost<'_>) -> bool,
    read: fn(&WardCost<'_>) -> ParseOutcome<StaticAbility>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("ward-cost-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("fixed-mana-cost"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_fixed_mana_cost(input)),
    },
    Reading {
        id: RuleId::new("compact-sacrifice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_compact_sacrifice(input)),
    },
    Reading {
        id: RuleId::new("discard-card-type"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_discard_card_type(input)),
    },
    Reading {
        id: RuleId::new("payment-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_payment_clause(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &WardCost<'_>) -> ParseOutcome<RuleMatch<StaticAbility>> {
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
    let mut distinct: Vec<RegistryCandidate<StaticAbility>> = Vec::new();
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
    let outcome = resolve_ranked_candidates(REGISTRY, distinct, diagnostics, || {
        crate::lexer::parser_token_word_refs(input.tokens).join(" ")
    });
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_fixed_mana_cost(input: &WardCost<'_>) -> Result<Option<StaticAbility>, CardTextError> {
    let cost_tokens = input.tokens;
    if let Some(mana) = parse_leaf_fixed_mana_cost_prefix_tokens(&cost_tokens)
        && trim_edge_punctuation_tokens(&cost_tokens[mana.consumed..]).is_empty()
    {
        return Ok(Some(StaticAbility::ward(ironsmith_core::TotalCost::<
            crate::model::CompilerCost,
        >::mana(mana.cost))));
    }
    Ok(None)
}
fn read_compact_sacrifice(input: &WardCost<'_>) -> Result<Option<StaticAbility>, CardTextError> {
    let cost_tokens = input.tokens;
    if let Some(cost) = parse_compact_sacrifice_ward_cost(&cost_tokens)? {
        return Ok(Some(StaticAbility::ward(cost)));
    }
    Ok(None)
}
fn read_discard_card_type(input: &WardCost<'_>) -> Result<Option<StaticAbility>, CardTextError> {
    let cost_tokens = input.tokens;
    if let Some(cost) = parse_ward_discard_card_type_cost(&cost_tokens) {
        return Ok(Some(StaticAbility::ward(cost)));
    }
    Ok(None)
}
fn read_payment_clause(input: &WardCost<'_>) -> Result<Option<StaticAbility>, CardTextError> {
    let cost_tokens = input.tokens;
    if let Some(cost) = parse_payment_clause_as_total_cost(&cost_tokens)? {
        return Ok(Some(StaticAbility::ward(cost)));
    }
    Ok(None)
}
