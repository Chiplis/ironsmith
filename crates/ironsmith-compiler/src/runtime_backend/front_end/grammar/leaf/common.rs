use winnow::ascii::multispace0;
#[cfg(test)]
use winnow::ascii::space1;
use winnow::combinator::{alt, delimited, eof, peek, terminated};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
#[cfg(test)]
use winnow::token::literal;
use winnow::token::one_of;

use crate::cards::builders::CardTextError;

use super::super::primitives;

pub(super) fn spaced<'a, O, E, P>(parser: P) -> impl Parser<&'a str, O, E>
where
    P: Parser<&'a str, O, E>,
    E: winnow::error::ParserError<&'a str>,
{
    delimited(multispace0, parser, multispace0)
}

pub(super) fn phrase<'a>(
    expected: &'static str,
) -> impl Parser<&'a str, (), ErrMode<ContextError>> {
    terminated(expected, word_boundary).void()
}

#[cfg(test)]
pub(super) fn text_phrase_words<'a>(
    expected: &'static [&'static str],
) -> impl Parser<&'a str, (), ErrMode<ContextError>> {
    move |input: &mut &'a str| {
        for (idx, word) in expected.iter().enumerate() {
            if idx > 0 {
                space1.parse_next(input)?;
            }
            literal(*word).parse_next(input)?;
        }
        word_boundary.parse_next(input)
    }
}

pub(super) fn word_boundary(input: &mut &str) -> WResult<()> {
    peek(alt((
        eof.value(()),
        one_of([
            ' ', '\t', '\n', '\r', ',', '.', ';', ':', ')', '(', '/', '-',
        ])
        .void(),
    )))
    .parse_next(input)
}

pub(super) fn finish_text_parse<O>(
    raw: &str,
    parser: impl for<'a> Parser<&'a str, O, ErrMode<ContextError>>,
    label: &str,
) -> Result<O, CardTextError> {
    let normalized = raw.trim().to_ascii_lowercase();
    let mut input = normalized.as_str();
    let mut parser = primitives::maybe_trace(label, delimited(multispace0, parser, multispace0));
    let parsed = parser
        .parse_next(&mut input)
        .map_err(|err| CardTextError::ParseError(format!("rewrite {label} parse failed: {err}")))?;
    if !input.trim().is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite {label} parser left trailing input: '{}'",
            input.trim()
        )));
    }
    Ok(parsed)
}
