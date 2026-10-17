//! "For each of those permanents, choose a different opponent. Each opponent
//! gains control of the permanent for which they were chosen." (Dack Fayden,
//! Helping Hand)
//!
//! One loop over the preceding group: each member gets its own player, no
//! player chosen twice (the choice accumulates across the loop), and that
//! player gains control of that member for good. The two sentences are one
//! assignment; the second never reads "each permanent" on its own.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, EffectAst, ForEachEffectAst, PlayerAst, TargetAst,
};
use crate::effect::Until;
use crate::target::PlayerFilter;

pub(super) struct DistinctPlayerAssignmentGroup {
    effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn chooser_filter(word: &str) -> Option<PlayerFilter> {
    match word {
        "opponent" => Some(PlayerFilter::Opponent),
        "player" => Some(PlayerFilter::Any),
        _ => None,
    }
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<DistinctPlayerAssignmentGroup>, CardTextError> {
    let (Some(choice), Some(control)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let choice_words = crate::lexer::token_word_refs(choice.lowered());
    let [
        "for",
        "each",
        "of",
        "those" | "them",
        rest @ ..,
    ] = choice_words.as_slice()
    else {
        return Ok(None);
    };
    let rest = match rest {
        ["permanents" | "creatures" | "cards", rest @ ..] => rest,
        rest => rest,
    };
    let ["choose", "a", "different", player_noun] = rest else {
        return Ok(None);
    };
    let Some(filter) = chooser_filter(player_noun) else {
        return Ok(None);
    };
    let control_words = crate::lexer::token_word_refs(control.lowered());
    let [
        "each",
        recipient_noun,
        "gains",
        "control",
        "of",
        "the",
        "permanent" | "creature",
        "for",
        "which",
        "they",
        "were",
        "chosen",
    ] = control_words.as_slice()
    else {
        return Ok(None);
    };
    if recipient_noun != player_noun {
        return Ok(None);
    }
    let it = || TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
    let effects = vec![EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::CompilerReferenceTag::It.bind(),
        effects: vec![
            EffectAst::subject_verb_choose_player(
                PlayerAst::You,
                filter,
                crate::tag::declared_key("chosen_player_for_object"),
                false,
                ironsmith_compiler_semantic::model::ast::DISTINCT_FROM_EVERY_PRIOR_PLAYER_CHOICE,
            ),
            EffectAst::subject_verb_gain_control(PlayerAst::That, it(), Until::Forever),
        ],
    })];
    Ok(Some(DistinctPlayerAssignmentGroup {
        effects,
        first_sentence: sentence_idx,
        consumed: 2,
    }))
}

pub(super) fn continue_with(
    _group: &mut DistinctPlayerAssignmentGroup,
    _sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    Ok(false)
}

pub(super) fn finish(group: DistinctPlayerAssignmentGroup) -> Vec<EffectAst> {
    group.effects
}
