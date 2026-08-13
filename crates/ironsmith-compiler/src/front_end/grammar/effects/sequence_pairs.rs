use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[path = "sequence_pairs/copy.rs"]
mod copy;
pub(crate) use copy::*;
#[path = "sequence_pairs/library.rs"]
mod library;
pub(crate) use library::*;
#[path = "sequence_pairs/misc.rs"]
mod misc;
pub(crate) use misc::*;
#[path = "sequence_pairs/residual.rs"]
mod residual;
pub(crate) use residual::*;
#[path = "sequence_pairs/consult.rs"]
mod consult;
pub(crate) use consult::*;
#[path = "sequence_pairs/cloak.rs"]
mod cloak;
pub(crate) use cloak::*;

fn next_word<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    while !input.is_empty() {
        let token: &'a OwnedLexToken = any.parse_next(input)?;
        if token.as_word().is_some() {
            return Ok(token);
        }
    }
    Err(primitives::backtrack_err("sequence pair word", "word"))
}

pub(super) fn sequence_word<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        let token = next_word(input)?;
        if token.parser_text() == expected {
            Ok(())
        } else {
            Err(primitives::backtrack_err("sequence pair word", expected))
        }
    }
}

pub(super) fn sequence_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            sequence_word(*word).parse_next(input)?;
        }
        Ok(())
    }
}

pub(super) fn sequence_any_phrase<'a, 'b>(
    alternatives: &'b [&'static [&'static str]],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> + 'b {
    move |input: &mut LexStream<'a>| {
        for alternative in alternatives {
            let mut probe = input.clone();
            if sequence_phrase(alternative).parse_next(&mut probe).is_ok() {
                *input = probe;
                return Ok(());
            }
        }
        Err(primitives::backtrack_err(
            "sequence pair phrase",
            "supported phrase",
        ))
    }
}

pub(super) fn finish_sequence_words(input: &mut LexStream<'_>) -> WResult<()> {
    while !input.is_empty() {
        let token: &OwnedLexToken = any.parse_next(input)?;
        if token.as_word().is_some() {
            return Err(primitives::backtrack_err(
                "sequence pair end",
                "end of words",
            ));
        }
    }
    Ok(())
}

pub(super) fn matches_complete_sequence(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> bool {
    primitives::parse_all(
        tokens,
        (sequence_any_phrase(alternatives), finish_sequence_words),
        "sequence-pair-complete-phrase",
    )
    .is_ok()
}

pub(super) fn starts_sequence(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> bool {
    primitives::parse_prefix(tokens, sequence_any_phrase(alternatives)).is_some()
}

pub(super) fn seek_sequence_phrase<'a>(
    input: &mut LexStream<'a>,
    alternatives: &[&'static [&'static str]],
) -> WResult<usize> {
    let initial_len = input.len();
    while !input.is_empty() {
        let mut probe = input.clone();
        if sequence_any_phrase(alternatives)
            .parse_next(&mut probe)
            .is_ok()
        {
            return Ok(initial_len.saturating_sub(input.len()));
        }
        let _: &'a OwnedLexToken = any.parse_next(input)?;
    }
    Err(primitives::backtrack_err(
        "sequence pair search",
        "requested phrase",
    ))
}

pub(super) fn contains_sequence_phrase(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> bool {
    let mut input = LexStream::new(tokens);
    seek_sequence_phrase(&mut input, alternatives).is_ok()
}

pub(super) fn contains_sequence_word(tokens: &[OwnedLexToken], expected: &'static str) -> bool {
    let mut input = LexStream::new(tokens);
    while let Ok(token) = next_word(&mut input) {
        if token.is_word(expected) {
            return true;
        }
    }
    false
}

fn next_content_word<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    loop {
        let token = next_word(input)?;
        if !matches!(token.parser_text(), "a" | "an" | "the") {
            return Ok(token);
        }
    }
}

pub(super) fn sequence_content_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            let token = next_content_word(input)?;
            if token.parser_text() != *word {
                return Err(primitives::backtrack_err(
                    "sequence pair content phrase",
                    "requested content word",
                ));
            }
        }
        Ok(())
    }
}

pub(super) fn matches_complete_content_sequence(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> bool {
    for alternative in alternatives {
        let mut input = LexStream::new(tokens);
        if sequence_content_phrase(alternative)
            .parse_next(&mut input)
            .is_ok()
            && next_content_word(&mut input).is_err()
        {
            return true;
        }
    }
    false
}

pub(super) fn starts_content_sequence(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> bool {
    for alternative in alternatives {
        let mut input = LexStream::new(tokens);
        if sequence_content_phrase(alternative)
            .parse_next(&mut input)
            .is_ok()
        {
            return true;
        }
    }
    false
}

pub(super) fn contains_content_sequence(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> bool {
    let mut input = LexStream::new(tokens);
    while !input.is_empty() {
        for alternative in alternatives {
            let mut probe = input.clone();
            if sequence_content_phrase(alternative)
                .parse_next(&mut probe)
                .is_ok()
            {
                return true;
            }
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        if parsed.is_err() {
            break;
        }
    }
    false
}

pub(super) fn ends_content_sequence(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> bool {
    let mut input = LexStream::new(tokens);
    while !input.is_empty() {
        for alternative in alternatives {
            let mut probe = input.clone();
            if sequence_content_phrase(alternative)
                .parse_next(&mut probe)
                .is_ok()
                && next_content_word(&mut probe).is_err()
            {
                return true;
            }
        }
        if any::<_, winnow::error::ErrMode<winnow::error::ContextError>>
            .parse_next(&mut input)
            .is_err()
        {
            break;
        }
    }
    false
}

pub(super) fn same_words_without_articles(left: &[OwnedLexToken], right: &[OwnedLexToken]) -> bool {
    fn next_comparable_word<'a>(input: &mut LexStream<'a>) -> Option<&'a str> {
        next_content_word(input)
            .ok()
            .map(OwnedLexToken::parser_text)
    }

    let mut left_input = LexStream::new(left);
    let mut right_input = LexStream::new(right);
    loop {
        match (
            next_comparable_word(&mut left_input),
            next_comparable_word(&mut right_input),
        ) {
            (None, None) => return true,
            (Some(left), Some(right)) if left == right => {}
            _ => return false,
        }
    }
}
