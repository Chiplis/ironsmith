//! Remove counters event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
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
    /// The effect/damage source and acting player; game-rule removals omit both.
    pub source: Option<ObjectId>,
    pub actor: Option<PlayerId>,
    pub cause: EventCause,
}

impl RemoveCountersEvent {
    /// Create a new remove counters event.
    pub fn new(target: ObjectId, counter_type: CounterType, count: u32) -> Self {
        Self {
            target,
            counter_type,
            count,
            source: None,
            actor: None,
            cause: EventCause::effect(),
        }
    }

    pub fn with_attribution(mut self, source: Option<ObjectId>, actor: Option<PlayerId>) -> Self {
        self.source = source;
        self.actor = actor;
        self
    }

    pub fn with_cause(mut self, cause: EventCause) -> Self {
        self.cause = cause;
        self
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

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.target)
            .map(|o| game.controller_of(o))
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
        self.source
    }

    fn cause(&self) -> Option<&EventCause> {
        Some(&self.cause)
    }

    fn display(&self) -> String {
        format!(
            "Remove {} {} counter(s)",
            self.count,
            self.counter_type.description()
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Player counterpart of the object removal carrier. Both proposals have one
/// semantic event kind and use the same replacement/commit owner. Attribution
/// is explicit so rule actions such as radiation can remain unattributed.
#[derive(Debug, Clone)]
pub struct RemovePlayerCountersEvent {
    pub player: PlayerId,
    pub counter_type: CounterType,
    pub count: u32,
    pub source: Option<ObjectId>,
    pub actor: Option<PlayerId>,
    pub cause: EventCause,
}

impl RemovePlayerCountersEvent {
    pub fn new(
        player: PlayerId,
        counter_type: CounterType,
        count: u32,
        source: Option<ObjectId>,
        actor: Option<PlayerId>,
    ) -> Self {
        Self {
            player,
            counter_type,
            count,
            source,
            actor,
            cause: EventCause::effect(),
        }
    }
    pub fn with_attribution(mut self, source: Option<ObjectId>, actor: Option<PlayerId>) -> Self {
        self.source = source;
        self.actor = actor;
        self
    }
    pub fn with_cause(mut self, cause: EventCause) -> Self {
        self.cause = cause;
        self
    }
    pub fn with_count(&self, count: u32) -> Self {
        Self {
            count,
            ..self.clone()
        }
    }
    pub fn with_player(&self, player: PlayerId) -> Self {
        Self {
            player,
            ..self.clone()
        }
    }
}

impl GameEventType for RemovePlayerCountersEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::RemoveCounters
    }
    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }
    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![RedirectableTarget {
            target: Target::Player(self.player),
            description: "counter removal player",
            valid_redirect_types: RedirectValidTypes::PlayersOnly,
        }]
    }
    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if *old != Target::Player(self.player) {
            return None;
        }
        match new {
            Target::Player(player) => Some(Box::new(self.with_player(*player))),
            Target::Object(_) => None,
        }
    }
    fn source_object(&self) -> Option<ObjectId> {
        self.source
    }
    fn cause(&self) -> Option<&EventCause> {
        Some(&self.cause)
    }
    fn display(&self) -> String {
        format!(
            "Remove {} {} counter(s) from a player",
            self.count,
            self.counter_type.description()
        )
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Typed view shared by event dispatch, amount replacement and commitment.
/// An object-filter matcher can still downcast only the object carrier.
pub(crate) enum CounterRemovalEvent<'a> {
    Object(&'a RemoveCountersEvent),
    Player(&'a RemovePlayerCountersEvent),
}
impl<'a> CounterRemovalEvent<'a> {
    pub(crate) fn from_event(event: &'a dyn GameEventType) -> Option<Self> {
        if let Some(removal) = event.as_any().downcast_ref::<RemoveCountersEvent>() {
            Some(Self::Object(removal))
        } else {
            event
                .as_any()
                .downcast_ref::<RemovePlayerCountersEvent>()
                .map(Self::Player)
        }
    }
    pub(crate) fn target(&self) -> Target {
        match self {
            Self::Object(removal) => Target::Object(removal.target),
            Self::Player(removal) => Target::Player(removal.player),
        }
    }
    pub(crate) fn counter_type(&self) -> CounterType {
        match self {
            Self::Object(removal) => removal.counter_type,
            Self::Player(removal) => removal.counter_type,
        }
    }
    pub(crate) fn count(&self) -> u32 {
        match self {
            Self::Object(removal) => removal.count,
            Self::Player(removal) => removal.count,
        }
    }
    pub(crate) fn with_count(
        &self,
        event: &crate::events::Event,
        count: u32,
    ) -> crate::events::Event {
        match self {
            Self::Object(removal) => event.rewrap(removal.with_count(count)),
            Self::Player(removal) => event.rewrap(removal.with_count(count)),
        }
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
        assert_eq!(event.display(), "Remove 3 loyalty counter(s)");
    }
}
