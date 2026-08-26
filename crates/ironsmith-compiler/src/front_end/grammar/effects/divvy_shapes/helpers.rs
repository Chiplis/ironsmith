use winnow::combinator::{peek, repeat_till};
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;

fn normalized_word_eq(actual: &str, expected: &str) -> bool {
    actual
        .chars()
        .filter(|ch| *ch != '\'')
        .eq(expected.chars().filter(|ch| *ch != '\''))
}

fn dynamic_phrase<'a>(
    expected: &'a [&'a str],
) -> impl Parser<primitives::WordSliceInput<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>
+ 'a {
    move |input: &mut primitives::WordSliceInput<'a>| {
        let mut rest = *input;
        for expected_word in expected {
            let Some((actual, tail)) = rest.split_first() else {
                return Err(primitives::backtrack_err("divvy phrase", "word"));
            };
            if !normalized_word_eq(actual, expected_word) {
                return Err(primitives::backtrack_err("divvy phrase", "word"));
            }
            rest = tail;
        }
        *input = rest;
        Ok(())
    }
}

pub(super) fn exact(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (dynamic_phrase(expected), primitives::word_slice_eof)
        .parse_next(&mut input)
        .is_ok()
}

pub(super) fn prefix(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    dynamic_phrase(expected).parse_next(&mut input).is_ok()
}

pub(super) fn phrase_anywhere(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(dynamic_phrase(expected)))
        .void()
        .parse_next(&mut input)
        .is_ok()
}

pub(super) fn exact_sequence(sentence_words: &[Vec<&str>], expected: &[&[&str]]) -> bool {
    sentence_words.len() == expected.len()
        && sentence_words
            .iter()
            .zip(expected)
            .all(|(actual, expected)| exact(actual, expected))
}

pub(super) fn sequence_has_phrase(sentence_words: &[Vec<&str>], phrase: &[&str]) -> bool {
    sentence_words
        .iter()
        .any(|words| phrase_anywhere(words, phrase))
}
