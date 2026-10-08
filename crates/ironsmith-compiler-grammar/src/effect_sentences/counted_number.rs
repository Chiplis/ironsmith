//! "Count the number of <things>. <instruction about that number>."
//! (Rumbling Ruin, Chaos Moon). Counting is not an action of its own: the
//! count is taken as the ability resolves and the later sentences read it
//! (CR 608.2h). Each reference ("that number", "the number") is replaced by
//! the counted quantity itself; a restriction that stores the quantity in a
//! filter freezes it when it resolves (effects/restrictions.rs), and a
//! parity check reads it during the same resolution.
use crate::cards::builders::{CardTextError, EffectAst, OwnedLexToken};
use crate::lexer::parser_token_word_refs;

/// Index of a "that number"/"the number" reference not followed by "of".
fn reference_index(sentence: &[OwnedLexToken]) -> Option<usize> {
    (0..sentence.len().saturating_sub(1)).find(|&index| {
        sentence[index].is_any_word(&["that", "the"])
            && sentence[index + 1].is_word("number")
            && !sentence.get(index + 2).is_some_and(|token| token.is_word("of"))
    })
}

fn counted_quantity(sentence: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let sentence = crate::util::trim_edge_punctuation_tokens(sentence);
    let words = parser_token_word_refs(sentence);
    if !words.starts_with(&["count", "the", "number", "of"]) || words.len() < 5 {
        return None;
    }
    sentence.get(1..)
}

/// `sentences[0]` counts; every later sentence must reference the count, and
/// `next` (the sentence after the group, if any) must not.
pub(crate) fn read(
    sentences: &[&[OwnedLexToken]],
    next: Option<&[OwnedLexToken]>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((count, rest)) = sentences.split_first() else {
        return Ok(None);
    };
    let Some(quantity) = counted_quantity(count) else {
        return Ok(None);
    };
    if rest.is_empty() || next.is_some_and(|sentence| reference_index(sentence).is_some()) {
        return Ok(None);
    }
    let mut effects = Vec::new();
    for sentence in rest {
        let Some(index) = reference_index(sentence) else {
            return Ok(None);
        };
        let mut substituted = sentence[..index].to_vec();
        substituted.extend_from_slice(quantity);
        substituted.extend_from_slice(&sentence[index + 2..]);
        effects.extend(super::parse_effect_sentence_lexed(&substituted)?);
    }
    Ok(Some(effects))
}
