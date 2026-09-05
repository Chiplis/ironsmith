//! The readings of one activated ability's effect body: the typed whole-body
//! programs (a chosen color's mana, for-each-color mana, each player and
//! their creatures, hidden look partitions, a named source's leading gain, the
//! gets-and-can't-be-blocked pair, a compound pump-and-grant) and the
//! source-boundary-preserving sentence grammar. Formerly a first-match ladder
//! in `activated_permission`; every reading runs, resolved by rank while the
//! overlaps are measured. Per-sentence parsing is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct ActivatedBody<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl ActivatedBody<'_> {
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
                RuleId::new("activated-effects-registry-reading"),
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
    admits: fn(&ActivatedBody<'_>) -> bool,
    read: fn(&ActivatedBody<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("activated-effects-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("choose-color-of-matching-object-mana-effect"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_choose_color_of_matching_object_mana_effect(input)),
    },
    Reading {
        id: RuleId::new("for-each-color-among-add-mana"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_for_each_color_among_add_mana(input)),
    },
    Reading {
        id: RuleId::new("each-player-and-their-creatures-damage"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_each_player_and_their_creatures_damage(input)),
    },
    Reading {
        id: RuleId::new("hidden-look-partition-activated"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_hidden_look_partition_activated(input)),
    },
    Reading {
        id: RuleId::new("named-source-leading-gain-activated"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_named_source_leading_gain_activated(input)),
    },
    Reading {
        id: RuleId::new("source-gets-unblockable"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_gets_unblockable(input)),
    },
    Reading {
        id: RuleId::new("compound-pump-and-grant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_compound_pump_and_grant(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &ActivatedBody<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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

fn read_choose_color_of_matching_object_mana_effect(
    input: &ActivatedBody<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_choose_color_of_matching_object_mana_effect(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_for_each_color_among_add_mana(
    input: &ActivatedBody<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if activated_effect_is_for_each_color_among_add_mana_lexed(tokens) {
        return Ok(Some(vec![crate::activation_helpers::parse_add_mana(
            tokens, None,
        )?]));
    }
    Ok(None)
}
fn read_each_player_and_their_creatures_damage(
    input: &ActivatedBody<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_each_player_and_their_creatures_damage_sentence(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_hidden_look_partition_activated(
    input: &ActivatedBody<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_hidden_look_partition_activated(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_named_source_leading_gain_activated(
    input: &ActivatedBody<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_named_source_leading_gain_activated(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_source_gets_unblockable(
    input: &ActivatedBody<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Keep the P/T modification and evasion restriction as one activated
    // program. The broad restriction-oriented source-boundary parser can
    // otherwise claim only the trailing `can't be blocked` arm.
    if let Some(effects) =
        crate::effect_sentences::parse_source_gets_unblockable_subject_verb(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_compound_pump_and_grant(
    input: &ActivatedBody<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let words = token_word_refs(tokens);
    // Activated bodies such as "Until end of turn, this creature gets
    // +1/+1 ... and gains menace", or the same shape with a trailing
    // duration, are one coordinated modifier. The generic
    // source-boundary path can otherwise claim only the leading `gets`.
    if (crate::grammar::effects::gain_ability_shapes::parse_leading_gain_duration_shape(&words)
        .is_some()
        || crate::grammar::effects::gain_ability_shapes::parse_get_then_ability_shape(tokens)
            .is_some())
        && let Some(effects) = crate::effect_sentences::parse_gain_ability_sentence(tokens)?
    {
        if contains_compound_pump_and_grant(&effects) {
            return Ok(Some(effects));
        }
    }
    Ok(None)
}
pub(super) fn read_source_boundary_sentences(
    input: &ActivatedBody<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if !is_secret_choice_conditional_with_otherwise_program(tokens)
        && let Ok(effects) = parse_effect_sentences_preserving_source_boundaries(tokens)
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
