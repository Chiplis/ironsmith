//! Saga chapter trigger.

use crate::events::EventKind;
use crate::events::other::CounterPlacedEvent;
use crate::object::CounterType;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger for saga chapters.
///
/// Per MTG Rule 714.2c: A chapter triggers when "the number of lore counters on a
/// Saga permanent is greater than or equal to the chapter number" AND "that chapter
/// ability hasn't triggered since a lore counter was put on that Saga permanent."
#[derive(Debug, Clone, PartialEq)]
pub struct SagaChapterTrigger {
    /// Which chapters this trigger fires for.
    pub chapters: Vec<u32>,
}

impl SagaChapterTrigger {
    pub fn new(chapters: Vec<u32>) -> Self {
        Self { chapters }
    }

    pub fn chapter(chapter: u32) -> Self {
        Self::new(vec![chapter])
    }
}

impl TriggerMatcher for SagaChapterTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CounterPlaced {
            return false;
        }
        let Some(e) = event.downcast::<CounterPlacedEvent>() else {
            return false;
        };

        // Only trigger on lore counters placed on this saga
        if e.permanent != ctx.source_id || e.counter_type != CounterType::Lore {
            return false;
        }

        // Get the saga's current lore count
        let Some(saga) = ctx.game.object(e.permanent) else {
            return false;
        };

        let current_count = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
        // Calculate what the count was before this counter addition
        let previous_count = current_count.saturating_sub(e.amount);

        // A chapter triggers if the threshold was CROSSED by this counter addition:
        // - Previous count was below the chapter number
        // - Current count is at or above the chapter number
        self.chapters
            .iter()
            .any(|&chapter| previous_count < chapter && current_count >= chapter)
    }

    fn display(&self) -> String {
        if self.chapters.len() == 1 {
            format!("Chapter {}", self.chapters[0])
        } else {
            let chapters_str: Vec<String> = self.chapters.iter().map(|c| c.to_string()).collect();
            format!("Chapters {}", chapters_str.join(", "))
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }

    fn saga_chapters(&self) -> Option<&[u32]> {
        Some(&self.chapters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_saga(game: &mut GameState, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), "Test Saga")
            .card_types(vec![CardType::Enchantment])
            .build();

        let id = game.create_object_from_card(&card, controller, Zone::Battlefield);

        // Add 1 lore counter
        if let Some(obj) = game.object_mut(id) {
            obj.counters.insert(CounterType::Lore, 1);
        }

        id
    }

    #[test]
    fn test_chapter_triggers_on_crossing_threshold() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let saga_id = create_saga(&mut game, alice);

        let trigger = SagaChapterTrigger::chapter(2);

        // Simulate adding a lore counter that crosses to chapter 2
        // Current count is 1, we're adding 1 more to make it 2
        if let Some(obj) = game.object_mut(saga_id) {
            obj.counters.insert(CounterType::Lore, 2);
        }

        // Create context after mutation
        let ctx = TriggerContext::for_source(saga_id, alice, &game);

        let event = TriggerEvent::new(CounterPlacedEvent::new(saga_id, CounterType::Lore, 1));

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_chapter_does_not_trigger_below_threshold() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let saga_id = create_saga(&mut game, alice);

        let trigger = SagaChapterTrigger::chapter(3);

        // Current count is 1, we're adding 1 to make it 2, but chapter 3 needs 3
        if let Some(obj) = game.object_mut(saga_id) {
            obj.counters.insert(CounterType::Lore, 2);
        }

        // Create context after mutation
        let ctx = TriggerContext::for_source(saga_id, alice, &game);

        let event = TriggerEvent::new(CounterPlacedEvent::new(saga_id, CounterType::Lore, 1));

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = SagaChapterTrigger::chapter(1);
        assert!(trigger.display().contains("Chapter 1"));

        let trigger2 = SagaChapterTrigger::new(vec![1, 2]);
        assert!(trigger2.display().contains("1, 2"));
    }
}
