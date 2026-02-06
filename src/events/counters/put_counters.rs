//! Put counters event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;

/// A put counters event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct PutCountersEvent {
    /// The permanent receiving counters
    pub target: ObjectId,
    /// The type of counter
    pub counter_type: CounterType,
    /// Number of counters to add
    pub count: u32,
}

impl PutCountersEvent {
    /// Create a new put counters event.
    pub fn new(target: ObjectId, counter_type: CounterType, count: u32) -> Self {
        Self {
            target,
            counter_type,
            count,
        }
    }

    /// Return a new event with doubled counter count.
    pub fn doubled(&self) -> Self {
        Self {
            count: self.count.saturating_mul(2),
            ..self.clone()
        }
    }

    /// Return a new event with additional counters.
    pub fn with_additional(&self, extra: u32) -> Self {
        Self {
            count: self.count.saturating_add(extra),
            ..self.clone()
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

impl GameEventType for PutCountersEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PutCounters
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
            description: "counter recipient",
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
        format!("Put {} {:?} counter(s)", self.count, self.counter_type)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_counters_event_creation() {
        let event = PutCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 3);

        assert_eq!(event.count, 3);
        assert_eq!(event.counter_type, CounterType::PlusOnePlusOne);
    }

    #[test]
    fn test_put_counters_doubled() {
        let event = PutCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 3);

        let doubled = event.doubled();
        assert_eq!(doubled.count, 6);
    }

    #[test]
    fn test_put_counters_with_additional() {
        let event = PutCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 3);

        let with_extra = event.with_additional(2);
        assert_eq!(with_extra.count, 5);
    }

    #[test]
    fn test_put_counters_event_kind() {
        let event = PutCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 3);
        assert_eq!(event.event_kind(), EventKind::PutCounters);
    }

    #[test]
    fn test_put_counters_redirect() {
        let event = PutCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 3);

        let old_target = Target::Object(ObjectId::from_raw(1));
        let new_target = Target::Object(ObjectId::from_raw(2));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_counters = replaced
            .as_any()
            .downcast_ref::<PutCountersEvent>()
            .unwrap();
        assert_eq!(replaced_counters.target, ObjectId::from_raw(2));
    }

    #[test]
    fn test_put_counters_redirect_to_player_fails() {
        let event = PutCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 3);

        let old_target = Target::Object(ObjectId::from_raw(1));
        let new_target = Target::Player(PlayerId::from_index(0));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_none());
    }

    #[test]
    fn test_put_counters_display() {
        let event = PutCountersEvent::new(ObjectId::from_raw(1), CounterType::PlusOnePlusOne, 3);
        assert_eq!(event.display(), "Put 3 PlusOnePlusOne counter(s)");
    }
}
