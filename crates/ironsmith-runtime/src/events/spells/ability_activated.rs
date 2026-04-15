//! Ability activated event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// An ability was activated.
#[derive(Debug, Clone)]
pub struct AbilityActivatedEvent {
    /// The source object whose ability was activated.
    pub source: ObjectId,
    /// The player who activated the ability.
    pub activator: PlayerId,
    /// Whether this was a mana ability.
    pub is_mana_ability: bool,
    /// Last-known snapshot of the source at activation time.
    pub snapshot: Option<ObjectSnapshot>,
}

impl AbilityActivatedEvent {
    /// Create a new ability-activated event.
    pub fn new(source: ObjectId, activator: PlayerId, is_mana_ability: bool) -> Self {
        Self {
            source,
            activator,
            is_mana_ability,
            snapshot: None,
        }
    }

    /// Attach a snapshot captured when the ability was activated.
    pub fn with_snapshot(mut self, snapshot: Option<ObjectSnapshot>) -> Self {
        self.snapshot = snapshot;
        self
    }
}

impl GameEventType for AbilityActivatedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::AbilityActivated
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.activator
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        if self.is_mana_ability {
            "Mana ability activated".to_string()
        } else {
            "Ability activated".to_string()
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.activator)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.activator)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}
