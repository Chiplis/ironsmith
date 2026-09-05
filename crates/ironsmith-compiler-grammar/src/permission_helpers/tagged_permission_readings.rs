//! The readings of one tagged cast-or-play permission clause before the
//! mana-spend suffix is split: "until this exiles another", casting a target
//! from your graveyard this turn, a revealed top-of-library permission, "for
//! as long as ... look at". Formerly a first-match ladder in
//! `permission_helpers`; every reading runs, resolved by rank while the
//! overlaps are measured.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_ranked_candidates,
};

/// The input the readings read.
pub(super) struct TaggedPermission<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
}

impl TaggedPermission<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("tagged-permission-registry-reading"),
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
    admits: fn(&TaggedPermission<'_>) -> bool,
    read: fn(&TaggedPermission<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("tagged-permission-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("until-source-exiles-another"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_until_source_exiles_another(input)),
    },
    Reading {
        id: RuleId::new("cast-target-from-graveyard-this-turn"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_cast_target_from_graveyard_this_turn(input)),
    },
    Reading {
        id: RuleId::new("revealed-top-library-permission"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_revealed_top_library_permission(input)),
    },
    Reading {
        id: RuleId::new("for-as-long-as-look-at-tagged"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_for_as_long_as_look_at_tagged(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &TaggedPermission<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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
    let outcome = resolve_ranked_candidates(REGISTRY, distinct, diagnostics, || {
        crate::lexer::parser_token_word_refs(input.tokens).join(" ")
    });
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_until_source_exiles_another(
    input: &TaggedPermission<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let trimmed = input.tokens;
    if let Some(effect) = parse_until_source_exiles_another_permission(&trimmed) {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_cast_target_from_graveyard_this_turn(
    input: &TaggedPermission<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let trimmed = input.tokens;
    if let Some(shape) = super::super::grammar::effects::clause_dispatch_shapes::parse_cast_target_from_your_graveyard_this_turn_shape(&trimmed)
        {
            let target = parse_target_phrase(shape.target_tokens)?;
            return Ok(Some(EffectAst::Sequence {
                effects: vec![
                    EffectAst::subject_verb_target_only(target),
                    EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        PlayerAst::You,
                        false,
                        false,
                        false,
                    ),
                ],
            }));
        }
    Ok(None)
}
fn read_revealed_top_library_permission(
    input: &TaggedPermission<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let trimmed = input.tokens;
    if let Some(effect) = parse_revealed_top_library_permission_clause(&trimmed)? {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_for_as_long_as_look_at_tagged(
    input: &TaggedPermission<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let trimmed = input.tokens;
    if let Some(permission_tokens) = strip_for_as_long_as_look_at_tagged_prefix_tokens(&trimmed)
        && let Some(permission) = parse_cast_or_play_tagged_clause(&permission_tokens)?
    {
        let mut look_filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
        look_filter.zone = Some(Zone::Exile);
        return Ok(Some(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_look_at_objects(PlayerAst::You, look_filter),
                permission,
            ],
        }));
    }
    Ok(None)
}
