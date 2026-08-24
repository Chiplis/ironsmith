use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::effect::{EventValueSpec, Value};
use crate::grammar::shared_util::count_shapes::parse_for_each_count_value_words;
use crate::grammar::{primitives, values};
use crate::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use ironsmith_core::ValueSurfaceHint;

use super::{
    is_each_opponent_library_shape, is_each_player_library_shape, parse_exile_library_owner_shape,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileLibraryPlayerShape {
    Player(PlayerAst),
    EachPlayer,
    EachOpponent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExileLibraryCardsShape {
    pub player: ExileLibraryPlayerShape,
    pub count: Value,
    pub face_down: bool,
}

fn trim_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0;
    let mut end = tokens.len();
    while start < end && tokens[start].kind == TokenKind::Comma {
        start += 1;
    }
    while end > start && tokens[end - 1].kind == TokenKind::Comma {
        end -= 1;
    }
    &tokens[start..end]
}

fn card_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)
}

fn library_player(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
    allow_each_opponent: bool,
) -> Option<ExileLibraryPlayerShape> {
    if allow_each_opponent && is_each_player_library_shape(tokens) {
        return Some(ExileLibraryPlayerShape::EachPlayer);
    }
    if allow_each_opponent && is_each_opponent_library_shape(tokens) {
        return Some(ExileLibraryPlayerShape::EachOpponent);
    }
    let owner = parse_exile_library_owner_shape(tokens, default_player)?;
    (owner.consumed_words == TokenWordView::new(tokens).len())
        .then_some(ExileLibraryPlayerShape::Player(owner.player))
}

fn strip_position_and_of<'a>(
    tokens: &'a [OwnedLexToken],
    position: &'static str,
) -> Option<&'a [OwnedLexToken]> {
    primitives::parse_prefix(
        tokens,
        (
            opt(primitives::kw("the")),
            primitives::kw(position),
            primitives::kw("of"),
        ),
    )
    .map(|(_, rest)| rest)
}

fn parse_position_count_and_owner<'a>(
    tokens: &'a [OwnedLexToken],
    position: &'static str,
) -> Option<(Value, bool, &'a [OwnedLexToken])> {
    let (_, after_position) = primitives::parse_prefix(
        tokens,
        (opt(primitives::kw("the")), primitives::kw(position)),
    )?;
    if let Some(((), after_cards)) = primitives::parse_prefix(after_position, card_word) {
        let (_, owner) = primitives::parse_prefix(after_cards, primitives::kw("of"))?;
        return Some((Value::Fixed(1), true, trim_commas(owner)));
    }
    let (count, used) = values::parse_value_prefix_lexed(after_position)?;
    let (_, after_cards) = primitives::parse_prefix(&after_position[used..], card_word)?;
    let (_, owner) = primitives::parse_prefix(after_cards, primitives::kw("of"))?;
    Some((count, false, trim_commas(owner)))
}

fn parse_position_count_without_owner(
    tokens: &[OwnedLexToken],
    position: &'static str,
) -> Option<(Value, bool)> {
    let (_, after_position) = primitives::parse_prefix(
        tokens,
        (opt(primitives::kw("the")), primitives::kw(position)),
    )?;
    if let Some(((), rest)) = primitives::parse_prefix(after_position, card_word)
        && trim_commas(rest).is_empty()
    {
        return Some((Value::Fixed(1), true));
    }
    let (count, used) = values::parse_value_prefix_lexed(after_position)?;
    let (_, rest) = primitives::parse_prefix(&after_position[used..], card_word)?;
    trim_commas(rest).is_empty().then_some((count, false))
}

#[cfg(test)]
#[path = "library_inline_tests.rs"]
mod tests;

#[path = "library/library_programs.rs"]
mod library_programs;
pub use library_programs::{
    parse_exile_bottom_library_shape, parse_exile_dynamic_top_library_shape,
    parse_exile_top_library_shape,
};
