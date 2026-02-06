//! Trigger matcher trait for the modular trigger system.
//!
//! This module defines the `TriggerMatcher` trait that all trigger implementations
//! must implement. Each trigger type (ETB, dies, upkeep, etc.) implements this trait
//! with its own matching logic.

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::FilterContext;

use super::trigger_event::TriggerEvent;

/// Context provided to trigger matchers for determining if they match an event.
///
/// Contains all the information a trigger needs to determine if it should fire.
#[derive(Debug, Clone)]
pub struct TriggerContext<'a> {
    /// The object ID of the permanent that has this triggered ability.
    pub source_id: ObjectId,

    /// The controller of the triggered ability source.
    pub controller: PlayerId,

    /// Filter context for evaluating object filters.
    pub filter_ctx: FilterContext,

    /// Reference to the game state for additional lookups.
    pub game: &'a GameState,
}

impl<'a> TriggerContext<'a> {
    /// Create a new trigger context.
    pub fn new(
        source_id: ObjectId,
        controller: PlayerId,
        filter_ctx: FilterContext,
        game: &'a GameState,
    ) -> Self {
        Self {
            source_id,
            controller,
            filter_ctx,
            game,
        }
    }

    /// Create a trigger context for a source permanent.
    pub fn for_source(source_id: ObjectId, controller: PlayerId, game: &'a GameState) -> Self {
        let filter_ctx = game.filter_context_for(controller, Some(source_id));
        Self::new(source_id, controller, filter_ctx, game)
    }
}

/// Trait for matching game events to trigger conditions.
///
/// All modular triggers implement this trait. Each trigger is responsible for:
/// - Determining if it matches a given game event
/// - Providing a human-readable description
/// - Indicating whether it uses snapshot-based matching
///
/// # Example
///
/// ```ignore
/// use ironsmith::triggers::{TriggerMatcher, TriggerContext, TriggerEvent};
/// use ironsmith::events::EventKind;
///
/// impl TriggerMatcher for MyTrigger {
///     fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
///         // Fast-path: check kind first
///         if event.kind() != EventKind::SpellCast {
///             return false;
///         }
///         // Then downcast if needed for specific fields
///         true
///     }
///
///     fn display(&self) -> String {
///         "When something happens".to_string()
///     }
///
///     fn clone_box(&self) -> Box<dyn TriggerMatcher> {
///         Box::new(self.clone())
///     }
/// }
/// ```
pub trait TriggerMatcher: std::fmt::Debug + Send + Sync {
    /// Check if this trigger matches the given game event.
    ///
    /// # Arguments
    ///
    /// * `event` - The game event that occurred (wrapped in TriggerEvent)
    /// * `ctx` - Context about the trigger source (source ID, controller, etc.)
    ///
    /// # Returns
    ///
    /// `true` if this trigger should fire for the given event.
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool;

    /// Human-readable display text for this trigger.
    ///
    /// Should describe what causes the trigger to fire.
    /// Example: "Whenever a creature dies"
    fn display(&self) -> String;

    /// Clone this trigger into a boxed trait object.
    ///
    /// This is required because `Clone` is not object-safe.
    fn clone_box(&self) -> Box<dyn TriggerMatcher>;

    /// Whether this trigger uses snapshot-based matching.
    ///
    /// Triggers for "leaves the battlefield" and "dies" events need to check
    /// the object's characteristics at the moment it left, not its current state.
    /// Return `true` if this trigger uses the snapshot from the event.
    fn uses_snapshot(&self) -> bool {
        false
    }

    /// How many times this trigger should fire for the given event.
    ///
    /// Most triggers fire once per event, but some (like "whenever you draw a card")
    /// need to fire once per card when multiple cards are drawn in a single action.
    ///
    /// Default is 1.
    fn trigger_count(&self, _event: &TriggerEvent) -> u32 {
        1
    }

    /// If this is a saga chapter trigger, return its chapter numbers.
    ///
    /// This lets callers use semantic data instead of parsing `display()`.
    fn saga_chapters(&self) -> Option<&[u32]> {
        None
    }
}

impl Clone for Box<dyn TriggerMatcher> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn TriggerMatcher> {
    fn eq(&self, other: &Self) -> bool {
        // Compare by display text since triggers don't have unique IDs
        self.display() == other.display()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test trigger that always matches.
    #[derive(Debug, Clone)]
    struct AlwaysMatchTrigger;

    impl TriggerMatcher for AlwaysMatchTrigger {
        fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
            true
        }

        fn display(&self) -> String {
            "Always trigger".to_string()
        }

        fn clone_box(&self) -> Box<dyn TriggerMatcher> {
            Box::new(self.clone())
        }
    }

    /// A trigger that never matches.
    #[derive(Debug, Clone)]
    struct NeverMatchTrigger;

    impl TriggerMatcher for NeverMatchTrigger {
        fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
            false
        }

        fn display(&self) -> String {
            "Never trigger".to_string()
        }

        fn clone_box(&self) -> Box<dyn TriggerMatcher> {
            Box::new(self.clone())
        }
    }

    #[test]
    fn test_trigger_matcher_trait_is_object_safe() {
        // This test verifies that TriggerMatcher can be used as a trait object
        let trigger: Box<dyn TriggerMatcher> = Box::new(AlwaysMatchTrigger);
        assert!(format!("{:?}", trigger).contains("AlwaysMatchTrigger"));
    }

    #[test]
    fn test_trigger_matcher_clone() {
        let trigger: Box<dyn TriggerMatcher> = Box::new(AlwaysMatchTrigger);
        let cloned = trigger.clone();
        assert_eq!(trigger.display(), cloned.display());
    }

    #[test]
    fn test_trigger_matcher_display_comparison() {
        // Compare via display() instead of PartialEq which isn't directly available for boxed trait objects
        let trigger1: Box<dyn TriggerMatcher> = Box::new(AlwaysMatchTrigger);
        let trigger2: Box<dyn TriggerMatcher> = Box::new(AlwaysMatchTrigger);
        let trigger3: Box<dyn TriggerMatcher> = Box::new(NeverMatchTrigger);

        assert_eq!(trigger1.display(), trigger2.display());
        assert_ne!(trigger1.display(), trigger3.display());
    }

    #[test]
    fn test_uses_snapshot_default() {
        let trigger = AlwaysMatchTrigger;
        assert!(!trigger.uses_snapshot());
    }
}
