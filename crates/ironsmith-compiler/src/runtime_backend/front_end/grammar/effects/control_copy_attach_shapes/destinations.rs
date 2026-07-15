use winnow::combinator::{alt, opt, repeat};
use winnow::prelude::*;

use crate::runtime_backend::front_end::grammar::{permission_shapes, primitives};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, trim_lexed_commas};
use crate::zone::Zone;

use super::common::{
    BattlefieldControllerShape, parse_battlefield_controller_prefix, parse_destination_zone,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryPlacementShape {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryPlacementOrderShape {
    Random,
    ChooserChooses,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LibraryChoiceDestinationShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LibraryPlacementDestinationShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) destination_tokens: &'a [OwnedLexToken],
    pub(crate) placement: LibraryPlacementShape,
    pub(crate) order: Option<LibraryPlacementOrderShape>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IntoDestinationShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) destination_tokens: &'a [OwnedLexToken],
    pub(crate) zone: Option<Zone>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SourceExiledOwnerLibraryBottomShape<'a> {
    pub(crate) source_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_source_exiled_owner_library_bottom_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourceExiledOwnerLibraryBottomShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (_, after_prefix) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["the", "owner", "of", "each", "card", "exiled", "with"])
            .void(),
    )?;
    let (puts_index, _, trailing) = primitives::find_prefix(after_prefix, || {
        (
            primitives::phrase(&["puts", "that", "card", "on"]),
            opt(primitives::kw("the")),
            primitives::phrase(&["bottom", "of", "their", "library"]),
        )
            .void()
    })?;
    if !crate::runtime_backend::token_word_refs(trailing).is_empty() {
        return None;
    }
    let source_tokens = trim_lexed_commas(after_prefix.get(..puts_index)?);
    (!source_tokens.is_empty()).then_some(SourceExiledOwnerLibraryBottomShape { source_tokens })
}

pub(crate) fn contains_source_exiled_owner_library_bottom_shape(
    tokens: &[OwnedLexToken],
) -> bool {
    permission_shapes::contains_tokens(
        tokens,
        &["owner", "of", "each", "card", "exiled", "with"],
    ) && permission_shapes::contains_tokens(
        tokens,
        &["that", "card", "on", "the", "bottom", "of", "their", "library"],
    )
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DestinationFirstTargetShape<'a> {
    Objects(&'a [OwnedLexToken]),
    Attached {
        attachment_target_tokens: &'a [OwnedLexToken],
        object_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DestinationFirstBattlefieldShape<'a> {
    pub(crate) tapped: bool,
    pub(crate) face_down: bool,
    pub(crate) controller: Option<BattlefieldControllerShape>,
    pub(crate) target: DestinationFirstTargetShape<'a>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OntoClauseShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) destination_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone)]
pub(crate) struct OntoBattlefieldDestinationShape {
    pub(crate) tapped: bool,
    pub(crate) attacking: bool,
    pub(crate) face_down: bool,
    pub(crate) source_from_command: bool,
    pub(crate) attached_to_tokens: Option<Vec<OwnedLexToken>>,
    pub(crate) rest_graveyard_target: Option<Vec<OwnedLexToken>>,
    pub(crate) controller: Option<BattlefieldControllerShape>,
    pub(crate) supported_tail: bool,
}

fn article(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'_>,
) -> winnow::error::ModalResult<()> {
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

pub(crate) fn parse_library_choice_destination_shape(
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

pub(crate) fn parse_library_placement_destination_shape(
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

pub(crate) fn is_exhaustive_hand_collection(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_lexed_commas(tokens);
    let plural_collection = permission_shapes::prefix_tokens(tokens, &["the", "cards", "in"])
        || permission_shapes::prefix_tokens(tokens, &["cards", "in"]);
    plural_collection
        && (primitives::contains_word(tokens, "hand") || primitives::contains_word(tokens, "hands"))
}

pub(crate) fn parse_into_destination_shape(
    tokens: &[OwnedLexToken],
) -> Option<IntoDestinationShape<'_>> {
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

pub(crate) fn parse_destination_first_battlefield_shape(
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

pub(crate) fn parse_onto_clause_shape(tokens: &[OwnedLexToken]) -> Option<OntoClauseShape<'_>> {
    let (index, _, destination_tokens) =
        primitives::find_prefix(tokens, || primitives::kw("onto"))?;
    let target_tokens = trim_lexed_commas(tokens.get(..index)?);
    if target_tokens.is_empty() {
        return None;
    }
    Some(OntoClauseShape {
        target_tokens,
        destination_tokens: trim_lexed_commas(destination_tokens),
    })
}

pub(crate) fn target_names_unowned_shared_zone(tokens: &[OwnedLexToken]) -> bool {
    [
        &["from", "a", "graveyard"][..],
        &["from", "any", "graveyard"][..],
        &["from", "a", "library"][..],
        &["from", "any", "library"][..],
    ]
    .iter()
    .any(|phrase| permission_shapes::contains_tokens(tokens, phrase))
}

fn token_is_ignored(token: &OwnedLexToken) -> bool {
    primitives::parse_prefix(
        std::slice::from_ref(token),
        alt((
            primitives::kw("and"),
            primitives::kw("tapped"),
            primitives::kw("attacking"),
            primitives::kw("face"),
            primitives::kw("down"),
        ))
        .void(),
    )
    .is_some()
}

fn word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut words = Vec::new();
    for token in tokens {
        if token.as_word().is_some()
            && !permission_shapes::exact_tokens_any(
                std::slice::from_ref(token),
                &[&["a"], &["an"], &["the"]],
            )
        {
            words.push(token.clone());
        }
    }
    words
}

pub(crate) fn parse_onto_battlefield_destination_shape(
    tokens: &[OwnedLexToken],
) -> Option<OntoBattlefieldDestinationShape> {
    let source_from_command =
        permission_shapes::contains_tokens(tokens, &["from", "command", "zone"])
            || permission_shapes::contains_tokens(tokens, &["from", "the", "command", "zone"]);
    let normalized = word_tokens(tokens);
    let (_, tail) = primitives::parse_prefix(&normalized, primitives::kw("battlefield").void())?;
    let mut destination_tail = tail.to_vec();
    let tapped = primitives::contains_word(&destination_tail, "tapped");
    let attacking = primitives::contains_word(&destination_tail, "attacking");
    let face_down = permission_shapes::contains_tokens(&destination_tail, &["face", "down"]);

    if let Some((index, _, rest)) = primitives::find_prefix(&destination_tail, || {
        primitives::phrase(&["from", "command", "zone"]).void()
    }) {
        let consumed = destination_tail
            .len()
            .saturating_sub(rest.len())
            .saturating_sub(index);
        destination_tail.drain(index..index + consumed);
    }
    let mut cleaned = Vec::new();
    for token in destination_tail {
        if !token_is_ignored(&token) {
            cleaned.push(token);
        }
    }
    destination_tail = cleaned;

    let mut attached_to_tokens = None;
    if let Some((_, rest)) = primitives::parse_prefix(
        &destination_tail,
        primitives::phrase(&["attached", "to"]).void(),
    ) {
        attached_to_tokens = Some(trim_lexed_commas(rest).to_vec());
        destination_tail.clear();
    }
    if let Some((index, _, _)) =
        primitives::find_prefix(&destination_tail, || primitives::kw("instead"))
    {
        destination_tail.truncate(index);
    }
    if let Some((_, rest)) =
        primitives::parse_prefix(&destination_tail, primitives::kw("and").void())
    {
        destination_tail = rest.to_vec();
    }

    let mut rest_graveyard_target = None;
    if let Some((index, _, destination)) =
        primitives::find_prefix(&destination_tail, || primitives::kw("into"))
        && parse_destination_zone(destination) == Some(Zone::Graveyard)
    {
        rest_graveyard_target = Some(destination_tail.get(..index)?.to_vec());
        destination_tail.clear();
    }

    let parsed_controller = parse_battlefield_controller_prefix(&destination_tail);
    let controller = parsed_controller.map(|shape| shape.controller);
    let supported_tail = destination_tail.is_empty() || parsed_controller.is_some();
    Some(OntoBattlefieldDestinationShape {
        tapped,
        attacking,
        face_down,
        source_from_command,
        attached_to_tokens,
        rest_graveyard_target,
        controller,
        supported_tail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_library_and_battlefield_destinations() {
        let choice = lex_line(
            "target card on its owner's choice of the top or bottom of their library",
            0,
        )
        .unwrap();
        assert!(
            parse_library_choice_destination_shape(&choice).is_some(),
            "{:?}",
            crate::runtime_backend::front_end::lexer::TokenWordView::new(&choice).to_word_refs()
        );

        let onto = lex_line(
            "target creature onto the battlefield tapped and attacking",
            0,
        )
        .unwrap();
        let clause = parse_onto_clause_shape(&onto).unwrap();
        let destination =
            parse_onto_battlefield_destination_shape(clause.destination_tokens).unwrap();
        assert!(destination.tapped);
        assert!(destination.attacking);
        assert!(destination.supported_tail);
    }

    #[test]
    fn library_placement_keeps_target_player_words_out_of_destination_surface() {
        let owner_destination =
            lex_line("a creature you control on top of its owner's library", 0).unwrap();
        let owner_shape = parse_library_placement_destination_shape(&owner_destination).unwrap();
        assert_eq!(
            crate::runtime_backend::front_end::grammar::effects::control_copy_attach_shapes::parse_destination_player(
                owner_shape.destination_tokens,
            ),
            None,
        );

        let plural_owner_destination =
            lex_line("all creatures on the bottom of their owners' libraries", 0).unwrap();
        let plural_owner_shape =
            parse_library_placement_destination_shape(&plural_owner_destination).unwrap();
        assert_eq!(
            crate::runtime_backend::front_end::grammar::effects::control_copy_attach_shapes::parse_destination_player(
                plural_owner_shape.destination_tokens,
            ),
            None,
        );
        assert_eq!(
            crate::runtime_backend::front_end::grammar::effects::control_copy_attach_shapes::parse_destination_player_reference_surface(
                plural_owner_shape.destination_tokens,
            ),
            None,
        );

        let your_destination =
            lex_line("a creature you control on top of your library", 0).unwrap();
        let your_shape = parse_library_placement_destination_shape(&your_destination).unwrap();
        assert_eq!(
            crate::runtime_backend::front_end::grammar::effects::control_copy_attach_shapes::parse_destination_player(
                your_shape.destination_tokens,
            ),
            Some(crate::cards::builders::PlayerAst::You),
        );
    }
}
