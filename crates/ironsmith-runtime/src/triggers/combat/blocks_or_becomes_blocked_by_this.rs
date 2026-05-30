//! "Whenever [filter] blocks or becomes blocked by this creature" trigger.

use crate::events::EventKind;
use crate::events::combat::{CreatureBecameBlockedEvent, CreatureBlockedEvent};
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BlocksOrBecomesBlockedByThisTrigger {
    pub filter: ObjectFilter,
}

impl BlocksOrBecomesBlockedByThisTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for BlocksOrBecomesBlockedByThisTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        match event.kind() {
            EventKind::CreatureBlocked => {
                let Some(e) = event.downcast::<CreatureBlockedEvent>() else {
                    return false;
                };
                if e.attacker != ctx.source_id {
                    return false;
                }
                ctx.game
                    .object(e.blocker)
                    .is_some_and(|obj| self.filter.matches(obj, &ctx.filter_ctx, ctx.game))
                    || e.blocker_snapshot.as_ref().is_some_and(|snapshot| {
                        self.filter
                            .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
                    })
            }
            EventKind::CreatureBecameBlocked => {
                let Some(e) = event.downcast::<CreatureBecameBlockedEvent>() else {
                    return false;
                };
                if !e.blockers.contains(&ctx.source_id) {
                    return false;
                }
                ctx.game
                    .object(e.attacker)
                    .is_some_and(|obj| self.filter.matches(obj, &ctx.filter_ctx, ctx.game))
                    || e.attacker_snapshot.as_ref().is_some_and(|snapshot| {
                        self.filter
                            .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
                    })
            }
            _ => false,
        }
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} blocks or becomes blocked by this creature",
            self.filter.description()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        has_deathtouch: bool,
    ) -> crate::ids::ObjectId {
        let builder = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2));
        let card = builder.build();
        let object_id = game.create_object_from_card(&card, controller, Zone::Battlefield);
        if has_deathtouch
            && let Some(object) = game.object_mut(object_id)
        {
            object.abilities.push(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::deathtouch(),
            ));
        }
        object_id
    }

    #[test]
    fn matches_when_filtered_creature_blocks_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Attacker", alice, false);
        let blocker = create_creature(&mut game, "Deathtoucher", bob, true);
        let event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(blocker, source),
            crate::provenance::ProvNodeId::default(),
        );

        let trigger = BlocksOrBecomesBlockedByThisTrigger::new(
            ObjectFilter::creature()
                .with_static_ability(crate::static_abilities::StaticAbilityId::Deathtouch),
        );
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn matches_when_filtered_creature_becomes_blocked_by_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Blocker", alice, false);
        let attacker = create_creature(&mut game, "Deathtoucher", bob, true);
        let event = TriggerEvent::new_with_provenance(
            CreatureBecameBlockedEvent::with_target_and_blockers(
                attacker,
                vec![source],
                None,
                None,
                Vec::new(),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        let trigger = BlocksOrBecomesBlockedByThisTrigger::new(
            ObjectFilter::creature()
                .with_static_ability(crate::static_abilities::StaticAbilityId::Deathtouch),
        );
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_combat_is_with_another_creature() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Observer", alice, false);
        let other_attacker = create_creature(&mut game, "Other Attacker", alice, false);
        let blocker = create_creature(&mut game, "Deathtoucher", bob, true);
        let event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(blocker, other_attacker),
            crate::provenance::ProvNodeId::default(),
        );

        let trigger = BlocksOrBecomesBlockedByThisTrigger::new(
            ObjectFilter::creature()
                .with_static_ability(crate::static_abilities::StaticAbilityId::Deathtouch),
        );
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(!trigger.matches(&event, &ctx));
    }
}
