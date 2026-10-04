use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::PlayerId;
use std::any::Any;

/// One unique attacking-player/directly-attacked-player pair in a declaration.
/// Planeswalkers, battles, and creatures put onto the battlefield attacking do
/// not produce this event (CR 508.3b/e).
#[derive(Debug, Clone)]
pub struct PlayerAttackDeclarationEvent {
    pub attacker: PlayerId,
    pub defender: PlayerId,
    pub turn_number: u32,
    pub combat_phase: u32,
}
impl GameEventType for PlayerAttackDeclarationEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PlayerAttackDeclaration
    }
    fn affected_player(&self, _: &GameState) -> PlayerId {
        self.defender
    }
    fn with_target_replaced(&self, _: &Target, _: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }
    fn display(&self) -> String {
        "A player attacks another player".into()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn player(&self) -> Option<PlayerId> {
        Some(self.defender)
    }
    fn controller(&self) -> Option<PlayerId> {
        Some(self.attacker)
    }
}
