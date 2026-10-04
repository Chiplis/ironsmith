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
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub enum EventKind {
    /// Damage being dealt
    Damage,
    /// A prevention effect prevented some or all damage
    DamagePrevented,
    /// Object changing zones
    ZoneChange,
    /// Notification that an object left the game, with no destination zone.
    ObjectLeavesGame,
    /// Player drawing cards
    Draw,
    /// Player gaining life
    LifeGain,
    /// Player losing life
    LifeLoss,
    /// Player losing the game
    PlayerLosesGame,
    /// Counters being placed on a permanent
    PutCounters,
    /// Counters being removed from a permanent
    RemoveCounters,
    /// Permanent becoming tapped
    BecomeTapped,
    /// Permanent becoming untapped
    BecomeUntapped,
    /// A player reaches the turn-based action that untaps their permanents
    PermanentsUntapStep,
    /// Permanent being destroyed
    Destroy,
    /// Permanent being sacrificed
    Sacrifice,
    /// A player gave a gift as a Gift ability resolved
    GiftGiven,
    /// One effect would create one or more tokens
    CreateTokens,
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
    /// A player played a land
    LandPlayed,
    /// An activated or mana ability was activated
    AbilityActivated,
    /// A triggered ability triggered and became pending
    AbilityTriggered,
    /// Mana was added to a player's mana pool
    ManaAdded,
    /// One concrete mana unit was committed to a payment transaction.
    ManaSpent,
    /// A permanent became the target of a spell or ability
    BecomesTargeted,
    /// A creature attacked
    CreatureAttacked,
    /// A creature attacked and wasn't blocked
    CreatureAttackedAndUnblocked,
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
    /// Beginning of cleanup step
    BeginningOfCleanupStep,
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
    /// A permanent mutated
    Mutated,
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
    /// A player rolled a die.
    DieRolled,
    /// A player flipped a coin.
    CoinFlipped,
    /// The day/night designation changed from day to night or night to day.
    DayNightChanged,
    /// Players finished voting (for council's dilemma, etc.)
    PlayersFinishedVoting,
    /// A permanent transformed
    Transformed,
    /// A permanent converted
    Converted,
    /// A permanent was turned face up
    TurnedFaceUp,
    /// A permanent came under a different player's control
    ControlChanged,
    /// A permanent phased out
    PermanentPhasedOut,
    /// An object became unattached
    ObjectBecameUnattached,
    /// A spell was countered
    SpellCountered,
    /// A card was revealed
    CardRevealed,
    /// A state-triggered ability's condition became true
    StateTrigger,
    /// A Saga chapter ability resolved
    ChapterAbilityResolved,
    /// An Aura, Equipment or Fortification became attached.
    ObjectBecameAttached,
    /// A permanent phased in (not a zone change).
    PermanentPhasedIn,
    /// One card was moved by an actual mill instruction.
    CardMilled,
    /// A player declared one or more attackers attacking another player.
    PlayerAttackDeclaration,
    /// A batch of existing unspent mana would leave a pool.
    ManaLost,
    /// A successfully completed life payment (not generic life loss).
    LifePaid,
    /// A different player acquired the monarch designation.
    MonarchChanged,
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
pub trait GameEventTypeClone {
    /// Clone this event into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn GameEventType>;
}

impl<T> GameEventTypeClone for T
where
    T: GameEventType + Clone + 'static,
{
    fn clone_boxed(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }
}

pub trait GameEventType: Debug + Send + Sync + GameEventTypeClone {
    /// Get the event kind for fast dispatch without downcasting.
    fn event_kind(&self) -> EventKind;

    /// Whether this carrier proposes an operation that replacements may alter.
    /// A completed non-zone departure notification cannot be replaced.
    fn is_replacement_proposal(&self) -> bool { true }

    /// Clone this event into a boxed trait object.
    fn clone_box(&self) -> Box<dyn GameEventType> {
        GameEventTypeClone::clone_boxed(self)
    }

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

    /// Captured cause of a completed action, when this event kind owns one.
    /// A missing cause cannot prove a spell/ability or its controller.
    fn cause(&self) -> Option<&crate::events::cause::EventCause> { None }

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
    /// This is the lightweight trigger-facing accessor used to bind "that player"
    /// style references during triggered ability resolution.
    ///
    /// For phase events, this is the active player.
    /// For life gain/loss, this is the affected player.
    fn player(&self) -> Option<PlayerId> {
        None
    }

    /// Get the player that triggered abilities should treat as "that player".
    ///
    /// By default this reuses `player()`, but the explicit name makes it easier to
    /// distinguish from `affected_player()` at call sites.
    fn trigger_player(&self) -> Option<PlayerId> {
        self.player()
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
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub enum ReplacementPriority {
    /// 616.1a: True self-replacement effects per CR 614.15
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
pub trait ReplacementMatcherClone {
    /// Clone this matcher into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn ReplacementMatcher>;
}

impl<T> ReplacementMatcherClone for T
where
    T: ReplacementMatcher + Clone + 'static,
{
    fn clone_boxed(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(self.clone())
    }
}

pub trait ReplacementMatcher: Debug + Send + Sync + ReplacementMatcherClone + Any {

    /// Exact mana-event predicate for compact evaluation. Wrappers with
    /// additional conditions must expose those conditions or leave this unknown.
    fn mana_predicate(&self) -> Option<crate::events::mana::ManaEventPredicate<'_>> {
        None
    }

    /// Conservative event-kind dependency used by specialized planners. False
    /// proves this matcher cannot observe the event; unknown matchers keep the
    /// default and are evaluated by the authoritative replacement machinery.
    fn may_match_event_kind(&self, _kind: EventKind) -> bool {
        true
    }

    /// CR 614.12: only a replacement affecting this specific entrant can
    /// function from the entering object's own text before it is on the field.
    fn applies_from_entering_source(&self) -> bool {
        false
    }

    /// Check if this matcher matches the given event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to check
    /// * `ctx` - Context including game state and controller info
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the replacement applies, `Ok(false)` for a complete
    /// non-match, or a discovery error when applicability cannot be calculated.
    fn matches_event(
        &self, event: &dyn GameEventType, ctx: &EventContext,
    ) -> Result<bool, crate::static_ability_processor::StaticEffectDiscoveryError> {
        ctx.with_complete_query(event, |complete| self.matches_prepared_event(event, complete))
    }

    /// Pure predicate over a validated context. Callers cannot construct this
    /// context from arbitrary unchecked game snapshots.
    fn matches_prepared_event(
        &self, event: &dyn GameEventType,
        ctx: &crate::events::context::PreparedEventContext,
    ) -> bool;

    /// Fallible entry matching against a complete prospective query context.
    /// Discovery failure is distinct from a successfully evaluated non-match.
    /// This entry convenience uses the same checked matching contract as
    /// every other event kind. The owning processor propagates errors before
    /// committing the proposal or consuming its replacements.
    fn matches_entry_event(
        &self,
        event: &crate::events::EnterBattlefieldEvent,
        ctx: &EventContext,
    ) -> Result<bool, crate::static_ability_processor::StaticEffectDiscoveryError> {
        self.matches_event(event, ctx)
    }

    /// For token-creation replacements: the filter a token group must match
    /// for this replacement to modify it (`None` = every group).
    fn token_group_filter(&self) -> Option<&crate::target::ObjectFilter> {
        None
    }

    /// Whether this matcher applies only because a token merged permanent has
    /// a nontoken card component. CR 730.3e uses this to partition the
    /// replacement between card and token components.
    fn matches_merged_card_component_only(
        &self,
        event: &crate::events::zones::ZoneChangeEvent,
        ctx: &EventContext,
    ) -> Result<bool, crate::static_ability_processor::StaticEffectDiscoveryError> {
        ctx.with_complete_query(event, |complete|
            self.matches_prepared_merged_card_component_only(event, complete))
    }

    /// Component partitioning uses the same checked context as ordinary matching.
    fn matches_prepared_merged_card_component_only(
        &self,
        _event: &crate::events::zones::ZoneChangeEvent,
        _ctx: &crate::events::context::PreparedEventContext,
    ) -> bool {
        false
    }

    /// Whether this matcher is a regeneration shield (CR 701.19). "Can't be
    /// regenerated" (CR 701.19c) disables only these replacements.
    fn is_regeneration_shield(&self) -> bool {
        false
    }

    /// Get the priority of this replacement effect per Rule 616.1.
    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    /// Clone this matcher into a boxed trait object.
    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        ReplacementMatcherClone::clone_boxed(self)
    }

    /// Human-readable description of what this matcher matches.
    fn display(&self) -> String;
}

impl dyn ReplacementMatcher {
    /// Inspect the exact native predicate for executable descriptor encoding.
    /// Display text is not a semantic identifier: different predicates can
    /// render identically. Unknown native types must remain distinguishable so
    /// the codec can report an unsupported descriptor instead of guessing.
    pub fn downcast_ref<T: ReplacementMatcher + 'static>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }
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
    fn native_replacement_matcher_access_preserves_identical_display_predicates() {
        use crate::events::{LifeGainEvent, WouldGainLifeMatcher, WouldLoseLifeMatcher};
        use crate::target::{FilterContext, PlayerFilter};

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let context = EventContext::new(alice, None, FilterContext::new(alice), &game);
        let first: Box<dyn ReplacementMatcher> =
            Box::new(WouldGainLifeMatcher::new(PlayerFilter::Specific(alice)));
        let second: Box<dyn ReplacementMatcher> =
            Box::new(WouldGainLifeMatcher::new(PlayerFilter::Specific(bob)));
        assert_eq!(first.display(), second.display());

        for (matcher, recipient) in [(first, alice), (second, bob)] {
            let cloned = matcher.clone();
            let native = cloned.as_ref().downcast_ref::<WouldGainLifeMatcher>()
                .expect("native clone retains its exact predicate type");
            assert_eq!(native.player_filter, PlayerFilter::Specific(recipient));
            assert!(cloned.as_ref().downcast_ref::<WouldLoseLifeMatcher>().is_none());
            // Rebuild from the concrete descriptor and test real applicability,
            // rather than treating equal display text as equal behavior.
            let restored: Box<dyn ReplacementMatcher> = Box::new(native.clone());
            for affected in [alice, bob] {
                let event = LifeGainEvent::new(affected, 3);
                assert_eq!(restored.matches_event(&event, &context).unwrap(), affected == recipient);
                assert_eq!(matcher.matches_event(&event, &context).unwrap(), affected == recipient);
            }
        }
    }

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
