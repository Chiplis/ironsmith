pub(crate) fn find_phrase_start(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() || words.len() < expected.len() {
        return None;
    }
    words
        .windows(expected.len())
        .position(|window| window == expected)
}

pub(crate) fn find_phrase_start_or_zero(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() {
        Some(0)
    } else {
        find_phrase_start(words, expected)
    }
}

pub(crate) fn find_any_phrase_start<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    expected
        .iter()
        .filter_map(|phrase| find_phrase_start(words, phrase).map(|idx| (*phrase, idx)))
        .min_by_key(|(_, idx)| *idx)
}

pub(crate) fn find_any_phrase_start_or_zero<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    expected
        .iter()
        .filter_map(|phrase| find_phrase_start_or_zero(words, phrase).map(|idx| (*phrase, idx)))
        .min_by_key(|(_, idx)| *idx)
}

pub(crate) fn find_phrase_value<T: Clone>(
    words: &[&str],
    expected: &[(&[&str], T)],
) -> Option<(T, usize)> {
    expected
        .iter()
        .filter_map(|(phrase, value)| find_phrase_start(words, phrase).map(|idx| (value, idx)))
        .min_by_key(|(_, idx)| *idx)
        .map(|(value, idx)| (value.clone(), idx))
}

pub(crate) fn find_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> Option<usize> {
    crate::slice_primitives::find_window_by(words, window_len, predicate)
}

pub(crate) fn contains_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> bool {
    find_window_by(words, window_len, predicate).is_some()
}

pub(crate) fn contains_phrase(words: &[&str], expected: &[&str]) -> bool {
    find_phrase_start(words, expected).is_some()
}

pub(crate) fn contains_phrase_or_empty(words: &[&str], expected: &[&str]) -> bool {
    expected.is_empty() || contains_phrase(words, expected)
}

pub(crate) fn contains_any_phrase(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| contains_phrase(words, phrase))
}

pub(crate) fn contains_any_phrase_or_empty(words: &[&str], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| contains_phrase_or_empty(words, phrase))
}

pub(crate) fn equals(words: &[&str], expected: &[&str]) -> bool {
    words == expected
}

pub(crate) fn equals_any(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| equals(words, phrase))
}

pub(crate) fn matching_phrase<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<&'p [&'p str]> {
    expected
        .iter()
        .copied()
        .find(|phrase| equals(words, phrase))
}

pub(crate) fn matching_value<T: Clone>(words: &[&str], expected: &[(&[&str], T)]) -> Option<T> {
    crate::slice_primitives::matching_value(words, expected)
}

pub(crate) fn ends_with(words: &[&str], expected: &[&str]) -> bool {
    words.len() >= expected.len() && words[words.len() - expected.len()..] == *expected
}

pub(crate) fn ends_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| ends_with(words, phrase))
}

pub(crate) fn starts_with(words: &[&str], expected: &[&str]) -> bool {
    words.len() >= expected.len() && words[..expected.len()] == *expected
}

pub(crate) fn starts_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| starts_with(words, phrase))
}

pub(crate) fn strip_prefix<'a>(words: &'a [&'a str], expected: &[&str]) -> Option<&'a [&'a str]> {
    starts_with(words, expected).then(|| &words[expected.len()..])
}

pub(crate) fn strip_suffix<'a>(words: &'a [&'a str], expected: &[&str]) -> Option<&'a [&'a str]> {
    ends_with(words, expected).then(|| &words[..words.len().saturating_sub(expected.len())])
}

pub(crate) fn strip_any_prefix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    expected
        .iter()
        .find_map(|phrase| strip_prefix(words, phrase).map(|tail| (*phrase, tail)))
}

pub(crate) fn strip_prefix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    expected.iter().find_map(|(phrase, value)| {
        starts_with(words, phrase).then(|| (value.clone(), &words[phrase.len()..]))
    })
}

pub(crate) fn strip_first_word<'w, 'a>(
    words: &'w [&'a str],
    expected: &str,
) -> Option<&'w [&'a str]> {
    words
        .first()
        .is_some_and(|word| *word == expected)
        .then(|| &words[1..])
}

pub(crate) fn strip_first_word_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&str, T)],
) -> Option<(T, &'w [&'a str])> {
    let first = words.first()?;
    expected
        .iter()
        .find_map(|(word, value)| (*first == *word).then(|| (value.clone(), &words[1..])))
}

pub(crate) fn strip_any_suffix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    expected
        .iter()
        .find_map(|phrase| strip_suffix(words, phrase).map(|head| (*phrase, head)))
}

pub(crate) fn strip_suffix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    expected.iter().find_map(|(phrase, value)| {
        ends_with(words, phrase).then(|| (value.clone(), &words[..words.len() - phrase.len()]))
    })
}

pub(crate) fn contains_word(words: &[&str], expected: &str) -> bool {
    words.iter().any(|word| *word == expected)
}

pub(crate) fn find_word(words: &[&str], expected: &str) -> Option<usize> {
    find_word_where(words, |word| word == expected)
}

pub(crate) fn find_any_word(words: &[&str], expected: &[&str]) -> Option<usize> {
    find_word_where(words, |word| {
        expected.iter().any(|expected_word| word == *expected_word)
    })
}

pub(crate) fn find_word_where(
    words: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    words.iter().position(|word| predicate(word))
}

pub(crate) fn rfind_word_where(
    words: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    words.iter().rposition(|word| predicate(word))
}

pub(crate) fn contains_any_word(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().any(|word| contains_word(words, word))
}

pub(crate) fn contains_all_words(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().all(|word| contains_word(words, word))
}
