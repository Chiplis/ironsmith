//! "Whenever this creature becomes blocked by [filter]" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBecameBlockedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisBecomesBlockedByObjectTrigger {
    pub blocker_filter: ObjectFilter,
}

impl ThisBecomesBlockedByObjectTrigger {
    pub fn new(blocker_filter: ObjectFilter) -> Self {
        Self { blocker_filter }
    }
}

impl TriggerMatcher for ThisBecomesBlockedByObjectTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBecameBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBecameBlockedEvent>() else {
            return false;
        };
        if e.attacker != ctx.source_id {
            return false;
        }

        e.blockers.iter().any(|blocker| {
            ctx.game
                .object(*blocker)
                .is_some_and(|obj| self.blocker_filter.matches(obj, &ctx.filter_ctx, ctx.game))
        }) || e.blocker_snapshots.iter().any(|snapshot| {
            self.blocker_filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        })
    }

    fn display(&self) -> String {
        format!(
            "Whenever this creature becomes blocked by {}",
            self.blocker_filter.description()
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
        power: i32,
        toughness: i32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn matches_when_source_becomes_blocked_by_matching_creature() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Attacker", alice, 4, 4);
        let blocker = create_creature(&mut game, "Small Blocker", bob, 1, 1);
        let event = TriggerEvent::new_with_provenance(
            CreatureBecameBlockedEvent::with_target_and_blockers(
                source,
                vec![blocker],
                None,
                None,
                Vec::new(),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        let mut blocker_filter = ObjectFilter::creature();
        blocker_filter.power = Some(crate::filter::Comparison::LessThanOrEqual(1));
        let trigger = ThisBecomesBlockedByObjectTrigger::new(blocker_filter);
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn does_not_match_when_blocker_fails_filter() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Attacker", alice, 4, 4);
        let blocker = create_creature(&mut game, "Large Blocker", bob, 2, 2);
        let event = TriggerEvent::new_with_provenance(
            CreatureBecameBlockedEvent::with_target_and_blockers(
                source,
                vec![blocker],
                None,
                None,
                Vec::new(),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        let mut blocker_filter = ObjectFilter::creature();
        blocker_filter.power = Some(crate::filter::Comparison::LessThanOrEqual(1));
        let trigger = ThisBecomesBlockedByObjectTrigger::new(blocker_filter);
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(!trigger.matches(&event, &ctx));
    }
}
