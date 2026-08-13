#[path = "semantic_lowering/hideaway.rs"]
mod hideaway;
#[path = "semantic_lowering/keyword_shapes.rs"]
mod keyword_shapes;
#[path = "semantic_lowering/special_triggered_programs.rs"]
mod special_triggered_programs;
#[path = "semantic_lowering/statement_shapes.rs"]
mod statement_shapes;
#[path = "semantic_lowering/static_shapes.rs"]
mod static_shapes;
#[path = "semantic_lowering/triggered_shapes.rs"]
mod triggered_shapes;
#[path = "semantic_lowering/villainous_choice_shapes.rs"]
mod villainous_choice_shapes;

pub(crate) use hideaway::*;
pub(crate) use keyword_shapes::*;
pub(crate) use special_triggered_programs::*;
pub(crate) use statement_shapes::*;
pub(crate) use static_shapes::*;
pub(crate) use triggered_shapes::*;
pub(crate) use villainous_choice_shapes::*;

use winnow::combinator::{eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::primitives;

fn parse_word_phrase<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    for expected_word in expected {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err(
                "semantic lowering phrase",
                "word",
            ));
        };
        if *word != *expected_word {
            return Err(primitives::backtrack_err(
                "semantic lowering phrase",
                "word",
            ));
        }
        *input = rest;
    }
    Ok(())
}

fn parse_apostrophe_insensitive_phrase<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    for expected_word in expected {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err(
                "semantic lowering phrase",
                "word",
            ));
        };
        if word.replace(['\'', '’'], "") != *expected_word {
            return Err(primitives::backtrack_err(
                "semantic lowering phrase",
                "word",
            ));
        }
        *input = rest;
    }
    Ok(())
}

fn phrase_is_prefix(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_word_phrase(&mut input, expected).is_ok()
}

fn phrase_is_exact(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        |input: &mut primitives::WordSliceInput<'_>| parse_word_phrase(input, expected),
        eof,
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}

fn phrase_is_suffix(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        (
            |input: &mut primitives::WordSliceInput<'_>| parse_word_phrase(input, expected),
            eof,
        )
            .void(),
    )
    .void()
    .parse_next(&mut input)
    .is_ok()
}

fn phrase_is_present(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(|candidate: &mut primitives::WordSliceInput<'_>| {
            parse_word_phrase(candidate, expected)
        }),
    )
    .void()
    .parse_next(&mut input)
    .is_ok()
}

fn apostrophe_insensitive_phrase_is_present(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(|candidate: &mut primitives::WordSliceInput<'_>| {
            parse_apostrophe_insensitive_phrase(candidate, expected)
        }),
    )
    .void()
    .parse_next(&mut input)
    .is_ok()
}

fn phrase_location(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped: &[&str] = repeat_till(
        0..,
        any.void(),
        peek(|candidate: &mut primitives::WordSliceInput<'_>| {
            parse_word_phrase(candidate, expected)
        }),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    Some(skipped.len())
}

fn word_is_present(words: &[&str], expected: &str) -> bool {
    phrase_is_present(words, &[expected])
}

fn any_phrase_is_present(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|expected| phrase_is_present(words, expected))
}

fn every_phrase_is_present(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .all(|expected| phrase_is_present(words, expected))
}

fn any_word_is_present(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|expected_word| word_is_present(words, expected_word))
}
