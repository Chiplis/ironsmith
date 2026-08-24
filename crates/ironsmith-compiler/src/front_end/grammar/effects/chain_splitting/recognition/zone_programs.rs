use super::*;

pub(super) fn has_zone_word(tokens: &[OwnedLexToken]) -> bool {
    token_word_refs(tokens)
        .iter()
        .any(|word| is_zone_word(word))
}

pub(super) fn is_zone_word(word: &str) -> bool {
    leaf::parse_leaf_zone_complete(word).is_ok()
}
