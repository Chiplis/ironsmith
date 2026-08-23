use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{leaf, primitives};
use crate::target::PlayerFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLifeTieConditionShape {
    pub minimum_players: u32,
    pub tied_players: PlayerFilter,
}

#[derive(Debug, Clone)]
pub struct PlayerLifeTieChoiceConditionalShape<'a> {
    pub minimum_players: u32,
    pub tied_players: PlayerFilter,
    pub consequence_tokens: &'a [OwnedLexToken],
}

fn parse_life_extreme(input: &mut LexStream<'_>) -> WResult<PlayerFilter> {
    alt((
        primitives::phrase(&["lowest", "life", "total"]).value(PlayerFilter::LowestLifeTied),
        primitives::phrase(&["most", "life", "total"]).value(PlayerFilter::MostLifeTied),
    ))
    .parse_next(input)
}

fn parse_player_life_tie_condition_core(
    input: &mut LexStream<'_>,
) -> WResult<PlayerLifeTieConditionShape> {
    let minimum_players = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    if minimum_players < 2 {
        return Err(primitives::backtrack_err(
            "life-total tie count",
            "two or more players",
        ));
    }
    primitives::phrase(&["or", "more", "players", "are", "tied", "for"])
        .void()
        .parse_next(input)?;
    let tied_players = parse_life_extreme.parse_next(input)?;
    Ok(PlayerLifeTieConditionShape {
        minimum_players,
        tied_players,
    })
}

fn parse_player_life_tie_condition_lexed(
    input: &mut LexStream<'_>,
) -> WResult<PlayerLifeTieConditionShape> {
    let condition = parse_player_life_tie_condition_core(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(condition)
}

fn parse_player_life_tie_choice_conditional_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PlayerLifeTieChoiceConditionalShape<'a>> {
    primitives::kw("if").void().parse_next(input)?;
    let condition = parse_player_life_tie_condition_core(input)?;
    primitives::comma().void().parse_next(input)?;
    primitives::phrase(&["you", "choose", "one", "of", "them"])
        .void()
        .parse_next(input)?;
    primitives::comma().void().parse_next(input)?;
    opt(primitives::kw("and")).parse_next(input)?;
    let consequence_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PlayerLifeTieChoiceConditionalShape {
        minimum_players: condition.minimum_players,
        tied_players: condition.tied_players,
        consequence_tokens,
    })
}

pub fn parse_player_life_tie_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeTieConditionShape> {
    primitives::parse_all(
        tokens,
        parse_player_life_tie_condition_lexed,
        "player life-total tie condition",
    )
    .ok()
}

pub fn parse_player_life_tie_choice_conditional_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeTieChoiceConditionalShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_player_life_tie_choice_conditional_lexed,
        "player life-total tie choice conditional",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_typed_player_life_tie_conditions() {
        let lowest = lex_line("Two or more players are tied for lowest life total", 0).unwrap();
        assert_eq!(
            parse_player_life_tie_condition_tokens(&lowest),
            Some(PlayerLifeTieConditionShape {
                minimum_players: 2,
                tied_players: PlayerFilter::LowestLifeTied,
            })
        );

        let most = lex_line("Three or more players are tied for most life total.", 0).unwrap();
        assert_eq!(
            parse_player_life_tie_condition_tokens(&most),
            Some(PlayerLifeTieConditionShape {
                minimum_players: 3,
                tied_players: PlayerFilter::MostLifeTied,
            })
        );

        let choice = lex_line(
            "If two or more players are tied for lowest life total, you choose one of them, and that player gains control of this creature.",
            0,
        )
        .unwrap();
        let shape = parse_player_life_tie_choice_conditional_tokens(&choice).unwrap();
        assert_eq!(shape.minimum_players, 2);
        assert_eq!(shape.tied_players, PlayerFilter::LowestLifeTied);
        assert_eq!(
            super::super::super::super::lexer::render_token_slice(shape.consequence_tokens),
            "that player gains control of this creature"
        );
    }
}
