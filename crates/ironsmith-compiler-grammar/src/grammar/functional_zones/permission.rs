use super::*;

pub(super) fn normalized_activated_zone_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    primitives::TokenWordView::new(tokens)
        .word_refs()
        .into_iter()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect()
}
