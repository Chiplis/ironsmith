use winnow::combinator::{eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{OwnedLexToken, TokenWordView};
use super::super::primitives;

pub(super) fn matches_prefix_tokens(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    parse_phrase_words(&mut input, phrase).is_ok()
}

pub(super) fn matches_any_prefix_tokens(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| matches_prefix_tokens(tokens, phrase))
}

pub(super) fn matches_exact_tokens(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    (
        |input: &mut primitives::WordSliceInput<'_>| parse_phrase_words(input, phrase),
        eof,
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}

pub(super) fn matches_any_exact_tokens(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| matches_exact_tokens(tokens, phrase))
}

pub(super) fn phrase_offset_words(words: &[&str], phrase: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let skipped: &[&str] = repeat_till(
        0..,
        any.void(),
        peek(|candidate: &mut primitives::WordSliceInput<'_>| {
            parse_phrase_words(candidate, phrase)
        })
        .void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(&mut input)
    .ok()?;
    Some(skipped.len())
}

pub(super) fn parse_phrase_words<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    for expected_word in expected {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err("ability word", "phrase word"));
        };
        if *word != *expected_word {
            return Err(primitives::backtrack_err("ability word", "phrase word"));
        }
        *input = rest;
    }
    Ok(())
}

pub(super) fn take_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<&'a str> {
    any.parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn dynamic_surface_helpers_are_winnow_backed() {
        let tokens = lex_line("Activate only during combat.", 0).unwrap();
        assert!(matches_prefix_tokens(
            &tokens,
            &["activate", "only", "during", "combat"]
        ));
        assert!(matches_exact_tokens(
            &tokens,
            &["activate", "only", "during", "combat"]
        ));
        assert_eq!(
            phrase_offset_words(
                &["activate", "only", "during", "combat"],
                &["during", "combat"]
            ),
            Some(2)
        );
    }
}
