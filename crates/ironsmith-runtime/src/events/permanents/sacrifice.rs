//! Sacrifice event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A sacrifice event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct SacrificeEvent {
    /// The permanent being sacrificed
    pub permanent: ObjectId,
    /// The source requiring the sacrifice
    pub source: Option<ObjectId>,
    /// Last-known snapshot of the sacrificed permanent.
    pub snapshot: Option<ObjectSnapshot>,
    /// The player who sacrificed the permanent.
    pub sacrificing_player: Option<PlayerId>,
}

impl SacrificeEvent {
    /// Create a new sacrifice event.
    pub fn new(permanent: ObjectId, source: Option<ObjectId>) -> Self {
        Self {
            permanent,
            source,
            snapshot: None,
            sacrificing_player: None,
        }
    }

    /// Create a sacrifice event from a specific source.
    pub fn from_source(permanent: ObjectId, source: ObjectId) -> Self {
        Self {
            permanent,
            source: Some(source),
            snapshot: None,
            sacrificing_player: None,
        }
    }

    /// Return a new event with a different permanent.
    pub fn with_permanent(&self, permanent: ObjectId) -> Self {
        Self {
            permanent,
            source: self.source,
            snapshot: self.snapshot.clone(),
            sacrificing_player: self.sacrificing_player,
        }
    }

    /// Attach LKI snapshot and explicit sacrificing player.
    pub fn with_snapshot(
        mut self,
        snapshot: Option<ObjectSnapshot>,
        sacrificing_player: Option<PlayerId>,
    ) -> Self {
        self.snapshot = snapshot;
        self.sacrificing_player = sacrificing_player;
        self
    }
}

impl GameEventType for SacrificeEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Sacrifice
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        if let Some(player) = self.sacrificing_player {
            return player;
        }
        if let Some(snapshot) = self.snapshot.as_ref() {
            return snapshot.controller;
        }
        game.object(self.permanent)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        // Sacrifice typically can't be redirected (you sacrifice your own stuff)
        // but we include it for completeness - validation will reject invalid redirects
        vec![RedirectableTarget {
            target: Target::Object(self.permanent),
            description: "sacrifice target",
            valid_redirect_types: RedirectValidTypes::ObjectsOnly,
        }]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &Target::Object(self.permanent) != old {
            return None;
        }

        if let Target::Object(new_obj) = new {
            Some(Box::new(self.with_permanent(*new_obj)))
        } else {
            None
        }
    }

    fn source_object(&self) -> Option<ObjectId> {
        self.source
    }

    fn display(&self) -> String {
        "Sacrifice permanent".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }

    fn player(&self) -> Option<PlayerId> {
        self.sacrificing_player
            .or_else(|| self.snapshot.as_ref().map(|s| s.controller))
    }

    fn controller(&self) -> Option<PlayerId> {
        self.player()
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sacrifice_event_creation() {
        let event = SacrificeEvent::new(ObjectId::from_raw(1), Some(ObjectId::from_raw(2)));

        assert_eq!(event.permanent, ObjectId::from_raw(1));
        assert_eq!(event.source, Some(ObjectId::from_raw(2)));
    }

    #[test]
    fn test_sacrifice_event_kind() {
        let event = SacrificeEvent::new(ObjectId::from_raw(1), None);
        assert_eq!(event.event_kind(), EventKind::Sacrifice);
    }

    #[test]
    fn test_sacrifice_event_display() {
        let event = SacrificeEvent::new(ObjectId::from_raw(1), None);
        assert_eq!(event.display(), "Sacrifice permanent");
    }
}
