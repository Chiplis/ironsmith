//! The readings of one granted object-ability segment: a single non-static
//! keyword action, a parsed activated or triggered ability, an attached
//! non-static keyword, a cycling line, an equip line. Formerly a first-match
//! ladder in `anthem_grant_conditionals`; every reading runs, resolved by
//! rank while the overlaps are measured. The colon-activated line is the
//! fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct GrantedObjectSegment<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) raw_segment: &'a [OwnedLexToken],
    pub(super) clause_words: &'a [&'a str],
    pub(super) attached_subject: bool,
}

impl GrantedObjectSegment<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<(ParsedAbility, String)>, CardTextError>,
    ) -> ParseOutcome<(ParsedAbility, String)> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("granted-object-segment-registry-reading"),
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
    admits: fn(&GrantedObjectSegment<'_>) -> bool,
    read: fn(&GrantedObjectSegment<'_>) -> ParseOutcome<(ParsedAbility, String)>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("granted-object-segment-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("single-keyword-action"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_single_keyword_action(input)),
    },
    Reading {
        id: RuleId::new("parsed-object-ability"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_parsed_object_ability(input)),
    },
    Reading {
        id: RuleId::new("attached-nonstatic-keyword"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_attached_nonstatic_keyword(input)),
    },
    Reading {
        id: RuleId::new("cycling-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cycling_line(input)),
    },
    Reading {
        id: RuleId::new("equip-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_equip_line(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(
    input: &GrantedObjectSegment<'_>,
) -> ParseOutcome<RuleMatch<(ParsedAbility, String)>> {
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
    let mut distinct: Vec<RegistryCandidate<(ParsedAbility, String)>> = Vec::new();
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

fn read_single_keyword_action(
    input: &GrantedObjectSegment<'_>,
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let ability_tokens = input.tokens;
    if let Some(actions) = parse_ability_line(&ability_tokens)
        && actions.len() == 1
        && let Some(granted) = nonstatic_keyword_action_as_granted_object_ability(
            actions.into_iter().next().expect("single action exists"),
        )
    {
        return Ok(Some(granted));
    }
    Ok(None)
}
fn read_parsed_object_ability(
    input: &GrantedObjectSegment<'_>,
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let clause_words = input.clause_words;
    let ability_tokens = input.tokens;
    if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some((*ability, display)));
    }
    Ok(None)
}
fn read_attached_nonstatic_keyword(
    input: &GrantedObjectSegment<'_>,
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let ability_tokens = input.tokens;
    if let Some(parsed) = parse_attached_nonstatic_keyword_ability(&ability_tokens)? {
        return Ok(Some(parsed));
    }
    Ok(None)
}
fn read_cycling_line(
    input: &GrantedObjectSegment<'_>,
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let ability_tokens = input.tokens;
    if let Some(parsed) = parse_cycling_line(&ability_tokens)? {
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }
    Ok(None)
}
fn read_equip_line(
    input: &GrantedObjectSegment<'_>,
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let ability_tokens = input.tokens;
    if let Some(parsed) = parse_equip_line_lexed(&ability_tokens)? {
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }
    Ok(None)
}
