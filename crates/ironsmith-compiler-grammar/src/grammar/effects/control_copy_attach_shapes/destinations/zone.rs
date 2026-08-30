use super::*;

pub fn parse_onto_battlefield_destination_shape(
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

    let mut controller = None;
    if let Some(parsed) = parse_battlefield_controller_prefix(&destination_tail) {
        controller = Some(parsed.controller);
        destination_tail = parsed.rest.to_vec();
    }

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

    if controller.is_none()
        && let Some(parsed) = parse_battlefield_controller_prefix(&destination_tail)
    {
        controller = Some(parsed.controller);
        destination_tail = parsed.rest.to_vec();
    }
    let supported_tail = destination_tail.is_empty();
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
