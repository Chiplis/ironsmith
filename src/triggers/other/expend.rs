//! "Whenever [player] expend N" trigger.

use crate::events::EventKind;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ExpendTrigger {
    pub player: PlayerFilter,
    pub amount: u32,
}

impl ExpendTrigger {
    pub fn new(player: PlayerFilter, amount: u32) -> Self {
        Self { player, amount }
    }
}

impl TriggerMatcher for ExpendTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::KeywordAction {
            return false;
        }
        let Some(e) = event.downcast::<KeywordActionEvent>() else {
            return false;
        };
        if e.action != KeywordActionKind::Expend || e.amount != self.amount {
            return false;
        }

        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        }
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => format!("Whenever you expend {}", self.amount),
            PlayerFilter::Opponent => format!("Whenever an opponent expends {}", self.amount),
            PlayerFilter::Any => format!("Whenever a player expends {}", self.amount),
            _ => format!("Whenever a player expends {}", self.amount),
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::other::KeywordActionEvent;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn matches_when_player_expend_amount_matches() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = ExpendTrigger::new(PlayerFilter::You, 4);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new(KeywordActionEvent::new(
            KeywordActionKind::Expend,
            alice,
            ObjectId::from_raw(99),
            4,
        ));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_wrong_amount() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = ExpendTrigger::new(PlayerFilter::You, 4);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new(KeywordActionEvent::new(
            KeywordActionKind::Expend,
            alice,
            ObjectId::from_raw(99),
            8,
        ));

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn display_includes_amount() {
        let trigger = ExpendTrigger::new(PlayerFilter::You, 4);
        assert_eq!(trigger.display(), "Whenever you expend 4");
    }
}
