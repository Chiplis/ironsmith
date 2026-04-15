//! "Whenever this creature blocks [filter]" trigger.

use crate::filter::ObjectFilterExt as _;
use crate::events::EventKind;
use crate::events::combat::CreatureBlockedEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisBlocksObjectTrigger {
    pub blocked_filter: ObjectFilter,
}

impl ThisBlocksObjectTrigger {
    pub fn new(blocked_filter: ObjectFilter) -> Self {
        Self { blocked_filter }
    }
}

impl TriggerMatcher for ThisBlocksObjectTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBlockedEvent>() else {
            return false;
        };
        if e.blocker != ctx.source_id {
            return false;
        }
        ctx.game
            .object(e.attacker)
            .is_some_and(|obj| self.blocked_filter.matches(obj, &ctx.filter_ctx, ctx.game))
    }

    fn display(&self) -> String {
        format!(
            "Whenever this creature blocks {}",
            self.blocked_filter.description()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        subtypes: Vec<Subtype>,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn matches_when_source_blocks_matching_attacker() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Blocker", alice, vec![]);
        let vampire_attacker = create_creature(&mut game, "Vampire", bob, vec![Subtype::Vampire]);
        let event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(source, vampire_attacker),
            crate::provenance::ProvNodeId::default(),
        );

        let trigger =
            ThisBlocksObjectTrigger::new(ObjectFilter::creature().with_subtype(Subtype::Vampire));
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_attacker_fails_filter() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Blocker", alice, vec![]);
        let zombie_attacker = create_creature(&mut game, "Zombie", bob, vec![Subtype::Zombie]);
        let event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(source, zombie_attacker),
            crate::provenance::ProvNodeId::default(),
        );

        let trigger =
            ThisBlocksObjectTrigger::new(ObjectFilter::creature().with_subtype(Subtype::Vampire));
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = ThisBlocksObjectTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("blocks"));
    }
}
