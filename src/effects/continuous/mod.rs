//! Continuous effect helpers.
//!
//! These effects provide a composable way to register continuous effects
//! (e.g., power/toughness changes, ability grants) without duplicating
//! registration boilerplate.

mod apply_continuous;

pub use apply_continuous::ApplyContinuousEffect;
