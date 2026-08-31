use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, Copy)]
pub struct AttackingPlayerDrawLoseShape<'a> {
    pub draw_tokens: &'a [OwnedLexToken],
    pub lose_tokens: &'a [OwnedLexToken],
}

fn remainder<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
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

pub fn parse_attacking_player_draw_lose_shape(
    tokens: &[OwnedLexToken],
) -> Option<AttackingPlayerDrawLoseShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        attacking_draw_lose,
        "registry-attacking-draw-lose",
    )
}
