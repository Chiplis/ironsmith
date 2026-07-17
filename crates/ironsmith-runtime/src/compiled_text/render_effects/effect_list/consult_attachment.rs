use super::*;

/// Render a singular reveal-until result that is moved to the battlefield,
/// attached using the exact moved-result tag, and followed by disposal of the
/// consult remainder. Every clause is linked through typed tags; no surface
/// text or card identity is used to infer the sequence.
pub(super) fn describe_consult_battlefield_attachment_remainder(
    effects: &[Effect],
) -> Option<String> {
    let [consult_effect, move_effect, attach_effect, remainder_effect] = effects else {
        return None;
    };

    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.max_exposed.is_some()
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
    {
        return None;
    }

    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.library_order.is_some()
        || !matches!(
            move_to_zone.verb_surface,
            ironsmith_core::MoveToZoneVerbSurface::Canonical
                | ironsmith_core::MoveToZoneVerbSurface::Put
        )
        || move_to_zone.target_plural_surface
        || move_to_zone.actor_surface.is_some()
        || move_to_zone.destination_player_surface.is_some()
        || move_to_zone.destination_player_reference_surface.is_some()
        || move_to_zone.exiled_with_source_surface.is_some()
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || move_to_zone.controller_surface_explicit
        || !move_to_zone.enters_with_counters.is_empty()
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.attack_target_mode.is_some()
        || move_to_zone.enters_face_down
        || move_to_zone.transfer_exiled_with_source_links
        || !choose_spec_references_tagged_object(&move_to_zone.target, &consult.match_tag)
    {
        return None;
    }

    let attach = structural_unwrap_render_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    // Reuse the generic move/attachment recognizer to prove that the
    // attachment consumes the result tag produced by this exact move.
    describe_put_onto_battlefield_attached(&[move_effect, attach_effect])?;
    let attachment_target = describe_choose_spec(&attach.target);

    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != consult.all_tag
        || remainder.keep_tagged.as_ref() != Some(&consult.match_tag)
        || remainder.player != consult.player
        || remainder.surface != ironsmith_core::LibraryRemainderSurface::Rest
    {
        return None;
    }
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };

    let rendered_consult = describe_effect(consult_effect);
    let consult_text = rendered_consult
        .trim()
        .trim_end_matches('.')
        .strip_prefix("You ")
        .map(capitalize_first)
        .unwrap_or_else(|| capitalize_first(rendered_consult.trim().trim_end_matches('.')));

    Some(format!(
        "{consult_text}. Put that card onto the battlefield attached to {attachment_target}, then put the rest on the bottom of your library{order}"
    ))
}
