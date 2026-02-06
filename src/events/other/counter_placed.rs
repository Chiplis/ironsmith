//! Counter placed event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;

/// A counter was placed on a permanent event.
///
/// Triggered when one or more counters are placed on a permanent.
#[derive(Debug, Clone)]
pub struct CounterPlacedEvent {
    /// The permanent that received the counter(s)
    pub permanent: ObjectId,
    /// The type of counter placed
    pub counter_type: CounterType,
    /// The number of counters placed
    pub amount: u32,
}

impl CounterPlacedEvent {
    /// Create a new counter placed event.
    pub fn new(permanent: ObjectId, counter_type: CounterType, amount: u32) -> Self {
        Self {
            permanent,
            counter_type,
            amount,
        }
    }
}

impl GameEventType for CounterPlacedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CounterPlaced
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.permanent)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        format!(
            "{} {:?} counter(s) placed on permanent",
            self.amount, self.counter_type
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }

    fn player(&self) -> Option<PlayerId> {
        None
    }

    fn controller(&self) -> Option<PlayerId> {
        None // Determined from game state
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_placed_event_creation() {
        let event = CounterPlacedEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 3);
        assert_eq!(event.permanent, ObjectId::from_raw(1));
        assert_eq!(event.counter_type, CounterType::PlusOnePlusOne);
        assert_eq!(event.amount, 3);
    }

    #[test]
    fn test_counter_placed_event_kind() {
        let event = CounterPlacedEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 1);
        assert_eq!(event.event_kind(), EventKind::CounterPlaced);
    }
}
