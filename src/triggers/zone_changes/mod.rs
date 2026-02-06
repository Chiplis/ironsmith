//! Zone change triggers.
//!
//! This module contains triggers that fire when objects move between zones,
//! such as entering/leaving the battlefield and dying.
//!
//! `ZoneChangeTrigger` is the composable primitive for these patterns.

mod enters_tapped;
mod enters_untapped;
mod zone_change_trigger;

pub use enters_tapped::EntersBattlefieldTappedTrigger;
pub use enters_untapped::EntersBattlefieldUntappedTrigger;

// Composable zone change trigger and supporting types
pub use zone_change_trigger::{CountMode, PlayerRelation, ZoneChangeTrigger, ZonePattern};
