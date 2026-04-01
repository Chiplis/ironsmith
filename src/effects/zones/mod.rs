//! Zone change effects.
//!
//! This module contains effects that move objects between zones,
//! such as destroy, exile, sacrifice, and return to hand.

use std::collections::HashSet;

use crate::DecisionMaker;
use crate::decisions::context::{DecisionContext, OrderContext, enrich_display_hints};
use crate::event_processor::{EventOutcome, process_zone_change_with_additional_effects};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::ReplacementEffect;
use crate::zone::Zone;

mod battlefield_entry;
mod destroy;
mod destroy_no_regen;
mod exchange_zones;
mod exile;
mod exile_until_source_leaves;
mod haunt_exile;
mod may_move_to_zone;
mod move_to_library_nth_from_top;
mod move_to_zone;
mod put_onto_battlefield;
mod reorder_graveyard;
mod reorder_library_top;
mod return_all_to_battlefield;
mod return_from_graveyard_or_exile_to_battlefield;
mod return_from_graveyard_to_battlefield;
mod return_from_graveyard_to_hand;
mod return_to_hand;
mod sacrifice;
mod shuffle_objects_into_library;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppliedZoneChange {
    pub final_zone: Zone,
    pub new_object_id: Option<ObjectId>,
    pub new_object_ids: Vec<ObjectId>,
}

pub(crate) fn finalize_zone_change_move(
    game: &mut GameState,
    object_id: ObjectId,
    final_zone: Zone,
    cause: crate::events::cause::EventCause,
) -> AppliedZoneChange {
    let new_object_id = game.move_object(object_id, final_zone, cause);
    let mut new_object_ids = game.take_zone_change_results(object_id);
    if new_object_ids.is_empty()
        && let Some(id) = new_object_id
    {
        new_object_ids.push(id);
    }
    AppliedZoneChange {
        final_zone,
        new_object_id,
        new_object_ids,
    }
}

fn normalize_order_response(response: Vec<ObjectId>, original: &[ObjectId]) -> Vec<ObjectId> {
    let mut remaining = original.to_vec();
    let mut out = Vec::with_capacity(original.len());
    for id in response {
        if let Some(pos) = remaining.iter().position(|candidate| *candidate == id) {
            out.push(id);
            remaining.remove(pos);
        }
    }
    out.extend(remaining);
    out
}

fn split_result_order_chooser(
    game: &GameState,
    final_zone: Zone,
    cause: &crate::events::cause::EventCause,
    object_ids: &[ObjectId],
) -> Option<PlayerId> {
    let owner = object_ids
        .first()
        .and_then(|id| game.object(*id))
        .map(|object| object.owner)?;

    match final_zone {
        Zone::Library | Zone::Graveyard => Some(owner),
        Zone::Exile => cause.source_controller.or(Some(owner)),
        _ => None,
    }
}

fn split_result_order_description(final_zone: Zone) -> Option<&'static str> {
    match final_zone {
        Zone::Library => Some(
            "Choose the order of the split cards in the library. The first option becomes the top card among them.",
        ),
        Zone::Graveyard => Some("Choose the relative order of the split cards in the graveyard."),
        Zone::Exile => Some("Choose the relative order of the split cards in exile."),
        _ => None,
    }
}

fn current_split_result_order(
    game: &GameState,
    final_zone: Zone,
    object_ids: &[ObjectId],
) -> Vec<ObjectId> {
    let object_set = object_ids.iter().copied().collect::<HashSet<_>>();
    match final_zone {
        Zone::Library => {
            let Some(owner) = object_ids
                .first()
                .and_then(|id| game.object(*id))
                .map(|object| object.owner)
            else {
                return Vec::new();
            };

            game.player(owner)
                .map(|player| {
                    player
                        .library
                        .iter()
                        .rev()
                        .filter(|id| object_set.contains(id))
                        .copied()
                        .collect()
                })
                .unwrap_or_default()
        }
        Zone::Graveyard => {
            let Some(owner) = object_ids
                .first()
                .and_then(|id| game.object(*id))
                .map(|object| object.owner)
            else {
                return Vec::new();
            };

            game.player(owner)
                .map(|player| {
                    player
                        .graveyard
                        .iter()
                        .filter(|id| object_set.contains(id))
                        .copied()
                        .collect()
                })
                .unwrap_or_default()
        }
        Zone::Exile => game
            .exile
            .iter()
            .filter(|id| object_set.contains(id))
            .copied()
            .collect(),
        _ => Vec::new(),
    }
}

fn reorder_zone_subset_in_place(
    zone_objects: &mut [ObjectId],
    object_ids: &[ObjectId],
    desired_underlying_order: &[ObjectId],
) {
    if object_ids.len() <= 1 || desired_underlying_order.len() != object_ids.len() {
        return;
    }

    let object_set = object_ids.iter().copied().collect::<HashSet<_>>();
    let mut desired_iter = desired_underlying_order.iter().copied();
    for entry in zone_objects.iter_mut() {
        if object_set.contains(entry)
            && let Some(next_id) = desired_iter.next()
        {
            *entry = next_id;
        }
    }
}

fn apply_split_result_order(
    game: &mut GameState,
    final_zone: Zone,
    original_ids: &[ObjectId],
    ordered_ids: &[ObjectId],
) {
    match final_zone {
        Zone::Library => {
            let Some(owner) = original_ids
                .first()
                .and_then(|id| game.object(*id))
                .map(|object| object.owner)
            else {
                return;
            };
            let desired_underlying = ordered_ids.iter().rev().copied().collect::<Vec<_>>();
            if let Some(player) = game.player_mut(owner) {
                reorder_zone_subset_in_place(
                    &mut player.library,
                    original_ids,
                    &desired_underlying,
                );
            }
        }
        Zone::Graveyard => {
            let Some(owner) = original_ids
                .first()
                .and_then(|id| game.object(*id))
                .map(|object| object.owner)
            else {
                return;
            };
            if let Some(player) = game.player_mut(owner) {
                reorder_zone_subset_in_place(&mut player.graveyard, original_ids, ordered_ids);
            }
        }
        Zone::Exile => reorder_zone_subset_in_place(&mut game.exile, original_ids, ordered_ids),
        _ => {}
    }
}

pub(crate) fn maybe_prompt_for_split_result_order(
    game: &mut GameState,
    decision_maker: &mut dyn DecisionMaker,
    final_zone: Zone,
    cause: &crate::events::cause::EventCause,
    result: &mut AppliedZoneChange,
) {
    if result.new_object_ids.len() <= 1 {
        return;
    }

    let Some(chooser) = split_result_order_chooser(game, final_zone, cause, &result.new_object_ids)
    else {
        return;
    };
    let Some(description) = split_result_order_description(final_zone) else {
        return;
    };

    let current_order = current_split_result_order(game, final_zone, &result.new_object_ids);
    if current_order.len() <= 1 {
        return;
    }

    let items = current_order
        .iter()
        .map(|&id| {
            let name = game
                .object(id)
                .map(|object| object.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            (id, name)
        })
        .collect::<Vec<_>>();
    let order_ctx = enrich_display_hints(
        game,
        DecisionContext::Order(OrderContext::new(chooser, cause.source, description, items)),
    )
    .into_order();
    let ordered = normalize_order_response(
        decision_maker.decide_order(game, &order_ctx),
        &current_order,
    );
    if ordered == current_order {
        result.new_object_ids = ordered;
        result.new_object_id = result.new_object_ids.first().copied();
        return;
    }

    apply_split_result_order(game, final_zone, &result.new_object_ids, &ordered);
    result.new_object_ids = ordered;
    result.new_object_id = result.new_object_ids.first().copied();
}

pub(crate) fn apply_zone_change(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    decision_maker: &mut dyn DecisionMaker,
) -> EventOutcome<AppliedZoneChange> {
    apply_zone_change_with_additional_effects(game, object_id, from, to, cause, decision_maker, &[])
}

pub(crate) fn apply_zone_change_with_additional_effects(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    decision_maker: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
) -> EventOutcome<AppliedZoneChange> {
    match process_zone_change_with_additional_effects(
        game,
        object_id,
        from,
        to,
        cause.clone(),
        decision_maker,
        additional_effects,
    ) {
        EventOutcome::Proceed(final_zone) => {
            let mut result = finalize_zone_change_move(game, object_id, final_zone, cause.clone());
            if from == Zone::Battlefield && matches!(final_zone, Zone::Graveyard | Zone::Exile) {
                maybe_prompt_for_split_result_order(
                    game,
                    decision_maker,
                    final_zone,
                    &cause,
                    &mut result,
                );
                if !result.new_object_ids.is_empty() {
                    game.record_zone_change_results(object_id, result.new_object_ids.clone());
                }
            }
            EventOutcome::Proceed(result)
        }
        EventOutcome::Prevented => EventOutcome::Prevented,
        EventOutcome::Replaced => EventOutcome::Replaced,
        EventOutcome::NotApplicable => EventOutcome::NotApplicable,
    }
}

pub(crate) use battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};

pub use destroy::DestroyEffect;
pub use destroy_no_regen::DestroyNoRegenerationEffect;
pub use exchange_zones::ExchangeZonesEffect;
pub use exile::ExileEffect;
pub use exile_until_source_leaves::{ExileUntilDuration, ExileUntilEffect};
pub use haunt_exile::HauntExileEffect;
pub use may_move_to_zone::MayMoveToZoneEffect;
pub use move_to_library_nth_from_top::MoveToLibraryNthFromTopEffect;
pub use move_to_zone::{BattlefieldController, MoveToZoneEffect};
pub use put_onto_battlefield::PutOntoBattlefieldEffect;
pub use reorder_graveyard::ReorderGraveyardEffect;
pub use reorder_library_top::ReorderLibraryTopEffect;
pub use return_all_to_battlefield::ReturnAllToBattlefieldEffect;
pub use return_from_graveyard_or_exile_to_battlefield::ReturnFromGraveyardOrExileToBattlefieldEffect;
pub use return_from_graveyard_to_battlefield::ReturnFromGraveyardToBattlefieldEffect;
pub use return_from_graveyard_to_hand::ReturnFromGraveyardToHandEffect;
pub use return_to_hand::ReturnToHandEffect;
pub use sacrifice::{EachPlayerSacrificesEffect, SacrificeEffect, SacrificeTargetEffect};
pub use shuffle_objects_into_library::ShuffleObjectsIntoLibraryEffect;
