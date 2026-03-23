//! Trait-based event system for replacement and prevention effects.
//!
//! This module provides a modular, trait-based approach to handling game events
//! that can be intercepted by replacement effects. Each event type implements
//! the `GameEventType` trait, and each replacement condition implements the
//! `ReplacementMatcher` trait.
//!
//! # Architecture
//!
//! The event system follows the same patterns as `EffectExecutor` and `TriggerMatcher`:
//!
//! - **Event types** (e.g., `DamageEvent`, `LifeGainEvent`) implement `GameEventType`
//! - **Matchers** (e.g., `DamageToPlayerMatcher`) implement `ReplacementMatcher`
//! - **Factory methods** on `Event` create instances easily
//!
//! # Example
//!
//! ```ignore
//! use ironsmith::events::{Event, EventContext};
//! use ironsmith::event_processor::process_trait_event;
//!
//! // Create a damage event using the factory method
//! let event = Event::damage(source_id, DamageTarget::Player(player), 3, false);
//!
//! // Process through the replacement effect system
//! let result = process_trait_event(&game, event);
//!
//! // Check the result
//! if result.is_prevented() {
//!     println!("Damage was prevented!");
//! }
//! ```
//!
//! # Creating Custom Matchers
//!
//! ```ignore
//! use ironsmith::events::{ReplacementMatcher, EventContext, GameEventType, EventKind, downcast_event, DamageEvent};
//!
//! #[derive(Debug, Clone)]
//! struct MyDamageMatcher;
//!
//! impl ReplacementMatcher for MyDamageMatcher {
//!     fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
//!         if event.event_kind() != EventKind::Damage {
//!             return false;
//!         }
//!         let Some(damage) = downcast_event::<DamageEvent>(event) else {
//!             return false;
//!         };
//!         damage.amount >= 5 // Only match damage of 5 or more
//!     }
//!
//!     fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
//!         Box::new(self.clone())
//!     }
//!
//!     fn display(&self) -> String {
//!         "When 5+ damage would be dealt".into()
//!     }
//! }
//! ```

pub mod cause;
pub mod context;
pub mod traits;

// Event type modules
pub mod cards;
pub mod counters;
pub mod damage;
pub mod life;
pub mod permanents;
pub mod raw_event;
pub mod zones;

// New event type modules for unified trigger system
pub mod combat;
pub mod other;
pub mod phase;
pub mod spells;

// Re-export core types
pub use cause::{CauseFilter, CauseType, CauseTypeFilter, ControllerFilter, EventCause};
pub use context::EventContext;
pub use traits::{
    EventKind, GameEventType, RedirectValidTypes, RedirectableTarget, ReplacementMatcher,
    ReplacementPriority, downcast_event,
};

// Re-export event types
pub use cards::{DiscardEvent, DrawEvent};
pub use counters::{MoveCountersEvent, PutCountersEvent, RemoveCountersEvent};
pub use damage::DamageEvent;
pub use life::{LifeGainEvent, LifeLossEvent};
pub use permanents::{DestroyEvent, SacrificeEvent, TapEvent, UntapEvent};
pub use zones::{EnterBattlefieldEvent, ZoneChangeEvent};

// Re-export new event types
pub use combat::{
    AttackEventTarget, CreatureAttackedAndUnblockedEvent, CreatureAttackedEvent,
    CreatureBecameBlockedEvent, CreatureBlockedEvent,
};
pub use other::{
    BecameMonstrousEvent, CardDiscardedEvent, CardsDrawnEvent, CounterPlacedEvent,
    KeywordActionEvent, KeywordActionKind, LandPlayedEvent, MarkerChangeType, MarkersChangedEvent,
    PermanentTappedEvent, PermanentUntappedEvent, PlayerVote, PlayersFinishedVotingEvent,
    SearchLibraryEvent, StateTriggerEvent, TransformedEvent, TurnedFaceUpEvent,
};
pub use phase::{
    BeginningOfCombatEvent, BeginningOfDrawStepEvent, BeginningOfEndStepEvent,
    BeginningOfPostcombatMainPhaseEvent, BeginningOfPrecombatMainPhaseEvent,
    BeginningOfUpkeepEvent, EndOfCombatEvent,
};
pub use raw_event::RawEvent;
pub use spells::{AbilityActivatedEvent, BecomesTargetedEvent, SpellCastEvent, SpellCopiedEvent};

// Re-export matchers
pub use cards::matchers::*;
pub use counters::matchers::*;
pub use damage::matchers::*;
pub use life::matchers::*;
pub use permanents::matchers::*;
pub use zones::matchers::*;

use crate::game_event::DamageTarget;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

/// Wrapper around a boxed event type.
///
/// This provides a more ergonomic API for working with events and includes
/// factory methods for creating common event types.
#[derive(Debug, Clone)]
pub struct Event(pub RawEvent);

impl Event {
    /// Create an event from any type implementing GameEventType with explicit provenance.
    pub fn new_with_provenance<E: GameEventType + 'static>(
        event: E,
        provenance: ProvNodeId,
    ) -> Self {
        Self(RawEvent::new(event, provenance))
    }

    /// Create an event from a boxed payload with explicit provenance.
    pub fn from_boxed_with_provenance(
        event: Box<dyn GameEventType>,
        provenance: ProvNodeId,
    ) -> Self {
        Self(RawEvent::from_boxed(event, provenance))
    }

    /// Wrap an existing raw event envelope.
    pub fn from_raw(raw: RawEvent) -> Self {
        Self(raw)
    }

    /// Extract the raw event envelope.
    pub fn into_raw(self) -> RawEvent {
        self.0
    }

    /// Get the event kind for fast dispatch.
    pub fn kind(&self) -> EventKind {
        self.0.kind()
    }

    /// Get the inner event as a trait object.
    pub fn inner(&self) -> &dyn GameEventType {
        self.0.inner()
    }

    /// Get the provenance node for this event.
    pub fn provenance(&self) -> ProvNodeId {
        self.0.provenance()
    }

    /// Set provenance for this event.
    pub fn set_provenance(&mut self, provenance: ProvNodeId) {
        self.0.set_provenance(provenance);
    }

    /// Return this event with a different provenance node.
    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.set_provenance(provenance);
        self
    }

    /// Re-wrap a new payload while preserving provenance.
    pub fn rewrap<E: GameEventType + 'static>(&self, event: E) -> Self {
        Self::new_with_provenance(event, self.provenance())
    }

    /// Re-wrap a boxed payload while preserving provenance.
    pub fn rewrap_boxed(&self, event: Box<dyn GameEventType>) -> Self {
        Self::from_boxed_with_provenance(event, self.provenance())
    }

    // Factory methods for common event types

    /// Create a damage event.
    pub fn damage(
        source: ObjectId,
        target: DamageTarget,
        amount: u32,
        is_combat: bool,
        cause: EventCause,
    ) -> Self {
        Self::new_with_provenance(
            DamageEvent::with_cause(source, target, amount, is_combat, cause),
            ProvNodeId::default(),
        )
    }

    /// Create an unpreventable damage event.
    pub fn unpreventable_damage(
        source: ObjectId,
        target: DamageTarget,
        amount: u32,
        is_combat: bool,
        cause: EventCause,
    ) -> Self {
        Self::new_with_provenance(
            DamageEvent::unpreventable_with_cause(source, target, amount, is_combat, cause),
            ProvNodeId::default(),
        )
    }

    /// Create a life gain event.
    pub fn life_gain(player: PlayerId, amount: u32) -> Self {
        Self::new_with_provenance(LifeGainEvent { player, amount }, ProvNodeId::default())
    }

    /// Create a life loss event.
    pub fn life_loss(player: PlayerId, amount: u32, from_damage: bool) -> Self {
        Self::new_with_provenance(
            LifeLossEvent {
                player,
                amount,
                from_damage,
            },
            ProvNodeId::default(),
        )
    }

    /// Create a zone change event.
    pub fn zone_change(
        object: ObjectId,
        from: Zone,
        to: Zone,
        cause: EventCause,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        Self::new_with_provenance(
            ZoneChangeEvent::with_cause(object, from, to, cause, snapshot),
            ProvNodeId::default(),
        )
    }

    /// Create an enter battlefield event.
    pub fn enter_battlefield(
        object: ObjectId,
        from: Zone,
        enters_tapped: bool,
        enters_with_counters: Vec<(CounterType, u32)>,
    ) -> Self {
        Self::new_with_provenance(
            EnterBattlefieldEvent {
                object,
                from,
                enters_tapped,
                enters_with_counters,
                enters_as_copy_of: None,
                added_subtypes: Vec::new(),
            },
            ProvNodeId::default(),
        )
    }

    /// Create a "dies" zone change event (battlefield -> graveyard).
    pub fn dies(object_id: ObjectId, controller: PlayerId, snapshot: ObjectSnapshot) -> Self {
        let cause = EventCause::from_sba();
        let _ = controller;
        Self::new_with_provenance(
            ZoneChangeEvent::with_cause(
                object_id,
                Zone::Battlefield,
                Zone::Graveyard,
                cause,
                Some(snapshot),
            ),
            ProvNodeId::default(),
        )
    }

    /// Create a put counters event.
    pub fn put_counters(
        target: ObjectId,
        counter_type: CounterType,
        count: u32,
        cause: EventCause,
    ) -> Self {
        Self::new_with_provenance(
            PutCountersEvent::with_cause(target, counter_type, count, cause),
            ProvNodeId::default(),
        )
    }

    /// Create a remove counters event.
    pub fn remove_counters(target: ObjectId, counter_type: CounterType, count: u32) -> Self {
        Self::new_with_provenance(
            RemoveCountersEvent {
                target,
                counter_type,
                count,
            },
            ProvNodeId::default(),
        )
    }

    /// Create a move counters event.
    pub fn move_counters(
        from: ObjectId,
        to: ObjectId,
        counter_type: Option<CounterType>,
        count: Option<u32>,
    ) -> Self {
        Self::new_with_provenance(
            MoveCountersEvent {
                from,
                to,
                counter_type,
                count,
            },
            ProvNodeId::default(),
        )
    }

    /// Create a draw event.
    pub fn draw(player: PlayerId, count: u32, is_first_this_turn: bool) -> Self {
        Self::new_with_provenance(
            DrawEvent {
                player,
                count,
                is_first_this_turn,
            },
            ProvNodeId::default(),
        )
    }

    /// Create a discard event with a custom cause.
    pub fn discard_with_cause(card: ObjectId, player: PlayerId, cause: EventCause) -> Self {
        Self::new_with_provenance(
            DiscardEvent::with_cause(card, player, cause),
            ProvNodeId::default(),
        )
    }

    /// Create a tap event.
    pub fn tap(permanent: ObjectId) -> Self {
        Self::new_with_provenance(TapEvent { permanent }, ProvNodeId::default())
    }

    /// Create an untap event.
    pub fn untap(permanent: ObjectId) -> Self {
        Self::new_with_provenance(UntapEvent { permanent }, ProvNodeId::default())
    }

    /// Create a destroy event.
    pub fn destroy(permanent: ObjectId, source: Option<ObjectId>) -> Self {
        Self::new_with_provenance(DestroyEvent { permanent, source }, ProvNodeId::default())
    }

    /// Create a sacrifice event.
    pub fn sacrifice(permanent: ObjectId, source: Option<ObjectId>) -> Self {
        Self::new_with_provenance(
            SacrificeEvent::new(permanent, source),
            ProvNodeId::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_factory_damage() {
        let source = ObjectId::from_raw(1);
        let target = DamageTarget::Player(PlayerId::from_index(0));
        let event = Event::damage(source, target, 3, false, EventCause::effect());

        assert_eq!(event.kind(), EventKind::Damage);
    }

    #[test]
    fn test_event_factory_life_gain() {
        let event = Event::life_gain(PlayerId::from_index(0), 5);
        assert_eq!(event.kind(), EventKind::LifeGain);
    }

    #[test]
    fn test_event_factory_zone_change() {
        let event = Event::zone_change(
            ObjectId::from_raw(1),
            Zone::Hand,
            Zone::Battlefield,
            EventCause::effect(),
            None,
        );
        assert_eq!(event.kind(), EventKind::ZoneChange);
    }

    #[test]
    fn test_event_clone() {
        let event = Event::life_gain(PlayerId::from_index(0), 5);
        let cloned = event.clone();
        assert_eq!(event.kind(), cloned.kind());
    }
}
