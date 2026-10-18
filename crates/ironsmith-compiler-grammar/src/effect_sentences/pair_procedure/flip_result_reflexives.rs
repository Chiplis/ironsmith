//! "If you do, flip a coin. When you win the flip, copy that spell. You may
//! choose new targets for the copy. When you lose the flip, Breeches deals
//! damage equal to that spell's mana value to any target." (Breeches, the
//! Blastmaker): the flip-result reflexive triggers (CR 603.12) are results of
//! the coin flip, which itself only happens inside the "If you do" branch.
//! Read the flip and its result sentences as that branch's body, so each
//! reflexive binds to the flip rather than to the branch's antecedent, and a
//! plain sentence after a reflexive (choosing new targets for the copy)
//! belongs to that reflexive.
use super::*;
use crate::cards::builders::ConditionalEffectAst;
use crate::lexer::TokenKind;
use ironsmith_core::tag::TagKeyWalk as _;

fn is_flip_result_sentence(sentence: &SentenceInput) -> bool {
    let words = crate::lexer::token_word_refs(sentence.lowered());
    matches!(
        words.as_slice(),
        ["when", "you", "win" | "lose", "the", "flip", ..]
    )
}

fn unwrap_single(effects: &mut Vec<EffectAst>) -> &mut Vec<EffectAst> {
    let wrapped = effects.len() == 1 && matches!(effects[0], EffectAst::SourceSentence { .. });
    if !wrapped {
        return effects;
    }
    match &mut effects[0] {
        EffectAst::SourceSentence { effects: inner, .. } => inner,
        _ => unreachable!("checked above"),
    }
}

/// `statements` sentences in total: "If you do, flip a coin." and the
/// `statements - 1` flip-result sentences (with their plain followups).
pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    statements: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if statements < 3 || sentences.len() < sentence_idx + statements {
        return Ok(None);
    }
    let head = &sentences[sentence_idx];
    let head_words = crate::lexer::token_word_refs(head.lowered());
    if head_words.as_slice() != ["if", "you", "do", "flip", "a", "coin"] {
        return Ok(None);
    }
    let rest = &sentences[sentence_idx + 1..sentence_idx + statements];
    // The first completing sentence and the last one are flip results; the
    // group ends at the last flip result (a later sentence is not ours).
    if !rest.first().is_some_and(is_flip_result_sentence)
        || !rest.last().is_some_and(is_flip_result_sentence)
        || rest.iter().filter(|sentence| is_flip_result_sentence(sentence)).count() < 2
    {
        return Ok(None);
    }
    // The flip itself: the consequence of "If you do, ...".
    let Some(comma) = head.lexed().iter().position(|token| token.kind == TokenKind::Comma) else {
        return Ok(None);
    };
    let flip_tokens = &head.lexed()[comma + 1..];
    if flip_tokens.is_empty() {
        return Ok(None);
    }
    let mut body = crate::effect_sentences::parse_effect_sentence_lexed(flip_tokens)?;

    let mut flip_results = 0usize;
    for sentence in rest {
        let mut parsed = crate::effect_sentences::parse_effect_sentence_lexed(sentence.lowered())?;
        if is_flip_result_sentence(sentence) {
            let parsed_inner = unwrap_single(&mut parsed);
            if !matches!(
                parsed_inner.as_slice(),
                [EffectAst::Conditionals(ConditionalEffectAst::WhenResult { .. })]
            ) {
                return Ok(None);
            }
            // Each flip result is its own reflexive trigger: a later one's
            // "that spell" is still the triggering spell, never a copy that
            // only an earlier (exclusive) result created.
            if flip_results > 0 {
                let it = crate::tag::CompilerReferenceTag::It.as_str();
                for effect in parsed_inner.iter_mut() {
                    effect.map_tag_keys(&mut |key| {
                        if key.as_str() == it || key.as_str() == "__copied_stack_object__" {
                            *key = crate::tag::CompilerReferenceTag::Triggering.bind().into();
                        }
                    });
                }
            }
            flip_results += 1;
            body.append(parsed_inner);
            continue;
        }
        // A plain sentence after a flip result continues that reflexive.
        let Some(EffectAst::Conditionals(ConditionalEffectAst::WhenResult { effects, .. })) =
            body.last_mut()
        else {
            return Ok(None);
        };
        effects.append(unwrap_single(&mut parsed));
    }

    Ok(Some(vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: body,
    })]))
}
