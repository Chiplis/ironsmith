//! "Whenever this creature blocks" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBlockedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisBlocksTrigger;

impl TriggerMatcher for ThisBlocksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBlockedEvent>() else {
            return false;
        };
        e.blocker == ctx.source_id
    }

    fn display(&self) -> String {
        "Whenever this creature blocks".to_string()
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
    fn test_matches_own_block() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let attacker_id = ObjectId::from_raw(2);

        let trigger = ThisBlocksTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(CreatureBlockedEvent::new(source_id, attacker_id));

        assert!(trigger.matches(&event, &ctx));
    }
}
