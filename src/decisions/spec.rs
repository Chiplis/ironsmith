//! Decision specification traits and primitives.
//!
//! This module provides the core abstractions for the player decision system:
//! - `DecisionSpec`: A trait that all decision specifications implement
//! - `DecisionPrimitive`: Enum describing the primitive response shape for UI dispatch
//!
//! The key insight is that while there are many different decision *types* (sacrifice,
//! discard, choose targets, etc.), they all map to a small number of primitive
//! *shapes* (select objects, select options, pick a number, yes/no, etc.).

use crate::combat_state::AttackTarget;
use crate::decision::FallbackStrategy;
use crate::ids::ObjectId;

// ============================================================================
// Decision Primitive
// ============================================================================

/// The primitive response shapes for UI dispatch.
///
/// Each decision spec maps to one of these primitives, which tells the UI
/// how to render the decision and what response type to expect.
#[derive(Debug, Clone)]
pub enum DecisionPrimitive {
    /// Select N objects from a list.
    /// Used for: sacrifice, discard, exile, search library, etc.
    SelectObjects {
        /// Minimum objects to select (0 for optional selection)
        min: usize,
        /// Maximum objects to select (None = unlimited, i.e., "any number")
        max: Option<usize>,
    },

    /// Select N options by index.
    /// Used for: modes, choices, replacement effects, priority actions, etc.
    SelectOptions {
        /// Minimum options to select
        min: usize,
        /// Maximum options to select
        max: usize,
    },

    /// Select a number in range.
    /// Used for: X value, "choose a number", etc.
    SelectNumber {
        /// Minimum value (inclusive)
        min: u32,
        /// Maximum value (inclusive)
        max: u32,
    },

    /// Yes/No choice.
    /// Used for: may effects, ward payment, miracle trigger, etc.
    Boolean,

    /// Reorder items.
    /// Used for: order blockers, order attackers, scry, surveil.
    Order,

    /// Compound: Declare attackers (creature -> target mappings).
    DeclareAttackers,

    /// Compound: Declare blockers (attacker -> blockers mappings).
    DeclareBlockers,

    /// Compound: Distribute amount among targets.
    Distribute {
        /// Total amount to distribute
        total: u32,
        /// Minimum amount per target (usually 1 for damage)
        min_per_target: u32,
    },

    /// Select one or more colors.
    /// Used for: mana color selection
    SelectColors {
        /// Number of colors to select
        count: u32,
        /// If true, all selections must be the same color
        same_color: bool,
    },

    /// Select counters to remove (counter type + count pairs).
    SelectCounters {
        /// Maximum total counters to remove
        max_total: u32,
    },

    /// Partition objects into two groups (top/bottom for scry, library/graveyard for surveil).
    Partition,

    /// Distribute choices among different destinations.
    /// Used for: proliferate (select subset of permanents and players)
    SelectMixed,
}

// ============================================================================
// Decision Spec Trait
// ============================================================================

/// Trait that all decision specifications implement.
///
/// A decision spec is a typed description of what decision needs to be made.
/// The associated `Response` type ensures type-safe responses.
///
/// # Design Philosophy
///
/// Rather than having one monolithic `PlayerDecision` enum with 30+ variants,
/// we have many small spec types that each implement this trait. This gives us:
///
/// 1. **Type safety**: Each spec declares its response type
/// 2. **Composability**: Specs can be combined or wrapped
/// 3. **Self-documentation**: Each spec is explicit about what it needs
/// 4. **Easier extension**: Adding a new decision is just adding a new spec type
///
/// # Example
///
/// ```ignore
/// pub struct SacrificeSpec {
///     pub description: String,
/// }
///
/// impl DecisionSpec for SacrificeSpec {
///     type Response = ObjectId;
///
///     fn description(&self) -> String {
///         format!("Choose {} to sacrifice", self.description)
///     }
///
///     fn primitive(&self) -> DecisionPrimitive {
///         DecisionPrimitive::SelectObjects { min: 1, max: Some(1) }
///     }
///
///     fn default_response(&self, _: FallbackStrategy) -> ObjectId {
///         panic!("No default for mandatory sacrifice")
///     }
/// }
/// ```
pub trait DecisionSpec: std::fmt::Debug {
    /// The response type for this decision.
    type Response;

    /// Description for UI display.
    fn description(&self) -> String;

    /// What primitive kind this is (for UI rendering/dispatch).
    fn primitive(&self) -> DecisionPrimitive;

    /// Default response for auto-pass/fallback.
    ///
    /// This is called when no decision maker is present or when an automatic
    /// decision is needed (e.g., in tests).
    fn default_response(&self, strategy: FallbackStrategy) -> Self::Response;

    /// Build a context struct for the new DecisionMaker trait methods.
    ///
    /// This method creates a `DecisionContext` containing all the information
    /// needed by a DecisionMaker to render and process this decision. The
    /// context type matches the primitive type returned by `primitive()`.
    fn build_context(
        &self,
        player: crate::ids::PlayerId,
        source: Option<ObjectId>,
        game: &crate::game_state::GameState,
    ) -> crate::decisions::context::DecisionContext;
}

// ============================================================================
// Supporting Types
// ============================================================================

/// An attacker option for DeclareAttackers decisions.
#[derive(Debug, Clone)]
pub struct AttackerOption {
    /// The creature that can attack.
    pub creature: ObjectId,
    /// Valid targets this creature can attack.
    pub valid_targets: Vec<AttackTarget>,
    /// Whether this creature must attack if able.
    pub must_attack: bool,
}

/// A declared attacker.
#[derive(Debug, Clone)]
pub struct AttackerDeclaration {
    /// The attacking creature.
    pub creature: ObjectId,
    /// What the creature is attacking.
    pub target: AttackTarget,
}

/// Options for blocking a specific attacker.
#[derive(Debug, Clone)]
pub struct BlockerOption {
    /// The attacking creature.
    pub attacker: ObjectId,
    /// Creatures that can legally block this attacker.
    pub valid_blockers: Vec<ObjectId>,
    /// Minimum number of blockers required (for menace, etc.).
    pub min_blockers: usize,
}

/// A declared blocker.
#[derive(Debug, Clone)]
pub struct BlockerDeclaration {
    /// The blocking creature.
    pub blocker: ObjectId,
    /// The attacker being blocked.
    pub blocking: ObjectId,
}

/// An option for displaying to the user.
#[derive(Debug, Clone)]
pub struct DisplayOption {
    /// Index of this option.
    pub index: usize,
    /// Description of this option.
    pub description: String,
    /// Whether this option is currently legal/selectable.
    pub legal: bool,
}

impl DisplayOption {
    /// Create a new legal display option.
    pub fn new(index: usize, description: impl Into<String>) -> Self {
        Self {
            index,
            description: description.into(),
            legal: true,
        }
    }

    /// Create a new display option with explicit legality.
    pub fn with_legality(index: usize, description: impl Into<String>, legal: bool) -> Self {
        Self {
            index,
            description: description.into(),
            legal,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_option_creation() {
        let opt = DisplayOption::new(0, "Test option");
        assert_eq!(opt.index, 0);
        assert_eq!(opt.description, "Test option");
        assert!(opt.legal);
    }

    #[test]
    fn test_display_option_with_legality() {
        let opt = DisplayOption::with_legality(1, "Disabled", false);
        assert_eq!(opt.index, 1);
        assert!(!opt.legal);
    }
}
