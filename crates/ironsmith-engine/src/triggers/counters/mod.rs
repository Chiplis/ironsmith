//! Counter-related triggers.

mod counter_put_on;
mod counter_recipient_groups;
mod counter_removed_from;
mod player_gets_counters;
mod saga_chapter;

pub use counter_put_on::CounterPutOnTrigger;
pub(crate) use counter_recipient_groups::coalesce_counter_recipient_groups;
pub use counter_removed_from::CounterRemovedFromTrigger;
pub use player_gets_counters::PlayerGetsCountersTrigger;
pub use saga_chapter::SagaChapterTrigger;
