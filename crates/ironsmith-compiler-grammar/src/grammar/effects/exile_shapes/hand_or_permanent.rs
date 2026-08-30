use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EachOpponentExileChoiceShape {
    pub choice: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EachPlayerExileGroup {
    Player,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EachPlayerExileCountedHandPermanentShape {
    pub group: EachPlayerExileGroup,
}

fn hand_owner(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("their").void(),
        primitives::phrase(&["that", "player"]).void(),
        primitives::phrase(&["that", "players"]).void(),
        primitives::phrase(&["that", "player's"]).void(),
    ))
    .parse_next(input)
}

fn permanent_controller(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("they").void(),
        primitives::phrase(&["that", "player"]).void(),
        primitives::phrase(&["that", "players"]).void(),
        primitives::phrase(&["that", "player's"]).void(),
    ))
    .parse_next(input)
}

fn finish_non_words(input: &mut LexStream<'_>) -> WResult<()> {
    while !input.is_empty() {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(input);
        let token = parsed?;
        if token.as_word().is_some() {
            return Err(primitives::backtrack_err(
                "exile hand-or-permanent choice",
                "end of choice",
            ));
        }
    }
    Ok(())
}

fn hand_or_permanent_choice(input: &mut LexStream<'_>) -> WResult<()> {
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::phrase(&["card", "from"]).parse_next(input)?;
    hand_owner.parse_next(input)?;
    primitives::phrase(&["hand", "or"]).parse_next(input)?;
    opt(primitives::kw("a")).parse_next(input)?;
    primitives::kw("permanent").parse_next(input)?;
    permanent_controller.parse_next(input)?;
    alt((primitives::kw("control"), primitives::kw("controls")))
        .void()
        .parse_next(input)?;
    finish_non_words(input)
}

fn each_opponent_exiles(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("each").parse_next(input)?;
    alt((primitives::kw("opponent"), primitives::kw("opponents")))
        .void()
        .parse_next(input)?;
    alt((primitives::kw("exile"), primitives::kw("exiles")))
        .void()
        .parse_next(input)
}

#[cfg(test)]
#[path = "hand_or_permanent_inline_tests.rs"]
mod tests;

#[path = "hand_or_permanent/reference.rs"]
mod reference_programs;
use reference_programs::each_player_or_opponent_exiles;
pub use reference_programs::parse_each_player_exile_counted_hand_permanent_shape;
#[path = "hand_or_permanent/choice.rs"]
mod choice_programs;
pub use choice_programs::{
    is_exile_hand_or_permanent_choice_shape, parse_each_opponent_exile_choice_shape,
};
#[path = "hand_or_permanent/library.rs"]
mod library_programs;
use library_programs::counted_permanents_and_or_hand_cards;
#[path = "hand_or_permanent/core.rs"]
mod core_programs;
use core_programs::and_or;
