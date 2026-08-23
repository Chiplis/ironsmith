use std::any::Any;

use crate::events::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

#[derive(Debug, Clone)]
pub struct CoinFlippedEvent {
    pub player: PlayerId,
    pub source: ObjectId,
    pub face: ironsmith_core::CoinFace,
    pub call: Option<ironsmith_core::CoinFace>,
    pub winner: Option<PlayerId>,
    pub loser: Option<PlayerId>,
}

impl CoinFlippedEvent {
    pub fn flipper_won(&self) -> bool {
        self.winner == Some(self.player)
    }

    pub fn flipper_lost(&self) -> bool {
        self.loser == Some(self.player)
    }
}

impl GameEventType for CoinFlippedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CoinFlipped
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn display(&self) -> String {
        let face = match self.face {
            ironsmith_core::CoinFace::Heads => "heads",
            ironsmith_core::CoinFace::Tails => "tails",
        };
        format!("Player {:?} flipped {face}", self.player)
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }
}
