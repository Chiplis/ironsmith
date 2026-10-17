//! "If each of those creatures could block all creatures that the other is
//! blocking, remove both of them from combat. Each one then blocks all
//! creatures the other was blocking." (Sorrow's Path)
//!
//! Two statements describe one combat mutation over the pair named earlier:
//! the legality check, the removal, and the re-blocking are a single
//! exchange of the attackers each creature blocks.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::OwnedLexToken;

pub(super) struct ExchangeBlocksGroup {
    effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn words(tokens: &[OwnedLexToken]) -> Vec<String> {
    crate::lexer::token_word_refs(crate::util::trim_edge_punctuation_tokens(tokens))
        .into_iter()
        .map(|word| word.replace('\'', ""))
        .collect()
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<ExchangeBlocksGroup>, CardTextError> {
    let (Some(check), Some(reblock)) =
        (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let check_tokens = check.lowered();
    let check_words = words(check_tokens);
    let expected_check = [
        "if", "each", "of", "those", "creatures", "could", "block", "all", "creatures", "that",
        "the", "other", "is", "blocking", "remove", "both", "of", "them", "from", "combat",
    ];
    if check_words != expected_check {
        return Ok(None);
    }
    let expected_reblock = [
        "each", "one", "then", "blocks", "all", "creatures", "the", "other", "was", "blocking",
    ];
    if words(reblock.lowered()) != expected_reblock {
        return Ok(None);
    }
    let Some(those) = check_tokens.iter().position(|token| token.is_word("those")) else {
        return Ok(None);
    };
    let Some(pair_tokens) = check_tokens.get(those..those + 2) else {
        return Ok(None);
    };
    let blockers = crate::util::parse_target_phrase(pair_tokens)?;
    Ok(Some(ExchangeBlocksGroup {
        effects: vec![EffectAst::ExchangeBlocks { blockers }],
        first_sentence: sentence_idx,
        consumed: 2,
    }))
}

pub(super) fn continue_with(
    _group: &mut ExchangeBlocksGroup,
    _sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    Ok(false)
}

pub(super) fn finish(group: ExchangeBlocksGroup) -> Vec<EffectAst> {
    group.effects
}
