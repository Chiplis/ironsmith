//! "Whenever this creature blocks [filter]" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBlockedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::ids::ObjectId;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct ThisBlocksObjectTrigger {
    pub blocked_filter: ObjectFilter,
    /// `None` fires once per matching blocked object. `Some(N)` represents
    /// one grouped "N or more" declaration and fires exactly once.
    pub min_blocked_objects: Option<usize>,
}

impl ThisBlocksObjectTrigger {
    pub fn new(blocked_filter: ObjectFilter) -> Self {
        Self {
            blocked_filter,
            min_blocked_objects: None,
        }
    }

    pub fn with_minimum(blocked_filter: ObjectFilter, min_blocked_objects: usize) -> Self {
        Self {
            blocked_filter,
            min_blocked_objects: Some(min_blocked_objects.max(1)),
        }
    }

    fn matching_blocked_objects(&self, ctx: &TriggerContext) -> Option<(usize, Option<ObjectId>)> {
        let combat = ctx.game.combat.as_ref()?;
        let mut count = 0usize;
        let mut first = None;
        for (attacker, blockers) in &combat.blockers {
            if !blockers.contains(&ctx.source_id) {
                continue;
            }
            let matches = ctx.game.object(*attacker).is_some_and(|object| {
                self.blocked_filter
                    .matches(object, &ctx.filter_ctx, ctx.game)
            });
            if !matches {
                continue;
            }
            count += 1;
            first = Some(first.map_or(*attacker, |current: ObjectId| {
                if attacker.0 < current.0 {
                    *attacker
                } else {
                    current
                }
            }));
        }
        Some((count, first))
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
        let current_matches = ctx
            .game
            .object(e.attacker)
            .is_some_and(|obj| self.blocked_filter.matches(obj, &ctx.filter_ctx, ctx.game));
        if !current_matches {
            return false;
        }
        let Some(minimum) = self.min_blocked_objects else {
            return true;
        };
        let Some((count, first)) = self.matching_blocked_objects(ctx) else {
            return minimum == 1;
        };
        count >= minimum && first == Some(e.attacker)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureBlocked])
    }

    fn display(&self) -> String {
        if let Some(minimum) = self.min_blocked_objects {
            let description = self.blocked_filter.description();
            let subject = description
                .strip_prefix("a ")
                .or_else(|| description.strip_prefix("an "))
                .unwrap_or(description.as_str())
                .to_string();
            let subject = crate::compiled_text::pluralize_noun_phrase_for_trigger(&subject);
            let minimum = ironsmith_core::cardinal_word(minimum as u32)
                .unwrap_or_else(|| minimum.to_string());
            return format!("Whenever this creature blocks {minimum} or more {subject}");
        }
        format!(
            "Whenever this creature blocks {}",
            self.blocked_filter.description()
        )
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        self.min_blocked_objects?;
        if !self.matches(event, ctx) {
            return None;
        }
        Some(self.matching_blocked_objects(ctx)?.0 as i32)
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

    #[test]
    fn grouped_blocks_trigger_checks_the_threshold_fires_once_and_keeps_event_count() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Lairwatch Giant", alice, vec![]);
        let first_attacker = create_creature(&mut game, "First", bob, vec![]);
        let second_attacker = create_creature(&mut game, "Second", bob, vec![]);
        game.combat = Some(crate::combat_state::CombatState {
            blockers: std::collections::HashMap::from([
                (first_attacker, vec![source]),
                (second_attacker, vec![source]),
            ]),
            ..Default::default()
        });

        let first_event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(source, first_attacker),
            crate::provenance::ProvNodeId::default(),
        );
        let second_event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(source, second_attacker),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source, alice, &game);

        let ordinary = ThisBlocksObjectTrigger::new(ObjectFilter::creature());
        assert!(ordinary.matches(&first_event, &ctx));
        assert!(ordinary.matches(&second_event, &ctx));

        let grouped = ThisBlocksObjectTrigger::with_minimum(ObjectFilter::creature(), 2);
        assert!(grouped.matches(&first_event, &ctx));
        assert!(!grouped.matches(&second_event, &ctx));
        assert_eq!(grouped.event_value_amount(&first_event, &ctx), Some(2));
        assert_eq!(
            grouped.display(),
            "Whenever this creature blocks two or more creatures"
        );
    }

    #[test]
    fn grouped_blocks_trigger_does_not_fire_below_its_minimum() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Lairwatch Giant", alice, vec![]);
        let attacker = create_creature(&mut game, "Attacker", bob, vec![]);
        game.combat = Some(crate::combat_state::CombatState {
            blockers: std::collections::HashMap::from([(attacker, vec![source])]),
            ..Default::default()
        });
        let event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(source, attacker),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source, alice, &game);

        let grouped = ThisBlocksObjectTrigger::with_minimum(ObjectFilter::creature(), 2);
        assert!(!grouped.matches(&event, &ctx));
        assert_eq!(grouped.event_value_amount(&event, &ctx), None);
    }
}
