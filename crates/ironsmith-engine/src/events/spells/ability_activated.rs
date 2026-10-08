//! Ability activated event implementation.

use std::any::Any;

use crate::ability::Ability;
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
    /// Identity of the specific activated stack entry, independent of its source object.
    pub stack_entry_provenance: Option<crate::provenance::ProvNodeId>,
    /// Last-known snapshot of the source at activation time.
    pub snapshot: Option<ObjectSnapshot>,
    /// The specific ability that was activated, including continuous-effect grants.
    pub activated_ability: Option<Ability>,
    /// One snapshot per mana unit spent to activate this ability. This is
    /// preserved separately from the ability source so intervening-if clauses
    /// can test mana-source provenance after the source has left play.
    pub mana_sources_spent: Vec<ObjectSnapshot>,
    /// Total mana spent to activate this ability ("by spending four or more
    /// mana to activate it").
    pub mana_spent_total: u32,
    /// Actual acknowledged cost receipts. Legacy builders may supply only the
    /// compatibility projections above; None does not assert zero spending.
    pub mana_spend_evidence: Option<crate::events::mana::ManaSpendEvidence>,
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
            stack_entry_provenance: None,
            snapshot: None,
            activated_ability: None,
            mana_sources_spent: Vec::new(),
            mana_spent_total: 0,
            mana_spend_evidence: None,
        }
    }

    /// Bind the effective acquisition and source captured by the activation
    /// owner. Publication must not rediscover an ability after its costs act.
    pub(crate) fn from_effective_ability(
        source: ObjectId,
        activator: PlayerId,
        is_mana_ability: bool,
        ability: Option<Ability>,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        let (is_loyalty, has_tap) =
            ability
                .as_ref()
                .map_or((false, false), |ability| match &ability.kind {
                    crate::ability::AbilityKind::Activated(activated) => (
                        !is_mana_ability && activated.is_loyalty_ability(),
                        activated.has_tap_cost(),
                    ),
                    _ => (false, false),
                });
        Self::new(source, activator, is_mana_ability)
            .with_loyalty_ability(is_loyalty)
            .with_activation_cost_has_tap(has_tap)
            .with_activated_ability(ability)
            .with_snapshot(snapshot)
    }

    /// Mark whether the activated ability was a loyalty ability.
    pub fn with_loyalty_ability(mut self, is_loyalty_ability: bool) -> Self {
        self.is_loyalty_ability = is_loyalty_ability;
        self
    }

    pub fn with_stack_entry_provenance(
        mut self,
        provenance: Option<crate::provenance::ProvNodeId>,
    ) -> Self {
        self.stack_entry_provenance = provenance;
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

    /// Attach the effective activated ability captured at activation time.
    pub fn with_activated_ability(mut self, ability: Option<Ability>) -> Self {
        self.activated_ability = ability;
        self
    }

    pub fn with_mana_sources_spent(mut self, snapshots: Vec<ObjectSnapshot>) -> Self {
        self.mana_sources_spent = snapshots;
        self
    }

    pub(crate) fn with_mana_spend_evidence(
        mut self,
        evidence: crate::events::mana::ManaSpendEvidence,
    ) -> Result<Self, crate::effects::ExecutionError> {
        let purpose = if self.is_mana_ability {
            crate::ability::ManaPaymentPurpose::ActivateManaAbility
        } else {
            crate::ability::ManaPaymentPurpose::ActivateAbility
        };
        if evidence.payer != self.activator
            || evidence.payment_source != Some(self.source)
            || evidence.purpose != purpose
        {
            return Err(crate::effects::ExecutionError::IncompleteEvidence(
                "activation mana payment evidence belongs to another action".into(),
            ));
        }
        self.mana_spent_total = evidence.total()?;
        self.mana_sources_spent = evidence
            .units
            .iter()
            .filter_map(|unit| unit.source_snapshot.clone())
            .collect();
        self.mana_spend_evidence = Some(evidence);
        Ok(self)
    }

    pub(crate) fn spent_mana_total(&self) -> Result<u32, crate::effects::ExecutionError> {
        match self.mana_spend_evidence.as_ref() {
            Some(evidence) => evidence.total(),
            None => Ok(self.mana_spent_total),
        }
    }

    pub(crate) fn spent_mana_source_matches(
        &self,
        predicate: impl FnMut(&ObjectSnapshot) -> bool,
    ) -> Result<bool, crate::effects::ExecutionError> {
        match self.mana_spend_evidence.as_ref() {
            Some(evidence) => evidence.source_matches(predicate),
            None => Ok(self.mana_sources_spent.iter().any(predicate)),
        }
    }

    pub fn with_mana_spent_total(mut self, total: u32) -> Self {
        self.mana_spent_total = total;
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
