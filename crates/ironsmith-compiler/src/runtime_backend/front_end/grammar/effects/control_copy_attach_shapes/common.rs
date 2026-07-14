use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::effect::ChoiceCount;
use crate::runtime_backend::front_end::grammar::{leaf, permission_shapes, primitives};
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldControllerShape {
    You,
    Owner,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BattlefieldControllerPrefix<'a> {
    pub(crate) controller: BattlefieldControllerShape,
    pub(crate) rest: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CountedCardTargetShape<'a> {
    pub(crate) count: ChoiceCount,
    pub(crate) target_tokens: &'a [OwnedLexToken],
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

pub(crate) fn parse_battlefield_controller_prefix(
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

pub(crate) fn parse_destination_player(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
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

pub(crate) fn parse_destination_player_reference_surface(
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

pub(crate) fn explicitly_names_object_owner(tokens: &[OwnedLexToken]) -> bool {
    ["owner", "owners", "owner's", "owners'"]
        .iter()
        .any(|word| primitives::contains_word(tokens, word))
}

pub(crate) fn parse_destination_zone(tokens: &[OwnedLexToken]) -> Option<Zone> {
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

pub(crate) fn is_rest_reference(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::exact_tokens_any(tokens, &[&["the", "rest"], &["rest"]])
}

pub(crate) fn is_tagged_object_reference(tokens: &[OwnedLexToken]) -> bool {
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

pub(crate) fn is_plural_tagged_object_reference(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::exact_tokens_any(
        trim_lexed_commas(tokens),
        &[&["them"], &["those", "cards"], &["the", "exiled", "cards"]],
    )
}

pub(crate) fn starts_with_all_or_each(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        trim_lexed_commas(tokens),
        alt((primitives::kw("all"), primitives::kw("each"))).void(),
    )
    .is_some()
}

pub(crate) fn contains_graveyard_and_hand(tokens: &[OwnedLexToken]) -> bool {
    let graveyard = primitives::contains_word(tokens, "graveyard")
        || primitives::contains_word(tokens, "graveyards");
    let hand =
        primitives::contains_word(tokens, "hand") || primitives::contains_word(tokens, "hands");
    graveyard && hand
}

pub(crate) fn contains_from_it(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::contains_tokens(tokens, &["from", "it"])
}

pub(crate) fn contains_among_them(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "among") && primitives::contains_word(tokens, "them")
}

pub(crate) fn contains_permanent(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "permanent")
}

pub(crate) fn contains_sticker(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "sticker")
}

pub(crate) fn parse_counted_card_target_shape(
    tokens: &[OwnedLexToken],
) -> Option<CountedCardTargetShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let parsed = leaf::parse_leaf_choice_count_prefix_tokens(tokens)?;
    let after_count = trim_lexed_commas(tokens.get(parsed.consumed..)?);
    let (_, _) = primitives::parse_prefix(
        after_count,
        alt((primitives::kw("card"), primitives::kw("cards"))).void(),
    )?;
    Some(CountedCardTargetShape {
        count: parsed.count,
        target_tokens: after_count,
    })
}

pub(crate) fn parse_counted_those_cards(tokens: &[OwnedLexToken]) -> Option<u32> {
    let tokens = trim_lexed_commas(tokens);
    let (_, tail) = primitives::parse_prefix(tokens, primitives::kw("put").void())?;
    let parsed = leaf::parse_leaf_number_prefix_tokens(tail)?;
    let after_count = tail.get(parsed.consumed..)?;
    let mut input = LexStream::new(after_count);
    opt(primitives::kw("of")).parse_next(&mut input).ok()?;
    primitives::kw("those").parse_next(&mut input).ok()?;
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(&mut input)
        .ok()?;
    if !input.is_empty() {
        return None;
    }
    parsed.into_fixed().map(|(count, _)| count)
}

pub(crate) fn parse_delayed_hand_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let mut offset = 0usize;
    let mut last = None;
    while offset < tokens.len() {
        let Some((relative, _, _)) = primitives::find_prefix(&tokens[offset..], || {
            alt((primitives::kw("hand"), primitives::kw("hands"))).void()
        }) else {
            break;
        };
        let index = offset + relative;
        last = tokens.get(index + 1..);
        offset = index + 1;
    }
    last.map(trim_lexed_commas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_controller_and_counted_target_prefixes() {
        let controller = lex_line("under your control creatures", 0).unwrap();
        let parsed = parse_battlefield_controller_prefix(&controller).unwrap();
        assert_eq!(parsed.controller, BattlefieldControllerShape::You);
        assert!(permission_shapes::exact_tokens(parsed.rest, &["creatures"]));

        let target = lex_line("up to two cards from your graveyard", 0).unwrap();
        assert!(parse_counted_card_target_shape(&target).is_some());
    }

    #[test]
    fn distinguishes_pronoun_and_explicit_player_destinations() {
        let pronoun = lex_line("their graveyard", 0).unwrap();
        assert_eq!(
            parse_destination_player_reference_surface(&pronoun),
            Some(ironsmith_core::DestinationPlayerReferenceSurface::Pronoun)
        );

        let explicit = lex_line("that player's graveyard", 0).unwrap();
        assert_eq!(
            parse_destination_player_reference_surface(&explicit),
            Some(ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer)
        );

        for owner_destination in [
            "its owner's library",
            "their owners' libraries",
            "their owner's library",
        ] {
            let owner_destination = lex_line(owner_destination, 0).unwrap();
            assert_eq!(parse_destination_player(&owner_destination), None);
            assert_eq!(
                parse_destination_player_reference_surface(&owner_destination),
                None
            );
        }
    }
}
