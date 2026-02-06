//! "Whenever you lose life" trigger.

use crate::events::EventKind;
use crate::events::life::LifeLossEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct YouLoseLifeTrigger;

impl TriggerMatcher for YouLoseLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::LifeLoss {
            return false;
        }
        let Some(e) = event.downcast::<LifeLossEvent>() else {
            return false;
        };
        e.player == ctx.controller
    }

    fn display(&self) -> String {
        "Whenever you lose life".to_string()
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

    #[test]
    fn test_matches() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = YouLoseLifeTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(LifeLossEvent::from_effect(alice, 2));
        assert!(trigger.matches(&event, &ctx));
    }
}
