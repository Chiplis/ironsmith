//! A graveyard-cast permission and the exile replacement on its spell.
//!
//! "You may cast target instant or sorcery card from your graveyard this turn.
//! If that spell would be put into a graveyard, exile it instead." The first
//! sentence permits casting a targeted card; the second replaces where the
//! spell it becomes would go. Each is recognized on its own sentence, and the
//! replacement binds to the spell the permission tagged.

use super::dispatch_entry::SentenceInput;
use super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::graveyard_cast_with_exile_replacement;
use crate::cards::builders::{IfResultPredicate, CardTextError, EffectAst, ConditionalEffectAst};
use crate::grammar::effects::{self as effect_grammar, GraveyardCastReplacementShape};
use crate::lexer::OwnedLexToken;

pub(super) struct GraveyardCastGroup {
    permission: Vec<OwnedLexToken>,
    shape: GraveyardCastReplacementShape,
    replaced: bool,
    /// The permission sat under \"When you do,\": the pair is that result's effect.
    when_result: bool,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

/// Open at a graveyard-cast permission when the exile replacement follows it.
pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<GraveyardCastGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    // "When you do, you may cast target instant or sorcery card ...": the
    // permission under a reflexive result prefix, the whole pair its effect.
    let (permission, when_result) = match crate::grammar::structure::split_leading_result_prefix_lexed(sentence.lexed()) {
        Some(prefix)
            if prefix.kind == crate::grammar::structure::LeadingResultPrefixKind::When
                && prefix.predicate == IfResultPredicate::Did
                && crate::word_primitives::parse_sequence_prefix(
                    &crate::lexer::token_word_refs(prefix.trailing_tokens),
                    &["you", "may", "cast", "target"],
                ) =>
        {
            (
                crate::util::trim_commas(
                    SentenceInput::from_lexed(prefix.trailing_tokens).lowered(),
                ),
                true,
            )
        }
        _ => (crate::util::trim_commas(sentence.lowered()), false),
    };
    let Some(shape) = effect_grammar::parse_graveyard_cast_permission_shape(&permission) else {
        return Ok(None);
    };
    if !effect_grammar::is_graveyard_cast_replacement_sentence(&crate::util::trim_commas(
        next.lowered(),
    )) {
        return Ok(None);
    }
    // The permission must read as a targeted graveyard spell card for the
    // pair to be this procedure; the same check the program made.
    if graveyard_cast_with_exile_replacement(&permission, &shape)?.is_none() {
        return Ok(None);
    }
    Ok(Some(GraveyardCastGroup {
        permission,
        shape,
        when_result,
        replaced: false,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

/// The replacement statement, once.
pub(super) fn continue_with(
    group: &mut GraveyardCastGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.replaced {
        return Ok(false);
    }
    if !effect_grammar::is_graveyard_cast_replacement_sentence(&crate::util::trim_commas(
        sentence.lowered(),
    )) {
        return Ok(false);
    }
    group.replaced = true;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: GraveyardCastGroup) -> Vec<EffectAst> {
    let effects = graveyard_cast_with_exile_replacement(&group.permission, &group.shape)
        .ok()
        .flatten()
        .unwrap_or_default();
    if group.when_result {
        vec![EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
            predicate: IfResultPredicate::Did,
            effects,
        })]
    } else {
        effects
    }
}
