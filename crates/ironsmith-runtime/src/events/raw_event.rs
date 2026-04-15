use std::sync::Arc;

use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;

use super::{EventKind, GameEventType};

/// Shared event envelope used by both replacement and trigger pipelines.
#[derive(Clone)]
pub struct RawEvent {
    inner: Arc<dyn GameEventType>,
    provenance: ProvNodeId,
}

impl RawEvent {
    pub fn new<E: GameEventType + 'static>(event: E, provenance: ProvNodeId) -> Self {
        Self {
            inner: Arc::new(event),
            provenance,
        }
    }

    pub fn from_boxed(event: Box<dyn GameEventType>, provenance: ProvNodeId) -> Self {
        Self {
            inner: Arc::from(event),
            provenance,
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

    /// Get snapshot/LKI payload if present.
    pub fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.inner().snapshot()
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

    #[must_use]
    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.provenance = provenance;
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
