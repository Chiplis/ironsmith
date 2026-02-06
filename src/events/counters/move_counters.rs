//! Move counters event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;

/// A move counters event that can be processed through the replacement effect system.
///
/// This event has TWO redirectable targets: the source and destination permanents.
#[derive(Debug, Clone)]
pub struct MoveCountersEvent {
    /// The permanent counters are being moved FROM
    pub from: ObjectId,
    /// The permanent counters are being moved TO
    pub to: ObjectId,
    /// The type of counters being moved (None = all counter types)
    pub counter_type: Option<CounterType>,
    /// The count of counters to move (None = all counters of the type)
    pub count: Option<u32>,
}

impl MoveCountersEvent {
    /// Create a new move counters event.
    pub fn new(
        from: ObjectId,
        to: ObjectId,
        counter_type: Option<CounterType>,
        count: Option<u32>,
    ) -> Self {
        Self {
            from,
            to,
            counter_type,
            count,
        }
    }

    /// Return a new event with a different source.
    pub fn with_from(&self, from: ObjectId) -> Self {
        Self {
            from,
            ..self.clone()
        }
    }

    /// Return a new event with a different destination.
    pub fn with_to(&self, to: ObjectId) -> Self {
        Self { to, ..self.clone() }
    }
}

impl GameEventType for MoveCountersEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::MoveCounters
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        // The affected player is the controller of the destination
        game.object(self.to)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![
            RedirectableTarget {
                target: Target::Object(self.from),
                description: "counter source",
                valid_redirect_types: RedirectValidTypes::ObjectsOnly,
            },
            RedirectableTarget {
                target: Target::Object(self.to),
                description: "counter destination",
                valid_redirect_types: RedirectValidTypes::ObjectsOnly,
            },
        ]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        // Check if we're redirecting the source
        if &Target::Object(self.from) == old {
            if let Target::Object(new_obj) = new {
                return Some(Box::new(self.with_from(*new_obj)));
            }
            return None;
        }

        // Check if we're redirecting the destination
        if &Target::Object(self.to) == old {
            if let Target::Object(new_obj) = new {
                return Some(Box::new(self.with_to(*new_obj)));
            }
            return None;
        }

        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        match (&self.counter_type, self.count) {
            (Some(ct), Some(n)) => format!("Move {} {:?} counter(s)", n, ct),
            (Some(ct), None) => format!("Move all {:?} counters", ct),
            (None, Some(n)) => format!("Move {} counters", n),
            (None, None) => "Move all counters".to_string(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_counters_event_creation() {
        let event = MoveCountersEvent::new(
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            Some(CounterType::PlusOnePlusOne),
            Some(3),
        );

        assert_eq!(event.from, ObjectId::from_raw(1));
        assert_eq!(event.to, ObjectId::from_raw(2));
        assert_eq!(event.counter_type, Some(CounterType::PlusOnePlusOne));
        assert_eq!(event.count, Some(3));
    }

    #[test]
    fn test_move_counters_event_kind() {
        let event =
            MoveCountersEvent::new(ObjectId::from_raw(1), ObjectId::from_raw(2), None, None);
        assert_eq!(event.event_kind(), EventKind::MoveCounters);
    }

    #[test]
    fn test_move_counters_has_two_redirectable_targets() {
        let event =
            MoveCountersEvent::new(ObjectId::from_raw(1), ObjectId::from_raw(2), None, None);

        let targets = event.redirectable_targets();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].description, "counter source");
        assert_eq!(targets[1].description, "counter destination");
    }

    #[test]
    fn test_move_counters_redirect_source() {
        let event = MoveCountersEvent::new(
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            Some(CounterType::PlusOnePlusOne),
            Some(3),
        );

        let old_target = Target::Object(ObjectId::from_raw(1));
        let new_target = Target::Object(ObjectId::from_raw(10));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_move = replaced
            .as_any()
            .downcast_ref::<MoveCountersEvent>()
            .unwrap();
        assert_eq!(replaced_move.from, ObjectId::from_raw(10));
        assert_eq!(replaced_move.to, ObjectId::from_raw(2));
    }

    #[test]
    fn test_move_counters_redirect_destination() {
        let event = MoveCountersEvent::new(
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            Some(CounterType::PlusOnePlusOne),
            Some(3),
        );

        let old_target = Target::Object(ObjectId::from_raw(2));
        let new_target = Target::Object(ObjectId::from_raw(20));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_move = replaced
            .as_any()
            .downcast_ref::<MoveCountersEvent>()
            .unwrap();
        assert_eq!(replaced_move.from, ObjectId::from_raw(1));
        assert_eq!(replaced_move.to, ObjectId::from_raw(20));
    }

    #[test]
    fn test_move_counters_display() {
        let event1 = MoveCountersEvent::new(
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            Some(CounterType::PlusOnePlusOne),
            Some(3),
        );
        assert_eq!(event1.display(), "Move 3 PlusOnePlusOne counter(s)");

        let event2 =
            MoveCountersEvent::new(ObjectId::from_raw(1), ObjectId::from_raw(2), None, None);
        assert_eq!(event2.display(), "Move all counters");
    }
}
