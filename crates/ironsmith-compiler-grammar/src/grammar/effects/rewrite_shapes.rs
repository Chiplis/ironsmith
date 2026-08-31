use super::*;

use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::token::{any, take_till};

#[derive(Debug, Clone, PartialEq)]
pub struct EachPlayerCreaturesDamageShape {
    pub amount: Value,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompoundBuffUnblockableShape<'a> {
    pub buff_tokens: &'a [OwnedLexToken],
    pub subject_tokens: &'a [OwnedLexToken],
    pub unblockable_tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CantBlockedBasePowerToughnessShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub power: Value,
    pub toughness: Value,
}

fn parse_each_player_creatures_damage_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EachPlayerCreaturesDamageShape> {
    let _: &[OwnedLexToken] = take_till(0.., |token: &OwnedLexToken| {
        token.is_any_word(&["deal", "deals"])
    })
    .parse_next(input)?;
    alt((primitives::kw("deal"), primitives::kw("deals")))
        .void()
        .parse_next(input)?;
    let amount = super::super::leaf::parse_leaf_modal_value_token.parse_next(input)?;
    primitives::phrase(&["damage", "to", "each", "player", "and", "each"]).parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures")))
        .void()
        .parse_next(input)?;
    alt((
        primitives::phrase(&["they", "control"]),
        primitives::phrase(&["that", "player", "controls"]),
    ))
    .void()
    .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(EachPlayerCreaturesDamageShape { amount })
}

pub fn parse_each_player_creatures_damage_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerCreaturesDamageShape> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_each_player_creatures_damage_lexed,
        "each-player-creatures-damage",
    )
}

fn parse_unblockable_conjunction_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::kw("and").parse_next(input)?;
    alt((primitives::kw("can't"), primitives::kw("cant")))
        .void()
        .parse_next(input)?;
    primitives::phrase(&["be", "blocked"]).parse_next(input)
}

fn parse_compound_buff_unblockable_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CompoundBuffUnblockableShape<'a>> {
    let (subject_tokens, buff_tokens) = (
        repeat_till(1.., any.void(), peek(primitives::kw("gets")))
            .map(|((), _)| ())
            .take(),
        primitives::kw("gets"),
        repeat_till(1.., any.void(), peek(parse_unblockable_conjunction_lexed)).map(|((), _)| ()),
    )
        .map(|(subject_tokens, _, ())| subject_tokens)
        .with_taken()
        .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let unblockable_tail_tokens = (
        alt((primitives::kw("can't"), primitives::kw("cant"))).void(),
        primitives::phrase(&["be", "blocked"]),
    )
        .take()
        .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;

    Ok(CompoundBuffUnblockableShape {
        buff_tokens,
        subject_tokens,
        unblockable_tail_tokens,
    })
}

pub fn parse_compound_buff_unblockable_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CompoundBuffUnblockableShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_compound_buff_unblockable_lexed,
        "compound-buff-unblockable",
    )
}

#[cfg(test)]
#[path = "rewrite_shapes_inline_tests.rs"]
mod tests;

#[path = "rewrite_shapes/combat.rs"]
mod combat_programs;
pub use combat_programs::parse_cant_blocked_base_power_toughness_tokens;
use combat_programs::{parse_cant_be_blocked, parse_cant_blocked_base_power_toughness_lexed};
