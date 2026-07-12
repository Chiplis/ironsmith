use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::recognition::{
    comma_boundary_facts, preserve_and_reason, starts_with_each_player_or_opponent,
    then_followup_facts,
};

pub(crate) fn split_effect_chain_on_and_tokens(
    tokens: &[OwnedLexToken],
    extended: bool,
) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut input = LexStream::new(tokens);
    while !input.is_empty() {
        let idx = tokens.len().saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            break;
        };
        if !is_word(token, "and") {
            continue;
        }
        let current = trim_lexed_commas(tokens.get(start..idx).unwrap_or_default());
        let remaining = trim_lexed_commas(tokens.get(idx + 1..).unwrap_or_default());
        if preserve_and_reason(current, remaining, extended).is_some() {
            continue;
        }
        if !current.is_empty() {
            segments.push(current);
        }
        start = idx + 1;
    }
    let tail = trim_lexed_commas(tokens.get(start..).unwrap_or_default());
    if !tail.is_empty() {
        segments.push(tail);
    }
    segments
}

pub(crate) fn split_segments_on_comma_then_tokens<'a>(
    segments: Vec<&'a [OwnedLexToken]>,
    mut is_ability_head: impl FnMut(&[OwnedLexToken]) -> bool,
) -> Vec<&'a [OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        let starts_with_for_each = starts_with_each_player_or_opponent(segment);
        let mut split_point = None;
        let mut input = LexStream::new(segment);
        let mut inside_quotes = false;
        while !input.is_empty() {
            let idx = segment.len().saturating_sub(input.len());
            let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
            let Ok(token) = parsed else {
                break;
            };
            if token.kind == TokenKind::Quote {
                inside_quotes = !inside_quotes;
                continue;
            }
            if inside_quotes {
                continue;
            }
            let then_idx = if is_word(token, "then") {
                Some(idx)
            } else if token.kind == TokenKind::Comma
                && segment
                    .get(idx + 1)
                    .is_some_and(|next| is_word(next, "then"))
            {
                Some(idx + 1)
            } else {
                None
            };
            let Some(then_idx) = then_idx else {
                continue;
            };
            let before = trim_lexed_commas(segment.get(..idx).unwrap_or_default());
            let after = trim_lexed_commas(segment.get(then_idx + 1..).unwrap_or_default());
            let facts = then_followup_facts(before, after, starts_with_for_each);
            if facts.should_split(is_ability_head(after)) {
                split_point = Some((idx, then_idx));
                break;
            }
        }
        if let Some((idx, then_idx)) = split_point {
            let first = trim_lexed_commas(segment.get(..idx).unwrap_or_default());
            let second = trim_lexed_commas(segment.get(then_idx + 1..).unwrap_or_default());
            if !first.is_empty() {
                result.push(first);
            }
            if !second.is_empty() {
                result.push(second);
            }
        } else {
            result.push(segment);
        }
    }
    result
}

pub(crate) fn split_segments_on_comma_effect_head_tokens(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        let mut start = 0usize;
        let mut split_any = false;
        let mut input = LexStream::new(segment);
        let mut inside_quotes = false;
        while !input.is_empty() {
            let idx = segment.len().saturating_sub(input.len());
            let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
            let Ok(token) = parsed else {
                break;
            };
            if token.kind == TokenKind::Quote {
                inside_quotes = !inside_quotes;
                continue;
            }
            if inside_quotes || token.kind != TokenKind::Comma {
                continue;
            }
            let before = trim_lexed_commas(segment.get(start..idx).unwrap_or_default());
            let after = trim_lexed_commas(segment.get(idx + 1..).unwrap_or_default());
            if before.is_empty() || after.is_empty() {
                continue;
            }
            let facts = comma_boundary_facts(before, after);
            if facts.preserve_boundary {
                continue;
            }
            if facts.before_has_verb && facts.after_starts_effect {
                result.push(before);
                start = idx + 1;
                split_any = true;
            }
        }
        if split_any {
            let tail = trim_lexed_commas(segment.get(start..).unwrap_or_default());
            if !tail.is_empty() {
                result.push(tail);
            }
        } else {
            result.push(segment);
        }
    }
    result
}

fn is_word(token: &OwnedLexToken, expected: &'static str) -> bool {
    let mut input = LexStream::new(std::slice::from_ref(token));
    (
        super::super::super::primitives::kw(expected),
        super::super::super::primitives::end_of_block(),
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}
