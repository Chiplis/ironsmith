use super::*;

pub(super) fn opponent_turn_phrases() -> &'static [&'static [&'static str]] {
    &[
        &["during", "an", "opponents", "turn"],
        &["during", "an", "opponent's", "turn"],
        &["during", "an", "opponent", "s", "turn"],
        &["during", "opponents", "turn"],
        &["during", "opponent's", "turn"],
        &["during", "opponent", "s", "turn"],
        &["during", "each", "opponents", "turn"],
        &["during", "each", "opponent's", "turn"],
        &["during", "each", "opponent", "s", "turn"],
    ]
}

pub(super) fn ordinal_counts(first: u32) -> impl Iterator<Item = (&'static str, u32)> {
    [
        ("first", 1),
        ("second", 2),
        ("third", 3),
        ("fourth", 4),
        ("fifth", 5),
        ("sixth", 6),
        ("seventh", 7),
        ("eighth", 8),
        ("ninth", 9),
        ("tenth", 10),
    ]
    .into_iter()
    .filter(move |(_, count)| *count >= first)
}

pub(super) fn any_sequence_present(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|phrase| exact_phrase_occurs(words, phrase))
}

pub(super) fn all_words_present(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .all(|expected_word| exact_word_occurs(words, &[*expected_word]))
}
