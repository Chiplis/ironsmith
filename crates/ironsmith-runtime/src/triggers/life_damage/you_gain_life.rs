//! "Whenever you gain life" trigger.

use crate::events::EventKind;
use crate::events::life::LifeGainEvent;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct YouGainLifeTrigger {
    pub during_turn: Option<PlayerFilter>,
}

impl YouGainLifeTrigger {
    pub fn new() -> Self {
        Self { during_turn: None }
    }

    pub fn during_turn(during_turn: PlayerFilter) -> Self {
        Self {
            during_turn: Some(during_turn),
        }
    }
}

impl TriggerMatcher for YouGainLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::LifeGain {
            return false;
        }
        let Some(e) = event.downcast::<LifeGainEvent>() else {
            return false;
        };
        if e.player != ctx.controller {
            return false;
        }
        if let Some(during_turn) = &self.during_turn {
            let active_player = ctx.game.turn.active_player;
            return match during_turn {
                PlayerFilter::You => active_player == ctx.controller,
                PlayerFilter::Opponent => active_player != ctx.controller,
                PlayerFilter::Any | PlayerFilter::Active => true,
                PlayerFilter::Specific(id) => active_player == *id,
                _ => true,
            };
        }
        true
    }

    fn display(&self) -> String {
        if let Some(during_turn) = &self.during_turn {
            let suffix = match during_turn {
                PlayerFilter::You => " during your turn",
                PlayerFilter::Opponent => " during an opponent's turn",
                PlayerFilter::Specific(_) => " during that player's turn",
                _ => "",
            };
            format!("Whenever you gain life{suffix}")
        } else {
            "Whenever you gain life".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn test_matches_own_life_gain() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = YouGainLifeTrigger::new();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            LifeGainEvent::new(alice, 3),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_opponent_life_gain() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = YouGainLifeTrigger::new();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            LifeGainEvent::new(bob, 3),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }
}
