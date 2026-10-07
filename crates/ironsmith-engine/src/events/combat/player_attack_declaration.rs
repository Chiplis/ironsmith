use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::PlayerId;
use std::any::Any;

/// Immutable participants in a completed attack declaration. The controller
/// of an attacked planeswalker is captured before any attack trigger resolves.
#[derive(Debug, Clone)]
pub struct DeclaredAttackParticipant {
    pub creature: crate::ids::ObjectId,
    pub controller: PlayerId,
    pub target: crate::triggers::AttackEventTarget,
    pub defending_player: PlayerId,
}

/// One attacking-player/defending-player pair and target category in a
/// declaration. Matchers for directly attacking players exclude non-direct
/// pairs; the unqualified "a player attacks" also observes the latter.
/// Creatures put onto the battlefield attacking never produce this event.
#[derive(Debug, Clone)]
pub struct PlayerAttackDeclarationEvent {
    pub attacker: PlayerId,
    pub defender: PlayerId,
    pub turn_number: u32,
    pub combat_phase: u32,
    pub directly_attacked_player: bool,
    /// Complete declaration, including all targets of this declaring player.
    /// None is unavailable evidence, never an empty declaration.
    pub declaration: Option<std::sync::Arc<[DeclaredAttackParticipant]>>,
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
        if self.directly_attacked_player {
            "A player attacks another player".into()
        } else {
            "A player declares attackers against a planeswalker or Battle".into()
        }
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
