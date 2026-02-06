//! Remove counters event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;

/// A remove counters event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct RemoveCountersEvent {
    /// The permanent losing counters
    pub target: ObjectId,
    /// The type of counter
    pub counter_type: CounterType,
    /// Number of counters to remove
    pub count: u32,
}

impl RemoveCountersEvent {
    /// Create a new remove counters event.
    pub fn new(target: ObjectId, counter_type: CounterType, count: u32) -> Self {
        Self {
            target,
            counter_type,
            count,
        }
    }

    /// Return a new event with a different count.
    pub fn with_count(&self, count: u32) -> Self {
        Self {
            count,
            ..self.clone()
        }
    }

    /// Return a new event with a different target.
    pub fn with_target(&self, target: ObjectId) -> Self {
        Self {
            target,
            ..self.clone()
        }
    }
}

impl GameEventType for RemoveCountersEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::RemoveCounters
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.target)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![RedirectableTarget {
            target: Target::Object(self.target),
            description: "counter removal target",
            valid_redirect_types: RedirectValidTypes::ObjectsOnly,
        }]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &Target::Object(self.target) != old {
            return None;
        }

        if let Target::Object(new_obj) = new {
            Some(Box::new(self.with_target(*new_obj)))
        } else {
            None
        }
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        format!("Remove {} {:?} counter(s)", self.count, self.counter_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_counters_event_creation() {
        let event = RemoveCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 2);

        assert_eq!(event.count, 2);
        assert_eq!(event.counter_type, CounterType::PlusOnePlusOne);
    }

    #[test]
    fn test_remove_counters_event_kind() {
        let event = RemoveCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 2);
        assert_eq!(event.event_kind(), EventKind::RemoveCounters);
    }

    #[test]
    fn test_remove_counters_display() {
        let event = RemoveCountersEvent::new(ObjectId::from_raw(1), CounterType::Loyalty, 3);
        assert_eq!(event.display(), "Remove 3 Loyalty counter(s)");
    }
}
