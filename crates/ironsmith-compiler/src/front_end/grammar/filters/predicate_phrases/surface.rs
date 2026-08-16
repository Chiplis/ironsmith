use winnow::combinator::{eof, peek, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::super::lexer::{LexedClause, OwnedLexToken, TokenWordView};
use super::super::super::primitives::{self, WordSliceInput};

pub(super) fn exact(clause: LexedClause<'_>, expected: &[&str]) -> bool {
    exact_tokens(clause.tokens(), expected)
}

pub(super) fn exact_any(clause: LexedClause<'_>, alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| exact(clause, expected))
}

pub(super) fn prefix(clause: LexedClause<'_>, expected: &[&str]) -> bool {
    prefix_tokens(clause.tokens(), expected)
}

pub(super) fn prefix_any(clause: LexedClause<'_>, alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| prefix(clause, expected))
}

pub(super) fn contains(clause: LexedClause<'_>, expected: &[&str]) -> bool {
    find(clause, expected).is_some()
}

pub(super) fn find(clause: LexedClause<'_>, expected: &[&str]) -> Option<usize> {
    let view = TokenWordView::new(clause.tokens());
    find_words(&view.word_refs(), expected)
}

pub(super) fn exact_tokens(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let view = TokenWordView::new(tokens);
    exact_words(&view.word_refs(), expected)
}

pub(super) fn prefix_tokens(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let view = TokenWordView::new(tokens);
    prefix_words(&view.word_refs(), expected)
}

pub(super) fn exact_words(words: &[&str], expected: &[&str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    (dynamic_sequence(expected), eof.void())
        .void()
        .parse_next(&mut input)
        .is_ok()
}

pub(super) fn prefix_words(words: &[&str], expected: &[&str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    dynamic_sequence(expected).parse_next(&mut input).is_ok()
}

pub(super) fn find_words(words: &[&str], expected: &[&str]) -> Option<usize> {
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
        for word in expected {
            dynamic_word(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn dynamic_word<'a, 'p>(
    expected: &'p str,
) -> impl Parser<WordSliceInput<'a>, &'a str, ErrMode<ContextError>> + 'p {
    move |input: &mut WordSliceInput<'a>| {
        let Some((word, rest)) = input.split_first() else {
            return Err(primitives::backtrack_err("predicate word", "expected word"));
        };
        if *word != expected {
            return Err(primitives::backtrack_err("predicate word", "expected word"));
        }
        *input = rest;
        Ok(*word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn recognizes_exact_prefix_and_infix_surfaces_with_winnow() {
        let tokens = lex_line("this creature is tapped", 0).expect("lex fixture");
        let clause = LexedClause::new(&tokens);
        assert!(exact(clause, &["this", "creature", "is", "tapped"]));
        assert!(prefix(clause, &["this", "creature"]));
        assert_eq!(find(clause, &["is", "tapped"]), Some(2));
    }
}
