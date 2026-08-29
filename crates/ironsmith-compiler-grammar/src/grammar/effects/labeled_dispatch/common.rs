use winnow::combinator::{eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::primitives;

fn parse_phrase<'a>(input: &mut primitives::WordSliceInput<'a>, expected: &[&str]) -> WResult<()> {
    for expected_word in expected {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err("labeled dispatch phrase", "word"));
        };
        if *word != *expected_word {
            return Err(primitives::backtrack_err("labeled dispatch phrase", "word"));
        }
        *input = rest;
    }
    Ok(())
}

pub(super) fn exact(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        |candidate: &mut primitives::WordSliceInput<'_>| parse_phrase(candidate, expected),
        eof,
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}

pub(super) fn exact_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| exact(words, expected))
}

pub(super) fn prefix(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_phrase(&mut input, expected).is_ok()
}

pub(super) fn prefix_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| prefix(words, expected))
}

pub(super) fn present(words: &[&str], expected: &[&str]) -> bool {
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

pub(super) fn present_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| present(words, expected))
}

pub(super) fn word_offset(words: &[&str], expected: &[&str]) -> Option<usize> {
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

pub(super) fn first_word_offset_any(words: &[&str], expected: &[&str]) -> Option<usize> {
    expected
        .iter()
        .filter_map(|word| word_offset(words, std::slice::from_ref(word)))
        .min()
}
