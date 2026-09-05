//! The readings of one restriction subject ("<subject> can't ..."): a
//! distributive compound subject, a type-adjective conjunction, or a plain
//! object filter, read before the subject is a target phrase. Formerly a
//! first-match ladder in `activation_restriction_clauses`; every reading
//! runs, resolved by rank while the overlaps are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct RestrictionSubject<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl RestrictionSubject<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<ObjectFilter>, CardTextError>,
    ) -> ParseOutcome<ObjectFilter> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("restriction-subject-registry-reading"),
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
    admits: fn(&RestrictionSubject<'_>) -> bool,
    read: fn(&RestrictionSubject<'_>) -> ParseOutcome<ObjectFilter>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("restriction-subject-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("distributive-compound-subject"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_distributive_compound_subject(input)),
    },
    Reading {
        id: RuleId::new("type-adjective-conjunction"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_type_adjective_conjunction(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &RestrictionSubject<'_>) -> ParseOutcome<RuleMatch<ObjectFilter>> {
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

fn read_distributive_compound_subject(
    input: &RestrictionSubject<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    if let Some(filter) = parse_distributive_compound_subject_filter(tokens)? {
        return Ok(Some(filter));
    }
    Ok(None)
}
fn read_type_adjective_conjunction(
    input: &RestrictionSubject<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    if let Some(filter) = parse_type_adjective_conjunction_filter(tokens)? {
        return Ok(Some(filter));
    }
    Ok(None)
}
pub(super) fn read_object_filter(
    input: &RestrictionSubject<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = input.tokens;
    if let Ok(mut filter) = parse_object_filter(tokens, false)
        && filter != ObjectFilter::default()
    {
        if crate::grammar::filters::reference_tag_stage::has_plural_object_head_surface(tokens) {
            filter.set_plural_object_noun_surface(true);
        }
        return Ok(Some(filter));
    }
    Ok(None)
}
