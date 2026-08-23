use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken, TokenWordView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExileOwnerSurface {
    You,
    Their,
    ThatPlayer,
    DefendingPlayer,
    TargetPlayer,
    TargetOpponent,
    ItsController,
    ItsOwner,
    HisOrHer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedExileOwnerPrefix {
    pub player: PlayerAst,
    pub consumed_words: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExileOnePerCardTypeFromGraveyardShape {
    pub owner: PlayerAst,
}

fn player_owner_surface(input: &mut LexStream<'_>) -> WResult<ExileOwnerSurface> {
    alt((
        alt((
            primitives::phrase(&["defending", "player's"])
                .value(ExileOwnerSurface::DefendingPlayer),
            primitives::phrase(&["defending", "players"]).value(ExileOwnerSurface::DefendingPlayer),
            primitives::phrase(&["defending", "player"]).value(ExileOwnerSurface::DefendingPlayer),
            primitives::phrase(&["target", "opponent's"]).value(ExileOwnerSurface::TargetOpponent),
            primitives::phrase(&["target", "opponents"]).value(ExileOwnerSurface::TargetOpponent),
            primitives::phrase(&["target", "opponent"]).value(ExileOwnerSurface::TargetOpponent),
            primitives::phrase(&["target", "player's"]).value(ExileOwnerSurface::TargetPlayer),
            primitives::phrase(&["target", "players"]).value(ExileOwnerSurface::TargetPlayer),
            primitives::phrase(&["target", "player"]).value(ExileOwnerSurface::TargetPlayer),
        )),
        alt((
            primitives::phrase(&["that", "player's"]).value(ExileOwnerSurface::ThatPlayer),
            primitives::phrase(&["that", "players"]).value(ExileOwnerSurface::ThatPlayer),
            primitives::phrase(&["that", "player"]).value(ExileOwnerSurface::ThatPlayer),
        )),
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
        ExileOwnerSurface::DefendingPlayer => Some(PlayerAst::Defending),
        ExileOwnerSurface::TargetPlayer => Some(PlayerAst::Target),
        ExileOwnerSurface::TargetOpponent => Some(PlayerAst::TargetOpponent),
        ExileOwnerSurface::ItsController => Some(PlayerAst::ItsController),
        ExileOwnerSurface::ItsOwner => Some(PlayerAst::ItsOwner),
        ExileOwnerSurface::Their | ExileOwnerSurface::HisOrHer => None,
    }
}

pub fn parse_exile_one_per_card_type_from_graveyard_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExileOnePerCardTypeFromGraveyardShape> {
    let ((), rest) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&[
            "up", "to", "one", "card", "of", "each", "card", "type", "from",
        ]),
    )?;
    let ((owner, ()), rest) =
        primitives::parse_prefix(rest, (exile_owner_surface, graveyard_word))?;
    if !TokenWordView::new(rest).is_empty() {
        return None;
    }
    Some(ExileOnePerCardTypeFromGraveyardShape {
        owner: direct_owner(owner)?,
    })
}

pub fn parse_exile_graveyard_owner_shape(
    tokens: &[OwnedLexToken],
) -> Option<ParsedExileOwnerPrefix> {
    let ((surface, ()), rest) =
        primitives::parse_prefix(tokens, (exile_owner_surface, graveyard_word))?;
    Some(ParsedExileOwnerPrefix {
        player: direct_owner(surface).unwrap_or(PlayerAst::That),
        consumed_words: consumed_words(tokens, rest),
    })
}

pub fn parse_exile_library_owner_shape(
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

pub fn is_each_opponent_library_shape(tokens: &[OwnedLexToken]) -> bool {
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

pub fn is_each_player_library_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        (primitives::phrase(&["each", "player"]), library_word),
    )
    .is_some()
        || primitives::parse_prefix(
            tokens,
            (primitives::phrase(&["each", "players"]), library_word),
        )
        .is_some()
        || primitives::parse_prefix(
            tokens,
            (primitives::phrase(&["each", "player's"]), library_word),
        )
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

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
        assert!(is_each_player_library_shape(&lex("each player's library")));

        let each_type = parse_exile_one_per_card_type_from_graveyard_shape(&lex(
            "up to one card of each card type from defending player's graveyard",
        ))
        .unwrap();
        assert_eq!(each_type.owner, PlayerAst::Defending);
    }
}
