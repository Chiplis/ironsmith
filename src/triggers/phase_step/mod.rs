//! Phase and step triggers.
//!
//! This module contains triggers that fire at the beginning of game phases
//! and steps, such as upkeep, draw step, and end step.

mod beginning_of_combat;
mod beginning_of_draw_step;
mod beginning_of_end_step;
mod beginning_of_main_phase;
mod beginning_of_upkeep;
mod end_of_combat;

pub use beginning_of_combat::BeginningOfCombatTrigger;
pub use beginning_of_draw_step::BeginningOfDrawStepTrigger;
pub use beginning_of_end_step::BeginningOfEndStepTrigger;
pub use beginning_of_main_phase::{BeginningOfMainPhaseTrigger, MainPhaseType};
pub use beginning_of_upkeep::BeginningOfUpkeepTrigger;
pub use end_of_combat::EndOfCombatTrigger;
