use super::*;

pub(super) fn starts_with_keyword(tokens: &[OwnedLexToken], keyword: &str) -> bool {
    TokenWordView::new(tokens)
        .word_refs()
        .first()
        .is_some_and(|word| permission_shapes::exact_words(&[*word], &[keyword]))
}
