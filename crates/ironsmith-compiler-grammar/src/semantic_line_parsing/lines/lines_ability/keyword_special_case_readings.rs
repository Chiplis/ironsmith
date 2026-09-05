//! The readings of one keyword line with a special lowering: hideaway,
//! partner variants and "partner with", an optional cost with a cast trigger,
//! behold and waterbend additional costs. Formerly a first-match ladder in
//! `lines_ability`; every reading runs, resolved by rank while the overlaps
//! are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct KeywordSpecialCase<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) line: &'a RewriteKeywordLine,
}

impl KeywordSpecialCase<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<LineAst>, CardTextError>) -> ParseOutcome<LineAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("keyword-special-case-registry-reading"),
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
    admits: fn(&KeywordSpecialCase<'_>) -> bool,
    read: fn(&KeywordSpecialCase<'_>) -> ParseOutcome<LineAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("keyword-special-case-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("hideaway"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_hideaway(input)),
    },
    Reading {
        id: RuleId::new("partner-variant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_partner_variant(input)),
    },
    Reading {
        id: RuleId::new("partner-with"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_partner_with(input)),
    },
    Reading {
        id: RuleId::new("optional-cost-with-cast-trigger"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_optional_cost_with_cast_trigger(input)),
    },
    Reading {
        id: RuleId::new("chosen-type-behold-two-additional-cost"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_chosen_type_behold_two_additional_cost(input)),
    },
    Reading {
        id: RuleId::new("optional-behold-additional-cost"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_optional_behold_additional_cost(input)),
    },
    Reading {
        id: RuleId::new("optional-waterbend-additional-cost"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_optional_waterbend_additional_cost(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &KeywordSpecialCase<'_>) -> ParseOutcome<RuleMatch<LineAst>> {
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
    let mut distinct: Vec<RegistryCandidate<LineAst>> = Vec::new();
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

fn read_hideaway(input: &KeywordSpecialCase<'_>) -> Result<Option<LineAst>, CardTextError> {
    let parse_tokens = input.tokens;
    if let Some(chunk) = try_lower_hideaway_keyword(parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}
fn read_partner_variant(input: &KeywordSpecialCase<'_>) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let parse_tokens = input.tokens;
    if let Some(chunk) = try_lower_partner_variant_keyword(line, parse_tokens) {
        return Ok(Some(chunk));
    }
    Ok(None)
}
fn read_partner_with(input: &KeywordSpecialCase<'_>) -> Result<Option<LineAst>, CardTextError> {
    let parse_tokens = input.tokens;
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}
fn read_optional_cost_with_cast_trigger(
    input: &KeywordSpecialCase<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let parse_tokens = input.tokens;
    if let Some(chunk) = try_parse_optional_cost_with_cast_trigger(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}
fn read_chosen_type_behold_two_additional_cost(
    input: &KeywordSpecialCase<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let parse_tokens = input.tokens;
    if let Some(chunk) = try_parse_chosen_type_behold_two_additional_cost(line, parse_tokens) {
        return Ok(Some(chunk));
    }
    Ok(None)
}
fn read_optional_behold_additional_cost(
    input: &KeywordSpecialCase<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let parse_tokens = input.tokens;
    if let Some(chunk) = try_parse_optional_behold_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}
fn read_optional_waterbend_additional_cost(
    input: &KeywordSpecialCase<'_>,
) -> Result<Option<LineAst>, CardTextError> {
    let line = input.line;
    let parse_tokens = input.tokens;
    if let Some(chunk) = try_parse_optional_waterbend_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}
