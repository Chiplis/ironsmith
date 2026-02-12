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
    AttackEventTarget, CreatureAttackedEvent, CreatureBecameBlockedEvent, CreatureBlockedEvent,
};
pub use other::{
    BecameMonstrousEvent, CardDiscardedEvent, CardsDrawnEvent, CounterPlacedEvent,
    KeywordActionEvent, KeywordActionKind, MarkerChangeType, MarkersChangedEvent,
    PermanentTappedEvent, PermanentUntappedEvent, PlayerVote, PlayersFinishedVotingEvent,
    TransformedEvent, TurnedFaceUpEvent,
};
pub use phase::{
    BeginningOfCombatEvent, BeginningOfDrawStepEvent, BeginningOfEndStepEvent,
    BeginningOfPostcombatMainPhaseEvent, BeginningOfPrecombatMainPhaseEvent,
    BeginningOfUpkeepEvent, EndOfCombatEvent,
};
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
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

/// Wrapper around a boxed event type.
///
/// This provides a more ergonomic API for working with events and includes
/// factory methods for creating common event types.
#[derive(Debug, Clone)]
pub struct Event(pub Box<dyn GameEventType>);

impl Event {
    /// Create an event from any type implementing GameEventType.
    pub fn new<E: GameEventType + 'static>(event: E) -> Self {
        Self(Box::new(event))
    }

    /// Get the event kind for fast dispatch.
    pub fn kind(&self) -> EventKind {
        self.0.event_kind()
    }

    /// Get the inner event as a trait object.
    pub fn inner(&self) -> &dyn GameEventType {
        &*self.0
    }

    // Factory methods for common event types

    /// Create a damage event.
    pub fn damage(source: ObjectId, target: DamageTarget, amount: u32, is_combat: bool) -> Self {
        Self::new(DamageEvent {
            source,
            target,
            amount,
            is_combat,
            is_unpreventable: false,
        })
    }

    /// Create an unpreventable damage event.
    pub fn unpreventable_damage(
        source: ObjectId,
        target: DamageTarget,
        amount: u32,
        is_combat: bool,
    ) -> Self {
        Self::new(DamageEvent {
            source,
            target,
            amount,
            is_combat,
            is_unpreventable: true,
        })
    }

    /// Create a life gain event.
    pub fn life_gain(player: PlayerId, amount: u32) -> Self {
        Self::new(LifeGainEvent { player, amount })
    }

    /// Create a life loss event.
    pub fn life_loss(player: PlayerId, amount: u32, from_damage: bool) -> Self {
        Self::new(LifeLossEvent {
            player,
            amount,
            from_damage,
        })
    }

    /// Create a zone change event.
    pub fn zone_change(
        object: ObjectId,
        from: Zone,
        to: Zone,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        Self::new(ZoneChangeEvent::new(object, from, to, snapshot))
    }

    /// Create an enter battlefield event.
    pub fn enter_battlefield(
        object: ObjectId,
        from: Zone,
        enters_tapped: bool,
        enters_with_counters: Vec<(CounterType, u32)>,
    ) -> Self {
        Self::new(EnterBattlefieldEvent {
            object,
            from,
            enters_tapped,
            enters_with_counters,
            enters_as_copy_of: None,
        })
    }

    /// Create a "dies" zone change event (battlefield -> graveyard).
    pub fn dies(object_id: ObjectId, controller: PlayerId, snapshot: ObjectSnapshot) -> Self {
        let _ = controller;
        Self::new(ZoneChangeEvent::new(
            object_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ))
    }

    /// Create a put counters event.
    pub fn put_counters(target: ObjectId, counter_type: CounterType, count: u32) -> Self {
        Self::new(PutCountersEvent {
            target,
            counter_type,
            count,
        })
    }

    /// Create a remove counters event.
    pub fn remove_counters(target: ObjectId, counter_type: CounterType, count: u32) -> Self {
        Self::new(RemoveCountersEvent {
            target,
            counter_type,
            count,
        })
    }

    /// Create a move counters event.
    pub fn move_counters(
        from: ObjectId,
        to: ObjectId,
        counter_type: Option<CounterType>,
        count: Option<u32>,
    ) -> Self {
        Self::new(MoveCountersEvent {
            from,
            to,
            counter_type,
            count,
        })
    }

    /// Create a draw event.
    pub fn draw(player: PlayerId, count: u32, is_first_this_turn: bool) -> Self {
        Self::new(DrawEvent {
            player,
            count,
            is_first_this_turn,
        })
    }

    /// Create a discard event from an effect.
    pub fn discard_from_effect(card: ObjectId, player: PlayerId) -> Self {
        Self::new(DiscardEvent::from_effect(card, player))
    }

    /// Create a discard event as a cost.
    pub fn discard_as_cost(card: ObjectId, player: PlayerId) -> Self {
        Self::new(DiscardEvent::as_cost(card, player))
    }

    /// Create a discard event from a game rule (e.g., cleanup step).
    pub fn discard_from_game_rule(card: ObjectId, player: PlayerId) -> Self {
        Self::new(DiscardEvent::from_game_rule(card, player))
    }

    /// Create a discard event with a custom cause.
    pub fn discard_with_cause(card: ObjectId, player: PlayerId, cause: EventCause) -> Self {
        Self::new(DiscardEvent::with_cause(card, player, cause))
    }

    /// Create a tap event.
    pub fn tap(permanent: ObjectId) -> Self {
        Self::new(TapEvent { permanent })
    }

    /// Create an untap event.
    pub fn untap(permanent: ObjectId) -> Self {
        Self::new(UntapEvent { permanent })
    }

    /// Create a destroy event.
    pub fn destroy(permanent: ObjectId, source: Option<ObjectId>) -> Self {
        Self::new(DestroyEvent { permanent, source })
    }

    /// Create a sacrifice event.
    pub fn sacrifice(permanent: ObjectId, source: Option<ObjectId>) -> Self {
        Self::new(SacrificeEvent::new(permanent, source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_factory_damage() {
        let source = ObjectId::from_raw(1);
        let target = DamageTarget::Player(PlayerId::from_index(0));
        let event = Event::damage(source, target, 3, false);

        assert_eq!(event.kind(), EventKind::Damage);
    }

    #[test]
    fn test_event_factory_life_gain() {
        let event = Event::life_gain(PlayerId::from_index(0), 5);
        assert_eq!(event.kind(), EventKind::LifeGain);
    }

    #[test]
    fn test_event_factory_zone_change() {
        let event = Event::zone_change(ObjectId::from_raw(1), Zone::Hand, Zone::Battlefield, None);
        assert_eq!(event.kind(), EventKind::ZoneChange);
    }

    #[test]
    fn test_event_clone() {
        let event = Event::life_gain(PlayerId::from_index(0), 5);
        let cloned = event.clone();
        assert_eq!(event.kind(), cloned.kind());
    }
}
