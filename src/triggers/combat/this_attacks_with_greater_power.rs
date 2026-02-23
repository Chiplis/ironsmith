//! "Whenever this creature attacks with another creature with greater power" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when the source attacks and you control another attacker
/// with strictly greater power.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThisAttacksWithGreaterPowerTrigger;

impl ThisAttacksWithGreaterPowerTrigger {
    fn effective_power(ctx: &TriggerContext, id: crate::ids::ObjectId) -> Option<i32> {
        ctx.game
            .calculated_power(id)
            .or_else(|| ctx.game.object(id).and_then(|obj| obj.power()))
    }
}

impl TriggerMatcher for ThisAttacksWithGreaterPowerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(event) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        if event.attacker != ctx.source_id {
            return false;
        }

        let Some(source_power) = Self::effective_power(ctx, ctx.source_id) else {
            return false;
        };

        let Some(combat) = ctx.game.combat.as_ref() else {
            return false;
        };

        combat.attackers.iter().any(|attacker_info| {
            let attacker_id = attacker_info.creature;
            if attacker_id == ctx.source_id {
                return false;
            }
            let Some(attacker_obj) = ctx.game.object(attacker_id) else {
                return false;
            };
            if attacker_obj.controller != ctx.controller {
                return false;
            }
            Self::effective_power(ctx, attacker_id).is_some_and(|power| power > source_power)
        })
    }

    fn display(&self) -> String {
        "Whenever this creature attacks with another creature with greater power".to_string()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(
        game: &mut GameState,
        owner: PlayerId,
        id: u32,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(id), format!("Creature{id}").as_str())
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn matches_when_another_attacker_has_greater_power() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, alice, 1, 2, 2);
        let bigger = create_creature(&mut game, alice, 2, 4, 4);
        let mut combat = CombatState::default();
        combat.attackers = vec![
            AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            },
            AttackerInfo {
                creature: bigger,
                target: AttackTarget::Player(bob),
            },
        ];
        game.combat = Some(combat);

        let trigger = ThisAttacksWithGreaterPowerTrigger;
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            source,
            AttackEventTarget::Player(bob),
        ));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_no_other_greater_attacker() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, alice, 1, 3, 3);
        let smaller = create_creature(&mut game, alice, 2, 2, 2);
        let mut combat = CombatState::default();
        combat.attackers = vec![
            AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            },
            AttackerInfo {
                creature: smaller,
                target: AttackTarget::Player(bob),
            },
        ];
        game.combat = Some(combat);

        let trigger = ThisAttacksWithGreaterPowerTrigger;
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            source,
            AttackEventTarget::Player(bob),
        ));

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_for_other_attack_event() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(10);
        let other = ObjectId::from_raw(11);
        let trigger = ThisAttacksWithGreaterPowerTrigger;
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            other,
            AttackEventTarget::Player(bob),
        ));
        assert!(!trigger.matches(&event, &ctx));
    }
}
