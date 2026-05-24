//! Counter effects.
//!
//! This module contains effects that manipulate counters on objects and players,
//! such as putting counters, removing counters, moving counters, and proliferate.

mod for_each_counter_kind_put_or_remove;
mod move_all_counters;
mod move_counters;
mod move_one_counter;
mod proliferate;
mod put_counters;
mod remove_any_counters_among;
mod remove_any_counters_from_source;
mod remove_counters;
mod remove_up_to_any_counters;
mod remove_up_to_counters;

pub use for_each_counter_kind_put_or_remove::ForEachCounterKindPutOrRemoveEffect;
pub use move_all_counters::MoveAllCountersEffect;
pub use move_counters::MoveCountersEffect;
pub use move_one_counter::MoveOneCounterEffect;
pub use proliferate::ProliferateEffect;
pub use put_counters::PutCountersEffect;
pub use remove_any_counters_among::RemoveAnyCountersAmongEffect;
pub(crate) use remove_any_counters_among::{
    cost_display as remove_any_counters_among_cost_display,
    valid_targets_with_tags as remove_any_counters_among_valid_targets_with_tags,
};
pub use remove_any_counters_from_source::RemoveAnyCountersFromSourceEffect;
pub use remove_counters::RemoveCountersEffect;
pub use remove_up_to_any_counters::RemoveUpToAnyCountersEffect;
pub use remove_up_to_counters::RemoveUpToCountersEffect;
