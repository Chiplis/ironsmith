//! "Whenever [filter] attacks" trigger.

use crate::filter::ObjectFilterExt as _;
use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::ids::ObjectId;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks.
///
/// Used by cards that care about other creatures attacking.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksTrigger {
    /// Filter for creatures that trigger this ability.
    pub filter: ObjectFilter,
    /// If true, this trigger fires only once when one or more matching creatures attack.
    pub one_or_more: bool,
    /// Minimum number of total attackers required for this trigger to fire.
    pub min_total_attackers: usize,
}

impl AttacksTrigger {
    /// Create a new attacks trigger with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: false,
            min_total_attackers: 1,
        }
    }

    /// Create an attacks trigger that fires once for one-or-more attackers.
    pub fn one_or_more(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: true,
            min_total_attackers: 1,
        }
    }

    /// Create an attacks trigger that fires once for one-or-more attackers and
    /// only if at least `min_total_attackers` attackers were declared.
    pub fn one_or_more_with_min_total_attackers(
        filter: ObjectFilter,
        min_total_attackers: usize,
    ) -> Self {
        Self {
            filter,
            one_or_more: true,
            min_total_attackers: min_total_attackers.max(1),
        }
    }

    /// Create an attacks trigger for any creature.
    pub fn any_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }

    fn is_first_matching_attacker_this_combat(
        &self,
        attacker: ObjectId,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(combat) = ctx.game.combat.as_ref() else {
            return true;
        };
        for info in &combat.attackers {
            let Some(obj) = ctx.game.object(info.creature) else {
                continue;
            };
            if self.filter.matches(obj, &ctx.filter_ctx, ctx.game) {
                return info.creature == attacker;
            }
        }
        true
    }
}

impl TriggerMatcher for AttacksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        let Some(obj) = ctx.game.object(e.attacker) else {
            return false;
        };
        if !self.filter.matches(obj, &ctx.filter_ctx, ctx.game) {
            return false;
        }
        if e.total_attackers < self.min_total_attackers {
            return false;
        }
        if self.one_or_more {
            return self.is_first_matching_attacker_this_combat(e.attacker, ctx);
        }
        true
    }

    fn display(&self) -> String {
        let mut subject = self.filter.description();
        if let Some(stripped) = subject.strip_prefix("a ") {
            subject = stripped.to_string();
        } else if let Some(stripped) = subject.strip_prefix("an ") {
            subject = stripped.to_string();
        }

        if self.one_or_more {
            if self.min_total_attackers > 1 {
                return format!(
                    "Whenever {} or more {subject} attack",
                    self.min_total_attackers
                );
            }
            return format!("Whenever one or more {subject} attack");
        }
        if self.min_total_attackers > 1 {
            return format!(
                "Whenever {} or more {subject} attack",
                self.min_total_attackers
            );
        }
        format!("Whenever {} attacks", self.filter.description())
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
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn test_matches_creature_attack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);

        let trigger = AttacksTrigger::any_creature();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(creature_id, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = AttacksTrigger::any_creature();
        assert!(trigger.display().contains("attacks"));
    }

    #[test]
    fn test_one_or_more_only_matches_first_attacker_in_declaration() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger = AttacksTrigger::one_or_more(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let first_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_two,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));
    }

    #[test]
    fn test_one_or_more_with_min_total_attackers_requires_threshold() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);
        let attacker_three = create_creature(&mut game, "C", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_three,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger =
            AttacksTrigger::one_or_more_with_min_total_attackers(ObjectFilter::creature(), 3);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let below_threshold = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&below_threshold, &ctx));

        let first_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_two,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));
    }
}
