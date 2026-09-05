//! The readings of the counted filter of one "draw a card for each ..."
//! clause: turn history, the known counts, spells cast this turn, "this way"
//! metrics, counter references, aggregates and party size, read before the
//! filter is an object count. Formerly a first-match ladder in
//! `zone_move_verbs`; every reading runs, resolved by rank while the overlaps
//! are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct CountedFilter<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl CountedFilter<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<Value>, CardTextError>) -> ParseOutcome<Value> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("draw-for-each-count-registry-reading"),
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
    admits: fn(&CountedFilter<'_>) -> bool,
    read: fn(&CountedFilter<'_>) -> ParseOutcome<Value>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("draw-for-each-count-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("turn-history-count"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_turn_history_count(input)),
    },
    Reading {
        id: RuleId::new("known-count"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("turn-history-count")
        },
        read: |input| input.outcome(read_known_count(input)),
    },
    Reading {
        id: RuleId::new("spells-cast-this-turn-matching-count"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("turn-history-count")
        },
        read: |input| input.outcome(read_spells_cast_this_turn_matching_count(input)),
    },
    Reading {
        id: RuleId::new("this-way-metric"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_this_way_metric(input)),
    },
    Reading {
        id: RuleId::new("counter-reference"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_counter_reference(input)),
    },
    Reading {
        id: RuleId::new("aggregate-scope"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("this-way-metric") && !input.read_by("turn-history-count")
        },
        read: |input| input.outcome(read_aggregate_scope(input)),
    },
    Reading {
        id: RuleId::new("party-size"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_party_size(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &CountedFilter<'_>) -> ParseOutcome<RuleMatch<Value>> {
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
    let mut distinct: Vec<RegistryCandidate<Value>> = Vec::new();
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

fn read_turn_history_count(input: &CountedFilter<'_>) -> Result<Option<Value>, CardTextError> {
    let filter_tokens = input.tokens;
    if let Some(history_value) =
        crate::grammar::shared_util::value_semantics::parse_turn_history_count_value(filter_tokens)
    {
        return Ok(Some(
            history_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }
    Ok(None)
}
fn read_known_count(input: &CountedFilter<'_>) -> Result<Option<Value>, CardTextError> {
    let filter_tokens = input.tokens;
    if let Some(known_value) = parse_draw_for_each_known_count_value(filter_tokens)? {
        return Ok(Some(
            known_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }
    Ok(None)
}
fn read_spells_cast_this_turn_matching_count(
    input: &CountedFilter<'_>,
) -> Result<Option<Value>, CardTextError> {
    let filter_tokens = input.tokens;
    if let Some(cast_this_turn_value) =
            crate::grammar::shared_util::value_semantics::parse_spells_cast_this_turn_matching_count_value_lexed(filter_tokens)
        {
            return Ok(Some(cast_this_turn_value.with_surface_hint(
                ironsmith_core::ValueSurfaceHint::ForEach,
            )));
        }
    Ok(None)
}
fn read_this_way_metric(input: &CountedFilter<'_>) -> Result<Option<Value>, CardTextError> {
    let filter_tokens = input.tokens;
    if let Some(this_way_value) = parse_draw_for_each_this_way_metric_value(filter_tokens) {
        return Ok(Some(
            this_way_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }
    Ok(None)
}
fn read_counter_reference(input: &CountedFilter<'_>) -> Result<Option<Value>, CardTextError> {
    let filter_tokens = input.tokens;
    if let Some(counter_value) = parse_draw_for_each_counter_reference_value(filter_tokens) {
        return Ok(Some(
            counter_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }
    Ok(None)
}
fn read_aggregate_scope(input: &CountedFilter<'_>) -> Result<Option<Value>, CardTextError> {
    let filter_tokens = input.tokens;
    let filter_words = crate::lexer::token_word_refs(filter_tokens);
    if let Some(aggregate_value) =
        crate::grammar::shared_util::value_helper_shapes::parse_aggregate_scope_value_words(
            &filter_words,
        )
    {
        return Ok(Some(
            aggregate_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }
    Ok(None)
}
fn read_party_size(input: &CountedFilter<'_>) -> Result<Option<Value>, CardTextError> {
    let filter_tokens = input.tokens;
    let filter_words = crate::lexer::token_word_refs(filter_tokens);
    if let Some(player) =
        crate::grammar::shared_util::value_helper_shapes::parse_party_size_player(&filter_words)
    {
        return Ok(Some(
            Value::PartySize(player).with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }
    Ok(None)
}
