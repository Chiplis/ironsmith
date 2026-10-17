//! "When you next cast a spell this turn, exile cards from the top of your
//! library until you exile an instant or sorcery card with lesser mana value.
//! Until end of turn, you may cast that card without paying its mana cost.
//! Put each other card exiled this way on the bottom of your library in a
//! random order." (Codie, Vociferous Codex): the delayed trigger's body is a
//! multi-sentence procedure opened by the remainder of the trigger sentence.
//! Its continuation sentences complete that procedure (they name the card it
//! found and the cards it exiled), so they belong to the delayed trigger
//! rather than to the ability that creates it (CR 603.7).
use super::*;
use crate::cards::builders::DelayedEffectAst;
use crate::lexer::{OwnedLexToken, TokenKind};

fn delayed_body_tokens(sentence: &SentenceInput) -> Option<&[OwnedLexToken]> {
    let tokens = sentence.lexed();
    let words = crate::lexer::parser_token_word_refs(tokens);
    if !words.starts_with(&["when", "you", "next", "cast"]) {
        return None;
    }
    let comma = tokens.iter().position(|token| token.kind == TokenKind::Comma)?;
    let body = &tokens[comma + 1..];
    (!body.is_empty()).then_some(body)
}

/// `statements` sentences in total: the trigger sentence and its
/// procedure's `statements - 1` continuation sentences.
pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    statements: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if statements < 2 || sentences.len() < sentence_idx + statements {
        return Ok(None);
    }
    let Some(body) = delayed_body_tokens(&sentences[sentence_idx]) else {
        return Ok(None);
    };
    let mut program = vec![SentenceInput::from_lexed(body)];
    program.extend(
        sentences[sentence_idx + 1..sentence_idx + statements]
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence.lexed())),
    );
    let Some(procedure) = crate::effect_sentences::procedures::recognize(&program, 0)? else {
        return Ok(None);
    };
    if procedure.consumed_sentences != statements {
        return Ok(None);
    }
    let mut trigger_effects =
        crate::effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())?;
    let [EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, one_shot: true, .. })] =
        trigger_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    *effects = procedure.effects;
    Ok(Some(trigger_effects))
}
