use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::PlayerAst;
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone)]
pub struct JointDrawShape<'a> {
    pub other_player: PlayerAst,
    pub another_target_player: bool,
    pub amount_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub struct JointLifeShape<'a> {
    pub other_player: PlayerAst,
    pub gains: bool,
    pub amount_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub struct JointCreateShape<'a> {
    pub other_player: PlayerAst,
    pub effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub struct JointSacrificeShape<'a> {
    pub other_player: PlayerAst,
    pub object_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub struct JointObjectEachActionsShape<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub tagged_tokens: &'a [OwnedLexToken],
    pub action_tokens: &'a [OwnedLexToken],
}

fn player_object<'a>(input: &mut LexStream<'a>) -> WResult<(PlayerAst, bool)> {
    alt((
        (
            primitives::kw("another"),
            primitives::kw("target"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value((PlayerAst::Target, true)),
        (
            primitives::kw("target"),
            alt((primitives::kw("opponent"), primitives::kw("opponents"))),
        )
            .value((PlayerAst::TargetOpponent, false)),
        (
            primitives::kw("target"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value((PlayerAst::Target, false)),
        (
            primitives::kw("that"),
            alt((primitives::kw("player"), primitives::kw("players"))),
        )
            .value((PlayerAst::That, false)),
    ))
    .parse_next(input)
}

fn joint_prefix<'a>(input: &mut LexStream<'a>) -> WResult<(PlayerAst, bool)> {
    primitives::phrase(&["you", "and"]).parse_next(input)?;
    let player = player_object.parse_next(input)?;
    opt(primitives::kw("each")).parse_next(input)?;
    Ok(player)
}

fn remainder<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn joint_draw<'a>(input: &mut LexStream<'a>) -> WResult<JointDrawShape<'a>> {
    let (other_player, another_target_player) = joint_prefix.parse_next(input)?;
    alt((primitives::kw("draw"), primitives::kw("draws"))).parse_next(input)?;
    let amount_tokens = remainder.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(JointDrawShape {
        other_player,
        another_target_player,
        amount_tokens,
    })
}

fn joint_life<'a>(input: &mut LexStream<'a>) -> WResult<JointLifeShape<'a>> {
    let (other_player, _) = joint_prefix.parse_next(input)?;
    let gains = alt((
        alt((primitives::kw("gain"), primitives::kw("gains"))).value(true),
        alt((primitives::kw("lose"), primitives::kw("loses"))).value(false),
    ))
    .parse_next(input)?;
    let amount_tokens = remainder.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(JointLifeShape {
        other_player,
        gains,
        amount_tokens,
    })
}

fn joint_create<'a>(input: &mut LexStream<'a>) -> WResult<JointCreateShape<'a>> {
    let (other_player, _) = joint_prefix.parse_next(input)?;
    let effect_tokens = (
        alt((primitives::kw("create"), primitives::kw("creates"))),
        remainder,
    )
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(JointCreateShape {
        other_player,
        effect_tokens,
    })
}

fn joint_sacrifice<'a>(input: &mut LexStream<'a>) -> WResult<JointSacrificeShape<'a>> {
    let (other_player, _) = joint_prefix.parse_next(input)?;
    alt((primitives::kw("sacrifice"), primitives::kw("sacrifices"))).parse_next(input)?;
    let object_tokens = remainder.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(JointSacrificeShape {
        other_player,
        object_tokens,
    })
}

fn permanent_kind<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    alt((
        primitives::kw("artifact"),
        primitives::kw("battle"),
        primitives::kw("creature"),
        primitives::kw("enchantment"),
        primitives::kw("land"),
        primitives::kw("permanent"),
        primitives::kw("planeswalker"),
    ))
    .parse_next(input)
}

fn joint_object_each_actions<'a>(
    input: &mut LexStream<'a>,
) -> WResult<JointObjectEachActionsShape<'a>> {
    let source_tokens = (primitives::kw("this"), permanent_kind)
        .take()
        .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let tagged_tokens = (primitives::kw("that"), permanent_kind)
        .take()
        .parse_next(input)?;
    primitives::kw("each").parse_next(input)?;
    let action_tokens = remainder.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(JointObjectEachActionsShape {
        source_tokens,
        tagged_tokens,
        action_tokens,
    })
}

#[path = "joint/entrypoints.rs"]
mod entrypoints;
pub use entrypoints::*;

pub fn parse_joint_object_each_actions_shape(
    tokens: &[OwnedLexToken],
) -> Option<JointObjectEachActionsShape<'_>> {
    let shape = primitives::parse_all(
        tokens,
        joint_object_each_actions,
        "registry-joint-object-each-actions",
    )
    .ok()?;
    let source_kind = shape.source_tokens.get(1)?.as_word()?;
    let tagged_kind = shape.tagged_tokens.get(1)?.as_word()?;
    source_kind
        .eq_ignore_ascii_case(tagged_kind)
        .then_some(shape)
}

#[cfg(test)]
#[path = "joint_inline_tests.rs"]
mod tests;
