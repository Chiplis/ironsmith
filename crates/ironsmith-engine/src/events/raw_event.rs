use std::collections::HashMap;
use std::sync::Arc;

use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

use super::{EventKind, GameEventType};

/// Shared event envelope used by both replacement and trigger pipelines.
#[derive(Clone)]
pub struct RawEvent {
    inner: Arc<dyn GameEventType>,
    /// Identity belongs to the occurrence, not its current observation payload.
    /// Snapshot enrichment can replace `inner` while retained aliases still
    /// refer to this same occurrence.
    occurrence: Arc<()>,
    provenance: ProvNodeId,
    /// Branch-local proof of the requested observer families. Ordinary-only
    /// matching must not suppress a later request for delayed observers.
    ordinary_triggers_captured: bool,
    delayed_triggers_captured: bool,
    /// Proof owned by the completion boundary, copied by value across branches.
    /// A clone can carry the completed receipt without sharing mutable state
    /// with a checkpoint or an alias that predates completion.
    completed_action_provenance: Option<(ProvNodeId, Arc<()>)>,
    /// Identity shared by events produced by one simultaneous game action.
    ///
    /// This is presentation-neutral rules metadata used by grouped triggers
    /// such as "one or more creatures". It must survive until the pending
    /// trigger queue is drained so those events can be checked as one batch.
    simultaneous_batch: Option<ProvNodeId>,
    /// Only a queued singular-recipient counter trigger owns this projection.
    /// Keep physical per-kind receipts intact for matching and history. The
    /// legacy generic trigger scalar is i32; counter amounts must stay wide.
    counter_trigger_amount: Option<i64>,
    source_snapshot: Option<ObjectSnapshot>,
    lookback_source_snapshots: Vec<ObjectSnapshot>,
    /// Contextual player bindings carried across delayed-trigger boundaries.
    player_tags: HashMap<TagKey, Vec<PlayerId>>,
    defending_player_reference: Option<crate::combat_state::DefendingPlayerReference>,
}

impl RawEvent {
    /// Physical departure evidence for this exact incarnation. Contextual
    /// source decorations are not departure receipts; phasing is not departure.
    pub(crate) fn source_departure_snapshot(&self, object: ObjectId) -> Option<&ObjectSnapshot> {
        if let Some(event) = self.downcast::<super::zones::ObjectLeavesGameEvent>() {
            return (event.object == object).then_some(&event.snapshot);
        }
        self.downcast::<super::zones::ZoneChangeEvent>()?
            .snapshots()
            .iter()
            .find(|snapshot| snapshot.object_id == object)
    }

    /// Exact physical source evidence, including characteristics at phasing.
    pub(crate) fn source_last_known_snapshot(&self, object: ObjectId) -> Option<&ObjectSnapshot> {
        if let Some(event) = self.downcast::<super::PermanentPhasedOutEvent>() {
            return (event.permanent == object)
                .then_some(event.snapshot.as_ref())
                .flatten();
        }
        self.source_departure_snapshot(object)
    }

    pub fn new<E: GameEventType + 'static>(event: E, provenance: ProvNodeId) -> Self {
        Self {
            inner: Arc::new(event),
            occurrence: Arc::new(()),
            provenance,
            ordinary_triggers_captured: false,
            delayed_triggers_captured: false,
            completed_action_provenance: None,
            simultaneous_batch: None,
            counter_trigger_amount: None,
            source_snapshot: None,
            lookback_source_snapshots: Vec::new(),
            player_tags: HashMap::new(),
            defending_player_reference: None,
        }
    }

    pub fn from_boxed(event: Box<dyn GameEventType>, provenance: ProvNodeId) -> Self {
        Self {
            inner: Arc::from(event),
            occurrence: Arc::new(()),
            provenance,
            ordinary_triggers_captured: false,
            delayed_triggers_captured: false,
            completed_action_provenance: None,
            simultaneous_batch: None,
            counter_trigger_amount: None,
            source_snapshot: None,
            lookback_source_snapshots: Vec::new(),
            player_tags: HashMap::new(),
            defending_player_reference: None,
        }
    }

    /// Compatibility helper while migrating old trigger event constructors.
    pub fn new_with_provenance<E: GameEventType + 'static>(
        event: E,
        provenance: ProvNodeId,
    ) -> Self {
        Self::new(event, provenance)
    }

    /// Compatibility helper while migrating old trigger event constructors.
    pub fn from_boxed_with_provenance(
        event: Box<dyn GameEventType>,
        provenance: ProvNodeId,
    ) -> Self {
        Self::from_boxed(event, provenance)
    }

    #[inline]
    pub fn kind(&self) -> EventKind {
        self.inner.event_kind()
    }

    #[inline]
    pub fn inner(&self) -> &dyn GameEventType {
        &*self.inner
    }

    /// Attempt to downcast to a concrete event type.
    pub fn downcast<T: 'static>(&self) -> Option<&T> {
        self.inner().as_any().downcast_ref::<T>()
    }

    /// Get the primary object ID involved in this event, if any.
    pub fn object_id(&self) -> Option<ObjectId> {
        self.inner().object_id()
    }

    /// Get the player involved in this event, if any.
    pub fn player(&self) -> Option<PlayerId> {
        self.inner().player()
    }

    /// Get the player that triggered abilities should treat as "that player".
    pub fn trigger_player(&self) -> Option<PlayerId> {
        self.inner().trigger_player()
    }

    /// Get the controller involved in this event, if any.
    pub fn controller(&self) -> Option<PlayerId> {
        self.inner().controller()
    }

    /// Get the source object for this event, if any.
    pub fn source_object(&self) -> Option<ObjectId> {
        self.inner().source_object()
    }

    pub fn cause(&self) -> Option<&crate::events::cause::EventCause> {
        self.inner().cause()
    }

    /// Get snapshot/LKI payload if present.
    pub fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.inner().snapshot()
    }

    /// Get last-known information for the event source, if the event source has
    /// left the public zone it was expected to be in.
    pub fn source_snapshot(&self) -> Option<&ObjectSnapshot> {
        self.source_snapshot.as_ref()
    }

    /// Get pre-event snapshots of objects that could have been trigger sources
    /// for CR 603.10 look-back source discovery.
    pub fn lookback_source_snapshots(&self) -> &[ObjectSnapshot] {
        &self.lookback_source_snapshots
    }

    /// Player bindings captured by a delayed-trigger registration.
    pub fn player_tags(&self) -> &HashMap<TagKey, Vec<PlayerId>> {
        &self.player_tags
    }

    pub fn defending_player_reference(
        &self,
    ) -> Option<crate::combat_state::DefendingPlayerReference> {
        self.defending_player_reference
    }
    #[must_use]
    pub fn with_defending_player_reference(
        mut self,
        reference: crate::combat_state::DefendingPlayerReference,
    ) -> Self {
        self.defending_player_reference = Some(reference);
        self
    }

    /// Human-readable event description.
    pub fn display(&self) -> String {
        self.inner().display()
    }

    #[inline]
    pub fn provenance(&self) -> ProvNodeId {
        self.provenance
    }

    #[inline]
    pub fn set_provenance(&mut self, provenance: ProvNodeId) {
        self.provenance = provenance;
    }

    /// Identity shared by clones and enriched observations of this event,
    /// and by no equal-looking separate occurrence while this one is alive.
    #[inline]
    pub(crate) fn occurrence_key(&self) -> usize {
        Arc::as_ptr(&self.occurrence) as usize
    }

    pub(crate) fn triggers_captured(&self) -> bool {
        self.ordinary_triggers_captured && self.delayed_triggers_captured
    }

    pub(crate) fn ordinary_triggers_captured(&self) -> bool {
        self.ordinary_triggers_captured
    }
    pub(crate) fn delayed_triggers_captured(&self) -> bool {
        self.delayed_triggers_captured
    }
    pub(crate) fn mark_ordinary_triggers_captured(&mut self) {
        self.ordinary_triggers_captured = true;
    }
    pub(crate) fn mark_delayed_triggers_captured(&mut self) {
        self.delayed_triggers_captured = true;
    }

    /// Enrich an alias of this exact occurrence without sharing mutable proof
    /// across speculative branches or changing its payload/provenance.
    pub(crate) fn inherit_trigger_capture(&mut self, receipt: &Self) {
        if self.ptr_eq(receipt) {
            self.ordinary_triggers_captured |= receipt.ordinary_triggers_captured;
            self.delayed_triggers_captured |= receipt.delayed_triggers_captured;
        }
    }

    /// Only the matching boundary may assert this receipt proof.
    pub(crate) fn mark_triggers_captured(&mut self) {
        self.mark_ordinary_triggers_captured();
        self.mark_delayed_triggers_captured();
    }

    /// An ordinary matching receipt owns its normalized physical observation.
    /// Preserve a newly completed semantic receipt over an older raw row, and
    /// preserve this wrapper's contextual bindings through the shared restorer.
    pub(crate) fn with_retained_trigger_capture(&self, receipt: &Self) -> Self {
        if !self.ptr_eq(receipt) {
            return self.clone();
        }
        if receipt.ordinary_triggers_captured()
            && (receipt.completed_action_provenance().is_some()
                || self.completed_action_provenance().is_none())
        {
            self.with_completed_action_receipt(receipt)
        } else {
            let mut event = self.clone();
            event.inherit_trigger_capture(receipt);
            event
        }
    }

    pub(crate) fn completed_action_provenance(&self) -> Option<ProvNodeId> {
        self.completed_action_provenance.as_ref().map(|(id, _)| *id)
    }

    /// Only the completion owner may assert this after freezing the occurrence.
    pub(crate) fn mark_action_completed(&mut self, node: &crate::provenance::ProvenanceNode) {
        debug_assert_eq!(self.provenance, node.id);
        self.completed_action_provenance = Some((node.id, node.completion_witness()));
    }

    pub(crate) fn action_completion_matches_node(
        &self,
        node: &crate::provenance::ProvenanceNode,
    ) -> bool {
        self.completed_action_provenance
            .as_ref()
            .is_some_and(|(id, witness)| {
                *id == node.id && Arc::ptr_eq(witness, &node.completion_witness())
            })
    }

    /// Restore the semantic receipt while retaining this wrapper's contextual
    /// decorations and trigger-capture proof. The caller verifies identity.
    pub(crate) fn with_completed_action_receipt(&self, completed: &Self) -> Self {
        let mut event = self.clone();
        event.inner = completed.inner.clone();
        event.provenance = completed.provenance;
        event.completed_action_provenance = completed.completed_action_provenance.clone();
        event.simultaneous_batch = completed.simultaneous_batch;
        event.inherit_trigger_capture(completed);
        if event.source_snapshot.is_none() {
            event.source_snapshot = completed.source_snapshot.clone();
        }
        for snapshot in &completed.lookback_source_snapshots {
            if !event
                .lookback_source_snapshots
                .iter()
                .any(|previous| previous.object_id == snapshot.object_id)
            {
                event.lookback_source_snapshots.push(snapshot.clone());
            }
        }
        event
    }

    /// Return the simultaneous-action identity attached to this event.
    #[inline]
    pub fn simultaneous_batch(&self) -> Option<ProvNodeId> {
        self.simultaneous_batch
    }

    pub(crate) fn counter_trigger_amount(&self) -> Result<i64, crate::effects::ExecutionError> {
        self.counter_trigger_amount.map(Ok).unwrap_or_else(|| {
            self.downcast::<super::MarkersChangedEvent>()
                .map(|event| i64::from(event.amount))
                .ok_or_else(|| {
                    crate::effects::ExecutionError::IncompleteEvidence(
                        "counter trigger group lost its placement receipt".into(),
                    )
                })
        })
    }

    /// Queue ownership only: preserve the event payload, occurrence identity,
    /// actor and snapshots while retaining the checked sum for resolution.
    /// An unrepresentable total remains a typed failure, never a first-event
    /// fallback, wrapping value, or saturated successful amount.
    pub(crate) fn accumulate_counter_trigger_amount(
        &mut self,
        next: &Self,
    ) -> Result<(), crate::effects::ExecutionError> {
        let prior = self.counter_trigger_amount()?;
        let next = next.counter_trigger_amount()?;
        let amount = prior.checked_add(next).ok_or_else(|| {
            crate::effects::ExecutionError::ResourceLimitExceeded {
                resource: "counter trigger group amount",
                requested: prior as u128 + next as u128,
                maximum: i64::MAX as u128,
            }
        })?;
        // Check before changing even the private staged entry. Queue owners
        // publish nothing until the complete group's checked projection exists.
        self.counter_trigger_amount = Some(amount);
        Ok(())
    }

    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.provenance = provenance;
        self
    }

    /// Mark this event as part of one simultaneous game action.
    #[must_use]
    pub fn with_simultaneous_batch(mut self, batch: ProvNodeId) -> Self {
        self.simultaneous_batch = Some(batch);
        self
    }

    #[must_use]
    pub fn with_source_snapshot(mut self, snapshot: ObjectSnapshot) -> Self {
        self.source_snapshot = Some(snapshot);
        self
    }

    #[must_use]
    pub fn with_lookback_source_snapshots(mut self, snapshots: Vec<ObjectSnapshot>) -> Self {
        self.lookback_source_snapshots = snapshots;
        self
    }

    #[must_use]
    pub fn with_player_tags(mut self, player_tags: HashMap<TagKey, Vec<PlayerId>>) -> Self {
        self.player_tags = player_tags;
        self
    }

    /// The same occurrence metadata (provenance, simultaneous batch, source
    /// and look-back snapshots, player bindings) around a different payload.
    #[must_use]
    pub(crate) fn with_inner_event<E: GameEventType + 'static>(&self, event: E) -> Self {
        Self {
            inner: Arc::new(event),
            occurrence: self.occurrence.clone(),
            provenance: self.provenance,
            ordinary_triggers_captured: self.ordinary_triggers_captured,
            delayed_triggers_captured: self.delayed_triggers_captured,
            completed_action_provenance: self.completed_action_provenance.clone(),
            simultaneous_batch: self.simultaneous_batch,
            counter_trigger_amount: self.counter_trigger_amount,
            source_snapshot: self.source_snapshot.clone(),
            lookback_source_snapshots: self.lookback_source_snapshots.clone(),
            player_tags: self.player_tags.clone(),
            defending_player_reference: self.defending_player_reference,
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.occurrence, &other.occurrence)
    }
}

impl std::fmt::Debug for RawEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawEvent")
            .field("kind", &self.kind())
            .field("provenance", &self.provenance)
            .field(
                "ordinary_triggers_captured",
                &self.ordinary_triggers_captured,
            )
            .field("delayed_triggers_captured", &self.delayed_triggers_captured)
            .field(
                "completed_action_provenance",
                &self.completed_action_provenance,
            )
            .field("simultaneous_batch", &self.simultaneous_batch)
            .field(
                "defending_player_reference",
                &self.defending_player_reference,
            )
            .field("source_snapshot", &self.source_snapshot)
            .field("lookback_source_snapshots", &self.lookback_source_snapshots)
            .field("player_tags", &self.player_tags)
            .field("display", &self.inner().display())
            .finish()
    }
}

impl PartialEq for RawEvent {
    fn eq(&self, other: &Self) -> bool {
        if self.provenance == other.provenance && self.ptr_eq(other) {
            return true;
        }
        self.kind() == other.kind()
            && self.object_id() == other.object_id()
            && self.provenance == other.provenance
    }
}

impl Eq for RawEvent {}

#[cfg(test)]
mod counter_group_amount_tests {
    use super::*;

    #[test]
    fn checked_counter_group_failure_leaves_prior_projection_and_physical_receipt_intact() {
        let mut receipt = RawEvent::new(super::super::MarkersChangedEvent::added(
            crate::CounterType::Charge, ObjectId::from_raw(1), 1, None, None,
        ), ProvNodeId::default());
        for _ in 0..62 {
            receipt.accumulate_counter_trigger_amount(&receipt.clone()).unwrap();
        }
        let prior = receipt.clone();
        assert_eq!(receipt.counter_trigger_amount(), Ok(1i64 << 62));
        assert_eq!(receipt.accumulate_counter_trigger_amount(&prior),
            Err(crate::effects::ExecutionError::ResourceLimitExceeded {
                resource: "counter trigger group amount",
                requested: 1u128 << 63,
                maximum: i64::MAX as u128,
            }));
        assert_eq!(receipt.counter_trigger_amount(), prior.counter_trigger_amount());
        assert_eq!(receipt.downcast::<super::super::MarkersChangedEvent>().unwrap().amount, 1);
        assert!(receipt.ptr_eq(&prior));
        let enriched = receipt.with_inner_event(
            receipt.downcast::<super::super::MarkersChangedEvent>().unwrap().clone().with_count_after(7),
        );
        assert_eq!(enriched.counter_trigger_amount(), prior.counter_trigger_amount());
        let restored = enriched.with_completed_action_receipt(&prior);
        assert_eq!(restored.counter_trigger_amount(), prior.counter_trigger_amount());
        assert!(restored.ptr_eq(&prior));
    }
}
