//! Concrete decision specification types.
//!
//! This module contains all the specific `DecisionSpec` implementations
//! organized by category:
//!
//! - `boolean`: Yes/no decisions (may, ward, miracle, etc.)
//! - `number`: Numeric selections (X value, choose number)
//! - `objects`: Object selection (sacrifice, discard, search, etc.)
//! - `options`: Option selection (modes, choices, priority, etc.)
//! - `combat`: Combat decisions (attackers, blockers, ordering)
//! - `special`: Special cases (scry, surveil, distribute, etc.)

pub mod boolean;
pub mod combat;
pub mod number;
pub mod objects;
pub mod options;
pub mod special;

// Re-export all spec types for convenience
pub use boolean::*;
pub use combat::*;
pub use number::*;
pub use objects::*;
pub use options::*;
pub use special::*;
