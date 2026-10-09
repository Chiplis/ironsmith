//! Counter effects.
//!
//! This module contains effects that manipulate counters on objects and players,
//! such as putting counters, removing counters, moving counters, and proliferate.

mod double_counters;
mod for_each_counter_kind_put_or_remove;
mod move_all_counters;
mod move_counters;
mod move_one_counter;
mod object_counter_placement;
mod player_counter_placement;
mod prepared_payment;
mod proliferate;
mod put_counter_of_chosen_kind;
mod put_counters;
mod remove_any_counters_among;
mod remove_any_counters_from_source;
mod remove_counters;
pub(crate) use prepared_payment::{capture_counter_payment, capture_counter_payment_with_quantity};
mod remove_up_to_any_counters;
mod remove_up_to_counters;

pub use double_counters::DoubleCountersEffect;
pub use for_each_counter_kind_put_or_remove::ForEachCounterKindPutOrRemoveEffect;
pub use move_all_counters::MoveAllCountersEffect;
pub use move_counters::MoveCountersEffect;
pub use move_one_counter::MoveOneCounterEffect;
pub(crate) use object_counter_placement::{
    execute_object_counter_placement, execute_object_counter_placement_with_outputs,
};
pub(crate) use player_counter_placement::execute_player_counter_placement;
pub(crate) use player_counter_placement::execute_player_counter_placement_with_outputs;
pub(crate) use player_counter_placement::prepare_player_counter_instruction;
pub use proliferate::ProliferateEffect;
pub use put_counter_of_chosen_kind::{
    PutCounterOfChosenKindEffect, PutCounterOfKindChosenFromEffect,
};
pub use put_counters::PutCountersEffect;
pub use remove_any_counters_among::RemoveAnyCountersAmongEffect;
pub use remove_any_counters_among::cost_display as remove_any_counters_among_cost_display;
pub(crate) use remove_any_counters_among::total_available_with_tags;
pub(crate) use remove_any_counters_among::{
    total_available as remove_any_counters_among_total_available,
    valid_targets_with_tags as remove_any_counters_among_valid_targets_with_tags,
};
pub use remove_any_counters_from_source::RemoveAnyCountersFromSourceEffect;
pub use remove_counters::RemoveCountersEffect;
pub(crate) use remove_counters::counter_cost_x_from_outcome;
pub(crate) use remove_counters::prepare_game_rule_counter_removal;
pub(crate) use remove_counters::{
    PreparedCounterRemoval, commit_prepared_counter_removal_original_with_outputs,
    prepare_counter_removal,
};
pub use remove_up_to_any_counters::RemoveUpToAnyCountersEffect;
pub use remove_up_to_counters::RemoveUpToCountersEffect;

use crate::effects::ExecutionContext;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::object::CounterType;

/// CR 122.5: a counter can be moved only if it can be put onto the second
/// object; otherwise nothing is removed and nothing is put. This covers both
/// "can't have counters" and kind-specific prohibitions (Melira).
pub(crate) fn move_destination_can_receive_counters(
    game: &GameState,
    to_id: ObjectId,
    counter_type: CounterType,
) -> bool {
    // CR 702.26b: an ordinary move cannot use a phased-out destination.
    game.object(to_id).is_some()
        && !game.is_phased_out(to_id)
        && game.can_have_counter_type_placed(to_id, counter_type)
}

/// The "put" half of moving counters (CR 122.5, 122.8) is an ordinary
/// counter placement, so counter replacements apply to it (CR 614.1).
pub(crate) fn put_moved_counters_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    to_id: ObjectId,
    counter_type: CounterType,
    count: u32,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    let event = crate::events::Event::put_counters(to_id, counter_type, count, ctx.cause.clone())
        .with_provenance(ctx.provenance);
    execute_counter_placement_with_outputs(game, ctx, event)
}

/// The removal half of a live counter move is independently replaceable.
/// The caller keeps its preflight movement budget for the placement half;
/// replacing removal does not rewrite that separately proposed event.
pub(crate) fn remove_moved_counters_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    from_id: ObjectId,
    counter_type: CounterType,
    count: u32,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    let event = crate::events::Event::remove_counters(from_id, counter_type, count)
        .with_provenance(ctx.provenance);
    execute_counter_removal_with_outputs(game, ctx, event)
}

/// Bind the two endpoint roles by position, retaining empty assignments for
/// illegal targets. Equal filters do not make these the same target role.
fn assigned_counter_transfer_pair(ctx: &ExecutionContext) -> Option<(ObjectId, ObjectId)> {
    if ctx.target_assignments.is_empty() {
        if !ctx.announced_target_assignments.is_empty() {
            return None;
        }
        return ctx.resolve_two_object_targets();
    }
    let endpoint = |index: usize| {
        let assignment = ctx.target_assignments.get(index)?;
        if assignment.range.is_empty() {
            return None;
        }
        match ctx.targets.get(assignment.range.start)? {
            crate::effects::ResolvedTarget::Object(id) => Some(*id),
            _ => None,
        }
    };
    endpoint(0).zip(endpoint(1))
}
mod prepared_placement;
pub(crate) use prepared_placement::{
    PreparedCounterPlacement, commit_prepared_counter_original_with_outputs,
    commit_scoped_counter_placement_originals_with_outputs, execute_scoped_counter_placements,
    prepare_counter_placement,
};

mod placement;
pub(crate) use placement::{
    execute_counter_batch_with_outputs, execute_counter_placement_with_outputs,
    execute_counter_removal_cost, execute_counter_removal_cost_with_outputs,
    execute_counter_removal_with_outputs, execute_player_counter_removal_with_outputs,
    prepare_counter_removal_cost,
};
pub(crate) use placement::{
    execute_counter_removal_cost_batch, execute_counter_removal_cost_batch_with_outputs,
};

/// Commit one transfer budget. Live transfers have independently replaceable
/// removal and placement halves; historical transfers have placement only.
/// Callers bind live source identity before replacement programs can move it.
pub(crate) fn transfer_counters_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    source: Option<(ObjectId, crate::zone::Zone)>,
    destination: ObjectId,
    kind: CounterType,
    requested: u32,
) -> Result<crate::effects::CompletedEffectOutputs, crate::effects::ExecutionError> {
    use crate::effect::EffectOutcome;
    use crate::effects::CompletedEffectOutputs;
    if requested == 0 || !move_destination_can_receive_counters(game, destination, kind) {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let budget = match source {
        Some((id, zone)) => {
            if id == destination
                || game.is_phased_out(id)
                || !game.object(id).is_some_and(|object| object.zone == zone)
            {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            requested.min(game.counter_count(id, kind))
        }
        None => requested,
    };
    if budget == 0 {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let removal = source
                .map(|(id, _)| {
                    let event = crate::events::Event::remove_counters(id, kind, budget)
                        .with_provenance(ctx.provenance);
                    prepare_counter_removal(game, ctx, event)
                })
                .transpose()?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            let placement = prepare_counter_placement(
                game,
                ctx,
                crate::events::Event::put_counters(destination, kind, budget, ctx.cause.clone())
                    .with_provenance(ctx.provenance),
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            let children =
                crate::effects::composition::execute_simultaneous_originals_with_default_outputs(
                    game,
                    ctx,
                    true,
                    |game, ctx| {
                        let mut originals = Vec::new();
                        if let Some(removal) = removal {
                            originals.push(commit_prepared_counter_removal_original_with_outputs(
                                game, ctx, removal,
                            )?);
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(Vec::new());
                            }
                        }
                        originals.push(commit_prepared_counter_original_with_outputs(
                            game, ctx, placement,
                        )?);
                        Ok(originals)
                    },
                )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            // MoveAll/MoveOne keep their historical budget receipt while the
            // counted transfer instruction owns its actual-removal projection.
            Ok(CompletedEffectOutputs::with_primary_result(
                EffectOutcome::count(budget),
                children,
            ))
        },
    )
}
