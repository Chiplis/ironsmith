//! Permanent state change effects.
//!
//! This module contains effects that modify the state of permanents on the battlefield,
//! such as tapping, untapping, monstrosity, regeneration, and transformation.

use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::object::AttachmentTarget;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod attach_objects;
mod attach_to;
mod become_basic_land_type_choice;
mod become_color_choice;
mod become_creature_type_choice;
mod crew;
mod earthbend;
mod evolve;
mod flip;
mod grant_object_ability;
mod hanweir_battlements_meld;
mod monstrosity;
mod ninjutsu;
mod phase_out;
mod regenerate;
mod renown;
mod saddle;
mod soulbond_pair;
mod tap;
mod transform;
mod umbra_armor;
mod unearth;
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

    let subtypes = game.calculated_subtypes(attachment_id);
    if subtypes.contains(&Subtype::Aura) {
        let Some(filter) = attachment.aura_attach_filter.clone() else {
            return false;
        };
        let filter_ctx = game.filter_context_for(attachment.controller, Some(attachment_id));
        return filter.matches_target(target, &filter_ctx, game);
    }

    if subtypes.contains(&Subtype::Equipment) {
        if attachment.card_types.contains(&CardType::Creature) {
            return false;
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

pub(crate) fn attach_battlefield_object_to_target(
    game: &mut GameState,
    attachment_id: ObjectId,
    target: AttachmentTarget,
) -> bool {
    if !attachment_can_attach_to_target(game, attachment_id, target) {
        return false;
    }

    let previous_parent = game.object(attachment_id).and_then(|object| object.attached_to);
    if previous_parent == Some(target) {
        return false;
    }

    if !game.attach_object_to_target(attachment_id, target) {
        return false;
    }

    game.continuous_effects.record_attachment(attachment_id);
    true
}

pub use attach_objects::AttachObjectsEffect;
pub use attach_to::AttachToEffect;
pub use become_basic_land_type_choice::BecomeBasicLandTypeChoiceEffect;
pub use become_color_choice::BecomeColorChoiceEffect;
pub use become_creature_type_choice::BecomeCreatureTypeChoiceEffect;
pub use crew::CrewCostEffect;
pub use earthbend::EarthbendEffect;
pub use evolve::EvolveEffect;
pub use flip::FlipEffect;
pub use grant_object_ability::GrantObjectAbilityEffect;
pub use hanweir_battlements_meld::HanweirBattlementsMeldEffect;
pub use monstrosity::MonstrosityEffect;
pub use ninjutsu::{NinjutsuCostEffect, NinjutsuEffect};
pub use phase_out::PhaseOutEffect;
pub use regenerate::RegenerateEffect;
pub use renown::RenownEffect;
pub use saddle::{BecomeSaddledUntilEotEffect, SaddleCostEffect};
pub use soulbond_pair::SoulbondPairEffect;
pub use tap::TapEffect;
pub use transform::TransformEffect;
pub use umbra_armor::UmbraArmorEffect;
pub use unearth::UnearthEffect;
pub use untap::UntapEffect;
