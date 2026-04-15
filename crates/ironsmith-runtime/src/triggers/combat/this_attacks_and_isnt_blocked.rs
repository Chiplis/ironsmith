//! "Whenever this creature attacks and isn't blocked" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedAndUnblockedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisAttacksAndIsntBlockedTrigger;

impl TriggerMatcher for ThisAttacksAndIsntBlockedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttackedAndUnblocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedAndUnblockedEvent>() else {
            return false;
        };
        e.attacker == ctx.source_id
    }

    fn display(&self) -> String {
        "Whenever this creature attacks and isn't blocked".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};

    #[test]
    fn matches_own_unblocked_attack() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisAttacksAndIsntBlockedTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedAndUnblockedEvent::new(source_id, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn parser_supports_attacks_and_isnt_blocked_trigger() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ophidian Variant")
            .parse_text(
                "Whenever this creature attacks and isn't blocked, defending player loses 2 life.",
            )
            .expect("oracle line should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected a triggered ability");

        assert_eq!(
            triggered.trigger.display(),
            "Whenever this creature attacks and isn't blocked"
        );

        let dbg = format!("{:?}", triggered.trigger).to_ascii_lowercase();
        assert!(
            !dbg.contains("unimplemented_trigger"),
            "expected a supported trigger, got {dbg}"
        );
    }
}
