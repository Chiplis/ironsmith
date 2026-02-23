//! "Whenever this creature attacks the player with the most life or tied for most life" trigger.

use crate::events::EventKind;
use crate::events::combat::{AttackEventTarget, CreatureAttackedEvent};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when the source creature attacks a player with the most life
/// (including ties).
#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksPlayerWithMostLifeTrigger;

impl TriggerMatcher for ThisAttacksPlayerWithMostLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        if e.attacker != ctx.source_id {
            return false;
        }

        let AttackEventTarget::Player(defending_player) = e.target else {
            return false;
        };

        let defending_life = ctx
            .game
            .player(defending_player)
            .map(|p| p.life)
            .unwrap_or(i32::MIN);

        let max_life = ctx.game.players.iter().map(|player| player.life).max();
        max_life.is_some_and(|max_life| defending_life == max_life)
    }

    fn display(&self) -> String {
        "Whenever this creature attacks the player with the most life or tied for most life"
            .to_string()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn setup_game() -> GameState {
        GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        )
    }

    #[test]
    fn matches_when_attacking_player_with_most_life() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(1);
        game.player_mut(bob).expect("bob").life = 23;

        let trigger = ThisAttacksPlayerWithMostLifeTrigger;
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            source,
            AttackEventTarget::Player(bob),
        ));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn matches_when_attacking_player_tied_for_most_life() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source = ObjectId::from_raw(1);
        game.player_mut(bob).expect("bob").life = 25;
        game.player_mut(cara).expect("cara").life = 25;

        let trigger = ThisAttacksPlayerWithMostLifeTrigger;
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            source,
            AttackEventTarget::Player(cara),
        ));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_attacking_lower_life_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source = ObjectId::from_raw(1);
        game.player_mut(bob).expect("bob").life = 18;
        game.player_mut(cara).expect("cara").life = 27;

        let trigger = ThisAttacksPlayerWithMostLifeTrigger;
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            source,
            AttackEventTarget::Player(bob),
        ));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_attacking_planeswalker() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        let planeswalker = ObjectId::from_raw(99);

        let trigger = ThisAttacksPlayerWithMostLifeTrigger;
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            source,
            AttackEventTarget::Planeswalker(planeswalker),
        ));
        assert!(!trigger.matches(&event, &ctx));
    }
}
