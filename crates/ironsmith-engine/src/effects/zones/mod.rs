//! Zone change effects.
//!
//! This module contains effects that move objects between zones,
//! such as destroy, exile, sacrifice, and return to hand.

use std::collections::HashSet;

use crate::DecisionMaker;
use crate::decisions::context::{DecisionContext, OrderContext, enrich_display_hints};
use crate::events::processing::{EventOutcome, PreparedEventOutcome, ReplacementEventContext, prepare_zone_change_scoped, commit_prepared_zone_change};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::ReplacementEffect;
use crate::zone::Zone;

mod battlefield_entry;
mod become_plotted;
pub use become_plotted::BecomePlottedEffect;
mod destroy;
mod destroy_no_regen;
mod exchange_zones;
mod exile;
mod exile_until_source_leaves;
mod haunt_exile;
mod may_move_to_zone;
mod move_to_library_nth_from_top;
mod move_to_library_top_or_bottom_choice;
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
pub struct AppliedZoneChange {
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

pub(crate) fn take_recorded_zone_change(
    game: &mut GameState,
    object_id: ObjectId,
) -> Option<AppliedZoneChange> {
    let new_object_ids = game.take_zone_change_results(object_id);
    let final_zone = new_object_ids
        .first()
        .and_then(|id| game.object(*id))
        .map(|obj| obj.zone)?;
    Some(AppliedZoneChange {
        final_zone,
        new_object_id: new_object_ids.first().copied(),
        new_object_ids,
    })
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
    zone_objects: &mut crate::zone_sequence::ZoneSequence,
    object_ids: &[ObjectId],
    desired_underlying_order: &[ObjectId],
) {
    if object_ids.len() <= 1 || desired_underlying_order.len() != object_ids.len() {
        return;
    }

    let object_set = object_ids.iter().copied().collect::<HashSet<_>>();
    let mut desired_iter = desired_underlying_order.iter().copied();
    zone_objects.with_vec_mut(|ids| {
        for entry in ids.iter_mut() {
            if object_set.contains(entry)
                && let Some(next_id) = desired_iter.next()
            {
                *entry = next_id;
            }
        }
    });
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

/// Collect CR 404.3 ordering choices for one simultaneous graveyard batch.
///
/// `decision_view` is the immutable state from before the event. `game` is a
/// staged transaction containing the proposed zone changes. Every owner choice
/// is collected in APNAP order before any graveyard index is reordered, so a
/// deferred choice can discard the staged transaction without exposing a
/// partially committed event.
pub(crate) fn order_simultaneous_graveyard_batch(
    decision_view: &GameState,
    game: &mut GameState,
    decision_maker: &mut dyn DecisionMaker,
    source: Option<ObjectId>,
    moved_object_ids: &[ObjectId],
) -> bool {
    let mut by_owner = std::collections::HashMap::<PlayerId, Vec<ObjectId>>::new();
    for object_id in moved_object_ids.iter().copied() {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if object.zone != Zone::Graveyard {
            continue;
        }
        let owner = object.owner;
        let ids = by_owner.entry(owner).or_default();
        if !ids.contains(&object_id) {
            ids.push(object_id);
        }
    }

    let mut owners = decision_view.team_apnap_player_order();
    let mut remaining = by_owner.keys().copied().collect::<Vec<_>>();
    remaining.sort_by_key(|player| player.0);
    for owner in remaining {
        if !owners.contains(&owner) {
            owners.push(owner);
        }
    }

    let mut choices = Vec::new();
    for owner in owners {
        let Some(batch_ids) = by_owner.get(&owner) else {
            continue;
        };
        let batch_set = batch_ids.iter().copied().collect::<HashSet<_>>();
        let current_order = game
            .player(owner)
            .map(|player| {
                player
                    .graveyard
                    .iter()
                    .filter(|object_id| batch_set.contains(object_id))
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if current_order.len() <= 1 {
            continue;
        }

        let items = current_order
            .iter()
            .map(|object_id| {
                let name = game
                    .object(*object_id)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                (*object_id, name)
            })
            .collect();
        let context = OrderContext::new(
            owner,
            source,
            "Order cards put into your graveyard simultaneously. The last item becomes the top card.",
            items,
        );
        let response = decision_maker.decide_order(decision_view, &context);
        if decision_maker.awaiting_choice() {
            return false;
        }
        choices.push((
            owner,
            current_order.clone(),
            normalize_order_response(response, &current_order),
        ));
    }

    for (owner, current_order, ordered) in choices {
        if ordered == current_order {
            continue;
        }
        if let Some(player) = game.player_mut(owner) {
            reorder_zone_subset_in_place(&mut player.graveyard, &current_order, &ordered);
        }
    }
    true
}

pub(crate) fn maybe_prompt_for_split_result_order(
    game: &mut GameState,
    decision_maker: &mut dyn DecisionMaker,
    final_zone: Zone,
    cause: &crate::events::cause::EventCause,
    result: &mut AppliedZoneChange,
) {
    let zone_result_ids = result
        .new_object_ids
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.zone == final_zone)
        })
        .collect::<Vec<_>>();
    if zone_result_ids.len() <= 1 {
        return;
    }

    let Some(chooser) = split_result_order_chooser(game, final_zone, cause, &zone_result_ids)
    else {
        return;
    };
    let Some(description) = split_result_order_description(final_zone) else {
        return;
    };

    let current_order = current_split_result_order(game, final_zone, &zone_result_ids);
    if current_order.len() <= 1 {
        return;
    }

    let items = current_order
        .iter()
        .map(|&id| {
            let name = game
                .object(id)
                .map(|object| object.name.to_string())
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
    if final_zone == Zone::Exile {
        // CR 730.3b / 613.7m: the exiling player chooses the relative
        // timestamps of cards entering exile simultaneously. Earlier entries
        // in the chosen order receive earlier timestamps.
        for object_id in &ordered {
            game.effect_store
                .continuous_effects
                .record_entry(*object_id);
        }
    }
    if ordered != current_order {
        apply_split_result_order(game, final_zone, &zone_result_ids, &ordered);
    }

    let ordered_set = zone_result_ids.iter().copied().collect::<HashSet<_>>();
    let mut ordered_iter = ordered.into_iter();
    for object_id in &mut result.new_object_ids {
        if ordered_set.contains(object_id)
            && let Some(next_id) = ordered_iter.next()
        {
            *object_id = next_id;
        }
    }
    result.new_object_id = result.new_object_ids.first().copied();
}

pub(crate) fn apply_zone_change(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    apply_zone_change_with_additional_effects(game, object_id, from, to, cause, decision_maker, &[])
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_zone_change_with_additional_effects(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    decision_maker: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    apply_zone_change_owned(game, object_id, from, to, cause, decision_maker, additional_effects, None)
}

/// Effect callers supply their replacement scope, not just a copied temporary
/// effect list: an Instead payload must not reapply the replacement that made it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_zone_change_with_context_and_additional_effects(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
    additional_effects: &[ReplacementEffect],
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    apply_zone_change_with_context_and_additional_effects_and_snapshot(
        game, object_id, from, to, cause, ctx, additional_effects, None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_zone_change_with_context_and_additional_effects_and_snapshot(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let scope = ReplacementEventContext::with_scope(&game, crate::events::Event::zone_change(object_id, from, to, cause.clone(), snapshot)
            .with_provenance(ctx.provenance), &ctx.replacement);
    let result = apply_zone_change_owned(
        game, object_id, from, to, cause, &mut *ctx.decision_maker,
        additional_effects, Some(&scope),
    );
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        *game = checkpoint;
        context_checkpoint.restore(ctx);
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn apply_zone_change_owned(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    decision_maker: &mut dyn DecisionMaker,
    additional_effects: &[ReplacementEffect],
    scope: Option<&ReplacementEventContext>,
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    if decision_maker.awaiting_choice() { return Ok(PreparedEventOutcome { original: EventOutcome::Prevented, programs: Vec::new() }); }
    // Preparation, entry commit, and split-card ordering share one owner.
    // This is earlier than any replacement consumption or provisional entry.
    let checkpoint = game.clone();
    let result = (|| {
        let PreparedEventOutcome { original, mut programs } = prepare_zone_change_scoped(
            game, object_id, from, to, cause.clone(), decision_maker,
            additional_effects,
            scope.and_then(|context| crate::events::downcast_event::<crate::events::ZoneChangeEvent>(context.event.inner()))
                .filter(|event| event.objects.as_slice() == [object_id] && event.from == from)
                .and_then(|event| event.snapshot.clone()),
            scope, Vec::new(), None,
        )?;
        // The caller still owns authored post-move work (face-down exile,
        // attachments, result tags, etc.) and the entire original batch.
        // Retain additions even if its original operation did not move.
        let original = (|| -> Result<EventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
        match original {
            EventOutcome::Proceed(prepared) => {
                let mut committed = commit_prepared_zone_change(game, object_id, prepared, decision_maker)?;
                if decision_maker.awaiting_choice() { return Ok(EventOutcome::Prevented); }
                programs.append(&mut committed.programs);
                let new_object_id = match committed.original {
                    EventOutcome::Proceed(id) => Some(id),
                    EventOutcome::Prevented => return Ok(EventOutcome::Prevented),
                    EventOutcome::Replaced => return Ok(EventOutcome::Replaced),
                    EventOutcome::NotApplicable => return Ok(EventOutcome::NotApplicable),
                };
                let final_zone = game.object(new_object_id.unwrap()).map(|card| card.zone).ok_or_else(||
                    crate::effects::ExecutionError::InternalError("zone commit arrival disappeared before receipt".into()))?;
                let mut new_object_ids = game.take_zone_change_results(object_id);
                if new_object_ids.is_empty() && let Some(id) = new_object_id {
                    new_object_ids.push(id);
                }
                if new_object_ids.is_empty() { return Ok(EventOutcome::Prevented); }
                let mut result = AppliedZoneChange { final_zone, new_object_id, new_object_ids };
                if from == Zone::Battlefield && matches!(final_zone, Zone::Graveyard | Zone::Exile) {
                    maybe_prompt_for_split_result_order(game, decision_maker, final_zone, &cause, &mut result);
                    if decision_maker.awaiting_choice() { return Ok(EventOutcome::Prevented); }
                    if !result.new_object_ids.is_empty() {
                        game.record_zone_change_results(object_id, result.new_object_ids.clone());
                    }
                }
                Ok(EventOutcome::Proceed(result))
            }
            EventOutcome::Prevented => Ok(EventOutcome::Prevented),
            EventOutcome::Replaced => Ok(EventOutcome::Replaced),
            EventOutcome::NotApplicable => Ok(EventOutcome::NotApplicable),
        }
        })()?;
        Ok(PreparedEventOutcome { original, programs })
    })();
    if result.is_err() || decision_maker.awaiting_choice() { *game = checkpoint; }
    if decision_maker.awaiting_choice() { return result.map(|_| PreparedEventOutcome { original: EventOutcome::Prevented, programs: Vec::new() }); }
    result
}

/// Commit the exact retained zone proposal. Keep the returned receipt until
/// authored follow-up work and the complete original batch have finished,
/// then pass it to `finish_zone_change_receipts`. The owning operation must
/// checkpoint before preparation, including pending choices and failures.
pub fn commit_zone_change_proposal(
    game: &mut GameState, object: ObjectId,
    prepared: PreparedEventOutcome<crate::events::processing::PreparedZoneChange>,
    dm: &mut dyn DecisionMaker,
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    if dm.awaiting_choice() { return Ok(PreparedEventOutcome { original: EventOutcome::Prevented, programs: Vec::new() }); }
    let checkpoint = game.clone();
    let result = (|| {
        let PreparedEventOutcome { original, mut programs } = prepared;
        let mut committed = match original {
            EventOutcome::Proceed(proposal) => {
                let zone = proposal.context.zone_change_context.as_ref().or_else(||
                    crate::events::downcast_event::<crate::events::ZoneChangeEvent>(proposal.context.event.inner()))
                    .ok_or_else(|| crate::effects::ExecutionError::InternalError("zone proposal has no original context".into()))?;
                let from = zone.from; let cause = zone.cause.clone();
                let provenance = proposal.context.event.provenance(); let lookback = proposal.pre_event_lookback.clone();
                let receipt = commit_prepared_zone_change(game, object, proposal, dm)?;
                if dm.awaiting_choice() { return Ok(PreparedEventOutcome { original: EventOutcome::Prevented, programs: Vec::new() }); }
                let mut promoted = promote_committed_zone_change_receipt(game, object, receipt)?;
                if let EventOutcome::Proceed(change) = &mut promoted.original {
                    if from == Zone::Battlefield && matches!(change.final_zone, Zone::Graveyard | Zone::Exile) {
                        maybe_prompt_for_split_result_order(game, dm, change.final_zone, &cause, change);
                        if dm.awaiting_choice() { return Ok(PreparedEventOutcome { original: EventOutcome::Prevented, programs: Vec::new() }); }
                        game.record_zone_change_results(object, change.new_object_ids.clone());
                    }
                    if change.final_zone == Zone::Battlefield {
                        for id in &change.new_object_ids {
                            let event = if game.is_tapped(*id) { crate::events::EnterBattlefieldEvent::tapped(*id, from) }
                                else { crate::events::EnterBattlefieldEvent::new(*id, from) };
                            game.queue_trigger_event(provenance, crate::triggers::TriggerEvent::new_with_provenance(event, provenance)
                                .with_lookback_source_snapshots(lookback.clone()));
                        }
                    }
                }
                promoted
            }
            EventOutcome::Prevented => PreparedEventOutcome { original: EventOutcome::Prevented, programs: Vec::new() },
            EventOutcome::Replaced => PreparedEventOutcome { original: EventOutcome::Replaced, programs: Vec::new() },
            EventOutcome::NotApplicable => PreparedEventOutcome { original: EventOutcome::NotApplicable, programs: Vec::new() },
        };
        programs.append(&mut committed.programs); committed.programs = programs; Ok(committed)
    })();
    if result.is_err() || dm.awaiting_choice() { *game = checkpoint; }
    result
}

/// Promote an exact commit receipt without rerunning replacement processing.
/// The caller owns any preparation prefix and the enclosing instruction.
pub(crate) fn promote_committed_zone_change_receipt(
    game: &mut GameState, object: ObjectId,
    receipt: PreparedEventOutcome<ObjectId>,
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    let original = match receipt.original {
        EventOutcome::Proceed(id) => {
            let final_zone = game.object(id).ok_or_else(|| crate::effects::ExecutionError::InternalError(
                "zone commit arrival disappeared before receipt promotion".into()))?.zone;
            let mut ids = game.take_zone_change_results(object);
            if ids.is_empty() { ids.push(id); }
            game.record_zone_change_results(object, ids.clone());
            EventOutcome::Proceed(AppliedZoneChange { final_zone, new_object_id: Some(id), new_object_ids: ids })
        }
        EventOutcome::Prevented => EventOutcome::Prevented,
        EventOutcome::Replaced => EventOutcome::Replaced,
        EventOutcome::NotApplicable => EventOutcome::NotApplicable,
    };
    Ok(PreparedEventOutcome { original, programs: receipt.programs })
}

/// Complete captured additions after the owner has finished the whole original
/// zone-changing instruction. The owner must checkpoint before preparation;
/// errors and pending choices here roll back to that earlier checkpoint.
/// All object identities and snapshots are frozen before any added program runs.
pub fn finish_zone_change_receipts(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    mut outcome: crate::effect::EffectOutcome,
    receipts: Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    let frozen = freeze_zone_change_receipts(game, receipts);
    finish_zone_change_receipts_frozen(game, ctx, outcome, frozen)
}

pub(crate) struct FrozenZoneChangeReceipts(Vec<(ObjectId, Vec<ObjectId>, Vec<crate::snapshot::ObjectSnapshot>, Vec<crate::events::processing::PreparedReplacementProgram>)>);

pub(crate) fn freeze_zone_change_receipts(
    game: &mut GameState, receipts: Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>,
) -> FrozenZoneChangeReceipts {
    let mut prepared = Vec::new();
    for (object, receipt) in receipts {
        if receipt.programs.is_empty() { continue; }
        let mut ids = match &receipt.original {
            EventOutcome::Proceed(change) => change.new_object_ids.clone(),
            _ => Vec::new(),
        };
        if ids.is_empty() {
            // An Instead movement can retain its own exact arrival receipt.
            // Read and restore it; never chase a later object by stable ID.
            ids = game.take_zone_change_results(object);
            if !ids.is_empty() { game.record_zone_change_results(object, ids.clone()); }
        }
        if ids.is_empty() { ids.push(object); }
        let snapshots = ids.iter().filter_map(|id| game.object(*id).map(|object|
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, game)))
            .collect::<Vec<_>>();
        prepared.push((object, ids, snapshots, receipt.programs));
    }
    FrozenZoneChangeReceipts(prepared)
}

pub(crate) fn finish_zone_change_receipts_frozen(
    game: &mut GameState, ctx: &mut crate::effects::ExecutionContext,
    mut outcome: crate::effect::EffectOutcome, frozen: FrozenZoneChangeReceipts,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    game.freeze_completed_entry_events(outcome.events.iter_mut())?;
    let primary = outcome.value.clone(); let prepared = frozen.0;
    for (object, ids, snapshots, programs) in prepared {
        outcome = crate::effects::replacement::execute_deferred_replacement_programs_with_bindings(
            game, ctx, outcome, programs, |_, context, _| {
                let zone = context.zone_change_context.as_ref().or_else(||
                    crate::events::downcast_event::<crate::events::ZoneChangeEvent>(context.event.inner()));
                let matches = zone.is_some_and(|zone| zone.objects.as_slice() == [object])
                    || crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(context.event.inner())
                        .is_some_and(|entry| entry.object == object);
                if !matches {
                    return Err(ExecutionError::InternalError("zone addition lost its original object context".into()));
                }
                let captured = if snapshots.is_empty() {
                    zone.and_then(|zone| zone.snapshot.clone()).into_iter().collect::<Vec<_>>()
                } else { snapshots.clone() };
                Ok(crate::effects::replacement::ReplacementProgramBindings {
                    targets: Some(ids.iter().copied().map(crate::effects::ResolvedTarget::Object).collect()),
                    object_tags: vec![("it".to_owned(), captured.clone()), ("__it__".to_owned(), captured)],
                })
            },
        )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(crate::effect::EffectOutcome::count(0)); }
    }
    outcome.value = primary;
    Ok(outcome)
}

pub(crate) use battlefield_entry::{
    BattlefieldEntryReceipt, finish_battlefield_entry_receipts,
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_batch_with_options,
    move_to_battlefield_batch_with_options_and_zone_proposals,
    move_to_battlefield_with_options, resolve_battlefield_entry_counters,
};

pub use destroy::DestroyEffect;
pub use destroy_no_regen::DestroyNoRegenerationEffect;
pub use exchange_zones::ExchangeZonesEffect;
pub use exile::ExileEffect;
pub use exile_until_source_leaves::{ExileUntilDuration, ExileUntilEffect};
pub use haunt_exile::HauntExileEffect;
pub use may_move_to_zone::MayMoveToZoneEffect;
pub use move_to_library_nth_from_top::MoveToLibraryNthFromTopEffect;
pub use move_to_library_top_or_bottom_choice::MoveToLibraryTopOrBottomChoiceEffect;
pub use move_to_zone::{
    BattlefieldController, LibraryPlacementOrder, MoveToZoneAttackTargetMode, MoveToZoneEffect,
};
pub use put_onto_battlefield::PutOntoBattlefieldEffect;
pub use reorder_graveyard::ReorderGraveyardEffect;
pub use reorder_library_top::ReorderLibraryTopEffect;
pub use return_all_to_battlefield::ReturnAllToBattlefieldEffect;
pub use return_from_graveyard_or_exile_to_battlefield::ReturnFromGraveyardOrExileToBattlefieldEffect;
pub use return_from_graveyard_to_battlefield::{
    ReturnAsAuraOptions, ReturnFromGraveyardToBattlefieldEffect,
};
pub use return_from_graveyard_to_hand::ReturnFromGraveyardToHandEffect;
pub use return_to_hand::ReturnToHandEffect;
pub use sacrifice::{
    EachPlayerSacrificesEffect, SacrificeEffect, SacrificePlayerEffect, SacrificeTargetEffect,
};
pub use shuffle_objects_into_library::ShuffleObjectsIntoLibraryEffect;

#[cfg(test)]
mod zone_entry_carrier_tests;
