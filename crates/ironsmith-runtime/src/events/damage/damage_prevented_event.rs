//! Event emitted when a prevention effect actually prevents damage (CR 615.13).

use std::any::Any;

use crate::events::DamageTarget;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// One component of the damage affected by a single prevention-effect application.
#[derive(Debug, Clone)]
pub struct PreventedDamage {
    pub damage_source: ObjectId,
    pub target: DamageTarget,
    pub amount: u32,
    pub is_combat: bool,
    pub target_snapshot: Option<ObjectSnapshot>,
}

/// One application of a prevention effect that prevented some or all damage.
#[derive(Debug, Clone)]
pub struct DamagePreventedEvent {
    pub damage_source: ObjectId,
    pub target: DamageTarget,
    pub amount: u32,
    pub prevention_source: ObjectId,
    pub prevention_controller: PlayerId,
    pub is_combat: bool,
    pub target_snapshot: Option<ObjectSnapshot>,
    /// Every simultaneous damage event affected by this one application.
    pub applications: Vec<PreventedDamage>,
    /// Stable identity for a resolution-created shield, when applicable.
    pub prevention_shield: Option<crate::prevention::PreventionShieldId>,
}

impl DamagePreventedEvent {
    pub fn new(
        damage_source: ObjectId,
        target: DamageTarget,
        amount: u32,
        prevention_source: ObjectId,
        prevention_controller: PlayerId,
        is_combat: bool,
    ) -> Self {
        let application = PreventedDamage {
            damage_source,
            target,
            amount,
            is_combat,
            target_snapshot: None,
        };
        Self {
            damage_source,
            target,
            amount,
            prevention_source,
            prevention_controller,
            is_combat,
            target_snapshot: None,
            applications: vec![application],
            prevention_shield: None,
        }
    }

    pub fn with_target_snapshot(mut self, snapshot: ObjectSnapshot) -> Self {
        self.target_snapshot = Some(snapshot.clone());
        if let Some(application) = self.applications.first_mut() {
            application.target_snapshot = Some(snapshot);
        }
        self
    }

    pub fn with_prevention_shield(mut self, shield: crate::prevention::PreventionShieldId) -> Self {
        self.prevention_shield = Some(shield);
        self
    }

    pub fn merge_simultaneous(&mut self, other: Self) -> bool {
        if self.prevention_shield.is_none()
            || self.prevention_shield != other.prevention_shield
            || self.prevention_source != other.prevention_source
            || self.prevention_controller != other.prevention_controller
        {
            return false;
        }
        self.amount = self.amount.saturating_add(other.amount);
        self.applications.extend(other.applications);
        true
    }
}

impl GameEventType for DamagePreventedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::DamagePrevented
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        match self.target {
            DamageTarget::Player(player) => player,
            DamageTarget::Object(object) => game
                .current_controller(object)
                .unwrap_or(self.prevention_controller),
        }
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.prevention_source)
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.damage_source)
    }

    fn player(&self) -> Option<PlayerId> {
        match self.target {
            DamageTarget::Player(player) => Some(player),
            DamageTarget::Object(_) => None,
        }
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.prevention_controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.target_snapshot.as_ref()
    }

    fn display(&self) -> String {
        format!("Prevent {} damage", self.amount)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
