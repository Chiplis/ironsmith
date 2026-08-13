//! Permanent state change effects.
//!
//! This module contains effects that modify the state of permanents on the battlefield,
//! such as tapping, untapping, monstrosity, regeneration, and transformation.

use crate::filter::ObjectFilterExt;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::object::{AttachmentTarget, AuraAttachmentFilterRuntimeExt};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod attach_objects;
mod attach_to;
mod become_basic_land_type_choice;
mod become_color_choice;
mod become_creature_type_choice;
mod conspire;
mod crew;
mod detain;
mod earthbend;
mod evolve;
mod exert;
mod flip;
mod grant_object_ability;
mod meld;
mod monstrosity;
mod ninjutsu;
mod phase_in;
mod phase_out;
mod put_sticker;
mod reconfigure;
mod regenerate;
mod renown;
mod saddle;
mod solve_case;
mod soulbond_pair;
mod suspect;
mod tap;
mod transform;
mod turn_face_up;
mod umbra_armor;
mod unattach_objects;
mod unearth;
mod unlock_room_door;
mod untap;

pub(crate) fn attachment_can_attach_to_target(
    game: &GameState,
    attachment_id: ObjectId,
    target: AttachmentTarget,
) -> bool {
    if matches!(target, AttachmentTarget::Object(target_id) if attachment_id == target_id) {
        return false;
    }

    let Some(attachment) = game.object(attachment_id) else {
        return false;
    };
    if attachment.zone != Zone::Battlefield || !game.attachment_target_exists_on_battlefield(target)
    {
        return false;
    }

    let attachment_controller = game.controller_of(attachment);
    if !game.attachment_target_is_within_range(attachment_controller, target, Some(attachment_id)) {
        return false;
    }

    let subtypes = game.calculated_subtypes(attachment_id);
    if subtypes.contains(&Subtype::Aura) {
        let Some(filter) = game
            .current_characteristics(attachment_id)
            .and_then(|chars| chars.aura_attach_filter)
            .or_else(|| attachment.aura_attach_filter_owned())
        else {
            return false;
        };
        let filter_ctx = game.filter_context_for(attachment_controller, Some(attachment_id));
        return filter.matches_target(target, &filter_ctx, game);
    }

    if subtypes.contains(&Subtype::Equipment) {
        if attachment.card_types.contains(&CardType::Creature)
            && !attachment_has_reconfigure_ability(attachment)
        {
            return false;
        }
        if let Some(crate::object::AuraAttachmentFilter::Object(filter)) = game
            .current_characteristics(attachment_id)
            .and_then(|chars| chars.aura_attach_filter)
            .or_else(|| attachment.aura_attach_filter_owned())
        {
            let filter_ctx = game.filter_context_for(attachment_controller, Some(attachment_id));
            return matches!(target, AttachmentTarget::Object(target_id) if game
                .object(target_id)
                .is_some_and(|object| filter.matches(object, &filter_ctx, game)));
        }
        return matches!(target, AttachmentTarget::Object(target_id) if game.object_has_card_type(target_id, CardType::Creature));
    }

    if subtypes.contains(&Subtype::Fortification) {
        if attachment.card_types.contains(&CardType::Creature) {
            return false;
        }
        return matches!(target, AttachmentTarget::Object(target_id) if game.object_has_card_type(target_id, CardType::Land));
    }

    false
}

fn attachment_has_reconfigure_ability(attachment: &crate::object::Object) -> bool {
    attachment.abilities.iter().any(|ability| {
        crate::compiled_text::ability_surface_text(ability).starts_with("Reconfigure ")
    })
}

pub(crate) fn attach_battlefield_object_to_target(
    game: &mut GameState,
    attachment_id: ObjectId,
    target: AttachmentTarget,
) -> bool {
    if !attachment_can_attach_to_target(game, attachment_id, target) {
        return false;
    }

    let previous_parent = game
        .object(attachment_id)
        .and_then(|object| object.attached_to);
    if previous_parent == Some(target) {
        return false;
    }

    if !game.attach_object_to_target(attachment_id, target) {
        return false;
    }

    game.effect_store
        .continuous_effects
        .record_attachment(attachment_id);
    true
}

pub(crate) fn choose_color_as_becomes_attached(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext<'_>,
    attachment_id: ObjectId,
    target: AttachmentTarget,
) {
    let has_choice_ability = game
        .calculated_characteristics_arc(attachment_id)
        .map(|chars| chars.static_abilities.clone())
        .unwrap_or_else(|| {
            game.object(attachment_id)
                .map(|object| crate::ability::extract_static_abilities(&object.abilities))
                .unwrap_or_default()
                .into()
        })
        .into_iter()
        .any(|ability| ability.color_choice_as_becomes_attached().is_some());
    if !has_choice_ability {
        return;
    }

    let Some(chooser) = game.controller_of_id(attachment_id) else {
        return;
    };
    let options = [
        (crate::color::Color::White, "White"),
        (crate::color::Color::Blue, "Blue"),
        (crate::color::Color::Black, "Black"),
        (crate::color::Color::Red, "Red"),
        (crate::color::Color::Green, "Green"),
    ];
    let selectable = options
        .iter()
        .enumerate()
        .map(|(idx, (_, label))| crate::decisions::SelectableOption::new(idx, *label))
        .collect();
    let choice_ctx = crate::decisions::SelectOptionsContext::new(
        chooser,
        Some(attachment_id),
        "Choose a color",
        selectable,
        1,
        1,
    );
    let chosen = ctx
        .decision_maker
        .decide_options(game, &choice_ctx)
        .into_iter()
        .next();
    if ctx.decision_maker.awaiting_choice() {
        return;
    }
    let Some(chosen_idx) = chosen.filter(|idx| *idx < options.len()) else {
        return;
    };

    let (color, _) = options[chosen_idx];
    game.set_chosen_color(attachment_id, color);
    if let AttachmentTarget::Object(target_id) = target {
        game.set_chosen_color(target_id, color);
    }
}

pub use attach_objects::AttachObjectsEffect;
pub use attach_to::AttachToEffect;
pub use become_basic_land_type_choice::BecomeBasicLandTypeChoiceEffect;
pub use become_color_choice::BecomeColorChoiceEffect;
pub use become_creature_type_choice::BecomeCreatureTypeChoiceEffect;
pub use conspire::ConspireCostEffect;
pub use crew::CrewCostEffect;
pub use detain::DetainEffect;
pub use earthbend::EarthbendEffect;
pub use evolve::EvolveEffect;
pub use exert::ExertCostEffect;
pub use flip::FlipEffect;
pub use grant_object_ability::GrantObjectAbilityEffect;
pub use meld::MeldEffect;
pub use monstrosity::MonstrosityEffect;
pub use ninjutsu::{NinjutsuCostEffect, NinjutsuEffect, SneakCostEffect};
pub use phase_in::PhaseInEffect;
pub use phase_out::{PhaseOutDuration, PhaseOutEffect};
pub use put_sticker::PutStickerEffect;
pub use reconfigure::ReconfigureEffect;
pub use regenerate::RegenerateEffect;
pub use renown::RenownEffect;
pub use saddle::{BecomeSaddledUntilEotEffect, SaddleCostEffect};
pub use solve_case::SolveCaseEffect;
pub use soulbond_pair::SoulbondPairEffect;
pub use suspect::{ClearSuspectedEffect, SuspectEffect};
pub use tap::TapEffect;
pub use transform::{ConvertEffect, TransformEffect};
pub use turn_face_up::TurnFaceUpEffect;
pub use umbra_armor::UmbraArmorEffect;
pub use unattach_objects::UnattachObjectsEffect;
pub use unearth::UnearthEffect;
pub use unlock_room_door::UnlockRoomDoorEffect;
pub use untap::UntapEffect;
