//! Trigger matcher trait for the modular trigger system.
//!
//! This module defines the `TriggerMatcher` trait that all trigger implementations
//! must implement. Each trigger type (ETB, dies, upkeep, etc.) implements this trait
//! with its own matching logic.

use crate::events::{DamageTarget, EventKind};
use crate::filter::PlayerFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{FilterContext, PlayerFilter};
use std::collections::HashMap;

use super::TriggerEvent;

/// Rules grouping key for a trigger that says "one or more" damage sources or
/// recipients. Matches with the same key during one simultaneous action queue
/// the ability only once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimultaneousTriggerKey {
    /// All matching damage assignments in the action form one event group.
    DamageBatch,
    /// Damage assignments are grouped independently for each source.
    DamageSource(ObjectId),
    /// Damage assignments are grouped independently for each recipient.
    DamageTarget(DamageTarget),
}

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

    /// Structural identity of the enclosing triggered ability, when known.
    pub trigger_identity: Option<super::TriggerIdentity>,
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
            trigger_identity: None,
        }
    }

    pub fn with_trigger_identity(mut self, trigger_identity: super::TriggerIdentity) -> Self {
        self.trigger_identity = Some(trigger_identity);
        self
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

/// Match a player-valued turn restriction against every active member of a
/// shared turn. `active_player_id()` is only the CR 805 primary-player anchor;
/// it cannot by itself answer whether a nonprimary teammate is taking a turn.
pub(crate) fn current_turn_matches_player_filter(
    filter: &PlayerFilter,
    ctx: &TriggerContext<'_>,
    iterated_player: Option<PlayerId>,
) -> bool {
    match filter {
        PlayerFilter::You => ctx.game.is_active_player(ctx.controller),
        PlayerFilter::NotYou => {
            ctx.game.active_player_id().is_some() && !ctx.game.is_active_player(ctx.controller)
        }
        PlayerFilter::Opponent => ctx
            .game
            .active_players()
            .into_iter()
            .any(|player| ctx.game.are_opponents(ctx.controller, player)),
        PlayerFilter::Teammate => ctx
            .game
            .active_players()
            .into_iter()
            .any(|player| ctx.game.are_teammates(ctx.controller, player)),
        PlayerFilter::Any | PlayerFilter::Active => ctx.game.active_player_id().is_some(),
        PlayerFilter::Specific(player) => ctx.game.is_active_player(*player),
        PlayerFilter::IteratedPlayer => {
            iterated_player.is_some_and(|player| ctx.game.is_active_player(player))
        }
        _ => ctx.game.active_players().into_iter().any(|active_player| {
            let filter_ctx = ctx.filter_ctx.clone().with_active_player(active_player);
            filter.matches_player(active_player, &filter_ctx)
        }),
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

    /// Whether matching this event kind always requires the event's primary
    /// object to be this trigger's source.
    ///
    /// Registries may use this conservative hint to index source-local
    /// triggers. Returning `false` is always correct, just less efficient;
    /// returning `true` must never exclude a match for another object.
    fn source_must_match_event_object(&self, _event_kind: EventKind) -> bool {
        false
    }

    /// Return a grouping key when this matcher represents an authored
    /// "one or more" trigger over simultaneous events.
    fn simultaneous_trigger_key(&self, _event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
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
