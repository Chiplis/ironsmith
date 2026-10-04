//! Counter effects.
//!
//! This module contains effects that manipulate counters on objects and players,
//! such as putting counters, removing counters, moving counters, and proliferate.

mod double_counters;
mod for_each_counter_kind_put_or_remove;
mod move_all_counters;
mod move_counters;
mod move_one_counter;
mod player_counter_placement;
mod object_counter_placement;
mod proliferate;
mod put_counter_of_chosen_kind;
mod put_counters;
mod remove_any_counters_among;
mod remove_any_counters_from_source;
mod remove_counters;
mod remove_up_to_any_counters;
mod remove_up_to_counters;

pub use double_counters::DoubleCountersEffect;
pub(crate) use player_counter_placement::execute_player_counter_placement;
pub(crate) use object_counter_placement::execute_object_counter_placement;
pub use for_each_counter_kind_put_or_remove::ForEachCounterKindPutOrRemoveEffect;
pub use move_all_counters::MoveAllCountersEffect;
pub use move_counters::MoveCountersEffect;
pub use move_one_counter::MoveOneCounterEffect;
pub use proliferate::ProliferateEffect;
pub use put_counter_of_chosen_kind::PutCounterOfChosenKindEffect;
pub use put_counters::PutCountersEffect;
pub use remove_any_counters_among::RemoveAnyCountersAmongEffect;
pub use remove_any_counters_among::cost_display as remove_any_counters_among_cost_display;
pub(crate) use remove_any_counters_among::{
    total_available as remove_any_counters_among_total_available,
    valid_targets_with_tags as remove_any_counters_among_valid_targets_with_tags,
};
pub use remove_any_counters_from_source::RemoveAnyCountersFromSourceEffect;
pub use remove_counters::RemoveCountersEffect;
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
pub(crate) fn put_moved_counters(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    to_id: ObjectId,
    counter_type: CounterType,
    count: u32,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    let event = crate::events::Event::put_counters(to_id, counter_type, count, ctx.cause.clone())
        .with_provenance(ctx.provenance);
    execute_object_counter_placement(game, ctx, event)
}

/// The removal half of a live counter move is independently replaceable.
/// The caller keeps its preflight movement budget for the placement half;
/// replacing removal does not rewrite that separately proposed event.
pub(crate) fn remove_moved_counters(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    from_id: ObjectId,
    counter_type: CounterType,
    count: u32,
) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
    let event = crate::events::Event::remove_counters(from_id, counter_type, count)
        .with_provenance(ctx.provenance);
    remove_counters::execute_counter_removal_event(game, ctx, event)
}

/// Bind the two endpoint roles by position, retaining empty assignments for
/// illegal targets. Equal filters do not make these the same target role.
fn assigned_counter_transfer_pair(ctx: &ExecutionContext) -> Option<(ObjectId, ObjectId)> {
    if ctx.target_assignments.is_empty() {
        if !ctx.announced_target_assignments.is_empty() { return None; }
        return ctx.resolve_two_object_targets();
    }
    let endpoint = |index: usize| {
        let assignment = ctx.target_assignments.get(index)?;
        if assignment.range.is_empty() { return None; }
        match ctx.targets.get(assignment.range.start)? {
            crate::effects::ResolvedTarget::Object(id) => Some(*id), _ => None,
        }
    };
    endpoint(0).zip(endpoint(1))
}
mod prepared_placement;
pub(crate) use prepared_placement::{PreparedCounterPlacement, prepare_counter_placement, commit_prepared_counter_original};
