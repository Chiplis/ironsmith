//! Put counters event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;

/// A put counters event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct PutCountersEvent {
    /// The object or player receiving counters.
    pub target: Target,
    /// The type of counter
    pub counter_type: CounterType,
    /// Number of counters to add
    pub count: u32,
    /// Upper bound established by a replacement that also prohibits
    /// additional counters for this event. Later counter multipliers and
    /// additions must respect it.
    pub maximum_count: Option<u32>,
    /// What caused these counters to be put.
    pub cause: EventCause,
}

impl PutCountersEvent {
    /// Create a new put counters event with an explicit cause.
    pub fn with_cause(
        target: Target,
        counter_type: CounterType,
        count: u32,
        cause: EventCause,
    ) -> Self {
        Self {
            target,
            counter_type,
            count,
            maximum_count: None,
            cause,
        }
    }

    /// Return a new event with doubled counter count.
    pub fn doubled(&self) -> Self {
        self.with_count(self.count.saturating_mul(2))
    }

    /// Return a new event with additional counters.
    pub fn with_additional(&self, extra: u32) -> Self {
        self.with_count(self.count.saturating_add(extra))
    }

    /// Return a new event with a different count.
    pub fn with_count(&self, count: u32) -> Self {
        Self {
            count: self
                .maximum_count
                .map_or(count, |maximum| count.min(maximum)),
            ..self.clone()
        }
    }

    /// Set this event's replacement amount and prevent later replacement
    /// effects from increasing it beyond that amount.
    pub fn with_count_limit(&self, count: u32, maximum: u32) -> Self {
        let maximum = self
            .maximum_count
            .map_or(maximum, |existing| existing.min(maximum));
        Self {
            count: count.min(maximum),
            maximum_count: Some(maximum),
            ..self.clone()
        }
    }

    /// Return a new event with a different target.
    pub fn with_target(&self, target: Target) -> Self {
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

    fn affected_player(&self, game: &GameState) -> PlayerId {
        match self.target {
            Target::Object(object) => game
                .object(object)
                .map(|o| game.controller_of(o))
                .unwrap_or(game.turn.active_player),
            Target::Player(player) => player,
        }
    }

    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![RedirectableTarget {
            target: self.target,
            description: "counter recipient",
            valid_redirect_types: match self.target {
                Target::Object(_) => RedirectValidTypes::ObjectsOnly,
                Target::Player(_) => RedirectValidTypes::PlayersOnly,
            },
        }]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &self.target != old {
            return None;
        }

        match (self.target, *new) {
            (Target::Object(_), Target::Object(_)) | (Target::Player(_), Target::Player(_)) => {
                Some(Box::new(self.with_target(*new)))
            }
            _ => None,
        }
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        format!(
            "Put {} {} counter(s)",
            self.count,
            self.counter_type.description()
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effect_counters(count: u32) -> PutCountersEvent {
        PutCountersEvent::with_cause(
            Target::Object(ObjectId::from_raw(1)),
            CounterType::PlusOnePlusOne,
            count,
            EventCause::effect(),
        )
    }

    #[test]
    fn test_put_counters_event_creation() {
        let event = effect_counters(3);

        assert_eq!(event.count, 3);
        assert_eq!(event.counter_type, CounterType::PlusOnePlusOne);
    }

    #[test]
    fn test_put_counters_doubled() {
        let event = effect_counters(3);

        let doubled = event.doubled();
        assert_eq!(doubled.count, 6);
    }

    #[test]
    fn test_put_counters_with_additional() {
        let event = effect_counters(3);

        let with_extra = event.with_additional(2);
        assert_eq!(with_extra.count, 5);
    }

    #[test]
    fn count_limit_survives_later_counter_replacements() {
        let event = effect_counters(4).with_count_limit(1, 1);

        assert_eq!(event.count, 1);
        assert_eq!(event.doubled().count, 1);
        assert_eq!(event.with_additional(3).count, 1);
        assert_eq!(event.with_count(9).count, 1);
    }

    #[test]
    fn test_put_counters_event_kind() {
        let event = effect_counters(3);
        assert_eq!(event.event_kind(), EventKind::PutCounters);
    }

    #[test]
    fn test_put_counters_redirect() {
        let event = effect_counters(3);

        let old_target = Target::Object(ObjectId::from_raw(1));
        let new_target = Target::Object(ObjectId::from_raw(2));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_counters = replaced
            .as_any()
            .downcast_ref::<PutCountersEvent>()
            .unwrap();
        assert_eq!(
            replaced_counters.target,
            Target::Object(ObjectId::from_raw(2))
        );
    }

    #[test]
    fn test_put_counters_redirect_to_player() {
        let event = effect_counters(3);

        let old_target = Target::Object(ObjectId::from_raw(1));
        let new_target = Target::Player(PlayerId::from_index(0));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_none());
    }

    #[test]
    fn test_put_player_counters_redirect_to_player() {
        let event = PutCountersEvent::with_cause(
            Target::Player(PlayerId::from_index(0)),
            CounterType::Energy,
            3,
            EventCause::effect(),
        );

        let old_target = Target::Player(PlayerId::from_index(0));
        let new_target = Target::Player(PlayerId::from_index(1));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());
    }

    #[test]
    fn test_put_counters_display() {
        let event = effect_counters(3);
        assert_eq!(event.display(), "Put 3 +1/+1 counter(s)");
    }
}
