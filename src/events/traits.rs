//! Core traits for the trait-based event system.
//!
//! This module defines the `GameEventType` trait that all event implementations must implement,
//! and the `ReplacementMatcher` trait for matching events against replacement conditions.

use std::any::Any;
use std::fmt::Debug;

use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

use super::context::EventContext;

/// Fast dispatch enum for event kinds.
///
/// This allows O(1) type checking without downcasting for common operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// Damage being dealt
    Damage,
    /// Object changing zones
    ZoneChange,
    /// Player drawing cards
    Draw,
    /// Player gaining life
    LifeGain,
    /// Player losing life
    LifeLoss,
    /// Counters being placed on a permanent
    PutCounters,
    /// Counters being removed from a permanent
    RemoveCounters,
    /// Permanent becoming tapped
    BecomeTapped,
    /// Permanent becoming untapped
    BecomeUntapped,
    /// Permanent being destroyed
    Destroy,
    /// Permanent being sacrificed
    Sacrifice,
    /// Player searching their library
    SearchLibrary,
    /// Player shuffling their library
    ShuffleLibrary,
    /// Object entering the battlefield (specialized zone change)
    EnterBattlefield,
    /// Counters being moved between permanents
    MoveCounters,
    /// Markers (counters, etc.) changed (unified add/remove event)
    MarkersChanged,
    /// Card being discarded
    Discard,
    /// A spell was cast
    SpellCast,
    /// A spell was copied
    SpellCopied,
    /// A creature attacked
    CreatureAttacked,
    /// A creature blocked
    CreatureBlocked,
    /// A creature became blocked
    CreatureBecameBlocked,
    /// Beginning of upkeep step
    BeginningOfUpkeep,
    /// Beginning of draw step
    BeginningOfDrawStep,
    /// Beginning of end step
    BeginningOfEndStep,
    /// Beginning of combat
    BeginningOfCombat,
    /// End of combat
    EndOfCombat,
    /// Beginning of precombat main phase
    BeginningOfPrecombatMainPhase,
    /// Beginning of postcombat main phase
    BeginningOfPostcombatMainPhase,
    /// A creature became monstrous
    BecameMonstrous,
    /// A player drew one or more cards
    CardsDrawn,
    /// A player discarded a card
    CardDiscarded,
    /// A counter was placed on a permanent
    CounterPlaced,
    /// A permanent became tapped
    PermanentTapped,
    /// A permanent became untapped
    PermanentUntapped,
    /// A player performed a keyword action (investigate, scry, earthbend, etc.)
    KeywordAction,
    /// Players finished voting (for council's dilemma, etc.)
    PlayersFinishedVoting,
}

/// A target within an event that can potentially be redirected.
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectableTarget {
    /// The actual target value.
    pub target: Target,
    /// A description of this target for UI/debugging.
    pub description: &'static str,
    /// What kinds of targets this can be redirected to.
    pub valid_redirect_types: RedirectValidTypes,
}

/// What types of targets a redirect can point to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectValidTypes {
    /// Can redirect to players only.
    PlayersOnly,
    /// Can redirect to objects (permanents) only.
    ObjectsOnly,
    /// Can redirect to either players or objects.
    PlayersOrObjects,
}

impl RedirectValidTypes {
    /// Check if a target is valid for this redirect type.
    pub fn is_valid(&self, target: &Target) -> bool {
        match self {
            RedirectValidTypes::PlayersOnly => matches!(target, Target::Player(_)),
            RedirectValidTypes::ObjectsOnly => matches!(target, Target::Object(_)),
            RedirectValidTypes::PlayersOrObjects => true,
        }
    }
}

/// Core trait for all game events.
///
/// All event types (DamageEvent, LifeGainEvent, etc.) implement this trait.
/// This provides a unified interface for the event processor while allowing
/// type-specific behavior through the trait methods.
pub trait GameEventType: Debug + Send + Sync {
    /// Get the event kind for fast dispatch without downcasting.
    fn event_kind(&self) -> EventKind;

    /// Clone this event into a boxed trait object.
    ///
    /// Required because `Clone` is not object-safe.
    fn clone_box(&self) -> Box<dyn GameEventType>;

    /// Get the player affected by this event.
    ///
    /// Per Rule 616.1e, when multiple replacement effects at "Other" priority apply,
    /// the affected player (or controller of affected object) chooses the order.
    fn affected_player(&self, game: &GameState) -> PlayerId;

    /// Get all targets in this event that can be redirected.
    ///
    /// Returns targets with metadata about what they can be redirected to.
    /// Not all parts of an event are redirectable - sources, snapshots, and
    /// other metadata are not included.
    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![]
    }

    /// Create a new event with a target replaced.
    ///
    /// Returns `Some(new_event)` if the replacement was successful, or `None` if:
    /// - The old_target wasn't found in this event
    /// - The new_target isn't valid for this type of event
    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    /// Get the source object of this event, if it has one.
    ///
    /// Used for "redirect to source" effects.
    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    // === Accessor methods for trigger matching ===

    /// Get the primary object ID involved in this event, if any.
    ///
    /// For zone changes, this is the object changing zones.
    /// For damage, this is the damage source.
    fn object_id(&self) -> Option<ObjectId> {
        None
    }

    /// Get the player involved in this event, if any.
    ///
    /// For phase events, this is the active player.
    /// For life gain/loss, this is the affected player.
    fn player(&self) -> Option<PlayerId> {
        None
    }

    /// Get the controller of the object involved in this event, if any.
    fn controller(&self) -> Option<PlayerId> {
        None
    }

    /// Get the object snapshot for "last known information" if this event has one.
    ///
    /// Zone-change events can capture the object's state at the moment it moved,
    /// since the previous-zone object may no longer exist.
    ///
    /// For batch events, this returns the first snapshot.
    /// Use `snapshots()` to get all snapshots for batch event processing.
    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }

    /// Get all object snapshots for batch events.
    ///
    /// For events that contain multiple objects, this returns all snapshots.
    ///
    /// Default implementation returns a vec containing the single `snapshot()` if present.
    fn snapshots(&self) -> Vec<&ObjectSnapshot> {
        self.snapshot().into_iter().collect()
    }

    /// Human-readable description of what this event does.
    fn display(&self) -> String;

    /// Downcast to a concrete event type.
    ///
    /// Used when a matcher needs access to event-specific fields.
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn GameEventType> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Priority order for replacement effects per Rule 616.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ReplacementPriority {
    /// 616.1a: Self-replacement effects (affect only their source)
    SelfReplacement = 0,
    /// 616.1b: Control-changing effects
    ControlChanging = 1,
    /// 616.1c: Copy effects
    CopyEffect = 2,
    /// 616.1d: Effects that cause permanents to enter as back face (MDFCs)
    BackFace = 3,
    /// 616.1e: All other replacement effects (affected player/controller chooses)
    #[default]
    Other = 4,
}

/// Trait for checking if a replacement effect matches an event.
///
/// All replacement condition types implement this trait. Each matcher is responsible
/// for determining if it applies to a given event.
pub trait ReplacementMatcher: Debug + Send + Sync {
    /// Check if this matcher matches the given event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to check
    /// * `ctx` - Context including game state and controller info
    ///
    /// # Returns
    ///
    /// `true` if this replacement effect should apply to the event.
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool;

    /// Get the priority of this replacement effect per Rule 616.1.
    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    /// Clone this matcher into a boxed trait object.
    fn clone_box(&self) -> Box<dyn ReplacementMatcher>;

    /// Human-readable description of what this matcher matches.
    fn display(&self) -> String;
}

impl Clone for Box<dyn ReplacementMatcher> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Helper function to downcast an event to a concrete type.
///
/// Use this in matchers to access event-specific fields.
///
/// # Example
///
/// ```ignore
/// if event.event_kind() != EventKind::Damage {
///     return false;
/// }
/// let Some(damage) = downcast_event::<DamageEvent>(event) else {
///     return false;
/// };
/// // Now can access damage.amount, damage.target, etc.
/// ```
pub fn downcast_event<T: 'static>(event: &dyn GameEventType) -> Option<&T> {
    event.as_any().downcast_ref::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_kind_debug() {
        assert_eq!(format!("{:?}", EventKind::Damage), "Damage");
        assert_eq!(format!("{:?}", EventKind::LifeGain), "LifeGain");
    }

    #[test]
    fn test_redirect_valid_types() {
        let player_target = Target::Player(PlayerId::from_index(0));
        let object_target = Target::Object(ObjectId::from_raw(1));

        assert!(RedirectValidTypes::PlayersOnly.is_valid(&player_target));
        assert!(!RedirectValidTypes::PlayersOnly.is_valid(&object_target));

        assert!(!RedirectValidTypes::ObjectsOnly.is_valid(&player_target));
        assert!(RedirectValidTypes::ObjectsOnly.is_valid(&object_target));

        assert!(RedirectValidTypes::PlayersOrObjects.is_valid(&player_target));
        assert!(RedirectValidTypes::PlayersOrObjects.is_valid(&object_target));
    }

    #[test]
    fn test_replacement_priority_ordering() {
        assert!(ReplacementPriority::SelfReplacement < ReplacementPriority::ControlChanging);
        assert!(ReplacementPriority::ControlChanging < ReplacementPriority::CopyEffect);
        assert!(ReplacementPriority::CopyEffect < ReplacementPriority::BackFace);
        assert!(ReplacementPriority::BackFace < ReplacementPriority::Other);
    }

    #[test]
    fn test_replacement_priority_default() {
        assert_eq!(ReplacementPriority::default(), ReplacementPriority::Other);
    }
}
