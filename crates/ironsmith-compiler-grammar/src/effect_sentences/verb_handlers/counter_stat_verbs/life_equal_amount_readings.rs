//! The readings of the amount of one "life equal to ..." clause: the typed
//! amounts (a mana amount, devotion, a counted filter, an aggregate, the
//! life-event surfaces, a dynamic cost modifier) read before the stat-of-target
//! fallback. Formerly a first-match ladder in `counter_stat_verbs`; every
//! reading runs; two different readings of one input are an ambiguity error.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct LifeAmount<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) amount_words: &'a [&'a str],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl LifeAmount<'_> {
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
                RuleId::new("life-equal-amount-registry-reading"),
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
    admits: fn(&LifeAmount<'_>) -> bool,
    read: fn(&LifeAmount<'_>) -> ParseOutcome<Value>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("life-equal-amount-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("add-mana-equal-amount-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_add_mana_equal_amount_value(input)),
    },
    Reading {
        id: RuleId::new("devotion-value-from-add"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_devotion_value_from_add(input)),
    },
    Reading {
        id: RuleId::new("equal-to-number-of-filter-value"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("add-mana-equal-amount-value")
        },
        read: |input| input.outcome(read_equal_to_number_of_filter_value(input)),
    },
    Reading {
        id: RuleId::new("equal-to-aggregate-filter-value"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_equal_to_aggregate_filter_value(input)),
    },
    Reading {
        id: RuleId::new("life-equal-surface"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_life_equal_surface(input)),
    },
    Reading {
        id: RuleId::new("dynamic-cost-modifier-value"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("add-mana-equal-amount-value")
        },
        read: |input| input.outcome(read_dynamic_cost_modifier_value(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &LifeAmount<'_>) -> ParseOutcome<RuleMatch<Value>> {
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

fn read_add_mana_equal_amount_value(
    input: &LifeAmount<'_>,
) -> Result<Option<Value>, CardTextError> {
    let amount_tokens = input.tokens;
    if let Some(value) = parse_add_mana_equal_amount_value(amount_tokens) {
        return Ok(Some(
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
        ));
    }
    Ok(None)
}
fn read_devotion_value_from_add(input: &LifeAmount<'_>) -> Result<Option<Value>, CardTextError> {
    let amount_tokens = input.tokens;
    if let Some(value) = parse_devotion_value_from_add_clause(amount_tokens)? {
        return Ok(Some(
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
        ));
    }
    Ok(None)
}
fn read_equal_to_number_of_filter_value(
    input: &LifeAmount<'_>,
) -> Result<Option<Value>, CardTextError> {
    let amount_tokens = input.tokens;
    if let Some(value) = parse_equal_to_number_of_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    Ok(None)
}
fn read_equal_to_aggregate_filter_value(
    input: &LifeAmount<'_>,
) -> Result<Option<Value>, CardTextError> {
    let amount_tokens = input.tokens;
    if let Some(value) = parse_equal_to_aggregate_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    Ok(None)
}
fn read_life_equal_surface(input: &LifeAmount<'_>) -> Result<Option<Value>, CardTextError> {
    let amount_words = input.amount_words;
    if let Some(surface) = counter_grammar::parse_life_equal_surface(&amount_words) {
        let value = match surface {
            counter_grammar::LifeEqualSurface::LifeLostThisWay => {
                Value::EventValue(EventValueSpec::LifeAmount)
            }
            counter_grammar::LifeEqualSurface::DamagePreventedThisWay => {
                Value::EventValue(EventValueSpec::Amount)
            }
            counter_grammar::LifeEqualSurface::AllPlayersLifeLostThisTurn => {
                Value::LifeLostThisTurn(PlayerFilter::Any)
            }
            counter_grammar::LifeEqualSurface::IteratedPlayerLifeLostThisTurn => {
                Value::LifeLostThisTurn(PlayerFilter::IteratedPlayer)
            }
            counter_grammar::LifeEqualSurface::TargetPlayerDamageThisTurn => {
                Value::DamageDealtToPlayersThisTurn(PlayerFilter::target_player())
            }
        };
        return Ok(Some(value));
    }
    Ok(None)
}
fn read_dynamic_cost_modifier_value(
    input: &LifeAmount<'_>,
) -> Result<Option<Value>, CardTextError> {
    let amount_tokens = input.tokens;
    if let Some(value) = parse_dynamic_cost_modifier_value(amount_tokens)? {
        return Ok(Some(value));
    }
    Ok(None)
}
