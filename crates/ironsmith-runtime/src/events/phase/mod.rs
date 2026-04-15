//! Phase and step-related events.

mod beginning_of_combat;
mod beginning_of_draw_step;
mod beginning_of_end_step;
mod beginning_of_main_phase;
mod beginning_of_upkeep;
mod end_of_combat;

pub use beginning_of_combat::BeginningOfCombatEvent;
pub use beginning_of_draw_step::BeginningOfDrawStepEvent;
pub use beginning_of_end_step::BeginningOfEndStepEvent;
pub use beginning_of_main_phase::{
    BeginningOfPostcombatMainPhaseEvent, BeginningOfPrecombatMainPhaseEvent,
};
pub use beginning_of_upkeep::BeginningOfUpkeepEvent;
pub use end_of_combat::EndOfCombatEvent;
