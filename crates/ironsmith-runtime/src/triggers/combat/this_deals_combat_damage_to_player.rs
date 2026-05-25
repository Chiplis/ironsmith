//! "Whenever this creature deals combat damage to a player" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisDealsCombatDamageToPlayerTrigger;

impl TriggerMatcher for ThisDealsCombatDamageToPlayerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        // Must be combat damage to a player from the source
        e.is_combat && matches!(e.target, DamageTarget::Player(_)) && e.source == ctx.source_id
    }

    fn display(&self) -> String {
        "Whenever this creature deals combat damage to a player".to_string()
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        let e = event.downcast::<DamageEvent>()?;
        (e.is_combat && matches!(e.target, DamageTarget::Player(_)) && e.source == ctx.source_id)
            .then_some(e.amount as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn test_matches_combat_damage_to_player() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisDealsCombatDamageToPlayerTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                3,
                true, // is_combat
                crate::events::cause::EventCause::combat_damage(source_id),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_non_combat_damage() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisDealsCombatDamageToPlayerTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                3,
                false, // is_combat = false
                crate::events::cause::EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!trigger.matches(&event, &ctx));
        assert_eq!(trigger.event_value_amount(&event, &ctx), None);
    }

    #[test]
    fn test_event_value_amount_uses_combat_damage_amount() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisDealsCombatDamageToPlayerTrigger;
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                5,
                true,
                crate::events::cause::EventCause::combat_damage(source_id),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert_eq!(trigger.event_value_amount(&event, &ctx), Some(5));
    }
}
