//! Counter effects.
//!
//! This module contains effects that manipulate counters on objects and players,
//! such as putting counters, removing counters, moving counters, and proliferate.

mod move_all_counters;
mod move_counters;
mod proliferate;
mod put_counters;
mod remove_counters;
mod remove_up_to_any_counters;
mod remove_up_to_counters;

pub use move_all_counters::MoveAllCountersEffect;
pub use move_counters::MoveCountersEffect;
pub use proliferate::ProliferateEffect;
pub use put_counters::PutCountersEffect;
pub use remove_counters::RemoveCountersEffect;
pub use remove_up_to_any_counters::RemoveUpToAnyCountersEffect;
pub use remove_up_to_counters::RemoveUpToCountersEffect;
