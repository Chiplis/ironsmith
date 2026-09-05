//! The readings of one granted segment of a continuing anthem ("... and have
//! <ability>"): a parsed activated or triggered ability, a keyword line, a
//! static text marker, a ward life payment, or a static ability line. Formerly
//! a first-match ladder in `anthem_grant_conditionals`; every reading runs;
//! two different readings of one input are an ambiguity error.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct GrantedSegment<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) clause: &'a ParsedAnthemClause,
    pub(super) clause_words: &'a [&'a str],
}

impl GrantedSegment<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<Vec<StaticAbilityAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<StaticAbilityAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("anthem-granted-segment-registry-reading"),
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
    admits: fn(&GrantedSegment<'_>) -> bool,
    read: fn(&GrantedSegment<'_>) -> ParseOutcome<Vec<StaticAbilityAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("anthem-granted-segment-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("parsed-object-ability"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_parsed_object_ability(input)),
    },
    Reading {
        id: RuleId::new("keyword-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_keyword_line(input)),
    },
    Reading {
        id: RuleId::new("static-text-marker"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_static_text_marker(input)),
    },
    Reading {
        id: RuleId::new("ward-pay-life"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_ward_pay_life(input)),
    },
    Reading {
        id: RuleId::new("static-text-marker-with-period"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_static_text_marker_with_period(input)),
    },
    Reading {
        id: RuleId::new("static-ability-line"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_static_ability_line(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &GrantedSegment<'_>) -> ParseOutcome<RuleMatch<Vec<StaticAbilityAst>>> {
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
    let mut distinct: Vec<RegistryCandidate<Vec<StaticAbilityAst>>> = Vec::new();
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

fn read_parsed_object_ability(
    input: &GrantedSegment<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause = input.clause;
    let clause_words = input.clause_words;
    let ability_tokens = input.tokens;
    if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![grant_object_ability_for_anthem_subject(
            clause, *ability, display,
        )]));
    }
    Ok(None)
}
fn read_keyword_line(
    input: &GrantedSegment<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause = input.clause;
    let clause_words = input.clause_words;
    let ability_tokens = input.tokens;
    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        let granted = actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect::<Vec<_>>();
        if granted.is_empty() {
            return Ok(None);
        }
        return Ok(Some(
            granted
                .into_iter()
                .map(|ability| grant_for_anthem_subject(clause, ability))
                .collect(),
        ));
    }
    Ok(None)
}
fn read_static_text_marker(
    input: &GrantedSegment<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause = input.clause;
    let ability_tokens = input.tokens;
    if let Some(marker) = parse_static_text_marker_line(&ability_tokens) {
        return Ok(Some(vec![grant_for_anthem_subject(clause, marker)]));
    }
    Ok(None)
}
fn read_ward_pay_life(
    input: &GrantedSegment<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause = input.clause;
    let ability_tokens = input.tokens;
    let mut ability_tokens_with_period = ability_tokens.to_vec();
    ability_tokens_with_period.push(OwnedLexToken::period(
        crate::cards::builders::TextSpan::synthetic(),
    ));
    if let Some(amount) = super::super::grammar::abilities::parse_ward_pay_life_amount_lexed(
        &ability_tokens_with_period,
    ) {
        return Ok(Some(vec![grant_for_anthem_subject(
            clause,
            StaticAbility::ward(ironsmith_core::TotalCost::from_cost(
                crate::model::CompilerCost::Life(Value::Fixed(amount as i32)),
            )),
        )]));
    }
    Ok(None)
}
fn read_static_text_marker_with_period(
    input: &GrantedSegment<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause = input.clause;
    let ability_tokens = input.tokens;
    let mut ability_tokens_with_period = ability_tokens.to_vec();
    ability_tokens_with_period.push(OwnedLexToken::period(
        crate::cards::builders::TextSpan::synthetic(),
    ));
    if let Some(marker) = parse_static_text_marker_line(&ability_tokens_with_period) {
        return Ok(Some(vec![grant_for_anthem_subject(clause, marker)]));
    }
    Ok(None)
}
fn read_static_ability_line(
    input: &GrantedSegment<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause = input.clause;
    let ability_tokens = input.tokens;
    if let Some(abilities) = parse_static_ability_ast_line_lexed(&ability_tokens)? {
        return Ok(Some(grant_static_anthem_abilities_for_subject(
            clause, abilities,
        )));
    }
    Ok(None)
}
