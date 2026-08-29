use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::color::ColorSet;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnthemKeywordOrder {
    KeywordBeforeAnthem,
    AnthemBeforeKeyword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemKeywordHead {
    pub get_token: usize,
    pub have_token: usize,
    pub order: AnthemKeywordOrder,
    pub pre_grant_is_temporary: bool,
    pub clause_tail_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordBeforeAnthemShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub keyword_tokens: &'a [OwnedLexToken],
    pub anthem_tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemKeywordColorSegment {
    pub is_token: usize,
    pub color: ColorSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemKeywordCompoundSplit {
    pub split_token: usize,
    pub tail_start: usize,
    pub second_get_token: Option<usize>,
    pub second_tail_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemKeywordTrailingCondition<'a> {
    pub ability_tokens: &'a [OwnedLexToken],
    pub condition_tokens: &'a [OwnedLexToken],
    pub trailing_if_surface: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnthemKeywordTrailingConditionError {
    MissingAbility,
    MissingCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenTailSplit<'a> {
    pub head_tokens: &'a [OwnedLexToken],
    pub tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColonTailSplit {
    pub colon_token: usize,
    pub last_and_before_colon: Option<usize>,
}

pub fn parse_anthem_keyword_head(tokens: &[OwnedLexToken]) -> Option<AnthemKeywordHead> {
    let get_token = first_unquoted_token_between(tokens, 0, tokens.len(), parse_get_word)?;
    let have_token = first_unquoted_token_between(
        tokens,
        get_token.saturating_add(1),
        tokens.len(),
        parse_have_word,
    )
    .or_else(|| first_unquoted_token_between(tokens, 0, get_token, parse_have_word))?;
    if get_token == have_token {
        return None;
    }
    let order = if have_token < get_token {
        AnthemKeywordOrder::KeywordBeforeAnthem
    } else {
        AnthemKeywordOrder::AnthemBeforeKeyword
    };
    let pre_grant_is_temporary = order == AnthemKeywordOrder::AnthemBeforeKeyword
        && contains_parser(&tokens[..have_token], || {
            primitives::phrase(&["until", "end", "of", "turn"]).void()
        });
    let clause_tail_end = if order == AnthemKeywordOrder::AnthemBeforeKeyword
        && have_token > get_token + 2
        && token_matches(&tokens[have_token - 1], parse_and_word)
    {
        have_token - 1
    } else {
        have_token
    };
    Some(AnthemKeywordHead {
        get_token,
        have_token,
        order,
        pre_grant_is_temporary,
        clause_tail_end,
    })
}

pub fn parse_keyword_before_anthem_shape(
    tokens: &[OwnedLexToken],
    head: AnthemKeywordHead,
) -> Option<KeywordBeforeAnthemShape<'_>> {
    if head.order != AnthemKeywordOrder::KeywordBeforeAnthem {
        return None;
    }
    let subject_tokens = trim_edge_punctuation(tokens.get(..head.have_token)?);
    let mut keyword_tokens =
        trim_edge_punctuation(tokens.get(head.have_token.saturating_add(1)..head.get_token)?);
    if keyword_tokens
        .first()
        .is_some_and(|token| token_matches(token, parse_and_word))
    {
        keyword_tokens = trim_edge_punctuation(&keyword_tokens[1..]);
    }
    if keyword_tokens
        .last()
        .is_some_and(|token| token_matches(token, parse_and_word))
    {
        keyword_tokens =
            trim_edge_punctuation(&keyword_tokens[..keyword_tokens.len().saturating_sub(1)]);
    }
    if subject_tokens.is_empty() || keyword_tokens.is_empty() {
        return None;
    }
    Some(KeywordBeforeAnthemShape {
        subject_tokens,
        keyword_tokens,
        anthem_tail_tokens: tokens.get(head.get_token..)?,
    })
}

pub fn parse_anthem_keyword_color_segment(
    tokens: &[OwnedLexToken],
    head: AnthemKeywordHead,
) -> Option<AnthemKeywordColorSegment> {
    if head.order != AnthemKeywordOrder::AnthemBeforeKeyword {
        return None;
    }
    let start = head.get_token.saturating_add(2);
    let is_token = first_token_between(tokens, start, head.have_token, parse_is_word)?;
    let color_word = tokens.get(is_token + 1)?.as_word()?;
    let color = leaf::parse_leaf_color_complete(color_word).ok()?;
    Some(AnthemKeywordColorSegment { is_token, color })
}

pub fn parse_anthem_keyword_compound_split(
    tokens: &[OwnedLexToken],
    head: AnthemKeywordHead,
) -> Option<AnthemKeywordCompoundSplit> {
    if head.order != AnthemKeywordOrder::AnthemBeforeKeyword {
        return None;
    }
    if modifier_tail_is_attached_for_each_count(tokens, head) {
        return None;
    }
    let split_end = head.have_token.saturating_sub(1);
    let split_token = first_token_between(
        tokens,
        head.get_token.saturating_add(2),
        split_end,
        parse_and_word,
    )?;
    let tail_start = split_token + 1;
    let second_get_token = first_token_between(tokens, tail_start, head.have_token, parse_get_word);
    let second_tail_end = match second_get_token {
        Some(second_get)
            if head.have_token > second_get + 2
                && token_matches(&tokens[head.have_token - 1], parse_and_word) =>
        {
            head.have_token - 1
        }
        _ => head.have_token,
    };
    Some(AnthemKeywordCompoundSplit {
        split_token,
        tail_start,
        second_get_token,
        second_tail_end,
    })
}

fn modifier_tail_is_attached_for_each_count(
    tokens: &[OwnedLexToken],
    head: AnthemKeywordHead,
) -> bool {
    let Some(modifier) = super::parse_modifier_shape(tokens, head.get_token, head.clause_tail_end)
    else {
        return false;
    };
    let Some(super::AnthemTailShape::ForEach(tail)) = super::parse_tail_shape(modifier.tail_tokens)
    else {
        return false;
    };
    let Some(rest) = super::parse_for_each_rest(tail) else {
        return false;
    };
    matches!(
        super::parse_for_each_special_shape(rest),
        Some(super::ForEachSpecialShape::AttachedToSource { .. })
    )
}

pub fn split_anthem_keyword_trailing_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<AnthemKeywordTrailingCondition<'_>>, AnthemKeywordTrailingConditionError> {
    let tokens = trim_edge_punctuation(tokens);
    let (marker_token, condition_start, trailing_if_surface) =
        if let Some((marker_token, condition_start)) = first_phrase(tokens, parse_as_long_as) {
            (marker_token, condition_start, false)
        } else if let Some((marker_token, condition_start)) = first_phrase(tokens, parse_if_word) {
            (marker_token, condition_start, true)
        } else {
            return Ok(None);
        };
    let ability_tokens = trim_edge_punctuation(&tokens[..marker_token]);
    if ability_tokens.is_empty() {
        return Err(AnthemKeywordTrailingConditionError::MissingAbility);
    }
    let condition_tokens = trim_edge_punctuation(&tokens[condition_start..]);
    if condition_tokens.is_empty() {
        return Err(AnthemKeywordTrailingConditionError::MissingCondition);
    }
    Ok(Some(AnthemKeywordTrailingCondition {
        ability_tokens,
        condition_tokens,
        trailing_if_surface,
    }))
}

pub fn split_anthem_keyword_and_is(tokens: &[OwnedLexToken]) -> Option<TokenTailSplit<'_>> {
    split_adjacent_pair(tokens, parse_and_word, parse_is_word, 1, true)
}

pub fn split_anthem_keyword_and_have(tokens: &[OwnedLexToken]) -> Option<TokenTailSplit<'_>> {
    split_adjacent_pair(tokens, parse_and_word, parse_have_word, 2, false)
}

pub fn parse_colon_tail_split(tokens: &[OwnedLexToken]) -> Option<ColonTailSplit> {
    let colon_token = first_kind(tokens, TokenKind::Colon)?;
    let last_and_before_colon = last_token_before(tokens, colon_token, parse_and_word);
    Some(ColonTailSplit {
        colon_token,
        last_and_before_colon,
    })
}

fn split_adjacent_pair<'a>(
    tokens: &'a [OwnedLexToken],
    first: fn(&mut LexStream<'a>) -> WResult<()>,
    second: fn(&mut LexStream<'a>) -> WResult<()>,
    tail_offset: usize,
    require_head: bool,
) -> Option<TokenTailSplit<'a>> {
    let tokens = trim_edge_punctuation(tokens);
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if first(&mut candidate).is_ok() && second(&mut candidate).is_ok() {
            let head_tokens = trim_edge_punctuation(&tokens[..offset]);
            let tail_tokens = trim_edge_punctuation(&tokens[offset + tail_offset..]);
            if (require_head && head_tokens.is_empty()) || tail_tokens.is_empty() {
                return None;
            }
            return Some(TokenTailSplit {
                head_tokens,
                tail_tokens,
            });
        }
        take_token(&mut input).ok()?;
    }
}

fn first_token_between(
    tokens: &[OwnedLexToken],
    start: usize,
    end: usize,
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> Option<usize> {
    if start >= end || end > tokens.len() {
        return None;
    }
    let mut input = LexStream::new(&tokens[start..end]);
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parser(&mut candidate).is_ok() {
            return Some(start + offset);
        }
        take_token(&mut input).ok()?;
    }
}

fn first_unquoted_token_between(
    tokens: &[OwnedLexToken],
    start: usize,
    end: usize,
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> Option<usize> {
    if start >= end || end > tokens.len() {
        return None;
    }
    let mut input = LexStream::new(&tokens[..end]);
    let initial_len = input.len();
    let mut inside_quotes = false;
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let token = take_token(&mut input).ok()?;
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if offset < start || inside_quotes {
            continue;
        }
        let mut candidate = LexStream::new(&tokens[offset..end]);
        if parser(&mut candidate).is_ok() {
            return Some(offset);
        }
    }
}

fn first_phrase(
    tokens: &[OwnedLexToken],
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> Option<(usize, usize)> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let start = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parser(&mut candidate).is_ok() {
            return Some((start, initial_len.saturating_sub(candidate.len())));
        }
        take_token(&mut input).ok()?;
    }
}

fn first_kind(tokens: &[OwnedLexToken], kind: TokenKind) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    while let Ok(token) = take_token(&mut input) {
        if token.kind == kind {
            return Some(initial_len.saturating_sub(input.len() + 1));
        }
    }
    None
}

fn last_token_before(
    tokens: &[OwnedLexToken],
    end: usize,
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> Option<usize> {
    let mut input = LexStream::new(tokens.get(..end)?);
    let initial_len = input.len();
    let mut found = None;
    while !input.is_empty() {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parser(&mut candidate).is_ok() {
            found = Some(offset);
        }
        take_token(&mut input).ok()?;
    }
    found
}

fn contains_parser<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    let mut input = LexStream::new(tokens);
    loop {
        let mut candidate = input.clone();
        if make_parser().parse_next(&mut candidate).is_ok() {
            return true;
        }
        if take_token(&mut input).is_err() {
            return false;
        }
    }
}

fn token_matches(
    token: &OwnedLexToken,
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> bool {
    let mut input = LexStream::new(std::slice::from_ref(token));
    parser(&mut input).is_ok()
}

fn trim_edge_punctuation(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
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

fn parse_get_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("get"), primitives::kw("gets")))
        .void()
        .parse_next(input)
}

fn parse_have_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("have"), primitives::kw("has")))
        .void()
        .parse_next(input)
}

fn parse_and_word(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("and").void().parse_next(input)
}

fn parse_is_word(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("is").void().parse_next(input)
}

fn parse_if_word(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("if").void().parse_next(input)
}

fn parse_as_long_as(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["as", "long", "as"])
        .void()
        .parse_next(input)
}

fn take_token<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    any.parse_next(input)
}

#[cfg(test)]
#[path = "anthem_keyword_shapes_inline_tests.rs"]
mod tests;
