use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::PlayerAst;
use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, Copy)]
pub(crate) struct RegistryWordSplit<'a> {
    pub(crate) before: &'a [OwnedLexToken],
    pub(crate) after: &'a [OwnedLexToken],
}

fn target_opponent<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    (
        primitives::kw("target"),
        winnow::combinator::alt((primitives::kw("opponent"), primitives::kw("opponents"))),
    )
        .value(PlayerAst::TargetOpponent)
        .parse_next(input)
}

fn target_player<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    (
        primitives::kw("target"),
        winnow::combinator::alt((primitives::kw("player"), primitives::kw("players"))),
    )
        .value(PlayerAst::Target)
        .parse_next(input)
}

fn that_player<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    (
        primitives::kw("that"),
        winnow::combinator::alt((primitives::kw("player"), primitives::kw("players"))),
    )
        .value(PlayerAst::That)
        .parse_next(input)
}

pub(crate) fn parse_registry_player_object(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    primitives::parse_all(
        tokens,
        winnow::combinator::alt((target_opponent, target_player, that_player)),
        "registry-player-object",
    )
    .ok()
}

pub(crate) fn split_registry_clause_at_word(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<RegistryWordSplit<'_>> {
    let mut input = LexStream::new(tokens);
    let mut consumed_words = 0usize;
    while consumed_words < word_index {
        let token: &OwnedLexToken = any::<_, ErrMode<ContextError>>
            .parse_next(&mut input)
            .ok()?;
        if token.as_word().is_some() {
            consumed_words += 1;
        }
    }
    let token_index = tokens.len().checked_sub(input.len())?;
    Some(RegistryWordSplit {
        before: tokens.get(..token_index)?,
        after: tokens.get(token_index..)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

    #[test]
    fn parses_player_objects_and_word_boundaries() {
        let player = lex_line("target opponent", 0).unwrap();
        assert_eq!(
            parse_registry_player_object(&player),
            Some(PlayerAst::TargetOpponent)
        );

        let tokens = lex_line("two cards, then draw", 0).unwrap();
        let split = split_registry_clause_at_word(&tokens, 2).unwrap();
        assert_eq!(
            TokenWordView::new(split.before).to_word_refs(),
            vec!["two", "cards"]
        );
    }
}
