//! "Whenever [player] gives a gift" trigger.

use crate::events::EventKind;
use crate::events::other::GiftGivenEvent;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerGivesGiftTrigger {
    pub player: PlayerFilter,
}

impl PlayerGivesGiftTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for PlayerGivesGiftTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::GiftGiven {
            return false;
        }
        let Some(e) = event.downcast::<GiftGivenEvent>() else {
            return false;
        };

        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Active => e.player == ctx.game.turn.active_player,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        }
    }

    fn display(&self) -> String {
        let player_text = match &self.player {
            PlayerFilter::You => "you give a gift",
            PlayerFilter::Opponent => "an opponent gives a gift",
            PlayerFilter::Any => "a player gives a gift",
            PlayerFilter::Active => "the active player gives a gift",
            _ => "someone gives a gift",
        };
        format!("Whenever {player_text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn opponent_gives_gift_trigger_matches_opponent() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let trigger = PlayerGivesGiftTrigger::new(PlayerFilter::Opponent);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);
        let event = TriggerEvent::new_with_provenance(
            GiftGivenEvent::new(bob, alice, ObjectId::from_raw(9)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn opponent_gives_gift_trigger_does_not_match_controller() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let trigger = PlayerGivesGiftTrigger::new(PlayerFilter::Opponent);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);
        let event = TriggerEvent::new_with_provenance(
            GiftGivenEvent::new(alice, bob, ObjectId::from_raw(9)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn player_gives_gift_trigger_display_uses_oracle_wording() {
        let trigger = PlayerGivesGiftTrigger::new(PlayerFilter::Opponent);

        assert_eq!(trigger.display(), "Whenever an opponent gives a gift");
    }
}
