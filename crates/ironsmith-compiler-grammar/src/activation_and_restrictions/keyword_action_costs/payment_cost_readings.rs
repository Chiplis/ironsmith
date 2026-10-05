//! The readings of one payment clause as a total cost, after the "or"
//! alternative and the dynamic payment: a single graveyard-to-library payment,
//! an activation cost, a conjoined payment. Formerly a first-match ladder in
//! `keyword_action_costs`; every reading runs, resolved by rank while the
//! overlaps are measured. Payment effects are the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct PaymentClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl PaymentClause<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError>,
    ) -> ParseOutcome<ironsmith_core::TotalCost<crate::model::CompilerCost>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("payment-cost-registry-reading"),
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
    admits: fn(&PaymentClause<'_>) -> bool,
    read: fn(
        &PaymentClause<'_>,
    ) -> ParseOutcome<ironsmith_core::TotalCost<crate::model::CompilerCost>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("payment-cost-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("exile-all-owned-graveyard-payment"),
        head: HeadDiscriminator::Any,
        admits: |input| input.tokens.first().is_some_and(|token| token.is_word("exile")),
        read: |input| input.outcome(read_exile_all_owned_graveyard_payment(input)),
    },
    Reading {
        id: RuleId::new("graveyard-bottom-library-payment"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_graveyard_bottom_library_payment(input)),
    },
    Reading {
        id: RuleId::new("explicit-zero-mana-payment"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_explicit_zero_mana_payment(input)),
    },
    Reading {
        id: RuleId::new("activation-cost"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_activation_cost(input)),
    },
    Reading {
        id: RuleId::new("conjoined-payment"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conjoined_payment(input)),
    },
];

fn read_explicit_zero_mana_payment(
    input: &PaymentClause<'_>,
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let lead = parse_keyword_payment_lead_tokens(input.tokens);
    let tokens = &input.tokens[lead.payload_first..];
    let Ok(cost) = crate::grammar::leaf::parse_leaf_mana_cost_tokens(tokens) else { return Ok(None); };
    if cost.has_x() || cost.mana_value() != 0 { return Ok(None); }
    Ok(Some(ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Mana(cost))))
}

fn read_exile_all_owned_graveyard_payment(
    input: &PaymentClause<'_>,
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    if !crate::grammar::primitives::probe_all(input.tokens,
        crate::grammar::primitives::phrase(&["exile", "all", "cards", "from", "your", "graveyard"]),
        "exile-all-owned-graveyard-payment").is_some() { return Ok(None); }
    Ok(Some(ironsmith_core::TotalCost::from_cost(
        crate::model::CompilerCost::ValidatedEffect(Box::new(EffectAst::subject_verb_exile_all(
            ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(PlayerFilter::You).nontoken(), false,
        ))),
    )))
}

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(
    input: &PaymentClause<'_>,
) -> ParseOutcome<RuleMatch<ironsmith_core::TotalCost<crate::model::CompilerCost>>> {
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
    let mut distinct: Vec<
        RegistryCandidate<ironsmith_core::TotalCost<crate::model::CompilerCost>>,
    > = Vec::new();
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

fn read_graveyard_bottom_library_payment(
    input: &PaymentClause<'_>,
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let trimmed = input.tokens;
    if let Some(effect_cost) = parse_single_graveyard_bottom_library_compiler_payment(&trimmed) {
        return Ok(Some(effect_cost));
    }
    Ok(None)
}
fn read_activation_cost(
    input: &PaymentClause<'_>,
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let trimmed = input.tokens;
    // An all-set action is never a single chosen-card activation cost.
    if trimmed.first().is_some_and(|token| token.is_word("exile"))
        && trimmed.get(1).is_some_and(|token| token.is_word("all")) { return Ok(None); }
    if let Ok(total_cost) = parse_activation_cost(&trimmed)
        && !total_cost.is_free()
    {
        return Ok(Some(total_cost));
    }
    Ok(None)
}
fn read_conjoined_payment(
    input: &PaymentClause<'_>,
) -> Result<Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>, CardTextError> {
    let trimmed = input.tokens;
    if let Some(total_cost) = parse_conjoined_payment_clause_as_total_cost(&trimmed)? {
        return Ok(Some(total_cost));
    }
    Ok(None)
}
