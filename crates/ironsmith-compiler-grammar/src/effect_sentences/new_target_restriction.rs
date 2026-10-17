//! "Change the target of target spell that targets only a player. The new
//! target must be a player." (Rebound): the trailing sentence restricts the
//! retarget instruction of the preceding sentence (CR 115.7).

use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;

const NEW_TARGET_PREFIX: &[&str] = &["the", "new", "target", "must", "be"];

/// "The new targets can't be <X>." (Chef's Kiss): an exclusion that applies
/// to every retarget instruction of the preceding sentences.
const NEW_TARGETS_EXCLUSION_PREFIXES: &[&[&str]] = &[
    &["the", "new", "targets", "can't", "be"],
    &["the", "new", "targets", "cant", "be"],
    &["the", "new", "targets", "cannot", "be"],
    &["the", "new", "target", "can't", "be"],
    &["the", "new", "target", "cant", "be"],
    &["the", "new", "target", "cannot", "be"],
];

/// Split off a trailing "The new target must be <X>." sentence.
pub(super) fn split_new_target_restriction(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_trailing_sentence_with_prefixes(tokens, &[NEW_TARGET_PREFIX])
}

/// Split off a trailing "The new targets can't be <X>." sentence.
pub(super) fn split_new_targets_exclusion(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_trailing_sentence_with_prefixes(tokens, NEW_TARGETS_EXCLUSION_PREFIXES)
}

fn split_trailing_sentence_with_prefixes<'a>(
    tokens: &'a [OwnedLexToken],
    prefixes: &[&[&str]],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let mut sentence_start = 0;
    for (idx, token) in tokens.iter().enumerate() {
        if idx > 0 && tokens[idx - 1].is_period() {
            sentence_start = idx;
        }
        if idx != sentence_start || sentence_start == 0 || !token.is_word("the") {
            continue;
        }
        let words = crate::lexer::parser_token_word_refs(&tokens[idx..]);
        let Some(prefix) = prefixes
            .iter()
            .find(|prefix| crate::word_primitives::parse_sequence_prefix(&words, prefix))
        else {
            continue;
        };
        let end = tokens[idx..]
            .iter()
            .position(OwnedLexToken::is_period)
            .map_or(tokens.len(), |offset| idx + offset);
        // Only a final sentence: anything after it would change the order.
        if tokens[end..].iter().any(|token| !token.is_period()) {
            return None;
        }
        let body_start = crate::lexer::TokenWordView::new(&tokens[idx..end])
            .token_start_indices()
            .get(prefix.len())
            .map(|offset| idx + offset)?;
        return Some((&tokens[..idx], &tokens[body_start..end]));
    }
    None
}

/// "can't be you or a permanent you control": players other than the
/// excluded player and objects other than the excluded controller's.
pub(super) fn parse_new_targets_exclusion(
    tokens: &[OwnedLexToken],
) -> Result<ironsmith_core::NewTargetRestriction, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let unsupported = || {
        CardTextError::ParseError(format!(
            "unsupported new-target exclusion (clause: '{}')",
            words.join(" ")
        ))
    };
    let (excludes_you, object_words): (bool, &[&str]) = match words.as_slice() {
        ["you", "or", rest @ ..] => (true, rest),
        ["you"] => (true, &words[1..]),
        rest => (false, rest),
    };
    let player = if excludes_you { PlayerFilter::NotYou } else { PlayerFilter::Any };
    let object = if object_words.is_empty() {
        None
    } else {
        let start = words.len() - object_words.len();
        let object_start = crate::lexer::TokenWordView::new(tokens)
            .token_start_indices()
            .get(start)
            .copied()
            .ok_or_else(unsupported)?;
        let mut filter =
            crate::object_filters::parse_object_filter_lexed(&tokens[object_start..], false)?;
        // Only "<objects> you control" negates exactly: the remaining
        // objects of that kind are those another player controls.
        if filter.controller != Some(PlayerFilter::You) {
            return Err(unsupported());
        }
        filter.controller = Some(PlayerFilter::NotYou);
        Some(filter)
    };
    match object {
        Some(object) => Ok(ironsmith_core::NewTargetRestriction::PlayerOrObject { player, object }),
        None if !excludes_you => Err(unsupported()),
        None => Ok(ironsmith_core::NewTargetRestriction::Player(player)),
    }
}

/// Attach an exclusion to every retarget instruction (including those in a
/// nested sequence).
pub(super) fn attach_new_targets_exclusion(
    effects: &mut [EffectAst],
    restriction: ironsmith_core::NewTargetRestriction,
) -> Result<(), CardTextError> {
    fn walk(effects: &mut [EffectAst], restriction: &ironsmith_core::NewTargetRestriction) -> usize {
        let mut attached = 0;
        for effect in effects.iter_mut() {
            if effect.set_retarget_new_target_restriction(restriction.clone()) {
                attached += 1;
            } else if let EffectAst::Sequence { effects }
            | EffectAst::CommaThen { effects }
            | EffectAst::SourceSentence { effects, .. } = effect
            {
                attached += walk(effects, restriction);
            } else if let EffectAst::Coordination(coordination) = effect {
                for member in &mut coordination.members {
                    attached += walk(&mut member.effects, restriction);
                }
            }
        }
        attached
    }
    if walk(effects, &restriction) == 0 {
        return Err(CardTextError::ParseError(
            "'The new targets can't be ...' has no preceding retarget instruction".to_string(),
        ));
    }
    Ok(())
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
