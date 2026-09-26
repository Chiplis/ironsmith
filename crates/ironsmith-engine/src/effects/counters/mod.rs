//! Counter effects.
//!
//! This module contains effects that manipulate counters on objects and players,
//! such as putting counters, removing counters, moving counters, and proliferate.

mod double_counters;
mod for_each_counter_kind_put_or_remove;
mod move_all_counters;
mod move_counters;
mod move_one_counter;
mod proliferate;
mod put_counter_of_chosen_kind;
mod put_counters;
mod remove_any_counters_among;
mod remove_any_counters_from_source;
mod remove_counters;
mod remove_up_to_any_counters;
mod remove_up_to_counters;

pub use double_counters::DoubleCountersEffect;
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
    game.object(to_id).is_some() && game.can_have_counter_type_placed(to_id, counter_type)
}

/// The "put" half of moving counters (CR 122.5, 122.8) is an ordinary
/// counter placement, so counter replacements apply to it (CR 614.1).
pub(crate) fn put_moved_counters(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    to_id: ObjectId,
    counter_type: CounterType,
    count: u32,
) -> Option<crate::triggers::TriggerEvent> {
    if count == 0 {
        return None;
    }
    let final_count = crate::events::processing::process_put_counters_with_event_with_dm(
        game,
        to_id,
        counter_type,
        count,
        ctx.cause.clone(),
        &mut *ctx.decision_maker,
    );
    if final_count == 0 {
        return None;
    }
    game.add_counters_with_source(
        to_id,
        counter_type,
        final_count,
        Some(ctx.source),
        Some(ctx.controller),
    )
}
