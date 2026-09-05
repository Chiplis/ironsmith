//! The fallback readings of one effect sentence, after the prelude readings
//! and the unsupported-sentence diagnosis: a leading "player may" chain, a
//! multi-create chain, an uncounted sacrifice chain, the subject/verb
//! extension, the tap-then-untap primitives. Formerly a first-match ladder in
//! `labeled_prefixes`; every reading runs, resolved by rank while the overlaps
//! are measured. The sentence registries are the last fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct SentenceFallback<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) dispatch_shape: &'a effect_grammar::labeled_dispatch::LabeledDispatchShape<'a>,
}

impl SentenceFallback<'_> {
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
                RuleId::new("sentence-fallback-registry-reading"),
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
    admits: fn(&SentenceFallback<'_>) -> bool,
    read: fn(&SentenceFallback<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("sentence-fallback-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("leading-player-may-chain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_leading_player_may_chain(input)),
    },
    Reading {
        id: RuleId::new("multi-create-chain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_multi_create_chain(input)),
    },
    Reading {
        id: RuleId::new("uncounted-sacrifice-chain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_uncounted_sacrifice_chain(input)),
    },
    Reading {
        id: RuleId::new("subject-verb-extension"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_subject_verb_extension(input)),
    },
    Reading {
        id: RuleId::new("tap-then-untap-primitives"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tap_then_untap_primitives(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &SentenceFallback<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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

fn read_leading_player_may_chain(
    input: &SentenceFallback<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if super::super::parse_leading_player_may_lexed(tokens).is_some() {
        return parse_effect_chain_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_multi_create_chain(
    input: &SentenceFallback<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if super::super::looks_like_multi_create_chain_lexed(tokens) {
        if let Some(unless_action) = super::super::parse_or_action_clause_lexed(tokens)? {
            return Ok(Some(vec![unless_action]));
        }
        let mut effects = super::super::parse_effect_chain_inner_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_uncounted_sacrifice_chain(
    input: &SentenceFallback<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.starts_sacrifice && !dispatch_shape.sacrifice_counted {
        let mut effects = parse_effect_chain_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_subject_verb_extension(
    input: &SentenceFallback<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_tap_then_untap_primitives(
    input: &SentenceFallback<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = input.dispatch_shape;
    if dispatch_shape.tap_all_or_each_then_untap_all_or_each {
        let mut effects =
            super::super::parse_effect_chain_with_subject_verb_primitives_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
