use super::*;

pub(super) fn word_has_char_suffix(word: &str, suffix: &[char]) -> bool {
    let mut chars = word.chars().rev();
    suffix
        .iter()
        .rev()
        .all(|expected| chars.next().is_some_and(|ch| ch == *expected))
}

pub(super) fn find_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::find_words(words, expected).is_some())
}

pub(super) fn has_one_of_words(words: &[&str], alternatives: &[&str]) -> bool {
    alternatives
        .iter()
        .any(|word| permission_shapes::find_words(words, &[*word]).is_some())
}

pub(super) fn first_word_offset(words: &[&str], alternatives: &[&str]) -> Option<usize> {
    alternatives
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
}

pub(super) fn prefix_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::prefix_words(words, expected))
}

pub(super) fn prefix_at_one_of(words: &[&str], offset: usize, alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::starts_at_words(words, offset, expected))
}

pub(super) fn suffix_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::suffix_words(words, expected))
}

pub(super) fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

pub(super) fn starts_with_one_of_words(
    words: &[&str],
    offset: usize,
    alternatives: &[&str],
) -> bool {
    alternatives
        .iter()
        .any(|word| permission_shapes::starts_at_words(words, offset, &[*word]))
}
