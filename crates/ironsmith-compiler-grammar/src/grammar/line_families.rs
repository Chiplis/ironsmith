use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::attached_object_static_lines;
use super::leaf;
use super::primitives;
use super::structure;
use crate::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};

mod statement_shapes;
pub use statement_shapes::*;
mod document_dispatch;
pub use document_dispatch::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSplitShape<'a> {
    pub before: &'a [OwnedLexToken],
    pub after: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxSpeedBodyShape<'a> {
    pub body_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StationThresholdShape<'a> {
    pub threshold: i32,
    pub body_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KickerBranchShape<'a> {
    pub first_cost: &'a [OwnedLexToken],
    pub second_cost: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickerTicketMarkerShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveCounterPreventionThenTriggerShape<'a> {
    pub prevention_tokens: &'a [OwnedLexToken],
    pub prevention: attached_object_static_lines::RemoveCounterPreventionSpec<'a>,
    pub trigger_tokens: &'a [OwnedLexToken],
}

pub fn parse_comma_split(tokens: &[OwnedLexToken]) -> Option<TokenSplitShape<'_>> {
    let (index, _, _) = primitives::find_prefix(tokens, || primitives::comma().void())?;
    Some(TokenSplitShape {
        before: tokens.get(..index)?,
        after: tokens.get(index + 1..)?,
    })
}

pub fn parse_max_speed_body(tokens: &[OwnedLexToken]) -> Option<MaxSpeedBodyShape<'_>> {
    let (index, _, _) = primitives::find_prefix(tokens, || {
        alt((
            primitives::token_kind(TokenKind::Dash),
            primitives::token_kind(TokenKind::EmDash),
            primitives::token_kind(TokenKind::Colon),
        ))
        .void()
    })?;
    let body_tokens = tokens.get(index + 1..)?;
    (!TokenWordView::new(body_tokens).is_empty()).then_some(MaxSpeedBodyShape { body_tokens })
}

pub fn parse_visible_line_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut inside_double_quotes = false;
    let mut inside_single_quotes = false;

    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Quote => inside_double_quotes = !inside_double_quotes,
            TokenKind::Apostrophe => inside_single_quotes = !inside_single_quotes,
            TokenKind::LParen | TokenKind::Period
                if !inside_double_quotes && !inside_single_quotes =>
            {
                return &tokens[..index];
            }
            _ => {}
        }
    }

    tokens
}

pub fn parse_max_speed_trigger_split(tokens: &[OwnedLexToken]) -> Option<TokenSplitShape<'_>> {
    let visible = parse_visible_max_speed_tokens(tokens);
    let visible = if visible
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
        && visible
            .iter()
            .filter(|token| token.kind == TokenKind::Period)
            .count()
            == 1
    {
        &visible[..visible.len().saturating_sub(1)]
    } else {
        visible
    };
    parse_comma_split(visible)
}

pub fn parse_visible_max_speed_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut input = LexStream::new(tokens);
    visible_max_speed_tokens(&mut input).unwrap_or(tokens)
}

pub fn parse_station_threshold(tokens: &[OwnedLexToken]) -> Option<StationThresholdShape<'_>> {
    let (pipe, _, _) =
        primitives::find_prefix(tokens, || primitives::token_kind(TokenKind::Pipe).void())?;
    let [threshold_token, plus_token] = tokens.get(..pipe)? else {
        return None;
    };
    if !matches!(threshold_token.kind, TokenKind::Number | TokenKind::Word)
        || plus_token.kind != TokenKind::Plus
    {
        return None;
    }
    let threshold = crate::grammar::primitives::probe_shape(leaf::parse_number_i32_complete(
        threshold_token.parser_text(),
    ))?;
    let body_tokens = trim_commas(tokens.get(pipe + 1..)?);
    (!TokenWordView::new(body_tokens).is_empty()).then_some(StationThresholdShape {
        threshold,
        body_tokens,
    })
}

pub fn parse_station_creature_threshold(tokens: &[OwnedLexToken]) -> Option<i32> {
    primitives::find_prefix(tokens, || station_creature_threshold)
        .map(|(_, threshold, _)| threshold)
}

pub fn parse_sticker_ticket_marker(tokens: &[OwnedLexToken]) -> Option<StickerTicketMarkerShape> {
    let (dash, _, _) =
        primitives::find_prefix(tokens, || primitives::token_kind(TokenKind::EmDash).void())?;
    let cost_tokens = tokens.get(..dash)?;
    let body_tokens = tokens.get(dash + 1..)?;
    if cost_tokens.is_empty() || TokenWordView::new(body_tokens).is_empty() {
        return None;
    }
    let mut input = LexStream::new(cost_tokens);
    let _: Vec<()> = crate::grammar::primitives::take_leaf(
        &mut input,
        winnow::combinator::repeat(1.., ticket_symbol),
    )?;
    crate::grammar::primitives::take_leaf(&mut input, eof.void())?;
    Some(StickerTicketMarkerShape)
}

pub fn parse_partner_variant(
    tokens: &[OwnedLexToken],
) -> Option<super::semantic_lowering::PartnerVariantLabel> {
    super::semantic_lowering::parse_partner_variant_label_tokens(tokens)
}

pub fn parse_kicker_branches(tokens: &[OwnedLexToken]) -> Option<KickerBranchShape<'_>> {
    let (_, mut tail) = primitives::parse_prefix(tokens, primitives::kw("kicker"))?;
    if tail
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
    {
        tail = tail.get(1..)?;
    }
    let mut input = LexStream::new(tail);
    let cost_tokens = crate::grammar::primitives::take_leaf(&mut input, kicker_cost_tokens)?;
    let (separator, _, _) =
        primitives::find_prefix(cost_tokens, || primitives::kw("and/or").void())?;
    let first_cost = trim_commas(cost_tokens.get(..separator)?);
    let second_cost = trim_commas(cost_tokens.get(separator + 1..)?);
    if first_cost.is_empty() || second_cost.is_empty() {
        return None;
    }
    Some(KickerBranchShape {
        first_cost,
        second_cost,
    })
}

pub fn parse_remove_counter_prevention_then_trigger(
    tokens: &[OwnedLexToken],
) -> Option<RemoveCounterPreventionThenTriggerShape<'_>> {
    let sentences = structure::split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    let prevention_tokens = *sentences.first()?;
    let trigger_tokens = *sentences.get(1)?;
    let prevention =
        attached_object_static_lines::parse_remove_counter_prevention_tokens(prevention_tokens)?;
    primitives::parse_prefix(
        trigger_tokens,
        alt((
            primitives::kw("when"),
            primitives::kw("whenever"),
            primitives::kw("at"),
        ))
        .void(),
    )?;
    Some(RemoveCounterPreventionThenTriggerShape {
        prevention_tokens,
        prevention,
        trigger_tokens,
    })
}

fn visible_max_speed_tokens<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(
        0..,
        any.void(),
        peek(alt((
            primitives::token_kind(TokenKind::LParen).void(),
            eof.void(),
        ))),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)
}

fn kicker_cost_tokens<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(
        0..,
        any.void(),
        peek(alt((
            primitives::phrase(&["you", "may", "pay"]).void(),
            primitives::phrase(&["you", "may"]).void(),
            primitives::token_kind(TokenKind::Period).void(),
            primitives::token_kind(TokenKind::LParen).void(),
            eof.void(),
        ))),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)
}

fn station_creature_threshold(input: &mut LexStream<'_>) -> WResult<i32> {
    primitives::phrase(&["artifact", "creature", "at"]).parse_next(input)?;
    let threshold = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::token_kind(TokenKind::Plus).parse_next(input)?;
    i32::try_from(threshold)
        .map_err(|_| primitives::backtrack_err("station threshold", "32-bit number"))
}

fn ticket_symbol(input: &mut LexStream<'_>) -> WResult<()> {
    let token: &OwnedLexToken = any.parse_next(input)?;
    if token.kind != TokenKind::ManaGroup {
        return Err(primitives::backtrack_err(
            "ticket marker",
            "ticket mana group",
        ));
    }
    let mut symbol = token.parser_text();
    winnow::ascii::Caseless("{tk}")
        .void()
        .parse_next(&mut symbol)
}

fn trim_commas(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

#[cfg(test)]
mod tests;
