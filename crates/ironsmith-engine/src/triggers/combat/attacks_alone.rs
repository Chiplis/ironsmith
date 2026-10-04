//! "Whenever [filter] attacks alone" trigger.
#[cfg(test)]
use crate::GameState;

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks alone.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksAloneTrigger {
    /// Filter for creatures that trigger this ability.
    pub filter: ObjectFilter,
    pub per_player: bool,
}

impl AttacksAloneTrigger {
    pub fn against_player(filter: ObjectFilter) -> Self {
        Self {
            filter,
            per_player: true,
        }
    }
    /// Create a new attacks-alone trigger with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            per_player: false,
        }
    }
}

impl TriggerMatcher for AttacksAloneTrigger {
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        if self.per_player {
            let crate::triggers::AttackEventTarget::Player(player) = e.target else {
                return false;
            };
            // Only directly attacked players count (CR506.6), including other
            // controllers' attackers. Planeswalkers/Battles are separate objects.
            let Some(declaration) = e.declared_attackers.as_deref() else {
                return false;
            };
            if declaration
                .iter()
                .filter(|info| info.target == crate::combat_state::AttackTarget::Player(player))
                .count()
                != 1
                || !declaration.iter().any(|info| {
                    info.creature == e.attacker
                        && info.target == crate::combat_state::AttackTarget::Player(player)
                })
            {
                return false;
            }
        } else if e.total_attackers != 1 {
            return false;
        }
        if let Some(obj) = ctx.game.object(e.attacker) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} attacks {}alone",
            self.filter.description(),
            if self.per_player { "a player " } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
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
    fn test_matches_attacks_alone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let creature_id = create_creature(&mut game, "Samurai", alice);

        let trigger = AttacksAloneTrigger::new(ObjectFilter::creature().you_control());
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                creature_id,
                AttackEventTarget::Player(bob),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_when_not_alone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let creature_id = create_creature(&mut game, "Samurai", alice);

        let trigger = AttacksAloneTrigger::new(ObjectFilter::creature().you_control());
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                creature_id,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!trigger.matches(&event, &ctx));
    }
}

#[cfg(test)]
mod direct_player_alone_tests {
    use super::*;
    use crate::combat_state::{AttackTarget, AttackerInfo};
    use crate::ids::{ObjectId, PlayerId};
    #[test]
    fn direct_player_alone_requires_the_retained_declaration_not_a_later_combat() {
        let a = PlayerId(0);
        let b = PlayerId(1);
        let c = PlayerId(2);
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Attacker")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let first = game.create_object_from_card(&card, a, crate::zone::Zone::Battlefield);
        let second = game.create_object_from_card(&card, a, crate::zone::Zone::Battlefield);
        let matcher = AttacksAloneTrigger::against_player(ObjectFilter::creature().you_control());
        for (second_target, expected) in [
            (AttackTarget::Player(b), false),
            (AttackTarget::Player(c), true),
            (AttackTarget::Planeswalker(ObjectId(90)), true),
        ] {
            let event = TriggerEvent::new(
                CreatureAttackedEvent::with_total_attackers(
                    first,
                    crate::triggers::AttackEventTarget::Player(b),
                    2,
                )
                .with_declared_attackers(std::sync::Arc::from(vec![
                    AttackerInfo {
                        creature: first,
                        target: AttackTarget::Player(b),
                    },
                    AttackerInfo {
                        creature: second,
                        target: second_target,
                    },
                ])),
                Default::default(),
            );
            game.combat = None;
            assert_eq!(
                matcher.matches(&event, &TriggerContext::for_source(first, a, &game)),
                expected
            );
        }
        let missing = TriggerEvent::new(
            CreatureAttackedEvent::new(first, crate::triggers::AttackEventTarget::Player(b)),
            Default::default(),
        );
        assert!(!matcher.matches(&missing, &TriggerContext::for_source(first, a, &game)));
    }
}
