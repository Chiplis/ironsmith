use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::PlayerAst;
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone)]
pub(crate) struct JointDrawShape<'a> {
    pub(crate) other_player: PlayerAst,
    pub(crate) another_target_player: bool,
    pub(crate) amount_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub(crate) struct JointLifeShape<'a> {
    pub(crate) other_player: PlayerAst,
    pub(crate) gains: bool,
    pub(crate) amount_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub(crate) struct JointCreateShape<'a> {
    pub(crate) other_player: PlayerAst,
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub(crate) struct JointSacrificeShape<'a> {
    pub(crate) other_player: PlayerAst,
    pub(crate) object_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AttackingPlayerDrawLoseShape<'a> {
    pub(crate) draw_tokens: &'a [OwnedLexToken],
    pub(crate) lose_tokens: &'a [OwnedLexToken],
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

fn attacking_draw_lose<'a>(input: &mut LexStream<'a>) -> WResult<AttackingPlayerDrawLoseShape<'a>> {
    primitives::phrase(&["you", "and"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["attacking", "player"]).parse_next(input)?;
    opt(primitives::kw("each")).parse_next(input)?;
    alt((primitives::kw("draw"), primitives::kw("draws"))).parse_next(input)?;
    let draw_tokens = repeat_till(1.., any.void(), peek(primitives::kw("and")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    alt((primitives::kw("lose"), primitives::kw("loses"))).parse_next(input)?;
    let lose_tokens = remainder.parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(AttackingPlayerDrawLoseShape {
        draw_tokens,
        lose_tokens,
    })
}

pub(crate) fn parse_joint_draw_shape(tokens: &[OwnedLexToken]) -> Option<JointDrawShape<'_>> {
    primitives::parse_all(tokens, joint_draw, "registry-joint-draw").ok()
}

pub(crate) fn parse_joint_life_shape(tokens: &[OwnedLexToken]) -> Option<JointLifeShape<'_>> {
    primitives::parse_all(tokens, joint_life, "registry-joint-life").ok()
}

pub(crate) fn parse_joint_create_shape(tokens: &[OwnedLexToken]) -> Option<JointCreateShape<'_>> {
    primitives::parse_all(tokens, joint_create, "registry-joint-create").ok()
}

pub(crate) fn parse_joint_sacrifice_shape(
    tokens: &[OwnedLexToken],
) -> Option<JointSacrificeShape<'_>> {
    primitives::parse_all(tokens, joint_sacrifice, "registry-joint-sacrifice").ok()
}

pub(crate) fn parse_attacking_player_draw_lose_shape(
    tokens: &[OwnedLexToken],
) -> Option<AttackingPlayerDrawLoseShape<'_>> {
    primitives::parse_all(tokens, attacking_draw_lose, "registry-attacking-draw-lose").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

    #[test]
    fn parses_joint_player_actions() {
        let draw = lex_line("You and that player each draw two cards.", 0).unwrap();
        let shape = parse_joint_draw_shape(&draw).unwrap();
        assert_eq!(shape.other_player, PlayerAst::That);
        assert_eq!(
            TokenWordView::new(shape.amount_tokens).to_word_refs(),
            vec!["two", "cards"]
        );

        let create = lex_line("You and target opponent each create a Treasure token.", 0).unwrap();
        assert_eq!(
            TokenWordView::new(parse_joint_create_shape(&create).unwrap().effect_tokens)
                .to_word_refs(),
            vec!["create", "a", "treasure", "token"]
        );

        let sacrifice = lex_line("You and that player each sacrifice a creature.", 0).unwrap();
        let sacrifice = parse_joint_sacrifice_shape(&sacrifice).unwrap();
        assert_eq!(sacrifice.other_player, PlayerAst::That);
        assert_eq!(
            TokenWordView::new(sacrifice.object_tokens).to_word_refs(),
            vec!["a", "creature"]
        );
    }
}
