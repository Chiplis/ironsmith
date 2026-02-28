//! "Whenever [filter] attacks you or a planeswalker you control" trigger.

use crate::combat_state::AttackTarget;
use crate::events::EventKind;
use crate::events::combat::{AttackEventTarget, CreatureAttackedEvent};
use crate::ids::ObjectId;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks you or your planeswalkers.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksYouTrigger {
    pub filter: ObjectFilter,
    pub one_or_more: bool,
}

impl AttacksYouTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: false,
        }
    }

    pub fn one_or_more(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: true,
        }
    }

    fn attacks_controller_target(target: &AttackEventTarget, ctx: &TriggerContext) -> bool {
        match target {
            AttackEventTarget::Player(p) => *p == ctx.controller,
            AttackEventTarget::Planeswalker(pw) => ctx
                .game
                .object(*pw)
                .is_some_and(|o| o.controller == ctx.controller),
        }
    }

    fn combat_target_attacks_controller(target: &AttackTarget, ctx: &TriggerContext) -> bool {
        match target {
            AttackTarget::Player(p) => *p == ctx.controller,
            AttackTarget::Planeswalker(pw) => ctx
                .game
                .object(*pw)
                .is_some_and(|o| o.controller == ctx.controller),
        }
    }

    fn first_matching_attacker_for_player_this_combat(
        &self,
        attacker: ObjectId,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(attacker_obj) = ctx.game.object(attacker) else {
            return true;
        };
        let attacking_player = attacker_obj.controller;
        let Some(combat) = ctx.game.combat.as_ref() else {
            return true;
        };

        for info in &combat.attackers {
            let Some(obj) = ctx.game.object(info.creature) else {
                continue;
            };
            if obj.controller != attacking_player {
                continue;
            }
            if !Self::combat_target_attacks_controller(&info.target, ctx) {
                continue;
            }
            if !self.filter.matches(obj, &ctx.filter_ctx, ctx.game) {
                continue;
            }
            return info.creature == attacker;
        }
        true
    }
}

impl TriggerMatcher for AttacksYouTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };

        if !Self::attacks_controller_target(&e.target, ctx) {
            return false;
        }

        let Some(obj) = ctx.game.object(e.attacker) else {
            return false;
        };
        if !self.filter.matches(obj, &ctx.filter_ctx, ctx.game) {
            return false;
        }
        if self.one_or_more {
            return self.first_matching_attacker_for_player_this_combat(e.attacker, ctx);
        }
        true
    }

    fn display(&self) -> String {
        if self.one_or_more {
            let mut subject = self.filter.description();
            if let Some(stripped) = subject.strip_prefix("a ") {
                subject = stripped.to_string();
            } else if let Some(stripped) = subject.strip_prefix("an ") {
                subject = stripped.to_string();
            }
            return format!(
                "Whenever one or more {subject} attack you or a planeswalker you control"
            );
        }
        format!(
            "Whenever {} attacks you or a planeswalker you control",
            self.filter.description()
        )
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackerInfo, CombatState};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn test_display() {
        let trigger = AttacksYouTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("attacks you"));
    }

    #[test]
    fn test_one_or_more_matches_first_attacker_per_attacking_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", bob);
        let attacker_two = create_creature(&mut game, "B", bob);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(alice),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(alice),
        });
        game.combat = Some(combat);

        let trigger = AttacksYouTrigger::one_or_more(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let first_event = TriggerEvent::new(CreatureAttackedEvent::with_total_attackers(
            attacker_one,
            AttackEventTarget::Player(alice),
            2,
        ));
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new(CreatureAttackedEvent::with_total_attackers(
            attacker_two,
            AttackEventTarget::Player(alice),
            2,
        ));
        assert!(!trigger.matches(&second_event, &ctx));
    }
}
