use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind, render_token_slice};
use super::primitives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalCostWithCastTriggerShape<'a> {
    pub label_tokens: &'a [OwnedLexToken],
    pub optional_cost_effect_tokens: &'a [OwnedLexToken],
    pub followup_effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionalKeywordCostKind {
    Behold,
    Blight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalKeywordAdditionalCostShape<'a> {
    pub kind: OptionalKeywordCostKind,
    pub cost_tokens: &'a [OwnedLexToken],
    pub behold_subtype: Option<crate::types::Subtype>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeholdAndExileAdditionalCostShape {
    pub subtype: crate::types::Subtype,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartnerWithNameShape<'a> {
    pub name_tokens: &'a [OwnedLexToken],
}

pub fn parse_partner_with_name_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PartnerWithNameShape<'_>> {
    let (name_tokens, _) = primitives::parse_prefix(tokens, parse_partner_with_name_shape_lexed)?;
    Some(PartnerWithNameShape { name_tokens })
}

pub fn parse_partner_with_name_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    let shape = parse_partner_with_name_shape_tokens(tokens)?;
    let name = render_token_slice(shape.name_tokens)
        .trim()
        .replace('"', "");
    (!name.is_empty()).then_some(name)
}

pub fn parse_partner_visible_label_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
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

pub fn parse_optional_cost_with_cast_trigger_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OptionalCostWithCastTriggerShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_optional_cost_with_cast_trigger_lexed,
        "optional-cost-with-cast-trigger",
    )
    .ok()
}

pub fn parse_optional_keyword_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OptionalKeywordAdditionalCostShape<'_>> {
    let mut shape = primitives::parse_all(
        tokens,
        parse_optional_keyword_additional_cost_lexed,
        "optional-keyword-additional-cost",
    )
    .ok()?;
    if shape.kind == OptionalKeywordCostKind::Behold {
        let parsed =
            super::activation_costs::parse_behold_segment_tokens(shape.cost_tokens).ok()?;
        let super::activation_costs::ActivationCostSegmentCst::Behold { subtype, .. } = parsed
        else {
            return None;
        };
        shape.behold_subtype = Some(subtype);
    }
    Some(shape)
}

pub fn parse_behold_and_exile_additional_cost_tokens(
    tokens: &[OwnedLexToken],
) -> Option<BeholdAndExileAdditionalCostShape> {
    let behold_tokens = primitives::parse_all(
        tokens,
        parse_behold_and_exile_additional_cost_lexed,
        "behold-and-exile-additional-cost",
    )
    .ok()?;
    let parsed = super::activation_costs::parse_behold_segment_tokens(behold_tokens).ok()?;
    let super::activation_costs::ActivationCostSegmentCst::Behold { subtype, count } = parsed
    else {
        return None;
    };
    (count == 1).then_some(BeholdAndExileAdditionalCostShape { subtype })
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

#[cfg(test)]
#[path = "keyword_special_lines/tests.rs"]
mod tests;

#[path = "keyword_special_lines/resource.rs"]
mod resource_programs;
use resource_programs::{
    parse_behold_and_exile_additional_cost_lexed, parse_optional_keyword_additional_cost_lexed,
};
#[path = "keyword_special_lines/trigger.rs"]
mod trigger_programs;
use trigger_programs::parse_optional_cost_with_cast_trigger_lexed;
