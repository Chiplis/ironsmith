//! Damage-prevention event implementation.

use std::any::Any;

use crate::events::DamageTarget;
use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Event emitted after a prevention effect actually prevents damage.
#[derive(Debug, Clone)]
pub struct DamagePreventedEvent {
    /// The source whose damage was prevented.
    pub source: ObjectId,
    /// The target the damage would have been dealt to.
    pub target: DamageTarget,
    /// Amount of damage actually prevented.
    pub amount: u32,
    /// Whether the prevented damage was combat damage.
    pub is_combat: bool,
    /// What caused the original damage event.
    pub cause: EventCause,
}

impl DamagePreventedEvent {
    pub fn with_cause(
        source: ObjectId,
        target: DamageTarget,
        amount: u32,
        is_combat: bool,
        cause: EventCause,
    ) -> Self {
        Self {
            source,
            target,
            amount,
            is_combat,
            cause,
        }
    }
}

impl GameEventType for DamagePreventedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::DamagePrevented
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        match self.target {
            DamageTarget::Player(player) => player,
            DamageTarget::Object(obj_id) => game
                .object(obj_id)
                .map(|o| game.controller_of(o))
                .unwrap_or(game.turn.active_player),
        }
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        match self.target {
            DamageTarget::Player(player) => Some(player),
            DamageTarget::Object(_) => None,
        }
    }

    fn display(&self) -> String {
        let target_str = match self.target {
            DamageTarget::Player(_) => "player",
            DamageTarget::Object(_) => "permanent",
        };
        let combat_str = if self.is_combat { "combat " } else { "" };
        format!(
            "Prevent {} {}damage to {}",
            self.amount, combat_str, target_str
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
