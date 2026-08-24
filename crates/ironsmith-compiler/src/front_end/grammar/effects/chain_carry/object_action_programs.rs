use super::*;

pub(super) fn contains_beginning_end_step_tokens(tokens: &[OwnedLexToken]) -> bool {
    find_semantic_phrase(tokens, &["beginning", "of", "your", "next", "end", "step"])
        .or_else(|| find_semantic_phrase(tokens, &["beginning", "of", "the", "end", "step"]))
        .or_else(|| find_semantic_phrase(tokens, &["beginning", "of", "next", "end", "step"]))
        .or_else(|| {
            find_semantic_phrase(tokens, &["beginning", "of", "the", "next", "end", "step"])
        })
        .is_some()
}

pub(super) fn contains_beginning_upkeep_tokens(tokens: &[OwnedLexToken]) -> bool {
    find_semantic_phrase(tokens, &["beginning", "of", "your", "next", "upkeep"])
        .or_else(|| find_semantic_phrase(tokens, &["beginning", "of", "next", "upkeep"]))
        .or_else(|| find_semantic_phrase(tokens, &["beginning", "of", "the", "next", "upkeep"]))
        .is_some()
}
