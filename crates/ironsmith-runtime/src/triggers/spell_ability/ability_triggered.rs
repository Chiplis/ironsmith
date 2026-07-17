//! "Whenever another ability triggers" trigger.

use crate::events::EventKind;
use crate::events::spells::AbilityTriggeredEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityTriggeredTrigger {
    pub another: bool,
}

impl AbilityTriggeredTrigger {
    pub fn new(another: bool) -> Self {
        Self { another }
    }
}

impl TriggerMatcher for AbilityTriggeredTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::AbilityTriggered {
            return false;
        }
        let Some(event) = event.downcast::<AbilityTriggeredEvent>() else {
            return false;
        };

        let same_source = ctx
            .game
            .object(ctx.source_id)
            .is_some_and(|source| source.stable_id == event.source_stable_id)
            || crate::ids::StableId::from(ctx.source_id) == event.source_stable_id;
        !self.another || !(same_source && ctx.trigger_identity == Some(event.trigger_identity))
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::AbilityTriggered])
    }

    fn display(&self) -> String {
        if self.another {
            "Whenever another ability triggers".to_string()
        } else {
            "Whenever an ability triggers".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RawEvent;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId, StableId};
    use crate::provenance::ProvNodeId;
    use crate::triggers::TriggerIdentity;

    #[test]
    fn another_ability_excludes_only_the_same_trigger_identity() {
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = ObjectId::from_raw(41);
        let controller = PlayerId::from_index(0);
        let event = RawEvent::new(
            AbilityTriggeredEvent::new(
                source,
                StableId::from(source),
                controller,
                TriggerIdentity(7),
            ),
            ProvNodeId::default(),
        );
        let same = TriggerContext::for_source(source, controller, &game)
            .with_trigger_identity(TriggerIdentity(7));
        let different = TriggerContext::for_source(source, controller, &game)
            .with_trigger_identity(TriggerIdentity(8));
        let same_identity_different_source =
            TriggerContext::for_source(ObjectId::from_raw(42), controller, &game)
                .with_trigger_identity(TriggerIdentity(7));
        let matcher = AbilityTriggeredTrigger::new(true);

        assert!(!matcher.matches(&event, &same));
        assert!(matcher.matches(&event, &different));
        assert!(matcher.matches(&event, &same_identity_different_source));
    }
}
