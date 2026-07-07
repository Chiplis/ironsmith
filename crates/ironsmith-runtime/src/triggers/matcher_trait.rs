//! Trigger matcher trait for the modular trigger system.
//!
//! This module defines the `TriggerMatcher` trait that all trigger implementations
//! must implement. Each trigger type (ETB, dies, upkeep, etc.) implements this trait
//! with its own matching logic.

use crate::events::EventKind;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::FilterContext;
use std::collections::HashMap;

use super::TriggerEvent;

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

    pub fn for_delayed_source(
        source_id: ObjectId,
        controller: PlayerId,
        game: &'a GameState,
        tagged_objects: &HashMap<TagKey, Vec<ObjectSnapshot>>,
    ) -> Self {
        let mut filter_ctx = game.filter_context_for(controller, Some(source_id));
        filter_ctx.tagged_objects = tagged_objects.clone();
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
/// }
/// ```
pub trait TriggerMatcherClone {
    /// Clone this trigger into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn TriggerMatcher>;
}

impl<T> TriggerMatcherClone for T
where
    T: TriggerMatcher + Clone + 'static,
{
    fn clone_boxed(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

pub trait TriggerMatcher:
    std::fmt::Debug + Send + Sync + TriggerMatcherClone + std::any::Any
{
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

    /// Superset of event kinds this matcher can return true for.
    ///
    /// Returning `None` keeps the matcher in the wildcard bucket. Implementors
    /// may return a broad superset; under-approximating would drop triggers.
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        None
    }

    /// Human-readable display text for this trigger.
    ///
    /// Should describe what causes the trigger to fire.
    /// Example: "Whenever a creature dies"
    fn display(&self) -> String;

    /// Clone this trigger into a boxed trait object.
    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        TriggerMatcherClone::clone_boxed(self)
    }

    /// Whether this trigger uses snapshot-based matching.
    ///
    /// Triggers for "leaves the battlefield" and "dies" events need to check
    /// the object's characteristics at the moment it left, not its current state.
    /// Return `true` if this trigger uses the snapshot from the event.
    fn uses_snapshot(&self) -> bool {
        false
    }

    /// Whether the source of this triggered ability must be discovered from
    /// immediately before the event instead of from the post-event game state.
    ///
    /// This is separate from `uses_snapshot()`: snapshot matching answers what
    /// the event object looked like, while source look-back answers whether the
    /// ability itself existed immediately before the event (CR 603.10).
    fn looks_back_for_source(&self, _event: &TriggerEvent) -> bool {
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

    /// Context-aware trigger count for triggers whose per-event multiplicity
    /// depends on source-relative filters like "another".
    fn trigger_count_with_context(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> u32 {
        self.trigger_count(event)
    }

    /// Numeric value derived by this trigger from the matched event context.
    ///
    /// This is distinct from `trigger_count`: a trigger can queue once for a
    /// grouped event such as "one or more creatures attack" while resolving
    /// "that many" using the number of matching objects in the group.
    fn event_value_amount(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> Option<i32> {
        None
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
