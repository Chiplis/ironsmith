use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::effect::ChoiceCount;
use crate::grammar::{leaf, permission_shapes, primitives};
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlefieldControllerShape {
    You,
    Owner,
}

#[derive(Debug, Clone, Copy)]
pub struct BattlefieldControllerPrefix<'a> {
    pub controller: BattlefieldControllerShape,
    pub rest: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct CountedCardTargetShape<'a> {
    pub count: ChoiceCount,
    pub target_tokens: &'a [OwnedLexToken],
}

fn control_action(input: &mut LexStream<'_>) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("control"), primitives::kw("controls")))
        .void()
        .parse_next(input)
}

fn you_controller(input: &mut LexStream<'_>) -> winnow::error::ModalResult<()> {
    primitives::kw("under").parse_next(input)?;
    alt((primitives::kw("your"), primitives::kw("you")))
        .void()
        .parse_next(input)?;
    control_action.parse_next(input)
}

fn owner_controller(input: &mut LexStream<'_>) -> winnow::error::ModalResult<()> {
    primitives::kw("under").parse_next(input)?;
    alt((
        (
            alt((
                primitives::kw("its"),
                primitives::kw("his"),
                primitives::kw("her"),
                primitives::kw("their"),
            )),
            alt((
                primitives::kw("owner"),
                primitives::kw("owners"),
                primitives::kw("owner's"),
                primitives::kw("owners'"),
            )),
        )
            .void(),
        primitives::phrase(&["that", "players"]),
        primitives::phrase(&["that", "player"]),
    ))
    .void()
    .parse_next(input)?;
    control_action.parse_next(input)
}

pub fn parse_battlefield_controller_prefix(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldControllerPrefix<'_>> {
    let tokens = trim_lexed_commas(tokens);
    if let Some(((), rest)) = primitives::parse_prefix(tokens, you_controller) {
        return Some(BattlefieldControllerPrefix {
            controller: BattlefieldControllerShape::You,
            rest: trim_lexed_commas(rest),
        });
    }
    let ((), rest) = primitives::parse_prefix(tokens, owner_controller)?;
    Some(BattlefieldControllerPrefix {
        controller: BattlefieldControllerShape::Owner,
        rest: trim_lexed_commas(rest),
    })
}

pub fn parse_destination_player(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    if explicitly_names_object_owner(tokens) {
        return None;
    }
    if primitives::contains_word(tokens, "your") || primitives::contains_word(tokens, "you") {
        return Some(PlayerAst::You);
    }
    if permission_shapes::prefix_tokens(tokens, &["their"])
        || permission_shapes::prefix_tokens(tokens, &["that", "player"])
        || permission_shapes::prefix_tokens(tokens, &["that", "players"])
    {
        return Some(PlayerAst::That);
    }
    None
}

pub fn parse_destination_player_reference_surface(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::DestinationPlayerReferenceSurface> {
    if explicitly_names_object_owner(tokens) {
        return None;
    }
    if permission_shapes::contains_tokens(tokens, &["that", "player"])
        || permission_shapes::contains_tokens(tokens, &["that", "players"])
    {
        return Some(ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer);
    }
    if primitives::contains_word(tokens, "their") {
        return Some(ironsmith_core::DestinationPlayerReferenceSurface::Pronoun);
    }
    None
}

pub fn explicitly_names_object_owner(tokens: &[OwnedLexToken]) -> bool {
    ["owner", "owners", "owner's", "owners'"]
        .iter()
        .any(|word| primitives::contains_word(tokens, word))
}

pub fn parse_destination_zone(tokens: &[OwnedLexToken]) -> Option<Zone> {
    if primitives::contains_word(tokens, "hand") || primitives::contains_word(tokens, "hands") {
        return Some(Zone::Hand);
    }
    if primitives::contains_word(tokens, "graveyard")
        || primitives::contains_word(tokens, "graveyards")
    {
        return Some(Zone::Graveyard);
    }
    None
}

pub fn is_rest_reference(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::exact_tokens_any(tokens, &[&["the", "rest"], &["rest"]])
}

pub fn is_tagged_object_reference(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::exact_tokens_any(
        trim_lexed_commas(tokens),
        &[
            &["it"],
            &["them"],
            &["that", "card"],
            &["those", "card"],
            &["those", "cards"],
        ],
    )
}

pub fn is_plural_tagged_object_reference(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_lexed_commas(tokens);
    permission_shapes::exact_tokens_any(
        tokens,
        &[&["them"], &["those", "cards"], &["the", "exiled", "cards"]],
    ) || primitives::parse_prefix(tokens, primitives::kw("those").void()).is_some()
}

#[cfg(test)]
#[path = "common_inline_tests.rs"]
mod tests;

#[path = "common/trigger_programs.rs"]
mod trigger_programs;
pub use trigger_programs::parse_delayed_hand_tail;
#[path = "common/library_programs.rs"]
mod library_programs;
pub use library_programs::{parse_counted_card_target_shape, parse_counted_those_cards};
#[path = "common/core_programs.rs"]
mod core_programs;
pub use core_programs::{
    contains_among_them, contains_from_it, contains_permanent, contains_sticker,
    starts_with_all_or_each,
};
#[path = "common/zone_programs.rs"]
mod zone_programs;
pub use zone_programs::contains_graveyard_and_hand;
