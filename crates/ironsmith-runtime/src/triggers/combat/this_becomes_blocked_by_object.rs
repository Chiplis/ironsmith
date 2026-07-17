//! "Whenever this creature becomes blocked by [filter]" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBlockedEvent;
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

fn with_indefinite_article(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
        || lower.starts_with("each ")
        || lower.starts_with("all ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("those ")
        || lower.starts_with("target ")
        || lower.starts_with("any ")
        || lower.chars().next().is_some_and(|ch| ch.is_ascii_digit())
    {
        return trimmed.to_string();
    }
    let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

impl TriggerMatcher for ThisBecomesBlockedByObjectTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBlockedEvent>() else {
            return false;
        };
        if e.attacker != ctx.source_id {
            return false;
        }

        e.blocker_snapshot.as_ref().map_or_else(
            || {
                ctx.game
                    .object(e.blocker)
                    .is_some_and(|obj| self.blocker_filter.matches(obj, &ctx.filter_ctx, ctx.game))
            },
            |snapshot| {
                self.blocker_filter
                    .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
            },
        )
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureBlocked])
    }

    fn display(&self) -> String {
        format!(
            "Whenever this creature becomes blocked by {}",
            with_indefinite_article(&self.blocker_filter.description())
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
            CreatureBlockedEvent::new(blocker, source),
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
            CreatureBlockedEvent::new(blocker, source),
            crate::provenance::ProvNodeId::default(),
        );

        let mut blocker_filter = ObjectFilter::creature();
        blocker_filter.power = Some(crate::filter::Comparison::LessThanOrEqual(1));
        let trigger = ThisBecomesBlockedByObjectTrigger::new(blocker_filter);
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn matches_each_blocker_relationship_independently() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Attacker", alice, 4, 4);
        let small = create_creature(&mut game, "Small Blocker", bob, 1, 1);
        let large = create_creature(&mut game, "Large Blocker", bob, 3, 3);
        let mut blocker_filter = ObjectFilter::creature();
        blocker_filter.power = Some(crate::filter::Comparison::LessThanOrEqual(1));
        let trigger = ThisBecomesBlockedByObjectTrigger::new(blocker_filter);
        let ctx = TriggerContext::for_source(source, alice, &game);

        let small_event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(small, source),
            crate::provenance::ProvNodeId::default(),
        );
        let large_event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(large, source),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&small_event, &ctx));
        assert!(!trigger.matches(&large_event, &ctx));
    }

    #[test]
    fn display_adds_article_for_blocker_filter() {
        let trigger = ThisBecomesBlockedByObjectTrigger::new(ObjectFilter::creature());

        assert_eq!(
            trigger.display(),
            "Whenever this creature becomes blocked by a creature"
        );
    }
}
