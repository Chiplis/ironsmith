//! "Whenever this creature attacks while saddled" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when the source creature attacks while saddled.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksWhileSaddledTrigger;

impl TriggerMatcher for ThisAttacksWhileSaddledTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        e.attacker == ctx.source_id && ctx.game.is_saddled(ctx.source_id)
    }

    fn display(&self) -> String {
        "Whenever this creature attacks while saddled".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn matches_only_when_saddled() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisAttacksWhileSaddledTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(source_id, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));

        game.set_saddled_until_end_of_turn(source_id);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }
}
