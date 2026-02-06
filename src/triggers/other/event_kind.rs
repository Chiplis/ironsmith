//! Generic event-kind triggers.

use crate::events::EventKind;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that only checks event kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventKindTrigger {
    pub kind: EventKind,
    pub display_text: String,
}

impl EventKindTrigger {
    pub fn new(kind: EventKind, display_text: impl Into<String>) -> Self {
        Self {
            kind,
            display_text: display_text.into(),
        }
    }
}

impl TriggerMatcher for EventKindTrigger {
    fn matches(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        event.kind() == self.kind
    }

    fn display(&self) -> String {
        self.display_text.clone()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

/// Trigger that checks event kind and requires event object == source object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThisEventObjectTrigger {
    pub kind: EventKind,
    pub display_text: String,
}

impl ThisEventObjectTrigger {
    pub fn new(kind: EventKind, display_text: impl Into<String>) -> Self {
        Self {
            kind,
            display_text: display_text.into(),
        }
    }
}

impl TriggerMatcher for ThisEventObjectTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        event.kind() == self.kind && event.object_id() == Some(ctx.source_id)
    }

    fn display(&self) -> String {
        self.display_text.clone()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::other::{BecameMonstrousEvent, PlayersFinishedVotingEvent};
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use std::collections::HashMap;

    #[test]
    fn event_kind_trigger_matches_kind() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let trigger = EventKindTrigger::new(
            EventKind::PlayersFinishedVoting,
            "Whenever players finish voting",
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new(PlayersFinishedVotingEvent::new(
            source_id,
            alice,
            vec![],
            HashMap::new(),
            vec!["a".to_string(), "b".to_string()],
        ));
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn this_event_object_trigger_matches_source_object() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let other_id = ObjectId::from_raw(2);
        let trigger = ThisEventObjectTrigger::new(
            EventKind::BecameMonstrous,
            "When this creature becomes monstrous",
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let own_event = TriggerEvent::new(BecameMonstrousEvent::new(source_id, alice, 3));
        let other_event = TriggerEvent::new(BecameMonstrousEvent::new(other_id, alice, 3));
        assert!(trigger.matches(&own_event, &ctx));
        assert!(!trigger.matches(&other_event, &ctx));
    }
}
