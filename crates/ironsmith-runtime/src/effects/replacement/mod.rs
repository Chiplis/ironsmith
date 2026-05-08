//! Replacement effect helpers.

mod apply_replacement;
mod register_future_zone_replacement;
mod register_zone_replacement;

pub use apply_replacement::{ApplyReplacementEffect, ReplacementApplyMode};
pub use register_enter_under_control::RegisterEnterUnderControlReplacementEffect;
pub use register_future_zone_replacement::RegisterFutureZoneReplacementEffect;
pub use register_zone_replacement::RegisterZoneReplacementEffect;

mod register_enter_under_control;
pub(crate) use register_zone_replacement::zone_replacement_action;
