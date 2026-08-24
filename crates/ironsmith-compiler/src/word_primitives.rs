use winnow::Parser;
use winnow::error::{ContextError, ErrMode};

type WordInput<'a> = &'a [&'a str];

fn dynamic_sequence<'a, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut WordInput<'a>| {
        for expected_word in expected {
            let Some((word, rest)) = input.split_first() else {
                return Err(ErrMode::Backtrack(ContextError::new()));
            };
            if word != expected_word {
                return Err(ErrMode::Backtrack(ContextError::new()));
            }
            *input = rest;
        }
        Ok(())
    }
}

fn dynamic_choice_sequence<'a, 'p>(
    expected: &'p [&'p [&'p str]],
) -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut WordInput<'a>| {
        for accepted_words in expected {
            let Some((word, rest)) = input.split_first() else {
                return Err(ErrMode::Backtrack(ContextError::new()));
            };
            if !accepted_words.iter().any(|accepted| accepted == word) {
                return Err(ErrMode::Backtrack(ContextError::new()));
            }
            *input = rest;
        }
        Ok(())
    }
}

pub fn parse_sequence_start(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() {
        return None;
    }
    for start in 0..=words.len().saturating_sub(expected.len()) {
        let mut input = &words[start..];
        if dynamic_sequence(expected).parse_next(&mut input).is_ok() {
            return Some(start);
        }
    }
    None
}

pub fn parse_last_sequence_start(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() {
        return None;
    }
    let mut start = words.len().checked_sub(expected.len())?;
    loop {
        let mut input = &words[start..];
        if dynamic_sequence(expected).parse_next(&mut input).is_ok() {
            return Some(start);
        }
        if start == 0 {
            return None;
        }
        start -= 1;
    }
}

pub fn parse_sequence_complete(words: &[&str], expected: &[&str]) -> bool {
    if words.len() != expected.len() {
        return false;
    }
    let mut input = words;
    dynamic_sequence(expected).parse_next(&mut input).is_ok() && input.is_empty()
}

/// Parse a complete word sequence whose slots each declare their accepted
/// lexical alternatives. This keeps inflection choices inside the Winnow
/// leaf layer instead of spelling them as slice-pattern matches in domain
/// parsers.
pub fn parse_choice_sequence_complete(words: &[&str], expected: &[&[&str]]) -> bool {
    if words.len() != expected.len() {
        return false;
    }
    let mut input = words;
    dynamic_choice_sequence(expected)
        .parse_next(&mut input)
        .is_ok()
        && input.is_empty()
}

pub fn parse_choice_sequence_prefix(words: &[&str], expected: &[&[&str]]) -> bool {
    let mut input = words;
    dynamic_choice_sequence(expected)
        .parse_next(&mut input)
        .is_ok()
}

pub fn parse_choice_sequence_suffix(words: &[&str], expected: &[&[&str]]) -> bool {
    let Some(start) = words.len().checked_sub(expected.len()) else {
        return false;
    };
    parse_choice_sequence_complete(&words[start..], expected)
}

/// Recognize a lexical prefix on one normalized word. Domain parsers use
/// this for productive hyphenated forms while keeping raw string probing in
/// the shared Winnow leaf layer.
pub fn parse_word_prefix(word: &str, expected: &str) -> bool {
    let mut input = word;
    let parsed: Result<&str, ErrMode<ContextError>> =
        winnow::token::literal(expected).parse_next(&mut input);
    parsed.is_ok()
}

/// Strip an exact lexical suffix through the shared Winnow leaf layer.
pub fn strip_word_suffix<'a>(word: &'a str, expected: &str) -> Option<&'a str> {
    let prefix_len = word.len().checked_sub(expected.len())?;
    let mut input = word;
    let mut parser = (
        winnow::token::take(prefix_len),
        winnow::token::literal(expected),
    );
    let parsed: Result<(&str, &str), ErrMode<ContextError>> = parser.parse_next(&mut input);
    let (prefix, _) = parsed.ok()?;
    input.is_empty().then_some(prefix)
}

/// Parse a compact inclusive numeric range such as `2-5` through the shared
/// Winnow leaf layer. Domain parsers should not split normalized token text
/// themselves after lexing.
pub fn parse_ascii_numeric_range(word: &str) -> Option<(i32, i32)> {
    let mut input = word;
    let mut parser = (
        winnow::token::take_while(1.., |ch: char| ch.is_ascii_digit()),
        winnow::token::literal("-"),
        winnow::token::take_while(1.., |ch: char| ch.is_ascii_digit()),
    );
    let parsed: Result<(&str, &str, &str), ErrMode<ContextError>> = parser.parse_next(&mut input);
    let (min, _, max) = parsed.ok()?;
    if !input.is_empty() {
        return None;
    }
    Some((min.parse().ok()?, max.parse().ok()?))
}

pub fn parse_sequence_prefix(words: &[&str], expected: &[&str]) -> bool {
    let mut input = words;
    dynamic_sequence(expected).parse_next(&mut input).is_ok()
}

pub fn parse_sequence_suffix(words: &[&str], expected: &[&str]) -> bool {
    let Some(start) = words.len().checked_sub(expected.len()) else {
        return false;
    };
    parse_sequence_complete(&words[start..], expected)
}

pub fn parse_any_sequence_complete(words: &[&str], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|sequence| parse_sequence_complete(words, sequence))
}

pub fn parse_any_sequence_prefix(words: &[&str], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|sequence| parse_sequence_prefix(words, sequence))
}

pub fn parse_any_sequence_suffix(words: &[&str], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|sequence| parse_sequence_suffix(words, sequence))
}

pub fn find_phrase_start_or_zero(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() {
        Some(0)
    } else {
        parse_sequence_start(words, expected)
    }
}

pub fn find_any_phrase_start<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    expected
        .iter()
        .filter_map(|phrase| parse_sequence_start(words, phrase).map(|idx| (*phrase, idx)))
        .min_by_key(|(_, idx)| *idx)
}

pub fn find_any_phrase_span<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize, usize)> {
    find_any_phrase_start(words, expected).map(|(phrase, start)| (phrase, start, phrase.len()))
}

pub fn find_any_phrase_start_or_zero<'p>(
    words: &[&str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], usize)> {
    expected
        .iter()
        .filter_map(|phrase| find_phrase_start_or_zero(words, phrase).map(|idx| (*phrase, idx)))
        .min_by_key(|(_, idx)| *idx)
}

pub fn find_phrase_value<T: Clone>(
    words: &[&str],
    expected: &[(&[&str], T)],
) -> Option<(T, usize)> {
    expected
        .iter()
        .filter_map(|(phrase, value)| parse_sequence_start(words, phrase).map(|idx| (value, idx)))
        .min_by_key(|(_, idx)| *idx)
        .map(|(value, idx)| (value.clone(), idx))
}

pub fn find_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> Option<usize> {
    crate::slice_primitives::find_window_by(words, window_len, predicate)
}

pub fn contains_window_by(
    words: &[&str],
    window_len: usize,
    predicate: impl FnMut(&[&str]) -> bool,
) -> bool {
    find_window_by(words, window_len, predicate).is_some()
}

pub fn sequence_occurs(words: &[&str], expected: &[&str]) -> bool {
    parse_sequence_start(words, expected).is_some()
}

pub fn sequence_or_empty_occurs(words: &[&str], expected: &[&str]) -> bool {
    expected.is_empty() || sequence_occurs(words, expected)
}

pub fn any_sequence_occurs(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| sequence_occurs(words, phrase))
}

pub fn contains_any_phrase_or_empty(words: &[&str], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| sequence_or_empty_occurs(words, phrase))
}

pub fn equals(words: &[&str], expected: &[&str]) -> bool {
    words == expected
}

pub fn equals_any(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| equals(words, phrase))
}

pub fn equals_at(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words.get(idx..).is_some_and(|tail| equals(tail, expected))
}

pub fn equals_any_at(words: &[&str], idx: usize, expected: &[&[&str]]) -> bool {
    words
        .get(idx..)
        .is_some_and(|tail| equals_any(tail, expected))
}

pub fn at_is(words: &[&str], idx: usize, expected: &str) -> bool {
    words.get(idx).is_some_and(|word| *word == expected)
}

pub fn at_is_any(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words.get(idx).is_some_and(|word| expected.contains(word))
}

pub fn first_is(words: &[&str], expected: &str) -> bool {
    at_is(words, 0, expected)
}

pub fn first_is_any(words: &[&str], expected: &[&str]) -> bool {
    at_is_any(words, 0, expected)
}

pub fn last_is(words: &[&str], expected: &str) -> bool {
    words.last().is_some_and(|word| *word == expected)
}

pub fn last_is_any(words: &[&str], expected: &[&str]) -> bool {
    words.last().is_some_and(|word| expected.contains(word))
}

pub fn matching_phrase<'p>(words: &[&str], expected: &'p [&'p [&'p str]]) -> Option<&'p [&'p str]> {
    expected
        .iter()
        .copied()
        .find(|phrase| equals(words, phrase))
}

pub fn matching_value<T: Clone>(words: &[&str], expected: &[(&[&str], T)]) -> Option<T> {
    crate::slice_primitives::matching_value(words, expected)
}

pub fn ends_with(words: &[&str], expected: &[&str]) -> bool {
    words.len() >= expected.len() && words[words.len() - expected.len()..] == *expected
}

pub fn ends_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| ends_with(words, phrase))
}

pub fn starts_with(words: &[&str], expected: &[&str]) -> bool {
    words.len() >= expected.len() && words[..expected.len()] == *expected
}

pub fn starts_with_at(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words
        .get(idx..)
        .is_some_and(|tail| starts_with(tail, expected))
}

pub fn starts_with_any(words: &[&str], expected: &[&[&str]]) -> bool {
    expected.iter().any(|phrase| starts_with(words, phrase))
}

pub fn strip_prefix<'a>(words: &'a [&'a str], expected: &[&str]) -> Option<&'a [&'a str]> {
    starts_with(words, expected).then(|| &words[expected.len()..])
}

pub fn strip_suffix<'a>(words: &'a [&'a str], expected: &[&str]) -> Option<&'a [&'a str]> {
    ends_with(words, expected).then(|| &words[..words.len().saturating_sub(expected.len())])
}

pub fn strip_any_prefix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    expected
        .iter()
        .find_map(|phrase| strip_prefix(words, phrase).map(|tail| (*phrase, tail)))
}

pub fn strip_prefix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    expected.iter().find_map(|(phrase, value)| {
        starts_with(words, phrase).then(|| (value.clone(), &words[phrase.len()..]))
    })
}

pub fn strip_first_word<'w, 'a>(words: &'w [&'a str], expected: &str) -> Option<&'w [&'a str]> {
    words
        .first()
        .is_some_and(|word| *word == expected)
        .then(|| &words[1..])
}

pub fn strip_first_word_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&str, T)],
) -> Option<(T, &'w [&'a str])> {
    let first = words.first()?;
    expected
        .iter()
        .find_map(|(word, value)| (*first == *word).then(|| (value.clone(), &words[1..])))
}

pub fn strip_any_suffix<'a, 'p>(
    words: &'a [&'a str],
    expected: &'p [&'p [&'p str]],
) -> Option<(&'p [&'p str], &'a [&'a str])> {
    expected
        .iter()
        .find_map(|phrase| strip_suffix(words, phrase).map(|head| (*phrase, head)))
}

pub fn strip_suffix_value<'w, 'a, T: Clone>(
    words: &'w [&'a str],
    expected: &[(&[&str], T)],
) -> Option<(T, &'w [&'a str])> {
    expected.iter().find_map(|(phrase, value)| {
        ends_with(words, phrase).then(|| (value.clone(), &words[..words.len() - phrase.len()]))
    })
}

pub fn contains_word(words: &[&str], expected: &str) -> bool {
    words.contains(&expected)
}

pub fn find_word(words: &[&str], expected: &str) -> Option<usize> {
    select_word_position(words, |word| word == expected)
}

pub fn find_any_word(words: &[&str], expected: &[&str]) -> Option<usize> {
    select_word_position(words, |word| expected.contains(&word))
}

pub fn select_word_position(
    words: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    for (position, word) in words.iter().enumerate() {
        if predicate(word) {
            return Some(position);
        }
    }
    None
}

pub fn select_last_word_position(
    words: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    for (position, word) in words.iter().enumerate().rev() {
        if predicate(word) {
            return Some(position);
        }
    }
    None
}

pub fn contains_any_word(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().any(|word| contains_word(words, word))
}

pub fn contains_no_words(words: &[&str], expected: &[&str]) -> bool {
    !contains_any_word(words, expected)
}

pub fn contains_all_words(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().all(|word| contains_word(words, word))
}
