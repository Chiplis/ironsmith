use winnow::combinator::{eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::primitives;

fn parse_phrase<'a>(input: &mut primitives::WordSliceInput<'a>, expected: &[&str]) -> WResult<()> {
    for expected_word in expected {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err("token phrase", "word"));
        };
        if *word != *expected_word {
            return Err(primitives::backtrack_err("token phrase", "word"));
        }
        *input = rest;
    }
    Ok(())
}

pub(super) fn phrase_present(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(|candidate: &mut primitives::WordSliceInput<'_>| parse_phrase(candidate, expected)),
    )
    .void()
    .parse_next(&mut input)
    .is_ok()
}

pub(super) fn phrase_exact(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        |candidate: &mut primitives::WordSliceInput<'_>| parse_phrase(candidate, expected),
        eof,
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}

pub(super) fn strip_phrase_prefix<'a>(
    words: &'a [&'a str],
    expected: &[&str],
) -> Option<&'a [&'a str]> {
    let mut input: primitives::WordSliceInput<'a> = words;
    parse_phrase(&mut input, expected).ok()?;
    Some(input)
}

pub(super) fn word_present(words: &[&str], expected: &str) -> bool {
    phrase_present(words, &[expected])
}

pub(super) fn all_words_present(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .all(|expected_word| word_present(words, expected_word))
}

pub(super) fn any_word_present(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|expected_word| word_present(words, expected_word))
}

pub(super) fn phrase_offset(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped: &[&str] = repeat_till(
        0..,
        any.void(),
        peek(|candidate: &mut primitives::WordSliceInput<'_>| parse_phrase(candidate, expected)),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    Some(skipped.len())
}

pub(super) fn first_word_offset(words: &[&str], expected: &str) -> Option<usize> {
    phrase_offset(words, &[expected])
}
