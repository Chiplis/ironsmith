//! Ability-triggered event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerIdentity;

/// A triggered ability triggered and became pending.
#[derive(Debug, Clone)]
pub struct AbilityTriggeredEvent {
    /// The source object of the ability that triggered.
    pub source: ObjectId,
    /// Stable identity of that source object.
    pub source_stable_id: StableId,
    /// The controller of the ability at the moment it triggered.
    pub controller: PlayerId,
    /// Structural identity of the triggered ability.
    pub trigger_identity: TriggerIdentity,
    /// Last-known source information, when the source is no longer available.
    pub source_snapshot: Option<ObjectSnapshot>,
}

impl AbilityTriggeredEvent {
    pub fn new(
        source: ObjectId,
        source_stable_id: StableId,
        controller: PlayerId,
        trigger_identity: TriggerIdentity,
    ) -> Self {
        Self {
            source,
            source_stable_id,
            controller,
            trigger_identity,
            source_snapshot: None,
        }
    }

    pub fn with_source_snapshot(mut self, source_snapshot: Option<ObjectSnapshot>) -> Self {
        self.source_snapshot = source_snapshot;
        self
    }
}

impl GameEventType for AbilityTriggeredEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::AbilityTriggered
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Ability triggered".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.source_snapshot.as_ref()
    }
}
