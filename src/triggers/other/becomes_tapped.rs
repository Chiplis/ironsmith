//! "Whenever this permanent becomes tapped" trigger.

use crate::events::EventKind;
use crate::events::other::PermanentTappedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTappedTrigger;

impl TriggerMatcher for BecomesTappedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::PermanentTapped {
            return false;
        }
        let Some(e) = event.downcast::<PermanentTappedEvent>() else {
            return false;
        };
        e.permanent == ctx.source_id
    }

    fn display(&self) -> String {
        "Whenever this permanent becomes tapped".to_string()
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

        let trigger = BecomesTappedTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(PermanentTappedEvent::new(source_id));
        assert!(trigger.matches(&event, &ctx));
    }
}
