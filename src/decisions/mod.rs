//! Player decision system.
//!
//! This module provides:
//! - Decision specification traits and primitives (`spec` module)
//! - Context structs for the new DecisionMaker trait methods (`context` module)
//! - Concrete spec implementations for different decision types (`specs` module)
//! - Reusable helper functions for common decision patterns (`helpers` module)
//!
//! ## Architecture
//!
//! The decision system is built around the `DecisionSpec` trait, which allows
//! typed, self-documenting decisions. Each decision type maps to a primitive
//! shape (select objects, select options, boolean, etc.) for UI dispatch.
//!
//! ## Example
//!
//! ```ignore
//! use crate::decisions::specs::SacrificeSpec;
//! use crate::decisions::make_decision;
//!
//! let spec = SacrificeSpec { description: "a creature".into() };
//! let to_sacrifice = make_decision(game, dm, player, source, spec, Some(&candidates), None);
//! ```

pub mod context;
mod helpers;
mod make_decision;
pub mod spec;
pub mod specs;

// Re-export core traits and types
pub use spec::{
    AttackerDeclaration, AttackerOption, BlockerDeclaration, BlockerOption, DecisionPrimitive,
    DecisionSpec, DisplayOption,
};

// Re-export context types
pub use context::{
    AttackerOptionContext, AttackersContext, BlockerOptionContext, BlockersContext, BooleanContext,
    ColorsContext, CountersContext, DecisionContext, DistributeContext, DistributeTarget,
    NumberContext, OrderContext, PartitionContext, ProliferateContext, SelectObjectsContext,
    SelectOptionsContext, SelectableObject, SelectableOption, ViewCardsContext,
};

// Re-export the make_decision function
pub use make_decision::{
    FromPrimitiveResponse, make_boolean_decision, make_decision, make_decision_with_fallback,
    make_mana_color_decision, make_number_decision,
};

// Re-export helper functions
pub use helpers::{
    ask_choose_multiple, ask_choose_one, ask_mana_color, ask_may_choice, ask_sacrifice,
};

// Re-export all spec types for convenience
pub use specs::*;
