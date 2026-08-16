use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::target::PlayerFilter;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

/// A life-loss trigger whose multiplicity is evaluated across a group of
/// players for each game event, rather than once for each individual player.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayersLoseLifeOneOrMoreClause {
    pub(crate) player: PlayerFilter,
}

fn parse_players_lose_life_one_or_more_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PlayersLoseLifeOneOrMoreClause> {
    primitives::phrase(&["one", "or", "more"]).parse_next(input)?;
    alt((primitives::kw("opponent"), primitives::kw("opponents"))).parse_next(input)?;
    alt((primitives::kw("lose"), primitives::kw("loses"))).parse_next(input)?;
    primitives::kw("life").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(PlayersLoseLifeOneOrMoreClause {
        player: PlayerFilter::Opponent,
    })
}

pub(crate) fn parse_players_lose_life_one_or_more_clause(
    tokens: &[OwnedLexToken],
) -> Option<PlayersLoseLifeOneOrMoreClause> {
    primitives::parse_all(
        tokens,
        parse_players_lose_life_one_or_more_lexed,
        "one-or-more players lose life trigger",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex_line;

    use super::*;

    #[test]
    fn parses_grouped_opponent_life_loss() {
        for text in [
            "one or more opponents lose life",
            "one or more opponent loses life.",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let parsed = parse_players_lose_life_one_or_more_clause(&tokens).unwrap();
            assert_eq!(parsed.player, PlayerFilter::Opponent);
        }
    }

    #[test]
    fn does_not_collapse_ordinary_player_life_loss() {
        for text in ["an opponent loses life", "you lose life"] {
            let tokens = lex_line(text, 0).unwrap();
            assert!(parse_players_lose_life_one_or_more_clause(&tokens).is_none());
        }
    }
}
