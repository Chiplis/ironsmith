//! "Whenever this creature attacks" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when the source creature attacks.
///
/// Used by cards like Goblin Guide, Geist of Saint Traft, and Hero of Bladehold.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksTrigger;

impl TriggerMatcher for ThisAttacksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        e.attacker == ctx.source_id
    }

    fn display(&self) -> String {
        "Whenever this creature attacks".to_string()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_matches_own_attack() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisAttacksTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            source_id,
            AttackEventTarget::Player(bob),
        ));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_other_attack() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let other_id = ObjectId::from_raw(2);

        let trigger = ThisAttacksTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            other_id,
            AttackEventTarget::Player(bob),
        ));

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = ThisAttacksTrigger;
        assert!(trigger.display().contains("attacks"));
    }
}
