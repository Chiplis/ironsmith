use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::rest;

use crate::grammar::{permission_shapes, primitives};
use crate::lexer::{
    LexStream, OwnedLexToken, TokenKind, TokenWordView, lex_line, render_token_slice,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SagaChapterHeader {
    pub chapters: Vec<u32>,
    pub presentation_label: Option<String>,
    pub body: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LevelHeader {
    pub minimum: u32,
    pub maximum: Option<u32>,
}

pub fn parse_saga_chapter_header(line: &str) -> Option<SagaChapterHeader> {
    let tokens = crate::util::lex_fragment(line.trim(), 0)?;
    let (chapters, rest_tokens) = primitives::parse_prefix(&tokens, parse_saga_prefix)?;
    let body_shape = parse_saga_chapter_body_tokens(rest_tokens);
    let body = render_token_slice(body_shape.body_tokens)
        .trim()
        .to_string();
    let presentation_label = body_shape
        .presentation_label_tokens
        .map(render_token_slice)
        .map(|label| label.trim().to_string());
    (!chapters.is_empty() && !body.is_empty()).then_some(SagaChapterHeader {
        chapters,
        presentation_label,
        body,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SagaChapterBodyShape<'a> {
    presentation_label_tokens: Option<&'a [OwnedLexToken]>,
    body_tokens: &'a [OwnedLexToken],
}

fn parse_saga_chapter_body_tokens(tokens: &[OwnedLexToken]) -> SagaChapterBodyShape<'_> {
    primitives::parse_all(
        tokens,
        parse_labeled_saga_chapter_body,
        "labeled saga chapter body",
    )
    .unwrap_or(SagaChapterBodyShape {
        presentation_label_tokens: None,
        body_tokens: tokens,
    })
}

fn parse_labeled_saga_chapter_body<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SagaChapterBodyShape<'a>> {
    let label_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..=4,
        alt((
            primitives::word_parser_text.void(),
            primitives::token_kind(TokenKind::Bang).void(),
        )),
        peek(
            alt((
                primitives::token_kind(TokenKind::Dash),
                primitives::token_kind(TokenKind::EmDash),
            ))
            .void(),
        ),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    alt((
        primitives::token_kind(TokenKind::Dash),
        primitives::token_kind(TokenKind::EmDash),
    ))
    .parse_next(input)?;
    let body_tokens = rest.parse_next(input)?;
    if body_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "saga chapter label",
            "non-empty effect body",
        ));
    }
    Ok(SagaChapterBodyShape {
        presentation_label_tokens: Some(label_tokens),
        body_tokens,
    })
}

pub fn parse_level_header(line: &str) -> Option<LevelHeader> {
    let tokens = crate::util::lex_fragment(line.trim(), 0)?;
    if !permission_shapes::prefix_tokens(&tokens, &["level"]) {
        return None;
    }
    let words = TokenWordView::new(&tokens);
    let range_start = words.token_index_after_words(1)?;
    let range_tokens = tokens.get(range_start..)?;
    parse_level_header_range_tokens(range_tokens)
        .or_else(|| words.get(1).and_then(parse_level_header_range_word))
}

fn parse_saga_prefix<'a>(input: &mut LexStream<'a>) -> WResult<Vec<u32>> {
    let chapters = repeat(
        1..,
        (parse_roman_chapter, opt(primitives::comma())).map(|(chapter, _)| chapter),
    )
    .parse_next(input)?;
    alt((
        primitives::token_kind(TokenKind::Dash),
        primitives::token_kind(TokenKind::EmDash),
    ))
    .void()
    .parse_next(input)?;
    Ok(chapters)
}

fn parse_roman_chapter(input: &mut LexStream<'_>) -> WResult<u32> {
    let roman = primitives::word_parser_text.parse_next(input)?;
    roman_to_int(roman)
        .ok_or_else(|| primitives::backtrack_err("saga chapter", "roman numeral from I through VI"))
}

fn roman_to_int(roman: &str) -> Option<u32> {
    match roman {
        "i" => Some(1),
        "ii" => Some(2),
        "iii" => Some(3),
        "iv" => Some(4),
        "v" => Some(5),
        "vi" => Some(6),
        _ => None,
    }
}

fn parse_level_header_range_tokens(tokens: &[OwnedLexToken]) -> Option<LevelHeader> {
    let first = tokens.first()?;
    let minimum = parse_u32_token(first)?;
    match tokens.get(1).map(|token| token.kind) {
        Some(TokenKind::Plus) => Some(LevelHeader {
            minimum,
            maximum: None,
        }),
        Some(TokenKind::Dash) => Some(LevelHeader {
            minimum,
            maximum: Some(parse_u32_token(tokens.get(2)?)?),
        }),
        _ => Some(LevelHeader {
            minimum,
            maximum: Some(minimum),
        }),
    }
}

fn parse_level_header_range_word(word: &str) -> Option<LevelHeader> {
    let mut chars = word.chars();
    let minimum = take_ascii_u32(&mut chars)?;
    match chars.next() {
        None => Some(LevelHeader {
            minimum,
            maximum: Some(minimum),
        }),
        Some('+') if chars.next().is_none() => Some(LevelHeader {
            minimum,
            maximum: None,
        }),
        Some('-') => {
            let maximum = take_ascii_u32(&mut chars)?;
            chars.next().is_none().then_some(LevelHeader {
                minimum,
                maximum: Some(maximum),
            })
        }
        _ => None,
    }
}

fn parse_u32_token(token: &OwnedLexToken) -> Option<u32> {
    let parsed = parse_level_header_range_word(token.parser_text())?;
    (parsed.maximum == Some(parsed.minimum)).then_some(parsed.minimum)
}

fn take_ascii_u32(chars: &mut std::str::Chars<'_>) -> Option<u32> {
    let mut value = 0u32;
    let mut saw_digit = false;
    while let Some(ch) = chars.clone().next() {
        if !ch.is_ascii_digit() {
            break;
        }
        chars.next();
        value = value.checked_mul(10)?.checked_add(ch.to_digit(10)?)?;
        saw_digit = true;
    }
    saw_digit.then_some(value)
}

#[cfg(test)]
#[path = "header_shapes/tests.rs"]
mod tests;
