//! Zone change effects.
//!
//! This module contains effects that move objects between zones,
//! such as destroy, exile, sacrifice, and return to hand.

use std::collections::HashSet;

use crate::DecisionMaker;
use crate::decisions::context::{DecisionContext, OrderContext, enrich_display_hints};
use crate::events::processing::{
    EventOutcome, PreparedEventOutcome, ReplacementEventContext, prepare_zone_change_scoped,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::ReplacementEffect;
use crate::zone::Zone;

mod land_play;
pub(crate) use land_play::{
    LandPlayAuthorization, LandPlayObservationKind, LandPlayObservationTiming,
    execute_land_play_program, land_play_restriction_applies,
    play_land_from_resolving_effect_with_outputs,
};
mod entry_observation;
pub(crate) mod movement_instruction;
mod prepared_move;
pub use entry_observation::battlefield_entry_observation;
pub(crate) use prepared_move::{
    PreparedZoneMove, commit_zone_moves, commit_zone_moves_with_completion,
    complete_movement_batch, execute_battlefield_entries, execute_battlefield_entries_with_outputs,
    execute_zone_moves, execute_zone_moves_with_outputs, group_zone_move_observations,
    observe_zone_move_originals, observe_zone_move_originals_with_cause, prepare_zone_moves,
    resolve_zone_move_objects,
};
mod battlefield_entry;
mod continuation;
pub(crate) use continuation::{
    ZoneInstructionDraws, complete_zone_instruction, prepare_zone_instruction_completion,
};
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

/// Exact successor identities produced by this movement, before additions.
/// Replacement-owned arrivals remain recorded for later receipt consumers.
/// A prevented or inapplicable action never inherits a stale movement result.
pub(crate) fn movement_arrivals(
    game: &mut GameState,
    object: ObjectId,
    receipt: &PreparedEventOutcome<AppliedZoneChange>,
) -> Vec<ObjectId> {
    match &receipt.original {
        EventOutcome::Proceed(change) => change.new_object_ids.clone(),
        EventOutcome::Replaced => {
            let ids = game.take_zone_change_results(object);
            if !ids.is_empty() {
                game.record_zone_change_results(object, ids.clone());
            }
            ids
        }
        EventOutcome::Prevented | EventOutcome::NotApplicable => Vec::new(),
    }
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
    apply_zone_change_owned(
        game,
        object_id,
        from,
        to,
        cause,
        decision_maker,
        additional_effects,
        None,
    )
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
        game,
        object_id,
        from,
        to,
        cause,
        ctx,
        additional_effects,
        None,
    )
}
/// Contextual compounds retain the actual entry packet with their movement.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_zone_change_with_context_and_additional_effects_with_outputs(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
    additional_effects: &[ReplacementEffect],
) -> Result<
    crate::events::processing::CommittedZoneChange<AppliedZoneChange>,
    crate::effects::ExecutionError,
> {
    apply_zone_change_with_context_and_additional_effects_and_snapshot_with_outputs(
        game,
        object_id,
        from,
        to,
        cause,
        ctx,
        additional_effects,
        None,
    )
}

/// Native movement owners can retain event-created draws while keeping the
/// exact normal zone preparation/commit and additional-program receipts.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_zone_change_with_context_and_draws(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
    additional: &[ReplacementEffect],
    draws: &mut ZoneInstructionDraws,
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    if !draws.prepared.contains_key(&object) {
        prepare_zone_change_with_context_and_draws(
            game, object, from, to, cause, ctx, additional, draws,
        )?;
    }
    let Some(mut prepared) = draws.prepared.remove(&object) else {
        return Ok(PreparedEventOutcome::pure(EventOutcome::NotApplicable));
    };
    if matches!(&prepared.original, EventOutcome::Proceed(_))
        && !game
            .object(object)
            .is_some_and(|current| current.zone == from)
    {
        prepared.original = EventOutcome::NotApplicable;
    }
    draws.commit_pending_replacement(game, object, &mut *ctx.decision_maker)?;
    let committed =
        commit_zone_change_proposal_with_outputs(game, object, prepared, &mut *ctx.decision_maker)?;
    Ok(draws.retain_committed_zone_receipt(committed))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_zone_change_with_context_and_draws(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
    additional: &[ReplacementEffect],
    draws: &mut ZoneInstructionDraws,
) -> Result<(), crate::effects::ExecutionError> {
    let snapshot = match draws.snapshots.get(&object) {
        Some(snapshot) => Some(snapshot.clone()),
        None => game
            .object(object)
            .map(|object| {
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                    object, game,
                )
            })
            .transpose()?,
    };
    if let Some(snapshot) = &snapshot {
        draws.snapshots.insert(object, snapshot.clone());
    }
    let scope = ReplacementEventContext::with_scope(
        game,
        crate::events::Event::zone_change(object, from, to, cause.clone(), snapshot.clone())
            .with_provenance(ctx.provenance),
        &ctx.replacement,
    );
    let start = draws.draws.0.len();
    let prepared = crate::events::processing::prepare_zone_change_scoped_with_draws(
        game,
        object,
        from,
        to,
        cause,
        &mut *ctx.decision_maker,
        additional,
        snapshot,
        Some(&scope),
        Vec::new(),
        None,
        Some(&mut draws.draws),
    )?;
    draws.record(object, start);
    draws.prepared.insert(object, prepared);
    Ok(())
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
    apply_zone_change_with_context_and_additional_effects_and_snapshot_with_outputs(
        game,
        object_id,
        from,
        to,
        cause,
        ctx,
        additional_effects,
        snapshot,
    )
    .map(|committed| committed.receipt)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_zone_change_with_context_and_additional_effects_and_snapshot_with_outputs(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    cause: crate::events::cause::EventCause,
    ctx: &mut crate::effects::ExecutionContext,
    additional_effects: &[ReplacementEffect],
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<
    crate::events::processing::CommittedZoneChange<AppliedZoneChange>,
    crate::effects::ExecutionError,
> {
    PreparedZoneMove::capture(game, object_id, from, to, cause, snapshot).commit_with_outputs(
        game,
        ctx,
        additional_effects,
    )
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
    if decision_maker.awaiting_choice() {
        return Ok(PreparedEventOutcome {
            original: EventOutcome::Prevented,
            programs: Vec::new(),
        });
    }
    // Preparation, entry commit, and split-card ordering share one owner.
    // This is earlier than any replacement consumption or provisional entry.
    crate::effects::composition::execute_decision_transaction(
        game,
        decision_maker,
        || PreparedEventOutcome {
            original: EventOutcome::Prevented,
            programs: Vec::new(),
        },
        |game, decision_maker| {
            let prepared = prepare_zone_change_scoped(
                game,
                object_id,
                from,
                to,
                cause,
                decision_maker,
                additional_effects,
                scope
                    .and_then(|context| {
                        crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                            context.event.inner(),
                        )
                    })
                    .filter(|event| event.objects.as_slice() == [object_id] && event.from == from)
                    .and_then(|event| event.snapshot.clone()),
                scope,
                Vec::new(),
                None,
            )?;
            commit_zone_change_proposal(game, object_id, prepared, decision_maker)
        },
    )
}

/// Commit the exact retained zone proposal. Keep the returned receipt until
/// authored follow-up work and the complete original batch have finished,
/// then pass it to `finish_zone_change_receipts`. The owning operation must
/// checkpoint before preparation, including pending choices and failures.
pub fn commit_zone_change_proposal(
    game: &mut GameState,
    object: ObjectId,
    prepared: PreparedEventOutcome<crate::events::processing::PreparedZoneChange>,
    dm: &mut dyn DecisionMaker,
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    commit_zone_change_proposal_with_outputs(game, object, prepared, dm)
        .map(|committed| committed.receipt)
}

/// Preserve the native entry producer's references through receipt promotion.
pub(crate) fn commit_zone_change_proposal_with_outputs(
    game: &mut GameState,
    object: ObjectId,
    prepared: PreparedEventOutcome<crate::events::processing::PreparedZoneChange>,
    dm: &mut dyn DecisionMaker,
) -> Result<
    crate::events::processing::CommittedZoneChange<AppliedZoneChange>,
    crate::effects::ExecutionError,
> {
    if dm.awaiting_choice() {
        return Ok(
            crate::events::processing::CommittedZoneChange::from_receipt(PreparedEventOutcome {
                original: EventOutcome::Prevented,
                programs: Vec::new(),
            }),
        );
    }
    let mut published_outputs = Vec::new();
    let result =
        crate::effects::composition::execute_result_decision_transaction(game, dm, |game, dm| {
            let PreparedEventOutcome {
                original,
                mut programs,
            } = prepared;
            let mut committed = match original {
                EventOutcome::Proceed(proposal) => {
                    if !proposal.original_is_current(game, object)? {
                        return Ok(PreparedEventOutcome {
                            original: EventOutcome::NotApplicable,
                            programs,
                        });
                    }
                    let zone = proposal
                        .context
                        .zone_change_context
                        .as_ref()
                        .or_else(|| {
                            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                                proposal.context.event.inner(),
                            )
                        })
                        .ok_or_else(|| {
                            crate::effects::ExecutionError::InternalError(
                                "zone proposal has no original context".into(),
                            )
                        })?;
                    let from = zone.from;
                    let cause = zone.cause.clone();
                    let provenance = proposal.context.event.provenance();
                    let lookback = proposal.pre_event_lookback.clone();
                    let committed =
                        crate::events::processing::commit_prepared_zone_change_with_outputs(
                            game, object, proposal, dm,
                        )?;
                    published_outputs.extend(committed.published_outputs);
                    let receipt = committed.receipt;
                    if dm.awaiting_choice() {
                        return Ok(PreparedEventOutcome {
                            original: EventOutcome::Prevented,
                            programs: Vec::new(),
                        });
                    }
                    let mut promoted =
                        promote_committed_zone_change_receipt(game, object, receipt)?;
                    if let EventOutcome::Proceed(change) = &mut promoted.original {
                        if from == Zone::Battlefield
                            && matches!(change.final_zone, Zone::Graveyard | Zone::Exile)
                        {
                            maybe_prompt_for_split_result_order(
                                game,
                                dm,
                                change.final_zone,
                                &cause,
                                change,
                            );
                            if dm.awaiting_choice() {
                                return Ok(PreparedEventOutcome {
                                    original: EventOutcome::Prevented,
                                    programs: Vec::new(),
                                });
                            }
                            game.record_zone_change_results(object, change.new_object_ids.clone());
                        }
                        if change.final_zone == Zone::Battlefield {
                            for id in &change.new_object_ids {
                                let enters_tapped = game.is_tapped(*id);
                                let event = battlefield_entry_observation(
                                    game,
                                    *id,
                                    from,
                                    enters_tapped,
                                    provenance,
                                    lookback.clone(),
                                )?;
                                game.queue_trigger_event(provenance, event);
                            }
                        }
                    }
                    promoted
                }
                EventOutcome::Prevented => PreparedEventOutcome {
                    original: EventOutcome::Prevented,
                    programs: Vec::new(),
                },
                EventOutcome::Replaced => PreparedEventOutcome {
                    original: EventOutcome::Replaced,
                    programs: Vec::new(),
                },
                EventOutcome::NotApplicable => PreparedEventOutcome {
                    original: EventOutcome::NotApplicable,
                    programs: Vec::new(),
                },
            };
            programs.append(&mut committed.programs);
            committed.programs = programs;
            Ok(committed)
        });
    if dm.awaiting_choice() {
        published_outputs.clear();
    }
    result.map(|receipt| crate::events::processing::CommittedZoneChange {
        receipt,
        published_outputs,
    })
}

/// Promote an exact commit receipt without rerunning replacement processing.
/// The caller owns any preparation prefix and the enclosing instruction.
pub(crate) fn promote_committed_zone_change_receipt(
    game: &mut GameState,
    object: ObjectId,
    receipt: PreparedEventOutcome<ObjectId>,
) -> Result<PreparedEventOutcome<AppliedZoneChange>, crate::effects::ExecutionError> {
    let original = match receipt.original {
        EventOutcome::Proceed(id) => {
            let final_zone = game
                .object(id)
                .ok_or_else(|| {
                    crate::effects::ExecutionError::InternalError(
                        "zone commit arrival disappeared before receipt promotion".into(),
                    )
                })?
                .zone;
            let mut ids = game.take_zone_change_results(object);
            if ids.is_empty() {
                ids.push(id);
            }
            game.record_zone_change_results(object, ids.clone());
            EventOutcome::Proceed(AppliedZoneChange {
                final_zone,
                new_object_id: Some(id),
                new_object_ids: ids,
            })
        }
        EventOutcome::Prevented => EventOutcome::Prevented,
        EventOutcome::Replaced => EventOutcome::Replaced,
        EventOutcome::NotApplicable => EventOutcome::NotApplicable,
    };
    Ok(PreparedEventOutcome {
        original,
        programs: receipt.programs,
    })
}

/// Complete captured additions after the owner has finished the whole original
/// zone-changing instruction. The owner must checkpoint before preparation;
/// errors and pending choices here roll back to that earlier checkpoint.
/// All object identities and snapshots are frozen before any added program runs.
pub fn finish_zone_change_receipts(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    outcome: crate::effect::EffectOutcome,
    receipts: Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    finish_zone_change_receipts_with_outputs(game, ctx, outcome, receipts)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn finish_zone_change_receipts_with_outputs<O: crate::effects::OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    outcome: O,
    receipts: Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    let frozen = freeze_zone_change_receipts(game, receipts);
    finish_zone_change_receipts_frozen_with_outputs(game, ctx, outcome, frozen)
}

pub(crate) struct FrozenZoneChangeReceipts(
    Result<
        Vec<(
            ObjectId,
            Vec<ObjectId>,
            Vec<crate::snapshot::ObjectSnapshot>,
            Vec<crate::events::processing::PreparedReplacementProgram>,
        )>,
        crate::effects::ExecutionError,
    >,
);

pub(crate) fn freeze_zone_change_receipts(
    game: &mut GameState,
    receipts: Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>,
) -> FrozenZoneChangeReceipts {
    let captured = (|| {
        let mut prepared = Vec::new();
        for (object, receipt) in receipts {
            if receipt.programs.is_empty() {
                continue;
            }
            let mut ids = match &receipt.original {
                EventOutcome::Proceed(change) => change.new_object_ids.clone(),
                _ => Vec::new(),
            };
            if ids.is_empty() {
                // An Instead movement can retain its own exact arrival receipt.
                // Read and restore it; never chase a later object by stable ID.
                ids = game.take_zone_change_results(object);
                if !ids.is_empty() {
                    game.record_zone_change_results(object, ids.clone());
                }
            }
            if ids.is_empty() {
                ids.push(object);
            }
            let snapshots = ids.iter().filter_map(|id| game.object(*id).map(|object|
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)))
                .collect::<Result<Vec<_>, crate::effects::ExecutionError>>()?;
            prepared.push((object, ids, snapshots, receipt.programs));
        }
        Ok(prepared)
    })();
    if let Err(error) = &captured {
        game.record_token_resource_failure(error);
    }
    FrozenZoneChangeReceipts(captured)
}

pub(crate) fn bind_frozen_zone_programs(
    frozen: FrozenZoneChangeReceipts,
) -> Result<
    Vec<(
        crate::events::processing::PreparedReplacementProgram,
        crate::effects::replacement::ReplacementProgramBindings,
    )>,
    crate::effects::ExecutionError,
> {
    let mut bound = Vec::new();
    for (object, ids, snapshots, programs) in frozen.0? {
        for program in programs {
            let context = &program.context;
            let zone = context.zone_change_context.as_ref().or_else(|| {
                crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                    context.event.inner(),
                )
            });
            let matches = zone.is_some_and(|zone| zone.objects.as_slice() == [object])
                || crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(
                    context.event.inner(),
                )
                .is_some_and(|entry| entry.object == object);
            if !matches {
                return Err(crate::effects::ExecutionError::InternalError(
                    "zone addition lost its original object context".into(),
                ));
            }
            let captured = if snapshots.is_empty() {
                zone.and_then(|zone| zone.snapshot.clone())
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                snapshots.clone()
            };
            let bindings = crate::effects::replacement::ReplacementProgramBindings {
                targets: Some(
                    ids.iter()
                        .copied()
                        .map(crate::effects::ResolvedTarget::Object)
                        .collect(),
                ),
                object_tags: vec![
                    ("it".to_owned(), captured.clone()),
                    ("__it__".to_owned(), captured),
                ],
            };
            bound.push((program, bindings));
        }
    }
    Ok(bound)
}

pub(crate) fn finish_zone_change_receipts_frozen(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    outcome: crate::effect::EffectOutcome,
    frozen: FrozenZoneChangeReceipts,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    finish_zone_change_receipts_frozen_with_outputs(game, ctx, outcome, frozen)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn finish_zone_change_receipts_frozen_with_outputs<
    O: crate::effects::OriginalEffectOutput,
>(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    outcome: O,
    frozen: FrozenZoneChangeReceipts,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    let mut outputs = outcome.into_outputs();
    game.freeze_completed_entry_events(outputs.outcome.events.iter_mut())?;
    crate::effects::outcome_recording::complete_outcome(
        game,
        None,
        Some(ctx.controller),
        &mut outputs.outcome,
        Vec::new(),
    );
    outputs.synchronize_observations();
    let primary = outputs.outcome.value.clone();
    let prepared = frozen.0?;
    for (object, ids, snapshots, programs) in prepared {
        outputs = crate::effects::replacement::complete_replacement_programs_with_original_outputs(
            game,
            ctx,
            outputs,
            |game, ctx, original| {
                crate::effects::replacement::complete_deferred_replacement_programs_with_bindings(
                    game,
                    ctx,
                    original,
                    programs,
                    |_, context, _| {
                        let zone = context.zone_change_context.as_ref().or_else(|| {
                            crate::events::downcast_event::<crate::events::ZoneChangeEvent>(
                                context.event.inner(),
                            )
                        });
                        let matches =
                            zone.is_some_and(|zone| zone.objects.as_slice() == [object])
                                || crate::events::downcast_event::<
                                    crate::events::EnterBattlefieldEvent,
                                >(context.event.inner())
                                .is_some_and(|entry| entry.object == object);
                        if !matches {
                            return Err(ExecutionError::InternalError(
                                "zone addition lost its original object context".into(),
                            ));
                        }
                        let captured = if snapshots.is_empty() {
                            zone.and_then(|zone| zone.snapshot.clone())
                                .into_iter()
                                .collect::<Vec<_>>()
                        } else {
                            snapshots.clone()
                        };
                        Ok(crate::effects::replacement::ReplacementProgramBindings {
                            targets: Some(
                                ids.iter()
                                    .copied()
                                    .map(crate::effects::ResolvedTarget::Object)
                                    .collect(),
                            ),
                            object_tags: vec![
                                ("it".to_owned(), captured.clone()),
                                ("__it__".to_owned(), captured),
                            ],
                        })
                    },
                )
            },
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                crate::effect::EffectOutcome::count(0),
            ));
        }
    }
    outputs.outcome.value = primary;
    Ok(outputs)
}

pub(crate) use battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, BattlefieldEntryReceipt,
    PreparedBattlefieldEntryBatch, finish_battlefield_entry_receipts,
    finish_battlefield_entry_receipts_with_outputs, move_to_battlefield_batch_with_companions,
    move_to_battlefield_batch_with_options, move_to_battlefield_with_options,
    prepare_battlefield_entry_batch, resolve_battlefield_entry_counters,
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

pub(crate) use sacrifice::sacrifice_selected_objects_with_original_outputs;
