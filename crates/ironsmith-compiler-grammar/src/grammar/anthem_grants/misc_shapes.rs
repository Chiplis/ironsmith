use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::super::lexer::{OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordIfColorShape<'a> {
    pub keyword_tokens: &'a [OwnedLexToken],
    pub color_tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentEquipShape<'a> {
    pub cost_tokens: &'a [OwnedLexToken],
    pub condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrailingGrantSegmentShape<'a> {
    pub body_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdditionalEntryCounterSurface;

pub fn parse_keyword_if_color_shape(tokens: &[OwnedLexToken]) -> Option<KeywordIfColorShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (keyword_tokens, color_tail_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("if").void())?;
    let keyword_tokens = trim_lexed_commas(keyword_tokens);
    let color_tail_tokens = trim_lexed_commas(color_tail_tokens);
    (!keyword_tokens.is_empty() && !color_tail_tokens.is_empty()).then_some(KeywordIfColorShape {
        keyword_tokens,
        color_tail_tokens,
    })
}

pub fn split_keyword_if_color_segments(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    primitives::split_lexed_slices_on_comma(tokens)
        .into_iter()
        .map(super::trim_anthem_clause_tokens)
        .map(strip_leading_and)
        .filter(|segment| !segment.is_empty())
        .collect()
}

pub fn parse_equipment_equip_shape(tokens: &[OwnedLexToken]) -> Option<EquipmentEquipShape<'_>> {
    let tokens = strip_metalcraft_label(tokens).unwrap_or(tokens);
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, after_prefix) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["equipment", "you", "control", "have", "equip"]),
    )?;
    let prefix_end = tokens.len().saturating_sub(after_prefix.len());
    let (relative_as, _, _) =
        primitives::find_prefix(after_prefix, || primitives::phrase(&["as", "long", "as"]))?;
    let as_token = prefix_end + relative_as;
    let cost_tokens = trim_lexed_commas(tokens.get(prefix_end..as_token)?);
    let condition_tokens = tokens.get(as_token..)?;
    (!cost_tokens.is_empty() && !condition_tokens.is_empty()).then_some(EquipmentEquipShape {
        cost_tokens,
        condition_tokens,
    })
}

pub fn split_trailing_grant_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut preserve_commas = false;
    let mut inside_quotes = false;
    let mut current_has_closed_quote = false;
    let mut idx = 0usize;

    while idx < tokens.len() {
        let token = &tokens[idx];
        if !inside_quotes
            && current_has_closed_quote
            && starts_unseparated_grant_after_quote(&tokens[idx..])
        {
            segments.push(std::mem::take(&mut current));
            preserve_commas = false;
            current_has_closed_quote = false;
            continue;
        }
        if token.kind == TokenKind::Quote {
            let was_inside_quotes = inside_quotes;
            inside_quotes = !inside_quotes;
            current_has_closed_quote |= was_inside_quotes;
        }

        let next_non_comma_is_quote = next_non_comma_is_quote(&tokens[idx + 1..]);
        if !inside_quotes
            && token_is_and(token)
            && !current.is_empty()
            && (next_non_comma_is_quote || current_has_closed_quote)
        {
            segments.push(std::mem::take(&mut current));
            preserve_commas = false;
            current_has_closed_quote = false;
            idx += 1;
            continue;
        }

        if token.kind == TokenKind::Comma && !preserve_commas {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            current_has_closed_quote = false;
            idx += 1;
            continue;
        }

        current.push(token.clone());
        let trimmed = trim_lexed_commas(&current);
        preserve_commas = inside_quotes
            || trimmed.iter().any(|token| token.kind == TokenKind::Colon)
            || starts_with_trigger_intro(trimmed);
        idx += 1;
    }

    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

pub fn parse_trailing_grant_segment(
    tokens: &[OwnedLexToken],
) -> Option<TrailingGrantSegmentShape<'_>> {
    let mut body_tokens = trim_grant_segment_edges(tokens);
    while let Some((_, rest)) = primitives::parse_prefix(body_tokens, primitives::kw("and")) {
        body_tokens = trim_grant_segment_edges(rest);
    }
    (!body_tokens.is_empty()).then_some(TrailingGrantSegmentShape { body_tokens })
}

pub fn parse_additional_entry_counter_surface(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalEntryCounterSurface> {
    let (_, _, _) = primitives::find_prefix(tokens, || {
        alt((
            primitives::phrase(&["enters", "with", "additional"]),
            primitives::phrase(&["enters", "with", "a", "additional"]),
            primitives::phrase(&["enters", "with", "an", "additional"]),
        ))
        .void()
    })?;
    Some(AdditionalEntryCounterSurface)
}

fn strip_metalcraft_label(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (dash, _, body) = primitives::find_prefix(tokens, || {
        alt((
            primitives::token_kind(TokenKind::Dash),
            primitives::token_kind(TokenKind::EmDash),
        ))
        .void()
    })?;
    let label_tokens = tokens.get(..dash)?;
    if label_tokens.is_empty()
        || primitives::parse_all(
            label_tokens,
            primitives::kw("metalcraft"),
            "metalcraft label",
        )
        .is_err()
    {
        return None;
    }
    Some(body)
}

fn strip_leading_and(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut tokens = trim_lexed_commas(tokens);
    while let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::kw("and")) {
        tokens = trim_lexed_commas(rest);
    }
    tokens
}

fn trim_grant_segment_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        end -= 1;
    }
    &tokens[start..end]
}

fn starts_with_trigger_intro(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_trigger_intro).is_some()
}

fn parse_trigger_intro(input: &mut super::super::super::lexer::LexStream<'_>) -> WResult<()> {
    (
        winnow::combinator::opt(primitives::kw("and")),
        alt((
            primitives::kw("when").void(),
            primitives::kw("whenever").void(),
            primitives::phrase(&["at", "the"]),
        )),
    )
        .void()
        .parse_next(input)
}

fn parse_and_word(input: &mut super::super::super::lexer::LexStream<'_>) -> WResult<()> {
    primitives::kw("and").void().parse_next(input)
}

fn token_is_and(token: &OwnedLexToken) -> bool {
    let mut input = super::super::super::lexer::LexStream::new(std::slice::from_ref(token));
    parse_and_word(&mut input).is_ok()
}

fn next_non_comma_is_quote(tokens: &[OwnedLexToken]) -> bool {
    let mut input = super::super::super::lexer::LexStream::new(tokens);
    loop {
        let parsed: WResult<&OwnedLexToken> = winnow::token::any.parse_next(&mut input);
        let Ok(token) = parsed else {
            return false;
        };
        if token.kind != TokenKind::Comma {
            return token.kind == TokenKind::Quote;
        }
    }
}

fn starts_unseparated_grant_after_quote(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, primitives::kw("equip")).is_some()
}

#[cfg(test)]
#[path = "misc_shapes_tests.rs"]
mod tests;
