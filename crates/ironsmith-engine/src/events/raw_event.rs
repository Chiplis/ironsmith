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
    /// Receipt proof: both ordinary and delayed triggers were matched before
    /// a later instruction. Keep the physical event for quantities/history,
    /// but never discover those triggers again when the receipt is published.
    triggers_captured: bool,
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
    source_snapshot: Option<ObjectSnapshot>,
    lookback_source_snapshots: Vec<ObjectSnapshot>,
    /// Contextual player bindings carried across delayed-trigger boundaries.
    player_tags: HashMap<TagKey, Vec<PlayerId>>,
}

impl RawEvent {
    pub fn new<E: GameEventType + 'static>(event: E, provenance: ProvNodeId) -> Self {
        Self {
            inner: Arc::new(event),
            occurrence: Arc::new(()),
            provenance,
            triggers_captured: false,
            completed_action_provenance: None,
            simultaneous_batch: None,
            source_snapshot: None,
            lookback_source_snapshots: Vec::new(),
            player_tags: HashMap::new(),
        }
    }

    pub fn from_boxed(event: Box<dyn GameEventType>, provenance: ProvNodeId) -> Self {
        Self {
            inner: Arc::from(event),
            occurrence: Arc::new(()),
            provenance,
            triggers_captured: false,
            completed_action_provenance: None,
            simultaneous_batch: None,
            source_snapshot: None,
            lookback_source_snapshots: Vec::new(),
            player_tags: HashMap::new(),
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
        self.triggers_captured
    }

    /// Only the matching boundary may assert this receipt proof.
    pub(crate) fn mark_triggers_captured(&mut self) {
        self.triggers_captured = true;
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
        event.triggers_captured |= completed.triggers_captured;
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
            triggers_captured: self.triggers_captured,
            completed_action_provenance: self.completed_action_provenance.clone(),
            simultaneous_batch: self.simultaneous_batch,
            source_snapshot: self.source_snapshot.clone(),
            lookback_source_snapshots: self.lookback_source_snapshots.clone(),
            player_tags: self.player_tags.clone(),
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
            .field("triggers_captured", &self.triggers_captured)
            .field(
                "completed_action_provenance",
                &self.completed_action_provenance,
            )
            .field("simultaneous_batch", &self.simultaneous_batch)
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
