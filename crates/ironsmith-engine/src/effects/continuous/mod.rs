//! Continuous effect helpers.
//!
//! These effects provide a composable way to register continuous effects
//! (e.g., power/toughness changes, ability grants) without duplicating
//! registration boilerplate.

mod apply_continuous;
pub(crate) use apply_continuous::sacrificed_cost_land_types;
mod exchange_text_boxes;
mod change_text;

pub use apply_continuous::{ApplyContinuousEffect, RuntimeModification};
pub use exchange_text_boxes::ExchangeTextBoxesEffect;
pub use change_text::ChangeTextEffect;

pub(crate) use apply_continuous::materialize_duration_predicate;

pub(crate) use apply_continuous::register_resolved_continuous_batch;
pub(crate) use apply_continuous::apply_captured_modifications;
pub(crate) use apply_continuous::{commit_control_assignments, ControlAssignmentPolicy};
