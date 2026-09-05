//! The readings of one "tap ..." clause: tap or untap all, a first target
//! plus an opponent's chosen second target, the chosen object set, a
//! quantified filter, tap-then-return, tap or untap a target. Formerly a
//! first-match ladder in `tap_actions`; every reading runs, resolved by rank
//! while the overlaps are measured. The single target phrase is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct TapClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl TapClause<'_> {
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
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("tap-clause-registry-reading"),
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
    admits: fn(&TapClause<'_>) -> bool,
    read: fn(&TapClause<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("tap-clause-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("tap-or-untap-all"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tap_or_untap_all(input)),
    },
    Reading {
        id: RuleId::new("opponent-chosen-second-target"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_opponent_chosen_second_target(input)),
    },
    Reading {
        id: RuleId::new("chosen-object-set"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_chosen_object_set(input)),
    },
    Reading {
        id: RuleId::new("quantified-filter"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("tap-or-untap-all")
        },
        read: |input| input.outcome(read_quantified_filter(input)),
    },
    Reading {
        id: RuleId::new("tap-then-return"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tap_then_return(input)),
    },
    Reading {
        id: RuleId::new("tap-or-untap-target"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tap_or_untap_target(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &TapClause<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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
    let mut distinct: Vec<RegistryCandidate<EffectAst>> = Vec::new();
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

fn read_tap_or_untap_all(input: &TapClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_tap_or_untap_all(tokens)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_opponent_chosen_second_target(
    input: &TapClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    for (index, token) in tokens.iter().enumerate() {
        if token.as_word() != Some("and") {
            continue;
        }
        let Some(choice) =
            crate::grammar::choices::parse_possessive_object_choice_tokens(&tokens[index + 1..])
        else {
            continue;
        };
        if choice.actor != crate::grammar::choices::PossessiveObjectChoiceActor::Opponent {
            continue;
        }
        let first_tokens = trim_commas(&tokens[..index]);
        let first = parse_target_phrase(&first_tokens)?;
        let second = parse_target_phrase(&choice.object_tokens)?;
        let target_tag =
            crate::util::helper_tag_for_tokens(&tokens[index + 1..], "opponent_chosen_target");
        return Ok(Some(EffectAst::Coordinated {
            effects: vec![
                EffectAst::subject_verb_tap(first),
                EffectAst::Sequence {
                    effects: vec![
                        EffectAst::TagAffected {
                            effect: Box::new(
                                EffectAst::subject_verb_explicit_target_only_for_chooser(
                                    second,
                                    PlayerAst::Opponent,
                                ),
                            ),
                            tag: target_tag.clone(),
                        },
                        EffectAst::subject_verb_tap(TargetAst::Tagged(target_tag, None)),
                    ],
                },
            ],
            leading_duration: false,
            result_conjunction: false,
        }));
    }
    Ok(None)
}
fn read_chosen_object_set(input: &TapClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(filter_tokens) = parse_chosen_object_set_filter_tokens(tokens) {
        let mut filter = parse_object_filter(filter_tokens, false)?;
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(Some(EffectAst::subject_verb_tap_all(filter)));
    }
    Ok(None)
}
fn read_quantified_filter(input: &TapClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(filter_tokens) = parse_tap_quantified_filter_tokens(tokens) {
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(Some(EffectAst::subject_verb_tap_all(filter)));
    }
    Ok(None)
}
fn read_tap_then_return(input: &TapClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) = parse_tap_then_return_tokens(tokens) {
        let tap_tokens = trim_commas(shape.tap_tokens);
        let return_tokens = trim_commas(shape.return_tokens);
        if !tap_tokens.is_empty() && !return_tokens.is_empty() {
            let target = parse_target_phrase(&tap_tokens)?;
            let return_effect = parse_return(&return_tokens)?;
            return Ok(Some(EffectAst::Sequence {
                effects: vec![EffectAst::subject_verb_tap(target), return_effect],
            }));
        }
    }
    Ok(None)
}
fn read_tap_or_untap_target(input: &TapClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    // Handle "tap or untap <target>" as a choice between tapping and untapping.
    if let Some(target_tokens) = parse_tap_or_untap_target_tokens(tokens) {
        let target = parse_target_phrase(target_tokens)?;
        return Ok(Some(EffectAst::subject_verb_tap_or_untap(target.clone())));
    }
    Ok(None)
}
