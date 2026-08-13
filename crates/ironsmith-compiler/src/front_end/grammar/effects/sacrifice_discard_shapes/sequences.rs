use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::Value;
use crate::grammar::{leaf, primitives};
use crate::front_end::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EachPlayerMayDiscardHandAndDrawShape {
    pub(crate) draw_count: Value,
}

fn card_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)
}

fn each_player_may_discard_hand_and_draw(
    input: &mut LexStream<'_>,
) -> WResult<EachPlayerMayDiscardHandAndDrawShape> {
    primitives::phrase(&["each", "player", "may", "discard", "their", "hand"]).parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    primitives::kw("draw").parse_next(input)?;
    let draw_count = leaf::parse_leaf_number_or_x_prefix_lexed
        .parse_next(input)?
        .into_value()
        .ok_or_else(|| primitives::backtrack_err("draw count", "representable value"))?;
    card_word.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(EachPlayerMayDiscardHandAndDrawShape { draw_count })
}

pub(crate) fn parse_each_player_may_discard_hand_and_draw_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EachPlayerMayDiscardHandAndDrawShape> {
    primitives::parse_all(
        tokens,
        each_player_may_discard_hand_and_draw,
        "each-player optional hand wheel",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_each_player_optional_discard_and_draw_as_one_typed_scope() {
        let tokens = lex_line(
            "Each player may discard their hand and draw seven cards.",
            0,
        )
        .unwrap();
        let shape = parse_each_player_may_discard_hand_and_draw_tokens(&tokens).unwrap();
        assert_eq!(shape.draw_count, Value::Fixed(7));
    }
}
