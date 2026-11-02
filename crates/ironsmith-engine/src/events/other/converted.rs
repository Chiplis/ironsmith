//! Permanent converted event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A permanent converted.
#[derive(Debug, Clone)]
pub struct ConvertedEvent {
    /// The permanent that converted.
    pub permanent: ObjectId,
    /// Completed post-change characteristics of this exact incarnation.
    pub snapshot: Option<ObjectSnapshot>,
    /// A complete producer packet freezes all event-time trigger sources.
    pub complete_source_lookback: bool,
}

impl ConvertedEvent {
    /// Create a new converted event.
    pub fn new(permanent: ObjectId) -> Self {
        Self {
            permanent,
            snapshot: None,
            complete_source_lookback: false,
        }
    }
    pub fn with_complete_source_lookback(mut self) -> Self {
        self.complete_source_lookback = true;
        self
    }
    pub fn with_snapshot(mut self, snapshot: Option<ObjectSnapshot>) -> Self {
        self.snapshot = snapshot;
        self
    }
}

impl GameEventType for ConvertedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Converted
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        self.snapshot
            .as_ref()
            .map(|snapshot| snapshot.controller)
            .or_else(|| game.object(self.permanent).map(|o| game.controller_of(o)))
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent converted".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn controller(&self) -> Option<PlayerId> {
        self.snapshot.as_ref().map(|snapshot| snapshot.controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }
}
