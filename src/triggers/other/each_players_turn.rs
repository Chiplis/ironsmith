//! "At the beginning of each player's turn" trigger.

use crate::events::EventKind;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct EachPlayersTurnTrigger;

impl TriggerMatcher for EachPlayersTurnTrigger {
    fn matches(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        // This triggers at the beginning of upkeep for any player
        event.kind() == EventKind::BeginningOfUpkeep
    }

    fn display(&self) -> String {
        "At the beginning of each player's turn".to_string()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::BeginningOfUpkeepEvent;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn test_matches() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = EachPlayersTurnTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        assert!(trigger.matches(&TriggerEvent::new(BeginningOfUpkeepEvent::new(alice)), &ctx));
        assert!(trigger.matches(&TriggerEvent::new(BeginningOfUpkeepEvent::new(bob)), &ctx));
    }
}
