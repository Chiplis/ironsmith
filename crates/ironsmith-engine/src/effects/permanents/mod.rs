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
mod designation;
mod detain;
mod earthbend;
mod evolve;
mod exert;
mod face_up_transition;
mod flip;
mod grant_object_ability;
mod meld;
mod monstrosity;
mod next_adapt_ignores_counters;
mod ninjutsu;
mod phase_in;
mod phase_out;
mod prepare;
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
mod turn_face_down;
mod turn_face_up;
pub(crate) use face_up_transition::{FaceUpChoiceController, turn_face_up_with_choices};
mod umbra_armor;
mod unattach_objects;
mod unearth;
mod unlock_room_door;
mod untap;

/// Whether `player` currently has protection from everything (The One Ring,
/// Teferi's Protection).
///
/// Player protection lowers to its two observable halves from one source:
/// "can't be the target of spells or abilities" plus "prevent all damage that
/// would be dealt to you". Recognize that pair so the enchant half of
/// CR 702.16c applies to players too.
pub(crate) fn player_has_protection_from_everything(
    game: &GameState,
    player: crate::ids::PlayerId,
) -> bool {
    if game.effect_store.cant_effects.can_target_player(player) {
        return false;
    }
    let shields = game.effect_store.prevention_effects.shields();
    game.effect_store
        .restriction_effects
        .iter()
        .filter(|restriction| {
            !restriction.is_pending()
                && matches!(
                    restriction.restriction,
                    crate::effect::Restriction::BeTargetedPlayer(_)
                )
        })
        .any(|restriction| {
            shields.iter().any(|shield| {
                shield.source == restriction.source
                    && shield.damage_filter == crate::prevention::DamageFilter::all()
                    && match shield.protected {
                        crate::prevention::PreventionTarget::Player(protected) => {
                            protected == player
                        }
                        crate::prevention::PreventionTarget::You => shield.controller == player,
                        _ => false,
                    }
            })
        })
}

/// Whether `player` has protection from the object `source` (CR 702.16):
/// the single player-protection query. Fed by `player_protections`
/// (PlayerProtectionFrom abilities: Absolute Virtue, "protection from the
/// chosen card type") and by protection from everything.
pub(crate) fn player_has_protection_from_object(
    game: &GameState,
    player: crate::ids::PlayerId,
    source: &crate::object::Object,
) -> bool {
    game.effect_store
        .cant_effects
        .player_protections
        .iter()
        .any(|protection| {
            protection.player == player && {
                let filter_ctx = game
                    .filter_context_for(protection.controller, Some(protection.protection_source));
                protection.source_filter.matches(source, &filter_ctx, game)
            }
        })
        || player_has_protection_from_everything(game, player)
        || player_has_composed_protection_from_object(game, player, source)
}

/// A resolution grant of player protection from a quality ("you gain
/// protection from that player until your next turn", "target player gains
/// protection from red") lowers to the same two observable halves as
/// protection from everything, but scoped to the quality: "can't be the
/// target of <quality> spells or abilities" plus "prevent all damage <quality>
/// sources would deal to that player", created together by one source. The
/// pair is that player's protection, so its enchant half applies too: an Aura
/// (a Curse) with the quality can't enchant the player, and an attached one is
/// put into its owner's graveyard as a state-based action (CR 702.16e,
/// 704.5m).
fn player_has_composed_protection_from_object(
    game: &GameState,
    player: crate::ids::PlayerId,
    source: &crate::object::Object,
) -> bool {
    let shields = game.effect_store.prevention_effects.shields();
    game.effect_store
        .restriction_effects
        .iter()
        .filter(|restriction| !restriction.is_pending())
        .filter_map(|restriction| match &restriction.restriction {
            crate::effect::Restriction::BeTargetedPlayerFrom(_, quality) => {
                Some((restriction, quality))
            }
            _ => None,
        })
        .any(|(restriction, quality)| {
            let paired_shield = shields.iter().any(|shield| {
                shield.source == restriction.source
                    && matches!(
                        shield.protected,
                        crate::prevention::PreventionTarget::Player(protected) if protected == player
                    )
                    && shield.damage_filter.from_source.as_ref() == Some(quality)
                    && crate::prevention::DamageFilter {
                        from_source: None,
                        ..shield.damage_filter.clone()
                    } == crate::prevention::DamageFilter::all()
            });
            paired_shield && {
                let filter_ctx =
                    game.filter_context_for(restriction.controller, Some(restriction.source));
                quality.matches(source, &filter_ctx, game)
            }
        })
}

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
    if attachment.zone != Zone::Battlefield || !game.attachment_target_exists(target) {
        return false;
    }

    // "can't be enchanted by other Auras" / "can't be equipped": a
    // prohibited attachment is illegal, so it can't attach and falls off
    // (CR 704.5m/n).
    if let AttachmentTarget::Object(host) = target
        && game
            .effect_store
            .cant_effects
            .cant_be_attached_by
            .iter()
            .any(|entry| entry.forbids(game, host, attachment))
    {
        return false;
    }
    let attachment_controller = game.controller_of(attachment);
    if !game.attachment_target_is_within_range(attachment_controller, target, Some(attachment_id)) {
        return false;
    }

    let subtypes = game.calculated_subtypes(attachment_id);
    if subtypes.contains(&Subtype::Aura) {
        // CR 702.16c/e/j: a player can't be enchanted by Auras with a quality
        // they have protection from; attached ones fall off as a state-based
        // action (704.5m).
        if let AttachmentTarget::Player(player) = target
            && player_has_protection_from_object(game, player, attachment)
        {
            return false;
        }
        let filter_ctx = game.filter_context_for(attachment_controller, Some(attachment_id));
        if let Some(chars) = game.current_characteristics(attachment_id) {
            let filters = chars
                .static_abilities
                .iter()
                .filter_map(|ability| ability.enchant_filter())
                .collect::<Vec<_>>();
            return !filters.is_empty()
                && filters
                    .iter()
                    .all(|filter| filter.matches_target(target, &filter_ctx, game));
        }
        return attachment
            .aura_attach_filter_owned()
            .is_some_and(|filter| filter.matches_target(target, &filter_ctx, game));
    }

    if !game.attachment_target_exists_on_battlefield(target) {
        return false;
    }

    // CR 301.5c / 301.6: an Equipment or Fortification that's currently a
    // creature (e.g. animated by March of the Machines) can't be attached.
    let attachment_is_creature = game.object_has_card_type(attachment_id, CardType::Creature);
    if subtypes.contains(&Subtype::Equipment) {
        if attachment_is_creature && !attachment_has_reconfigure_ability(game, attachment_id) {
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
        if attachment_is_creature {
            return false;
        }
        return matches!(target, AttachmentTarget::Object(target_id) if game.object_has_card_type(target_id, CardType::Land));
    }

    false
}

fn attachment_has_reconfigure_ability(game: &GameState, attachment_id: ObjectId) -> bool {
    game.calculated_characteristics(attachment_id)
        .is_some_and(|chars| {
            chars.abilities.iter().any(|ability| {
                matches!(&ability.kind, crate::ability::AbilityKind::Activated(activated)
                if activated.effects.iter().any(|effect| {
                    effect.downcast_ref::<ReconfigureEffect>().is_some()
                }))
            })
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
    if let AttachmentTarget::Object(target_id) = target
        && crate::targeting::has_protection_from_source(game, target_id, attachment_id)
    {
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
pub(crate) use attach_objects::{
    aura_can_enter_attached_to, entry_attachment_for_move, resolve_entry_attachment_target,
};
pub use attach_to::AttachToEffect;
pub use become_basic_land_type_choice::BecomeBasicLandTypeChoiceEffect;
pub use become_color_choice::BecomeColorChoiceEffect;
pub use become_creature_type_choice::BecomeCreatureTypeChoiceEffect;
pub use conspire::ConspireCostEffect;
pub use crew::CrewCostEffect;
pub(crate) use crew::{
    complete_crew_ability_resolution, complete_crew_ability_resolution_with_outputs,
};
pub use detain::DetainEffect;
pub use earthbend::EarthbendEffect;
pub use evolve::EvolveEffect;
pub(crate) use evolve::evolve_entering_creature_is_larger;
pub use exert::ExertCostEffect;
pub use flip::FlipEffect;
pub use grant_object_ability::GrantObjectAbilityEffect;
pub use meld::MeldEffect;
pub use monstrosity::MonstrosityEffect;
pub use next_adapt_ignores_counters::NextAdaptIgnoresCountersEffect;
pub use ninjutsu::{NinjutsuCostEffect, NinjutsuEffect, SneakCostEffect};
pub use phase_in::PhaseInEffect;
pub use phase_out::{PhaseOutDuration, PhaseOutEffect};
pub use prepare::PrepareEffect;
pub use put_sticker::PutStickerEffect;
pub use reconfigure::ReconfigureEffect;
pub use regenerate::RegenerateEffect;
pub use renown::RenownEffect;
pub use saddle::{BecomeSaddledUntilEotEffect, SaddleCostEffect};
pub use solve_case::{DayNightDesignation, SetClassLevelEffect, SetDayNightEffect, SolveCaseEffect};
pub use soulbond_pair::SoulbondPairEffect;
pub(crate) use soulbond_pair::soulbond_pairing_possible;
pub use suspect::{ClearSuspectedEffect, SuspectEffect};
pub use tap::TapEffect;
pub(crate) use tap::{TapAction, tap_cost_objects_with_outputs};
pub use transform::{ConvertEffect, TransformEffect};
pub use turn_face_down::TurnFaceDownEffect;
pub use turn_face_up::TurnFaceUpEffect;
pub use umbra_armor::UmbraArmorEffect;
pub use unattach_objects::UnattachObjectsEffect;
pub use unearth::UnearthEffect;
pub use unlock_room_door::UnlockRoomDoorEffect;
pub use untap::UntapEffect;

pub(crate) mod face_down_entry;
