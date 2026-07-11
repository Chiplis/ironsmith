use super::*;
use winnow::combinator::{alt, eof, opt};
use winnow::prelude::*;
use winnow::token::take_till;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SerialDamageFanout {
    pub(crate) source: Vec<OwnedLexToken>,
    pub(crate) parts: Vec<SerialDamagePart>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SerialDamagePart {
    pub(crate) amount: Value,
    pub(crate) target_tokens: Vec<OwnedLexToken>,
}

pub(crate) fn parse_serial_damage_fanout_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<SerialDamageFanout>, CardTextError> {
    primitives::parse_all_or_none(
        tokens,
        parse_serial_damage_fanout_lexed,
        "serial-damage-fanout",
    )
}

fn parse_serial_damage_fanout_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<SerialDamageFanout, ErrMode<ContextError>> {
    let source = take_till(0.., |token: &OwnedLexToken| {
        token.is_any_word(&["deal", "deals"])
    })
    .parse_next(input)?
    .to_vec();
    alt((primitives::kw("deal"), primitives::kw("deals")))
        .void()
        .parse_next(input)?;

    let first = parse_serial_damage_part_lexed.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let second = parse_serial_damage_part_lexed.parse_next(input)?;
    primitives::comma().parse_next(input)?;
    opt(primitives::kw("and")).parse_next(input)?;
    let third = parse_serial_damage_part_lexed.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;

    Ok(SerialDamageFanout {
        source,
        parts: vec![first, second, third],
    })
}

fn parse_serial_damage_part_lexed<'a>(
    input: &mut LexStream<'a>,
) -> Result<SerialDamagePart, ErrMode<ContextError>> {
    let amount = super::super::leaf::parse_leaf_modal_value_token.parse_next(input)?;
    primitives::kw("damage").parse_next(input)?;
    opt(primitives::kw("to")).parse_next(input)?;
    let target_tokens = take_till(1.., |token: &OwnedLexToken| {
        matches!(token.kind, TokenKind::Comma | TokenKind::Period)
    })
    .parse_next(input)?
    .to_vec();

    Ok(SerialDamagePart {
        amount,
        target_tokens,
    })
}
