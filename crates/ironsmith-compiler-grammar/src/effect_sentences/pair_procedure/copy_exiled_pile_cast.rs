//! "Then that player copies each instant or sorcery card exiled with this
//! enchantment. For each copy, the player may cast the copy without paying
//! its mana cost." (Eye of the Storm): every card in the source's exile pile
//! is copied (CR 707.12: a copy of a card is created in the card's zone) and
//! the player may cast each copy for free; the cards themselves stay exiled.
use super::*;
use crate::cards::builders::{ForEachEffectAst, PermissionEffectAst};

/// The copying player and the word index where the copied-card filter starts.
fn copier(all_words: &[&str]) -> Option<(PlayerAst, usize)> {
    let words = all_words.strip_prefix(&["then"][..]).unwrap_or(all_words);
    let (player, rest) = match words {
        ["that" | "the", "player", "copies", "each", rest @ ..] => (PlayerAst::That, rest),
        ["you", "copy", "each", rest @ ..] => (PlayerAst::You, rest),
        _ => return None,
    };
    (!rest.is_empty()).then(|| (player, all_words.len() - rest.len()))
}

fn cast_each_copy(sentence: &SentenceInput, copier: PlayerAst) -> bool {
    let words = crate::lexer::token_word_refs(sentence.lowered())
        .into_iter()
        .filter(|word| !matches!(*word, "," | "."))
        .collect::<Vec<_>>();
    let Some(rest) = words.strip_prefix(&["for", "each", "copy"][..]) else {
        return false;
    };
    let rest = match (copier, rest) {
        (PlayerAst::That, ["the" | "that", "player", rest @ ..])
        | (PlayerAst::That, ["they", rest @ ..])
        | (PlayerAst::You, ["you", rest @ ..]) => rest,
        _ => return false,
    };
    matches!(
        rest,
        ["may", "cast", "the" | "that", "copy", "without", "paying", "its", "mana", "cost"]
    )
}

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else {
        return Ok(None);
    };
    let tokens = crate::util::trim_edge_punctuation_tokens(first.lowered());
    let words = crate::lexer::token_word_refs(tokens);
    if words.len() != tokens.len() {
        // Punctuation inside the sentence: not this shape.
        return Ok(None);
    }
    let Some((player, filter_start)) = copier(&words) else {
        return Ok(None);
    };
    if !cast_each_copy(second, player) {
        return Ok(None);
    }
    let filter_words = &words[filter_start..];
    // Only a pile of cards exiled with the source qualifies.
    if !crate::word_primitives::sequence_occurs(filter_words, &["exiled", "with"]) {
        return Ok(None);
    }
    let Ok(mut filter) =
        crate::object_filters::parse_object_filter(&tokens[filter_start..], false)
    else {
        return Ok(None);
    };
    filter.zone = Some(Zone::Exile);
    Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachObject {
        filter,
        effects: vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player,
            effects: vec![EffectAst::subject_verb_cast_tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
                player,
                false,
                true,
                true,
                None,
            )],
        })],
    })]))
}
