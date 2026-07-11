use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, TokenWordView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExileOwnerSurface {
    You,
    Their,
    ThatPlayer,
    TargetPlayer,
    TargetOpponent,
    ItsController,
    ItsOwner,
    HisOrHer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ParsedExileOwnerPrefix {
    pub(crate) player: PlayerAst,
    pub(crate) consumed_words: usize,
}

fn player_owner_surface(input: &mut LexStream<'_>) -> WResult<ExileOwnerSurface> {
    alt((
        primitives::phrase(&["target", "opponent's"]).value(ExileOwnerSurface::TargetOpponent),
        primitives::phrase(&["target", "opponents"]).value(ExileOwnerSurface::TargetOpponent),
        primitives::phrase(&["target", "opponent"]).value(ExileOwnerSurface::TargetOpponent),
        primitives::phrase(&["target", "player's"]).value(ExileOwnerSurface::TargetPlayer),
        primitives::phrase(&["target", "players"]).value(ExileOwnerSurface::TargetPlayer),
        primitives::phrase(&["target", "player"]).value(ExileOwnerSurface::TargetPlayer),
        primitives::phrase(&["that", "player's"]).value(ExileOwnerSurface::ThatPlayer),
        primitives::phrase(&["that", "players"]).value(ExileOwnerSurface::ThatPlayer),
        primitives::phrase(&["that", "player"]).value(ExileOwnerSurface::ThatPlayer),
    ))
    .parse_next(input)
}

fn relation_owner_surface(input: &mut LexStream<'_>) -> WResult<ExileOwnerSurface> {
    alt((
        primitives::phrase(&["its", "controllers"]).value(ExileOwnerSurface::ItsController),
        primitives::phrase(&["its", "controller"]).value(ExileOwnerSurface::ItsController),
        primitives::phrase(&["its", "owners"]).value(ExileOwnerSurface::ItsOwner),
        primitives::phrase(&["its", "owner"]).value(ExileOwnerSurface::ItsOwner),
        primitives::phrase(&["his", "or", "her"]).value(ExileOwnerSurface::HisOrHer),
        primitives::kw("their").value(ExileOwnerSurface::Their),
        primitives::kw("your").value(ExileOwnerSurface::You),
    ))
    .parse_next(input)
}

fn exile_owner_surface(input: &mut LexStream<'_>) -> WResult<ExileOwnerSurface> {
    alt((player_owner_surface, relation_owner_surface)).parse_next(input)
}

fn graveyard_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("graveyard"), primitives::kw("graveyards")))
        .void()
        .parse_next(input)
}

fn library_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("library"), primitives::kw("libraries")))
        .void()
        .parse_next(input)
}

fn consumed_words(tokens: &[OwnedLexToken], rest: &[OwnedLexToken]) -> usize {
    TokenWordView::new(tokens)
        .len()
        .saturating_sub(TokenWordView::new(rest).len())
}

fn direct_owner(surface: ExileOwnerSurface) -> Option<PlayerAst> {
    match surface {
        ExileOwnerSurface::You => Some(PlayerAst::You),
        ExileOwnerSurface::ThatPlayer => Some(PlayerAst::That),
        ExileOwnerSurface::TargetPlayer => Some(PlayerAst::Target),
        ExileOwnerSurface::TargetOpponent => Some(PlayerAst::TargetOpponent),
        ExileOwnerSurface::ItsController => Some(PlayerAst::ItsController),
        ExileOwnerSurface::ItsOwner => Some(PlayerAst::ItsOwner),
        ExileOwnerSurface::Their | ExileOwnerSurface::HisOrHer => None,
    }
}

pub(crate) fn parse_exile_graveyard_owner_shape(
    tokens: &[OwnedLexToken],
) -> Option<ParsedExileOwnerPrefix> {
    let ((surface, ()), rest) =
        primitives::parse_prefix(tokens, (exile_owner_surface, graveyard_word))?;
    Some(ParsedExileOwnerPrefix {
        player: direct_owner(surface).unwrap_or(PlayerAst::That),
        consumed_words: consumed_words(tokens, rest),
    })
}

pub(crate) fn parse_exile_library_owner_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ParsedExileOwnerPrefix> {
    let ((surface, ()), rest) =
        primitives::parse_prefix(tokens, (opt(exile_owner_surface), library_word))?;
    let player = match surface {
        Some(ExileOwnerSurface::Their | ExileOwnerSurface::HisOrHer) => {
            if matches!(default_player, PlayerAst::Implicit) {
                PlayerAst::ItsController
            } else {
                default_player
            }
        }
        Some(surface) => direct_owner(surface)?,
        None => default_player,
    };
    Some(ParsedExileOwnerPrefix {
        player,
        consumed_words: consumed_words(tokens, rest),
    })
}

pub(crate) fn is_each_opponent_library_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        (primitives::phrase(&["each", "opponent"]), library_word),
    )
    .is_some()
        || primitives::parse_prefix(
            tokens,
            (primitives::phrase(&["each", "opponents"]), library_word),
        )
        .is_some()
        || primitives::parse_prefix(
            tokens,
            (primitives::phrase(&["each", "opponent's"]), library_word),
        )
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn parses_zone_owner_prefixes_with_the_historical_word_count() {
        let graveyard =
            parse_exile_graveyard_owner_shape(&lex("that player's graveyard cards")).unwrap();
        assert_eq!(graveyard.player, PlayerAst::That);
        assert_eq!(graveyard.consumed_words, 3);

        let library =
            parse_exile_library_owner_shape(&lex("their library"), PlayerAst::Implicit).unwrap();
        assert_eq!(library.player, PlayerAst::ItsController);
        assert_eq!(library.consumed_words, 2);
        assert!(is_each_opponent_library_shape(&lex(
            "each opponent's library"
        )));
    }
}
