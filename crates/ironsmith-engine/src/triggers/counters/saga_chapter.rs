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

        let entered_this_turn =
            crate::game_loop::source_entered_battlefield_this_turn(ctx.game, e.permanent);
        let read_ahead_suppresses_skipped_chapters =
            entered_this_turn && crate::game_loop::source_has_read_ahead(ctx.game, e.permanent);

        self.chapters.iter().any(|&chapter| {
            previous_count < chapter
                && current_count >= chapter
                && (!read_ahead_suppresses_skipped_chapters || current_count == chapter)
        })
    }

    fn display(&self) -> String {
        if self.chapters.len() == 1 {
            format!("Chapter {}", self.chapters[0])
        } else {
            let chapters_str: Vec<String> = self.chapters.iter().map(|c| c.to_string()).collect();
            format!("Chapters {}", chapters_str.join(", "))
        }
    }

    fn saga_chapters(&self) -> Option<&[u32]> {
        Some(&self.chapters)
    }
}
