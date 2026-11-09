//! Continuous effect helpers.
//!
//! These effects provide a composable way to register continuous effects
//! (e.g., power/toughness changes, ability grants) without duplicating
//! registration boilerplate.

mod apply_continuous;
pub(crate) use apply_continuous::sacrificed_cost_land_types;
pub(crate) use apply_continuous::capture_ability_object_references;
mod exchange_text_boxes;
mod change_text;
mod remove_optional_text_fragment;

pub use apply_continuous::{ApplyContinuousEffect, RuntimeModification};
pub(crate) use apply_continuous::is_static_characteristic_change_cost;
pub use exchange_text_boxes::ExchangeTextBoxesEffect;
pub use change_text::ChangeTextEffect;
pub use remove_optional_text_fragment::RemoveOptionalTextFragmentEffect;

pub(crate) use apply_continuous::materialize_duration_predicate;

pub(crate) use apply_continuous::register_resolved_continuous_batch;
pub(crate) use apply_continuous::apply_captured_modifications;
pub(crate) use apply_continuous::{commit_control_assignments, ControlAssignmentPolicy};

mod resolved_power_toughness;
pub(crate) use resolved_power_toughness::ResolvedPowerToughness;
