//! Ability-triggered event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerIdentity;
use crate::zone::Zone;

/// Zone-change provenance of the event that caused an ability to trigger.
///
/// This is copied from the original trigger event before the derived
/// `AbilityTriggeredEvent` is queued. Keeping the complete destination set
/// makes same-object ETB qualification work for both single and batch moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbilityTriggerZoneChangeCause {
    pub from: Zone,
    pub to: Zone,
    pub destination_objects: Vec<ObjectId>,
}

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
    /// The immediate event kind that caused this ability to trigger.
    pub cause_kind: Option<EventKind>,
    /// Primary object of the immediate cause, when it has one.
    pub cause_object: Option<ObjectId>,
    /// Full zone-change provenance for same-object entry qualification.
    pub zone_change_cause: Option<AbilityTriggerZoneChangeCause>,
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
            cause_kind: None,
            cause_object: None,
            zone_change_cause: None,
        }
    }

    pub fn with_source_snapshot(mut self, source_snapshot: Option<ObjectSnapshot>) -> Self {
        self.source_snapshot = source_snapshot;
        self
    }

    pub fn with_cause(
        mut self,
        cause_kind: EventKind,
        cause_object: Option<ObjectId>,
        zone_change_cause: Option<AbilityTriggerZoneChangeCause>,
    ) -> Self {
        self.cause_kind = Some(cause_kind);
        self.cause_object = cause_object;
        self.zone_change_cause = zone_change_cause;
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
