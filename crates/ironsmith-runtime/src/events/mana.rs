//! Mana-added event implementation.

use std::any::Any;

use crate::events::raw_event::RawEvent;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::snapshot::ObjectSnapshot;

/// Mana was added to a player's mana pool.
#[derive(Debug, Clone)]
pub struct ManaAddedEvent {
    /// Object whose ability or effect added the mana.
    pub source: ObjectId,
    /// Controller of the source ability or effect.
    pub controller: PlayerId,
    /// Player who received the mana.
    pub player: PlayerId,
    /// Mana symbols added by this event.
    pub mana: Vec<ManaSymbol>,
    /// Last-known snapshot of the source when the mana was added.
    pub snapshot: Option<ObjectSnapshot>,
}

impl ManaAddedEvent {
    pub fn new(
        source: ObjectId,
        controller: PlayerId,
        player: PlayerId,
        mana: Vec<ManaSymbol>,
    ) -> Self {
        Self {
            source,
            controller,
            player,
            mana,
            snapshot: None,
        }
    }

    pub fn with_snapshot(mut self, snapshot: Option<ObjectSnapshot>) -> Self {
        self.snapshot = snapshot;
        self
    }

    pub fn into_trigger_event(self) -> RawEvent {
        RawEvent::new_with_provenance(self, crate::provenance::ProvNodeId::default())
    }

    pub fn trigger_event(
        source: ObjectId,
        controller: PlayerId,
        player: PlayerId,
        mana: Vec<ManaSymbol>,
    ) -> RawEvent {
        Self::new(source, controller, player, mana).into_trigger_event()
    }
}

impl GameEventType for ManaAddedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ManaAdded
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }

    fn display(&self) -> String {
        "Mana added".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
