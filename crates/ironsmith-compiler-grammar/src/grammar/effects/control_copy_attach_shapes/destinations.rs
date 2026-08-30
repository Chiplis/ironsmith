use winnow::combinator::{alt, opt, repeat};
use winnow::prelude::*;

use crate::grammar::{permission_shapes, primitives};
use crate::lexer::{OwnedLexToken, trim_lexed_commas};
use crate::zone::Zone;

use super::common::{
    BattlefieldControllerShape, parse_battlefield_controller_prefix, parse_destination_zone,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryPlacementShape {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryPlacementOrderShape {
    Random,
    ChooserChooses,
}

#[derive(Debug, Clone, Copy)]
pub struct LibraryChoiceDestinationShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct LibraryPlacementDestinationShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub destination_tokens: &'a [OwnedLexToken],
    pub placement: LibraryPlacementShape,
    pub order: Option<LibraryPlacementOrderShape>,
}

#[derive(Debug, Clone, Copy)]
pub struct IntoDestinationShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub destination_tokens: &'a [OwnedLexToken],
    pub zone: Option<Zone>,
}

#[derive(Debug, Clone, Copy)]
pub struct SourceExiledOwnerLibraryBottomShape<'a> {
    pub source_tokens: &'a [OwnedLexToken],
}

pub fn parse_source_exiled_owner_library_bottom_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourceExiledOwnerLibraryBottomShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (_, after_prefix) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["the", "owner", "of", "each", "card", "exiled", "with"]).void(),
    )?;
    let (puts_index, _, trailing) = primitives::find_prefix(after_prefix, || {
        (
            primitives::phrase(&["puts", "that", "card", "on"]),
            opt(primitives::kw("the")),
            primitives::phrase(&["bottom", "of", "their", "library"]),
        )
            .void()
    })?;
    if !crate::lexer::token_word_refs(trailing).is_empty() {
        return None;
    }
    let source_tokens = trim_lexed_commas(after_prefix.get(..puts_index)?);
    (!source_tokens.is_empty()).then_some(SourceExiledOwnerLibraryBottomShape { source_tokens })
}

pub fn contains_source_exiled_owner_library_bottom_shape(tokens: &[OwnedLexToken]) -> bool {
    permission_shapes::contains_tokens(tokens, &["owner", "of", "each", "card", "exiled", "with"])
        && permission_shapes::contains_tokens(
            tokens,
            &[
                "that", "card", "on", "the", "bottom", "of", "their", "library",
            ],
        )
}

#[derive(Debug, Clone, Copy)]
pub enum DestinationFirstTargetShape<'a> {
    Objects(&'a [OwnedLexToken]),
    Attached {
        attachment_target_tokens: &'a [OwnedLexToken],
        object_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy)]
pub struct DestinationFirstBattlefieldShape<'a> {
    pub tapped: bool,
    pub face_down: bool,
    pub controller: Option<BattlefieldControllerShape>,
    pub target: DestinationFirstTargetShape<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct OntoClauseShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
    pub destination_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub struct OntoBattlefieldDestinationShape {
    pub tapped: bool,
    pub attacking: bool,
    pub face_down: bool,
    pub source_from_command: bool,
    pub attached_to_tokens: Option<Vec<OwnedLexToken>>,
    pub rest_graveyard_target: Option<Vec<OwnedLexToken>>,
    pub controller: Option<BattlefieldControllerShape>,
    pub supported_tail: bool,
}

fn article(input: &mut crate::lexer::LexStream<'_>) -> winnow::error::ModalResult<()> {
    alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    ))
    .void()
    .parse_next(input)
}

fn optional_owner_prefix(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    for phrase in [
        &["their"][..],
        &["his"][..],
        &["her"][..],
        &["your"][..],
        &["that", "player"][..],
        &["that", "players"][..],
        &["that", "player's"][..],
        &["that", "players'"][..],
        &["owner"][..],
        &["owners"][..],
        &["owner's"][..],
        &["owners'"][..],
        &["its", "owner"][..],
        &["its", "owners"][..],
        &["its", "owner's"][..],
        &["its", "owners'"][..],
        &["its"][..],
    ] {
        if let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::phrase(phrase)) {
            return rest;
        }
    }
    tokens
}

fn choice_destination(tokens: &[OwnedLexToken]) -> bool {
    let tokens = optional_owner_prefix(trim_lexed_commas(tokens));
    let Some((_, after_choice)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["choice", "of"]),
            opt(primitives::kw("either")),
            opt(primitives::kw("the")),
        )
            .void(),
    ) else {
        return false;
    };
    let Some((_, after_positions)) = primitives::parse_prefix(
        after_choice,
        alt((
            primitives::phrase(&["top", "or", "bottom"]),
            primitives::phrase(&["bottom", "or", "top"]),
        ))
        .void(),
    ) else {
        return false;
    };
    let Some((_, library_tail)) =
        primitives::parse_prefix(after_positions, primitives::kw("of").void())
    else {
        return false;
    };
    primitives::contains_word(library_tail, "library")
        || primitives::contains_word(library_tail, "libraries")
}

pub fn parse_library_choice_destination_shape(
    tokens: &[OwnedLexToken],
) -> Option<LibraryChoiceDestinationShape<'_>> {
    let (on_index, _, destination) = primitives::find_prefix(tokens, || primitives::kw("on"))?;
    if !choice_destination(destination) {
        return None;
    }
    let target_tokens = trim_lexed_commas(tokens.get(..on_index)?);
    (!target_tokens.is_empty()).then_some(LibraryChoiceDestinationShape { target_tokens })
}

fn placement_start(
    tokens: &[OwnedLexToken],
) -> Option<(usize, LibraryPlacementShape, &[OwnedLexToken])> {
    let top = primitives::find_prefix(tokens, || primitives::phrase(&["on", "top", "of"]).void())
        .map(|(index, _, rest)| (index, LibraryPlacementShape::Top, rest));
    let bottom = primitives::find_prefix(tokens, || {
        (
            primitives::kw("on"),
            opt(primitives::kw("the")),
            primitives::phrase(&["bottom", "of"]),
        )
            .void()
    })
    .map(|(index, _, rest)| (index, LibraryPlacementShape::Bottom, rest));
    match (top, bottom) {
        (Some(top), Some(bottom)) => Some(if top.0 <= bottom.0 { top } else { bottom }),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

pub fn parse_library_placement_destination_shape(
    tokens: &[OwnedLexToken],
) -> Option<LibraryPlacementDestinationShape<'_>> {
    let (index, placement, destination) = placement_start(tokens)?;
    if !(primitives::contains_word(destination, "library")
        || primitives::contains_word(destination, "libraries"))
    {
        return None;
    }
    let target_tokens = trim_lexed_commas(tokens.get(..index)?);
    let order = if primitives::contains_word(destination, "random")
        && primitives::contains_word(destination, "order")
    {
        Some(LibraryPlacementOrderShape::Random)
    } else if primitives::contains_word(destination, "any")
        && primitives::contains_word(destination, "order")
    {
        Some(LibraryPlacementOrderShape::ChooserChooses)
    } else {
        None
    };
    (!target_tokens.is_empty()).then_some(LibraryPlacementDestinationShape {
        target_tokens,
        destination_tokens: destination,
        placement,
        order,
    })
}

pub fn is_exhaustive_hand_collection(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_lexed_commas(tokens);
    let plural_collection = permission_shapes::prefix_tokens(tokens, &["the", "cards", "in"])
        || permission_shapes::prefix_tokens(tokens, &["cards", "in"]);
    plural_collection
        && (primitives::contains_word(tokens, "hand") || primitives::contains_word(tokens, "hands"))
}

pub fn parse_into_destination_shape(tokens: &[OwnedLexToken]) -> Option<IntoDestinationShape<'_>> {
    let (index, _, destination_tokens) =
        primitives::find_prefix(tokens, || primitives::kw("into"))?;
    let target_tokens = trim_lexed_commas(tokens.get(..index)?);
    if target_tokens.is_empty()
        || primitives::contains_word(target_tokens, "onto")
        || primitives::contains_word(target_tokens, "battlefield")
    {
        return None;
    }
    let destination_tokens = trim_lexed_commas(destination_tokens);
    Some(IntoDestinationShape {
        target_tokens,
        destination_tokens,
        zone: parse_destination_zone(destination_tokens),
    })
}

fn attachment_target_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let tokens = trim_lexed_commas(tokens);
    if let Some((_, rest)) = primitives::parse_prefix(tokens, primitives::kw("it").void()) {
        let used = tokens.len().checked_sub(rest.len())?;
        return Some((tokens.get(..used)?, trim_lexed_commas(rest)));
    }
    let (_, rest) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("that"),
            alt((
                primitives::kw("creature"),
                primitives::kw("permanent"),
                primitives::kw("object"),
                primitives::kw("aura"),
                primitives::kw("equipment"),
            )),
        )
            .void(),
    )?;
    let used = tokens.len().checked_sub(rest.len())?;
    Some((tokens.get(..used)?, trim_lexed_commas(rest)))
}

pub fn parse_destination_first_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<DestinationFirstBattlefieldShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (_, mut tail) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("onto"),
            repeat::<_, _, (), _, _>(0.., article),
            primitives::kw("battlefield"),
        )
            .void(),
    )?;
    let tapped =
        if let Some((_, rest)) = primitives::parse_prefix(tail, primitives::kw("tapped").void()) {
            tail = rest;
            true
        } else {
            false
        };
    let face_down = if let Some((_, rest)) =
        primitives::parse_prefix(tail, primitives::phrase(&["face", "down"]).void())
    {
        tail = rest;
        true
    } else {
        false
    };
    let mut controller = None;
    if let Some(parsed) = parse_battlefield_controller_prefix(tail) {
        controller = Some(parsed.controller);
        tail = parsed.rest;
    }
    let target_tokens = trim_lexed_commas(tail);
    if target_tokens.is_empty() {
        return None;
    }
    let target = if let Some((_, after_to)) = primitives::parse_prefix(
        target_tokens,
        primitives::phrase(&["attached", "to"]).void(),
    ) {
        let (attachment_target_tokens, object_tokens) = attachment_target_prefix(after_to)?;
        if object_tokens.is_empty() {
            return None;
        }
        DestinationFirstTargetShape::Attached {
            attachment_target_tokens,
            object_tokens,
        }
    } else {
        DestinationFirstTargetShape::Objects(target_tokens)
    };
    Some(DestinationFirstBattlefieldShape {
        tapped,
        face_down,
        controller,
        target,
    })
}

pub fn parse_onto_clause_shape(tokens: &[OwnedLexToken]) -> Option<OntoClauseShape<'_>> {
    let (index, _, destination_tokens) =
        primitives::find_prefix(tokens, || primitives::kw("onto"))?;
    let target_tokens = trim_lexed_commas(tokens.get(..index)?);
    if target_tokens.is_empty() {
        return None;
    }
    // A trailing where-X clause binds values in the moved object's filter; it
    // is not part of the zone destination. The sentence-level binding pass
    // applies it after this typed move has been lowered.
    let destination_tokens = if let Some((where_index, (), _)) =
        primitives::find_prefix(destination_tokens, || {
            primitives::phrase(&["where", "x", "is"]).void()
        }) {
        destination_tokens.get(..where_index)?
    } else {
        destination_tokens
    };
    Some(OntoClauseShape {
        target_tokens,
        destination_tokens: trim_lexed_commas(destination_tokens),
    })
}

#[cfg(test)]
#[path = "destinations_inline_tests.rs"]
mod tests;

#[path = "destinations/zone.rs"]
mod zone_programs;
pub use zone_programs::parse_onto_battlefield_destination_shape;
#[path = "destinations/object_action.rs"]
mod object_action_programs;
use object_action_programs::{token_is_ignored, word_tokens};
#[path = "destinations/reference.rs"]
mod reference_programs;
pub use reference_programs::target_names_unowned_shared_zone;
