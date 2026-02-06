//! Centralized targeting system for spells and abilities.
//!
//! This module provides a unified targeting system that handles:
//! - Computing legal targets for spells and abilities
//! - Validating targets during resolution
//! - Protection, hexproof, shroud, and ward handling
//!
//! ## Key Types
//!
//! - [`TargetingResult`] - The result of attempting to target something
//! - [`TargetingInvalidReason`] - Why a target is invalid
//! - [`PendingWardCost`] - A ward cost that needs to be paid
//!
//! ## Key Functions
//!
//! - [`can_target_object`] - Check if a source can target an object
//! - [`compute_legal_targets`] - Compute all legal targets for a target spec
//! - [`validate_targets`] - Validate targets during resolution

mod computation;
mod types;
mod ward;

pub use computation::*;
pub use types::*;
pub use ward::*;
