//! "[you may have] defending player play with their hand revealed for as long
//! as this creature remains on the battlefield" (Stromgald Spy): a resolved,
//! player-scoped reveal of that player's hand lasting while the source stays
//! on the battlefield.
use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let Some(verb) = words
        .iter()
        .position(|word| matches!(*word, "play" | "plays"))
    else {
        return Ok(None);
    };
    let player = match &words[..verb] {
        ["defending", "player"] | ["the", "defending", "player"] => PlayerFilter::Defending,
        ["each", "opponent"] | ["your", "opponents"] | ["each", "of", "your", "opponents"] => {
            PlayerFilter::Opponent
        }
        ["each", "player"] | ["all", "players"] | ["players"] => PlayerFilter::Any,
        ["you"] => PlayerFilter::You,
        _ => return Ok(None),
    };
    let rest = &words[verb + 1..];
    let Some(after_reveal) = (match rest {
        ["with", "their" | "his" | "her" | "your", "hand" | "hands", "revealed", tail @ ..] => {
            Some(tail)
        }
        ["with", "his", "or", "her", "hand", "revealed", tail @ ..] => Some(tail),
        _ => None,
    }) else {
        return Ok(None);
    };
    // The only lifetime this reveal supports is its source's stay on the
    // battlefield; any other duration is left to the grammar that owns it.
    let ["for", "as", "long", "as", source @ .., "remains", "on", "the", "battlefield"] =
        after_reveal
    else {
        return Ok(None);
    };
    if !(crate::util::is_source_reference_words(source) || *source == ["it"]) {
        return Ok(None);
    }
    Ok(Some(EffectAst::PlayWithHandRevealed(
        ironsmith_core::PlayWithHandRevealedEffect::new(player),
    )))
}
