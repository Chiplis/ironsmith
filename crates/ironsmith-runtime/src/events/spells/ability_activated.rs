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
    /// Whether this was a loyalty ability.
    pub is_loyalty_ability: bool,
    /// Whether the activated ability's activation cost contained X.
    pub activation_cost_has_x: bool,
    /// Whether the activated ability's activation cost contained {T}.
    pub activation_cost_has_tap: bool,
    /// Chosen X value for abilities whose activation cost contained X.
    pub x_value: Option<u32>,
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
            is_loyalty_ability: false,
            activation_cost_has_x: false,
            activation_cost_has_tap: false,
            x_value: None,
            snapshot: None,
        }
    }

    /// Mark whether the activated ability was a loyalty ability.
    pub fn with_loyalty_ability(mut self, is_loyalty_ability: bool) -> Self {
        self.is_loyalty_ability = is_loyalty_ability;
        self
    }

    pub fn with_x_value(mut self, x_value: Option<u32>) -> Self {
        self.x_value = x_value;
        self
    }

    pub fn with_activation_cost_has_x(mut self, activation_cost_has_x: bool) -> Self {
        self.activation_cost_has_x = activation_cost_has_x;
        self
    }

    pub fn with_activation_cost_has_tap(mut self, activation_cost_has_tap: bool) -> Self {
        self.activation_cost_has_tap = activation_cost_has_tap;
        self
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

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}
