use std::ops::Range;

use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::primitives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeadingUnlessClauseSplit {
    pub(crate) condition: Range<usize>,
    pub(crate) effect: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnlessPaymentKind {
    Cost,
    LifeEqualToItsToughness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnlessPaysShape<'a> {
    pub(crate) player_tokens: &'a [OwnedLexToken],
    pub(crate) payment_tokens: &'a [OwnedLexToken],
    pub(crate) kind: UnlessPaymentKind,
}

fn trim_payment_edges(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
        )
    {
        end -= 1;
    }
    &tokens[start..end]
}

pub(crate) fn parse_unless_pays_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<UnlessPaysShape<'_>> {
    let tokens = trim_payment_edges(tokens);
    let tokens = primitives::parse_prefix(tokens, primitives::kw("unless"))
        .map(|(_, rest)| rest)
        .unwrap_or(tokens);
    let (pays_idx, (), payment_tokens) = primitives::find_prefix(tokens, || {
        winnow::combinator::alt((primitives::kw("pay"), primitives::kw("pays"))).void()
    })?;
    let player_tokens = trim_payment_edges(&tokens[..pays_idx]);
    let payment_tokens = trim_payment_edges(payment_tokens);
    if player_tokens.is_empty() || payment_tokens.is_empty() {
        return None;
    }
    let kind = if primitives::parse_all(
        payment_tokens,
        primitives::phrase(&["life", "equal", "to", "its", "toughness"]),
        "unless payment life equal to referenced toughness",
    )
    .is_ok()
    {
        UnlessPaymentKind::LifeEqualToItsToughness
    } else {
        UnlessPaymentKind::Cost
    };
    Some(UnlessPaysShape {
        player_tokens,
        payment_tokens,
        kind,
    })
}

pub(crate) fn parse_leading_unless_clause_split_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeadingUnlessClauseSplit> {
    primitives::parse_prefix(tokens, primitives::kw("unless"))?;
    let mut input = LexStream::new(tokens);
    let boundary = parse_unless_effect_boundary_lexed
        .parse_next(&mut input)
        .ok()?;
    Some(LeadingUnlessClauseSplit {
        condition: 0..boundary,
        effect: boundary + usize::from(tokens[boundary].kind == TokenKind::Comma)..tokens.len(),
    })
}

fn parse_unless_effect_boundary_lexed<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let initial_len = input.len();
    let mut saw_payment = false;
    loop {
        let index = initial_len.saturating_sub(input.len());
        let token: &OwnedLexToken = any.parse_next(input)?;
        if token.kind == TokenKind::Comma {
            return Ok(index);
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "pay" | "pays") {
            saw_payment = true;
        } else if saw_payment && word == "search" {
            return Ok(index);
        }
    }
}

#[cfg(test)]
#[path = "unless_clause/tests.rs"]
mod tests;
