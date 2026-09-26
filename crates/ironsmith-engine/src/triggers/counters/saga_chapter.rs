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

impl SagaChapterTrigger {
    /// Number of this ability's chapters crossed by one lore-counter
    /// placement (CR 714.2b): each chapter number whose threshold lies in
    /// (before, after]. "I, II —" is two chapter abilities (CR 714.2c), so a
    /// single 0→2 placement triggers it twice.
    fn crossed_chapter_count(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        if event.kind() != EventKind::CounterPlaced {
            return 0;
        }
        let Some(e) = event.downcast::<CounterPlacedEvent>() else {
            return 0;
        };

        // Only trigger on lore counters placed on this saga
        if e.permanent != ctx.source_id || e.counter_type != CounterType::Lore {
            return 0;
        }

        let Some(saga) = ctx.game.object(e.permanent) else {
            return 0;
        };

        // Compare the counts immediately before and after this placement.
        // Older events without a recorded count fall back to the live count.
        let (previous_count, current_count) = match e.previous_count {
            Some(previous) => (previous, previous.saturating_add(e.amount)),
            None => {
                let current = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
                (current.saturating_sub(e.amount), current)
            }
        };

        let entered_this_turn =
            crate::game_loop::source_entered_battlefield_this_turn(ctx.game, e.permanent);
        let read_ahead_suppresses_skipped_chapters =
            entered_this_turn && crate::game_loop::source_has_read_ahead(ctx.game, e.permanent);

        self.chapters
            .iter()
            .filter(|&&chapter| {
                previous_count < chapter
                    && current_count >= chapter
                    && (!read_ahead_suppresses_skipped_chapters || current_count == chapter)
            })
            .count() as u32
    }
}

impl TriggerMatcher for SagaChapterTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.crossed_chapter_count(event, ctx) > 0
    }

    fn trigger_count_with_context(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        self.crossed_chapter_count(event, ctx).max(1)
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
