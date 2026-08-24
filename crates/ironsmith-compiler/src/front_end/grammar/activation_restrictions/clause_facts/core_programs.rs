use super::*;

pub(in super::super) fn exact(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_complete(words, expected).is_some()
}

pub(in super::super) fn exact_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| exact(words, expected))
}

pub(in super::super) fn prefix(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_prefix(words, expected).is_some()
}

pub(in super::super) fn prefix_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| prefix(words, expected))
}

pub(in super::super) fn prefix_remainder<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    primitives::parse_word_sequence_prefix(words, expected)
}

pub(in super::super) fn suffix(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_suffix(words, expected).is_some()
}

pub(in super::super) fn suffix_remainder<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    primitives::parse_word_sequence_suffix(words, expected)
}

pub(in super::super) fn contains(words: &[&str], expected: &[&str]) -> bool {
    primitives::parse_word_sequence_span(words, expected).is_some()
}
