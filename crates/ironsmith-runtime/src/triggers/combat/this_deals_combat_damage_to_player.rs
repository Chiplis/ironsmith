//! "Whenever this creature deals combat damage to a player" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::filter::PlayerFilterExt;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisDealsCombatDamageToPlayerTrigger {
    pub player: PlayerFilter,
}

impl ThisDealsCombatDamageToPlayerTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for ThisDealsCombatDamageToPlayerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        let DamageTarget::Player(damaged_player) = e.target else {
            return false;
        };
        // Must be combat damage to a matching player from the source.
        e.is_combat
            && e.source == ctx.source_id
            && self.player.matches_player(damaged_player, &ctx.filter_ctx)
    }

    fn display(&self) -> String {
        let player = match &self.player {
            PlayerFilter::Any => "a player".to_string(),
            PlayerFilter::Opponent => "an opponent".to_string(),
            _ => self.player.description(),
        };
        format!("Whenever this creature deals combat damage to {player}")
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        let e = event.downcast::<DamageEvent>()?;
        let DamageTarget::Player(damaged_player) = e.target else {
            return None;
        };
        (e.is_combat
            && e.source == ctx.source_id
            && self.player.matches_player(damaged_player, &ctx.filter_ctx))
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

        let trigger = ThisDealsCombatDamageToPlayerTrigger::new(PlayerFilter::Any);
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

        let trigger = ThisDealsCombatDamageToPlayerTrigger::new(PlayerFilter::Any);
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

        let trigger = ThisDealsCombatDamageToPlayerTrigger::new(PlayerFilter::Any);
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

    #[test]
    fn test_matches_respects_opponent_filter() {
        let game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = ThisDealsCombatDamageToPlayerTrigger::new(PlayerFilter::Opponent);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let hits_controller = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(alice),
                5,
                true,
                crate::events::cause::EventCause::combat_damage(source_id),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&hits_controller, &ctx));
        assert_eq!(trigger.event_value_amount(&hits_controller, &ctx), None);

        let hits_opponent = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                5,
                true,
                crate::events::cause::EventCause::combat_damage(source_id),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&hits_opponent, &ctx));
        assert_eq!(trigger.event_value_amount(&hits_opponent, &ctx), Some(5));
    }
}
