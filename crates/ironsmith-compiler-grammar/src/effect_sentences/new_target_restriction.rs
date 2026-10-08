//! "Change the target of target spell that targets only a player. The new
//! target must be a player." (Rebound): the trailing sentence restricts the
//! retarget instruction of the preceding sentence (CR 115.7).

use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;

const NEW_TARGET_PREFIX: &[&str] = &["the", "new", "target", "must", "be"];

/// Split off a trailing "The new target must be <X>." sentence.
pub(super) fn split_new_target_restriction(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let mut sentence_start = 0;
    for (idx, token) in tokens.iter().enumerate() {
        if idx > 0 && tokens[idx - 1].is_period() {
            sentence_start = idx;
        }
        if idx == sentence_start
            && sentence_start > 0
            && token.is_word("the")
            && crate::word_primitives::parse_sequence_prefix(
                &crate::lexer::parser_token_word_refs(&tokens[idx..]),
                NEW_TARGET_PREFIX,
            )
        {
            let end = tokens[idx..]
                .iter()
                .position(OwnedLexToken::is_period)
                .map_or(tokens.len(), |offset| idx + offset);
            // Only a final sentence: anything after it would change the order.
            if tokens[end..].iter().any(|token| !token.is_period()) {
                return None;
            }
            return Some((&tokens[..idx], &tokens[idx + NEW_TARGET_PREFIX.len()..end]));
        }
    }
    None
}

pub(super) fn parse_new_target_restriction(
    tokens: &[OwnedLexToken],
) -> Result<ironsmith_core::NewTargetRestriction, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if crate::word_primitives::parse_any_sequence_complete(&words, &[&["a", "player"], &["player"]])
    {
        return Ok(ironsmith_core::NewTargetRestriction::Player(PlayerFilter::Any));
    }
    Ok(ironsmith_core::NewTargetRestriction::Object(
        crate::object_filters::parse_object_filter_lexed(tokens, false)?,
    ))
}

/// Attach the restriction to the last retarget instruction.
pub(super) fn attach_new_target_restriction(
    effects: &mut [EffectAst],
    restriction: ironsmith_core::NewTargetRestriction,
) -> Result<(), CardTextError> {
    for effect in effects.iter_mut().rev() {
        if effect.set_retarget_new_target_restriction(restriction.clone()) {
            return Ok(());
        }
    }
    Err(CardTextError::ParseError(
        "'The new target must be ...' has no preceding retarget instruction".to_string(),
    ))
}
