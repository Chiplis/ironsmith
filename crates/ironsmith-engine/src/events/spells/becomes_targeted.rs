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
}

impl BecomesTargetedEvent {
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

    fn player(&self) -> Option<PlayerId> {
        Some(self.source_controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.source_controller)
    }
}
