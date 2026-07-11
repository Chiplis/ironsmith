use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind, render_token_slice};
use super::primitives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionalCostWithCastTriggerShape<'a> {
    pub(crate) label_tokens: &'a [OwnedLexToken],
    pub(crate) optional_cost_effect_tokens: &'a [OwnedLexToken],
    pub(crate) followup_effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionalKeywordCostKind {
    Behold,
    Blight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionalKeywordAdditionalCostShape<'a> {
    pub(crate) kind: OptionalKeywordCostKind,
    pub(crate) cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartnerWithNameShape<'a> {
    pub(crate) name_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_partner_with_name_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PartnerWithNameShape<'_>> {
    let (name_tokens, _) = primitives::parse_prefix(tokens, parse_partner_with_name_shape_lexed)?;
    Some(PartnerWithNameShape { name_tokens })
}

pub(crate) fn parse_partner_with_name_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    let shape = parse_partner_with_name_shape_tokens(tokens)?;
    let name = render_token_slice(shape.name_tokens)
        .trim()
        .replace('"', "");
    (!name.is_empty()).then_some(name)
}

pub(crate) fn parse_partner_visible_label_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    let shape =
        primitives::parse_all(tokens, parse_partner_visible_label_lexed, "partner-label").ok()?;
    let label = match shape {
        PartnerVisibleLabelShape::Separated {
            separator: PartnerLabelSeparator::Dash,
            label_tokens,
        } => format!("Partner - {}", render_token_slice(label_tokens).trim()),
        PartnerVisibleLabelShape::Separated {
            separator: PartnerLabelSeparator::EmDash,
            label_tokens,
        } => format!("Partner—{}", render_token_slice(label_tokens).trim()),
        PartnerVisibleLabelShape::Inline(visible_tokens) => {
            render_token_slice(visible_tokens).trim().to_string()
        }
    };
    (!label.is_empty()).then_some(label)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartnerVisibleLabelShape<'a> {
    Separated {
        separator: PartnerLabelSeparator,
        label_tokens: &'a [OwnedLexToken],
    },
    Inline(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartnerLabelSeparator {
    Dash,
    EmDash,
}

pub(crate) fn parse_optional_cost_with_cast_trigger_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OptionalCostWithCastTriggerShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_optional_cost_with_cast_trigger_lexed,
        "optional-cost-with-cast-trigger",
    )
    .ok()
}

pub(crate) fn parse_optional_keyword_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OptionalKeywordAdditionalCostShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_optional_keyword_additional_cost_lexed,
        "optional-keyword-additional-cost",
    )
    .ok()
}

fn parse_partner_with_name_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    primitives::phrase(&["partner", "with"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((
            primitives::token_kind(TokenKind::LParen).void(),
            primitives::token_kind(TokenKind::Period).void(),
            eof.value(()),
        ))),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)
}

fn parse_partner_visible_label_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PartnerVisibleLabelShape<'a>> {
    let shape = alt((
        (
            primitives::kw("partner").void(),
            alt((
                primitives::token_kind(TokenKind::Dash).value(PartnerLabelSeparator::Dash),
                primitives::token_kind(TokenKind::EmDash).value(PartnerLabelSeparator::EmDash),
            )),
            repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(partner_label_visible_end))
                .void()
                .take(),
        )
            .map(
                |(_, separator, label_tokens)| PartnerVisibleLabelShape::Separated {
                    separator,
                    label_tokens,
                },
            ),
        (
            inline_partner_label_head,
            repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(partner_label_visible_end))
                .void(),
        )
            .take()
            .map(PartnerVisibleLabelShape::Inline),
    ))
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(eof))
        .void()
        .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(shape)
}

fn partner_label_visible_end(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::token_kind(TokenKind::LParen).void(),
        primitives::token_kind(TokenKind::Period).void(),
        eof.value(()),
    ))
    .parse_next(input)
}

fn inline_partner_label_head(input: &mut LexStream<'_>) -> WResult<()> {
    any.verify(|token: &OwnedLexToken| {
        matches!(
            token.parser_word_pieces(),
            [partner, variant, ..] if partner.text == "partner" && !variant.text.is_empty()
        )
    })
    .void()
    .parse_next(input)
}

fn parse_optional_cost_with_cast_trigger_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<OptionalCostWithCastTriggerShape<'a>> {
    primitives::phrase(&[
        "as",
        "an",
        "additional",
        "cost",
        "to",
        "cast",
        "this",
        "spell",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;

    let label_tokens = (
        primitives::phrase(&["you", "may"]),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::period())).void(),
    )
        .take()
        .parse_next(input)?;
    let (_, optional_cost_effect_tokens) =
        primitives::parse_prefix(label_tokens, primitives::phrase(&["you", "may"]))
            .ok_or_else(|| primitives::backtrack_err("optional additional cost", "you may"))?;
    primitives::period().parse_next(input)?;
    primitives::phrase(&["when", "you", "do"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let followup_effect_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(OptionalCostWithCastTriggerShape {
        label_tokens,
        optional_cost_effect_tokens,
        followup_effect_tokens,
    })
}

fn parse_optional_keyword_additional_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<OptionalKeywordAdditionalCostShape<'a>> {
    primitives::phrase(&[
        "as",
        "an",
        "additional",
        "cost",
        "to",
        "cast",
        "this",
        "spell",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["you", "may"]).parse_next(input)?;
    let cost_tokens = (
        alt((
            primitives::kw("behold").value(OptionalKeywordCostKind::Behold),
            primitives::kw("blight").value(OptionalKeywordCostKind::Blight),
        )),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .void(),
    )
        .take()
        .parse_next(input)?;
    let kind = if cost_tokens
        .first()
        .is_some_and(|token| token.is_word("behold"))
    {
        OptionalKeywordCostKind::Behold
    } else {
        OptionalKeywordCostKind::Blight
    };
    primitives::sentence_end().parse_next(input)?;
    Ok(OptionalKeywordAdditionalCostShape { kind, cost_tokens })
}

#[cfg(test)]
#[path = "keyword_special_lines/tests.rs"]
mod tests;
