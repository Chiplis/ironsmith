//! TriggerEvent wrapper for the unified event system.
//!
//! This module provides the `TriggerEvent` type that wraps a `dyn GameEventType`
//! for use in the trigger matching system.

use std::sync::Arc;

use crate::events::{EventKind, GameEventType};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A wrapper around a game event for trigger matching.
///
/// This type wraps any `GameEventType` implementation using `Arc` for cheap cloning.
/// It provides accessor methods for common event data and supports downcasting
/// to concrete event types when needed.
///
/// # Example
///
/// ```ignore
/// use ironsmith::triggers::TriggerEvent;
/// use ironsmith::events::{SpellCastEvent, EventKind};
///
/// let event = TriggerEvent::new(SpellCastEvent::new(spell_id, caster_id));
///
/// // Fast kind checking
/// if event.kind() == EventKind::SpellCast {
///     // Downcast to access event-specific fields
///     if let Some(spell_cast) = event.downcast::<SpellCastEvent>() {
///         println!("Spell {} cast by {:?}", spell_cast.spell, spell_cast.caster);
///     }
/// }
/// ```
#[derive(Clone)]
pub struct TriggerEvent {
    inner: Arc<dyn GameEventType>,
}

impl TriggerEvent {
    /// Create a new trigger event from any type implementing GameEventType.
    pub fn new<E: GameEventType + 'static>(event: E) -> Self {
        Self {
            inner: Arc::new(event),
        }
    }

    /// Create a new trigger event from a boxed GameEventType.
    pub fn from_boxed(event: Box<dyn GameEventType>) -> Self {
        Self {
            inner: Arc::from(event),
        }
    }

    /// Get the event kind for fast dispatch without downcasting.
    #[inline]
    pub fn kind(&self) -> EventKind {
        self.inner.event_kind()
    }

    /// Attempt to downcast to a concrete event type.
    ///
    /// Returns `Some(&T)` if the inner event is of type `T`, otherwise `None`.
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }

    /// Get the object ID involved in this event, if any.
    #[inline]
    pub fn object_id(&self) -> Option<ObjectId> {
        self.inner.object_id()
    }

    /// Get the player involved in this event, if any.
    #[inline]
    pub fn player(&self) -> Option<PlayerId> {
        self.inner.player()
    }

    /// Get the controller of the object involved in this event, if any.
    #[inline]
    pub fn controller(&self) -> Option<PlayerId> {
        self.inner.controller()
    }

    /// Get the object snapshot for "last known information" if this event has one.
    #[inline]
    pub fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.inner.snapshot()
    }

    /// Get a human-readable description of this event.
    pub fn display(&self) -> String {
        self.inner.display()
    }

    /// Get the inner event as a trait object reference.
    pub fn inner(&self) -> &dyn GameEventType {
        &*self.inner
    }
}

impl std::fmt::Debug for TriggerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TriggerEvent")
            .field("kind", &self.kind())
            .field("display", &self.display())
            .finish()
    }
}

impl PartialEq for TriggerEvent {
    fn eq(&self, other: &Self) -> bool {
        // Compare by Arc pointer equality first (fast path)
        if Arc::ptr_eq(&self.inner, &other.inner) {
            return true;
        }
        // Fall back to comparing kind and object_id
        self.kind() == other.kind() && self.object_id() == other.object_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::events::spells::SpellCastEvent;
    use crate::events::zones::ZoneChangeEvent;
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn test_trigger_event_creation() {
        let event = TriggerEvent::new(SpellCastEvent::new(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            Zone::Hand,
        ));
        assert_eq!(event.kind(), EventKind::SpellCast);
    }

    #[test]
    fn test_trigger_event_clone() {
        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(PlayerId::from_index(0)));
        let cloned = event.clone();
        assert_eq!(event.kind(), cloned.kind());
    }

    #[test]
    fn test_trigger_event_downcast_success() {
        let event = TriggerEvent::new(SpellCastEvent::new(
            ObjectId::from_raw(42),
            PlayerId::from_index(1),
            Zone::Hand,
        ));

        let spell_cast = event.downcast::<SpellCastEvent>();
        assert!(spell_cast.is_some());
        assert_eq!(spell_cast.unwrap().spell, ObjectId::from_raw(42));
    }

    #[test]
    fn test_trigger_event_downcast_failure() {
        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(PlayerId::from_index(0)));

        let spell_cast = event.downcast::<SpellCastEvent>();
        assert!(spell_cast.is_none());
    }

    #[test]
    fn test_trigger_event_accessors() {
        let event = TriggerEvent::new(SpellCastEvent::new(
            ObjectId::from_raw(99),
            PlayerId::from_index(2),
            Zone::Hand,
        ));

        assert_eq!(event.object_id(), Some(ObjectId::from_raw(99)));
        assert_eq!(event.player(), Some(PlayerId::from_index(2)));
        assert_eq!(event.controller(), Some(PlayerId::from_index(2)));
        assert!(event.snapshot().is_none());
    }

    #[test]
    fn test_trigger_event_with_snapshot() {
        let snapshot = ObjectSnapshot::for_testing(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            "Test Creature",
        )
        .with_card_types(vec![CardType::Creature]);

        let event = TriggerEvent::new(ZoneChangeEvent::new(
            ObjectId::from_raw(1),
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ));

        assert!(event.snapshot().is_some());
        assert_eq!(event.snapshot().unwrap().name, "Test Creature");
    }

    #[test]
    fn test_trigger_event_debug() {
        let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(PlayerId::from_index(0)));
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("TriggerEvent"));
        assert!(debug_str.contains("BeginningOfUpkeep"));
    }
}
