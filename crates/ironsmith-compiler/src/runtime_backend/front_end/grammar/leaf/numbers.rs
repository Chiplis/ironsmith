use winnow::ascii::{dec_int, digit1};
use winnow::combinator::{alt, terminated};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::take_while;

use crate::cards::builders::CardTextError;
use crate::effect::Value;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
use super::common::{finish_text_parse, word_boundary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafNumber {
    Fixed(u32),
    X,
}

impl LeafNumber {
    pub(crate) fn into_value(self) -> Option<Value> {
        match self {
            Self::Fixed(value) => i32::try_from(value).ok().map(Value::Fixed),
            Self::X => Some(Value::X),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeafNumberPrefix {
    pub(crate) number: LeafNumber,
    pub(crate) consumed: usize,
}

impl LeafNumberPrefix {
    pub(crate) fn into_fixed(self) -> Option<(u32, usize)> {
        let LeafNumber::Fixed(value) = self.number else {
            return None;
        };
        Some((value, self.consumed))
    }

    pub(crate) fn into_value(self) -> Option<(Value, usize)> {
        Some((self.number.into_value()?, self.consumed))
    }
}

pub(crate) fn parse_unsigned_number(input: &mut &str) -> WResult<u32> {
    terminated(
        digit1.try_map(|digits: &str| digits.parse::<u32>()),
        word_boundary,
    )
    .context(StrContext::Label("unsigned number"))
    .context(StrContext::Expected(StrContextValue::Description(
        "base-10 number",
    )))
    .parse_next(input)
}

pub(crate) fn parse_article_number(input: &mut &str) -> WResult<u32> {
    terminated(alt(("an".value(1), "a".value(1))), word_boundary)
        .context(StrContext::Label("article number"))
        .context(StrContext::Expected(StrContextValue::Description(
            "a or an",
        )))
        .parse_next(input)
}

pub(crate) fn parse_word_number(input: &mut &str) -> WResult<u32> {
    let checkpoint = *input;
    let word = parse_number_word.parse_next(input)?;
    let normalized = word.to_ascii_lowercase();
    if let Some(value) = ironsmith_core::parse_cardinal_word(&normalized) {
        return Ok(value);
    }

    *input = checkpoint;
    Err(primitives::backtrack_err(
        "word number",
        "cardinal number word",
    ))
}

pub(crate) fn parse_number(input: &mut &str) -> WResult<u32> {
    alt((
        parse_unsigned_number,
        parse_word_number,
        parse_article_number,
    ))
    .context(StrContext::Label("number"))
    .context(StrContext::Expected(StrContextValue::Description(
        "numeric, word, or article count",
    )))
    .parse_next(input)
}

pub(crate) fn parse_number_i32(input: &mut &str) -> WResult<i32> {
    alt((
        terminated(dec_int, word_boundary),
        parse_number.try_map(i32::try_from),
    ))
    .context(StrContext::Label("signed-safe number"))
    .context(StrContext::Expected(StrContextValue::Description(
        "i32 numeric, word, or article count",
    )))
    .parse_next(input)
}

pub(crate) fn parse_leaf_die_sides_complete(raw: &str) -> Result<u32, CardTextError> {
    finish_text_parse(raw, parse_leaf_die_sides, "leaf-die-sides")
}

fn parse_leaf_die_sides(input: &mut &str) -> WResult<u32> {
    "d".parse_next(input)?;
    terminated(
        digit1.try_map(|digits: &str| digits.parse::<u32>()),
        word_boundary,
    )
    .parse_next(input)
}

#[cfg(test)]
pub(crate) fn parse_number_or_x(input: &mut &str) -> WResult<LeafNumber> {
    alt((
        terminated("x", word_boundary).value(LeafNumber::X),
        parse_number.map(LeafNumber::Fixed),
    ))
    .context(StrContext::Label("number or X"))
    .context(StrContext::Expected(StrContextValue::Description(
        "number or X",
    )))
    .parse_next(input)
}

pub(crate) fn parse_leaf_number_token_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    let word = primitives::word_parser_text.parse_next(input)?;
    parse_number_complete(word)
        .map_err(|_| primitives::backtrack_err("number", "numeric or counted quantity"))
}

pub(crate) fn parse_leaf_count_token<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    parse_leaf_number_token_lexed
        .context(StrContext::Label("count"))
        .parse_next(input)
}

pub(crate) fn parse_leaf_number_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    let mut probe = input.clone();
    let first = primitives::word_parser_text.parse_next(&mut probe)?;
    if let Some(value) = repetition_adverb_number_value(first) {
        *input = probe;
        return Ok(value);
    }

    parse_leaf_cardinal_number_prefix_lexed.parse_next(input)
}

pub(crate) fn parse_leaf_number_or_x_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LeafNumber> {
    let mut probe = input.clone();
    let first = primitives::word_parser_text.parse_next(&mut probe)?;
    if first == "x" {
        *input = probe;
        return Ok(LeafNumber::X);
    }

    parse_leaf_cardinal_number_prefix_lexed
        .map(LeafNumber::Fixed)
        .parse_next(input)
}

pub(crate) fn parse_leaf_number_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeafNumberPrefix> {
    parse_leaf_number_prefix_tokens_with(
        tokens,
        parse_leaf_number_prefix_lexed.map(LeafNumber::Fixed),
    )
}

pub(crate) fn parse_leaf_number_or_x_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeafNumberPrefix> {
    parse_leaf_number_prefix_tokens_with(tokens, parse_leaf_number_or_x_prefix_lexed)
}

pub(crate) fn parse_leaf_number_prefix_words(words: &[&str]) -> Option<LeafNumberPrefix> {
    let mut input = words;
    let number = parse_leaf_number_prefix_word_slice
        .map(LeafNumber::Fixed)
        .parse_next(&mut input)
        .ok()?;
    Some(LeafNumberPrefix {
        number,
        consumed: words.len().checked_sub(input.len())?,
    })
}

fn parse_leaf_cardinal_number_prefix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    let checkpoint = input.checkpoint();
    let mut probe = input.clone();
    let first = primitives::word_parser_text.parse_next(&mut probe)?;
    let mut words = vec![first];
    let mut best = cardinal_words_full_match(words.as_slice()).map(|value| (value, probe.clone()));
    loop {
        let mut next_probe = probe.clone();
        let Ok(word) = primitives::word_parser_text.parse_next(&mut next_probe) else {
            break;
        };
        words.push(word);
        if let Some(value) = cardinal_words_full_match(words.as_slice()) {
            best = Some((value, next_probe.clone()));
        }
        probe = next_probe;
    }

    if let Some((value, rest)) = best {
        *input = rest;
        return Ok(value);
    }

    input.reset(&checkpoint);
    Err(primitives::backtrack_err(
        "number prefix",
        "numeric or counted quantity",
    ))
}

pub(super) fn parse_leaf_number_prefix_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<u32> {
    let checkpoint = *input;
    let first = *checkpoint
        .first()
        .ok_or_else(|| primitives::backtrack_err("number prefix", "numeric or counted quantity"))?;

    if let Some(value) = repetition_adverb_number_value(first) {
        *input = &checkpoint[1..];
        return Ok(value);
    }

    if let Some((value, used)) = ironsmith_core::parse_cardinal_words(checkpoint) {
        *input = &checkpoint[used..];
        return Ok(value);
    }

    if let Ok(value) = parse_number_with_trailing_punctuation_complete(first) {
        *input = &checkpoint[1..];
        return Ok(value);
    }

    Err(primitives::backtrack_err(
        "number prefix",
        "numeric or counted quantity",
    ))
}

fn parse_leaf_number_prefix_tokens_with<'a>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<LexStream<'a>, LeafNumber, ErrMode<ContextError>>,
) -> Option<LeafNumberPrefix> {
    let (number, rest) = primitives::parse_prefix(tokens, parser)?;
    Some(LeafNumberPrefix {
        number,
        consumed: tokens.len().checked_sub(rest.len())?,
    })
}

pub(crate) fn parse_number_complete(raw: &str) -> Result<u32, CardTextError> {
    finish_text_parse(raw, parse_number, "leaf-number")
}

pub(crate) fn parse_number_i32_complete(raw: &str) -> Result<i32, CardTextError> {
    finish_text_parse(raw, parse_number_i32, "leaf-number-i32")
}

#[cfg(test)]
pub(crate) fn parse_number_or_x_complete(raw: &str) -> Result<LeafNumber, CardTextError> {
    finish_text_parse(raw, parse_number_or_x, "leaf-number-or-x")
}

fn repetition_adverb_number_value(word: &str) -> Option<u32> {
    if word.eq_ignore_ascii_case("once") {
        Some(1)
    } else if word.eq_ignore_ascii_case("twice") {
        Some(2)
    } else {
        None
    }
}

fn cardinal_words_full_match(words: &[&str]) -> Option<u32> {
    let (value, used) = ironsmith_core::parse_cardinal_words(words)?;
    (used == words.len()).then_some(value)
}

fn parse_number_with_trailing_punctuation_complete(raw: &str) -> Result<u32, CardTextError> {
    finish_text_parse(
        raw,
        parse_number_with_trailing_punctuation,
        "leaf-number-trailing-punctuation",
    )
}

fn parse_number_with_trailing_punctuation(input: &mut &str) -> WResult<u32> {
    (
        digit1.try_map(|digits: &str| digits.parse::<u32>()),
        take_while(1.., |ch: char| !ch.is_ascii_digit()),
    )
        .map(|(value, _)| value)
        .parse_next(input)
}

fn parse_number_word<'a>(input: &mut &'a str) -> WResult<&'a str> {
    terminated(
        take_while(1.., |ch: char| ch.is_ascii_alphabetic() || ch == '-'),
        word_boundary,
    )
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    fn fixed_prefix(raw: &str) -> LeafNumberPrefix {
        let tokens = lex_line(raw, 0).unwrap();
        parse_leaf_number_prefix_tokens(&tokens).unwrap()
    }

    #[test]
    fn lexed_prefix_parses_digits() {
        assert_eq!(
            fixed_prefix("12 cards"),
            LeafNumberPrefix {
                number: LeafNumber::Fixed(12),
                consumed: 1,
            }
        );
    }

    #[test]
    fn lexed_prefix_parses_cardinal_words() {
        assert_eq!(
            fixed_prefix("twenty one cards"),
            LeafNumberPrefix {
                number: LeafNumber::Fixed(21),
                consumed: 2,
            }
        );
    }

    #[test]
    fn lexed_prefix_parses_articles() {
        assert_eq!(
            fixed_prefix("an artifact"),
            LeafNumberPrefix {
                number: LeafNumber::Fixed(1),
                consumed: 1,
            }
        );
    }

    #[test]
    fn lexed_prefix_maps_x_to_runtime_value() {
        let tokens = lex_line("X cards", 0).unwrap();
        let prefix = parse_leaf_number_or_x_prefix_tokens(&tokens).unwrap();
        assert_eq!(prefix.into_value(), Some((Value::X, 1)));
    }

    #[test]
    fn fixed_prefix_preserves_once_and_twice() {
        assert_eq!(fixed_prefix("once each turn").into_fixed(), Some((1, 1)));
        assert_eq!(fixed_prefix("twice each turn").into_fixed(), Some((2, 1)));
    }

    #[test]
    fn prefix_leaves_trailing_punctuation_unconsumed() {
        assert_eq!(fixed_prefix("2.").into_fixed(), Some((2, 1)));
        assert_eq!(
            parse_leaf_number_prefix_words(&["2."]).and_then(LeafNumberPrefix::into_fixed),
            Some((2, 1))
        );
    }

    #[test]
    fn safe_i32_parser_preserves_signed_numeric_words() {
        assert_eq!(parse_number_i32_complete("-2").unwrap(), -2);
        assert_eq!(parse_number_i32_complete("three").unwrap(), 3);
        assert!(parse_number_i32_complete("4294967295").is_err());
    }

    #[test]
    fn die_sides_are_typed() {
        assert_eq!(parse_leaf_die_sides_complete("D20").unwrap(), 20);
        assert!(parse_leaf_die_sides_complete("die20").is_err());
    }
}
