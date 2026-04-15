//! "Whenever this creature becomes blocked" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBecameBlockedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisBecomesBlockedTrigger;

impl TriggerMatcher for ThisBecomesBlockedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBecameBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBecameBlockedEvent>() else {
            return false;
        };
        e.attacker == ctx.source_id
    }

    fn display(&self) -> String {
        "Whenever this creature becomes blocked".to_string()
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

        let trigger = ThisBecomesBlockedTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureBecameBlockedEvent::new(source_id, 2),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }
}
