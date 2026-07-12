use winnow::combinator::{eof, peek, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

pub(crate) use super::filters::{
    PermissionAtom, PermissionCaptureKind, PermissionCaptureRole, PermissionSequence,
};
use super::primitives::{self, WordSliceInput};
use crate::runtime_backend::lexer::{OwnedLexToken, TokenWordView};

pub(crate) fn exact_tokens(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: WordSliceInput<'_> = &words;
    (dynamic_sequence(expected), eof.void())
        .void()
        .parse_next(&mut input)
        .is_ok()
}

pub(crate) fn exact_tokens_any(tokens: &[OwnedLexToken], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| exact_tokens(tokens, expected))
}

pub(crate) fn prefix_tokens(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: WordSliceInput<'_> = &words;
    dynamic_sequence(expected).parse_next(&mut input).is_ok()
}

pub(crate) fn contains_tokens(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    find_words(&TokenWordView::new(tokens).word_refs(), expected).is_some()
}

pub(crate) fn contains_tokens_any(tokens: &[OwnedLexToken], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| contains_tokens(tokens, expected))
}

pub(crate) fn starts_at_words(words: &[&str], offset: usize, expected: &[&str]) -> bool {
    let Some(words) = words.get(offset..) else {
        return false;
    };
    let mut input: WordSliceInput<'_> = words;
    dynamic_sequence(expected).parse_next(&mut input).is_ok()
}

pub(crate) fn prefix_words(words: &[&str], expected: &[&str]) -> bool {
    starts_at_words(words, 0, expected)
}

pub(crate) fn exact_words(words: &[&str], expected: &[&str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    (dynamic_sequence(expected), eof.void())
        .void()
        .parse_next(&mut input)
        .is_ok()
}

pub(crate) fn exact_any_words(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| exact_words(words, expected))
}

pub(crate) fn suffix_words(words: &[&str], expected: &[&str]) -> bool {
    if expected.len() > words.len() {
        return false;
    }
    exact_words(&words[words.len() - expected.len()..], expected)
}

pub(crate) fn find_words(words: &[&str], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() {
        return None;
    }
    let mut input: WordSliceInput<'_> = words;
    let prefix =
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(dynamic_sequence(expected)))
            .map(|((), ())| ())
            .take()
            .parse_next(&mut input)
            .ok()?;
    Some(prefix.len())
}

fn dynamic_sequence<'a, 'p>(
    expected: &'p [&'p str],
) -> impl Parser<WordSliceInput<'a>, (), ErrMode<ContextError>> + 'p {
    move |input: &mut WordSliceInput<'a>| {
        for expected_word in expected {
            dynamic_word(expected_word).void().parse_next(input)?;
        }
        Ok(())
    }
}

fn dynamic_word<'a, 'p>(
    expected: &'p str,
) -> impl Parser<WordSliceInput<'a>, &'a str, ErrMode<ContextError>> + 'p {
    move |input: &mut WordSliceInput<'a>| {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err(
                "permission word",
                "expected word",
            ));
        };
        if *word != expected {
            return Err(primitives::backtrack_err(
                "permission word",
                "expected word",
            ));
        }
        *input = rest;
        Ok(*word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn permission_surfaces_are_winnow_parsed() {
        let tokens = lex_line("you may cast that spell", 0).expect("lex fixture");
        assert!(prefix_tokens(&tokens, &["you", "may", "cast"]));
        assert!(contains_tokens(&tokens, &["that", "spell"]));
        assert!(!exact_tokens(&tokens, &["you", "may", "cast"]));
    }
}
