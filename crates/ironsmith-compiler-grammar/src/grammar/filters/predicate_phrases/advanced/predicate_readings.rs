//! The typed predicate readings of an intervening-"if" or conditional clause,
//! formerly a first-match ladder of over a hundred rungs in `advanced.rs`. Every
//! reading runs, resolved by rank while the overlaps are measured; what no
//! reading claims is an unsupported predicate.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

#[path = "predicate_readings/part_1.rs"]
mod part_1;
#[path = "predicate_readings/part_2.rs"]
mod part_2;
#[path = "predicate_readings/part_3.rs"]
mod part_3;
#[path = "predicate_readings/part_4.rs"]
mod part_4;

/// The input the readings read.
pub(super) struct Predicate<'a> {
    /// The clause as written, with its leading "if".
    pub(super) tokens: &'a [OwnedLexToken],
    /// The clause without the leading "if".
    pub(super) predicate_tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl Predicate<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .flat_map(|part| part.iter())
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
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
                RuleId::new("predicate-registry-reading"),
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
    admits: fn(&Predicate<'_>) -> bool,
    read: fn(&Predicate<'_>) -> ParseOutcome<PredicateAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("predicate-registry");

/// The readings, in the order they were ranked.
const READINGS: &[&[Reading]] = &[
    part_1::READINGS,
    part_2::READINGS,
    part_3::READINGS,
    part_4::READINGS,
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &Predicate<'_>) -> ParseOutcome<RuleMatch<PredicateAst>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS.iter().flat_map(|part| part.iter()) {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        let outcome = (reading.read)(input).within(reading.id);
        match outcome {
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
    match &outcome {
        ParseOutcome::Match(matched) => {
            crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
        }
        ParseOutcome::Error(diagnostic) => {
            crate::parse_trace::event(format!("{REGISTRY}: error: {}", diagnostic.message));
        }
        ParseOutcome::NoMatch => {}
    }
    outcome
}

/// The diagnoses that stood between the readings: a predicate no reading
/// claims fails here, as it did in the ladder.
pub(super) fn diagnose(input: &Predicate<'_>) -> Result<(), CardTextError> {
    let predicate_tokens = input.predicate_tokens;
    if !predicate_tokens.iter().any(|token| {
        token
            .as_word()
            .is_some_and(|_| !is_article(token.parser_text()))
    }) {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }
    Ok(())
}
