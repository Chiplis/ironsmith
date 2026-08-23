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
    provenance: ProvNodeId,
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
            provenance,
            simultaneous_batch: None,
            source_snapshot: None,
            lookback_source_snapshots: Vec::new(),
            player_tags: HashMap::new(),
        }
    }

    pub fn from_boxed(event: Box<dyn GameEventType>, provenance: ProvNodeId) -> Self {
        Self {
            inner: Arc::from(event),
            provenance,
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

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl std::fmt::Debug for RawEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawEvent")
            .field("kind", &self.kind())
            .field("provenance", &self.provenance)
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
