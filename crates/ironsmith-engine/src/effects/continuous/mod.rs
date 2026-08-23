//! Continuous effect helpers.
//!
//! These effects provide a composable way to register continuous effects
//! (e.g., power/toughness changes, ability grants) without duplicating
//! registration boilerplate.

mod apply_continuous;
mod exchange_text_boxes;

pub use apply_continuous::{ApplyContinuousEffect, RuntimeModification};
pub use exchange_text_boxes::ExchangeTextBoxesEffect;
