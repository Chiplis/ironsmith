//! The readings of one put-counter sequence sentence: a placement sequence,
//! a choice sequence, a shared target, a counter placement with a follow-up,
//! a counter pair. Formerly a first-match ladder in `counter_marker_family`;
//! every reading runs, resolved by rank while the overlaps are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct PutCounterSequence<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) clause: SubjectVerbPrimitiveClause<'a>,
}

impl PutCounterSequence<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<EffectAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("put-counter-sequence-registry-reading"),
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
    admits: fn(&PutCounterSequence<'_>) -> bool,
    read: fn(&PutCounterSequence<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("put-counter-sequence-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("counter-placement-sequence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_counter_placement_sequence(input)),
    },
    Reading {
        id: RuleId::new("put-counter-choice-sequence"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_put_counter_choice_sequence(input)),
    },
    Reading {
        id: RuleId::new("shared-counter-target"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_shared_counter_target(input)),
    },
    Reading {
        id: RuleId::new("counter-followup"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_counter_followup(input)),
    },
    Reading {
        id: RuleId::new("counter-pair"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_counter_pair(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &PutCounterSequence<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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
    let mut distinct: Vec<RegistryCandidate<Vec<EffectAst>>> = Vec::new();
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

fn read_counter_placement_sequence(
    input: &PutCounterSequence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = input.clause;
    if let Some(placements) =
        counter_shapes::parse_counter_placement_sequence_tokens(clause.tokens())
    {
        return lower_counter_placements(placements).map(Some);
    }
    Ok(None)
}
fn read_put_counter_choice_sequence(
    input: &PutCounterSequence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = input.clause;
    if let Some(effects) = parse_put_counter_choice_sequence(clause)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_shared_counter_target(
    input: &PutCounterSequence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = input.clause;
    if let Some(shape) = counter_shapes::parse_shared_counter_target_tokens(clause.tokens()) {
        let target = parse_target_phrase(shape.target_tokens)?;
        let effects = shape
            .descriptors
            .into_iter()
            .map(|descriptor| {
                EffectAst::subject_verb_put_counters(
                    descriptor.counter_type,
                    Value::Fixed(descriptor.count as i32),
                    target.clone(),
                    None,
                    false,
                )
            })
            .collect();
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_counter_followup(
    input: &PutCounterSequence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = input.clause;
    if let Some(shape) = counter_shapes::parse_counter_followup_tokens(clause.tokens())
        && let Ok(first) = parse_put_counters(shape.counter_tokens)
        && let Ok(mut followup_effects) = parse_effect_chain(shape.followup_tokens)
        && !followup_effects.is_empty()
    {
        let source_target = match &first {
            effect if subject_verb_put_counters_target(effect).is_some() => {
                subject_verb_put_counters_target(effect)
            }
            EffectAst::Conditional { if_true, .. } if if_true.len() == 1 => {
                if_true.first().and_then(subject_verb_put_counters_target)
            }
            _ => None,
        };

        if let Some(source_target) = source_target {
            for effect in &mut followup_effects {
                retarget_it_effect_for_counter_followup(effect, &source_target);
            }

            let mut effects = vec![first];
            effects.append(&mut followup_effects);
            return Ok(Some(vec![EffectAst::Coordinated {
                effects,
                leading_duration: false,
                result_conjunction: false,
            }]));
        }
    }
    Ok(None)
}
fn read_counter_pair(
    input: &PutCounterSequence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = input.clause;
    if let Some(shape) = counter_shapes::parse_counter_pair_tokens(clause.tokens())
        && let (Ok(first), Ok(second)) = (
            parse_put_counters(shape.first_tokens),
            parse_put_counters(shape.second_tokens),
        )
    {
        return Ok(Some(vec![first, second]));
    }
    Ok(None)
}
