//! Becomes-targeted event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// An object or player became the target of a spell or ability.
#[derive(Debug, Clone)]
pub struct BecomesTargetedEvent {
    /// The object or player that became targeted.
    pub target: Target,
    /// The spell or ability source that targeted it.
    pub source: ObjectId,
    /// The controller of the source.
    pub source_controller: PlayerId,
    /// Whether the source was an ability (`true`) or spell (`false`).
    pub by_ability: bool,
    /// The targeting ability's own stack id, when it is an ability on the
    /// stack. An ability exists independently of its source (CR 113.7a), and
    /// one source can have several abilities on the stack, so "that spell or
    /// ability" names this entry rather than the source object.
    pub stack_ability: Option<ObjectId>,
    /// Characteristics of the exact target at selection time.
    pub target_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    /// Physical ability-source characteristics, separate from its stack proxy
    /// and the independently controlled spell/ability on the stack.
    pub physical_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
}

impl BecomesTargetedEvent {
    /// The physical source of an ability copy is the original source, while
    /// the ability being targeted/countered has its own immutable stack id.
    pub fn source_for_stack_entry(entry: &crate::game_state::StackEntry) -> ObjectId {
        if entry.is_ability {
            entry.source_snapshot.as_ref().map(|snapshot| snapshot.object_id).unwrap_or(entry.object_id)
        } else { entry.object_id }
    }

    pub fn from_stack_entry(target: Target, entry: &crate::game_state::StackEntry) -> Self {
        Self {
            target,
            source: Self::source_for_stack_entry(entry),
            source_controller: entry.controller,
            by_ability: entry.is_ability,
            stack_ability: entry.is_ability.then(|| entry.target_id()),
            target_snapshot: None,
            physical_source_snapshot: None,
        }
    }

    /// Freeze participants at the selection boundary. Re-publication must
    /// preserve these facts rather than recalculate after later instructions.
    pub fn with_participant_snapshots(mut self, game: &GameState) -> Self {
        use crate::snapshot::ObjectSnapshot;
        if self.target_snapshot.is_none() {
            self.target_snapshot = self.target_object().and_then(|id| game.object(id))
                .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game));
        }
        if self.physical_source_snapshot.is_none() {
            self.physical_source_snapshot = game.object(self.source)
                .filter(|_| !game.is_phased_out(self.source))
                .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
                .or_else(|| game.turn_store.turn_history.departed_object_snapshot(self.source).cloned())
                .or_else(|| self.stack_ability.and_then(|id| game.stack.iter()
                    .find(|entry| entry.is_ability && entry.target_id() == id))
                    .and_then(|entry| entry.source_snapshot.as_ref())
                    .filter(|snapshot| snapshot.object_id == self.source).cloned());
        }
        self
    }

    /// Create a new object becomes-targeted event.
    pub fn new(
        target: ObjectId,
        source: ObjectId,
        source_controller: PlayerId,
        by_ability: bool,
    ) -> Self {
        Self {
            target: Target::Object(target),
            source,
            source_controller,
            by_ability,
            stack_ability: None,
            target_snapshot: None,
            physical_source_snapshot: None,
        }
    }

    /// Create a new player becomes-targeted event.
    pub fn new_player(
        target: PlayerId,
        source: ObjectId,
        source_controller: PlayerId,
        by_ability: bool,
    ) -> Self {
        Self {
            target: Target::Player(target),
            source,
            source_controller,
            by_ability,
            stack_ability: None,
            target_snapshot: None,
            physical_source_snapshot: None,
        }
    }

    /// Create a new becomes-targeted event for any target kind.
    pub fn new_target(
        target: Target,
        source: ObjectId,
        source_controller: PlayerId,
        by_ability: bool,
    ) -> Self {
        Self {
            target,
            source,
            source_controller,
            by_ability,
            stack_ability: None,
            target_snapshot: None,
            physical_source_snapshot: None,
        }
    }

    /// Name the targeting ability's stack entry.
    pub fn with_stack_ability(mut self, stack_ability: Option<ObjectId>) -> Self {
        self.stack_ability = stack_ability;
        self
    }

    pub fn target_object(&self) -> Option<ObjectId> {
        match self.target {
            Target::Object(id) => Some(id),
            Target::Player(_) => None,
        }
    }

    pub fn target_player(&self) -> Option<PlayerId> {
        match self.target {
            Target::Object(_) => None,
            Target::Player(player) => Some(player),
        }
    }
}

impl GameEventType for BecomesTargetedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BecomesTargeted
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        match self.target {
            Target::Object(object_id) => game
                .object(object_id)
                .map(|o| game.controller_of(o))
                .unwrap_or(self.source_controller),
            Target::Player(player) => player,
        }
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &self.target != old {
            return None;
        }
        Some(Box::new(Self {
            target: *new,
            source: self.source,
            source_controller: self.source_controller,
            by_ability: self.by_ability,
            stack_ability: self.stack_ability,
            target_snapshot: None,
            physical_source_snapshot: self.physical_source_snapshot.clone(),
        }))
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn display(&self) -> String {
        "Object became targeted".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        self.target_object()
    }

    fn snapshot(&self) -> Option<&crate::snapshot::ObjectSnapshot> {
        self.target_snapshot.as_ref()
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.source_controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.source_controller)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provisional_reselection_prunes_only_the_exact_copied_ability() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = ObjectId::from_raw(42);
        let first = ObjectId::from_raw(101); let second = ObjectId::from_raw(102);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        for (ability, player) in [(first, a), (second, a), (second, b)] {
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                BecomesTargetedEvent::new_player(player, source, b, true).with_stack_ability(Some(ability)), Default::default());
            game.queue_trigger_event(Default::default(), event);
        }
        game.drop_pending_stale_becomes_targeted_events(source, true, Some(second), &[Target::Player(b)]);
        let pending = game.take_pending_trigger_events();
        let targets: Vec<_> = pending.iter().filter_map(|event| event.downcast::<BecomesTargetedEvent>()).collect();
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().any(|event| event.stack_ability == Some(first) && event.target_player() == Some(a)));
        assert!(targets.iter().any(|event| event.stack_ability == Some(second) && event.target_player() == Some(b)));
    }
}
