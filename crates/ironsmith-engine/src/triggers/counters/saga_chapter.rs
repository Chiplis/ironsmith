//! Saga chapter trigger.

use crate::events::EventKind;
use crate::events::other::{CounterPlacedEvent, MarkersChangedEvent};
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
        // Lore counters count however they are put on the Saga (CR 714.2b,
        // 122.6): the turn-based and entry lore counters report a
        // `CounterPlacedEvent`; effects (proliferate, "put a lore counter"),
        // and counters the Saga enters with, report a `MarkersChangedEvent`.
        let (permanent, amount, previous, after) = match event.kind() {
            EventKind::CounterPlaced => {
                let Some(e) = event.downcast::<CounterPlacedEvent>() else {
                    return 0;
                };
                if e.counter_type != CounterType::Lore {
                    return 0;
                }
                (e.permanent, e.amount, e.previous_count, None)
            }
            EventKind::MarkersChanged => {
                let Some(e) = event.downcast::<MarkersChangedEvent>() else {
                    return 0;
                };
                if !e.is_added() || e.marker.as_counter() != Some(CounterType::Lore) {
                    return 0;
                }
                let Some(permanent) = e.object() else {
                    return 0;
                };
                (permanent, e.amount, None, e.count_after)
            }
            _ => return 0,
        };

        // Only trigger on lore counters placed on this saga
        if permanent != ctx.source_id || amount == 0 {
            return 0;
        }

        let Some(saga) = ctx.game.object(permanent) else {
            return 0;
        };

        // Compare the counts immediately before and after this placement.
        // Older events without a recorded count fall back to the live count.
        let (previous_count, current_count) = match (previous, after) {
            (Some(previous), _) => (previous, previous.saturating_add(amount)),
            (None, Some(after)) => (after.saturating_sub(amount), after),
            (None, None) => {
                let current = saga.counters.get(&CounterType::Lore).copied().unwrap_or(0);
                (current.saturating_sub(amount), current)
            }
        };

        let entered_this_turn =
            crate::game_loop::source_entered_battlefield_this_turn(ctx.game, permanent);
        let read_ahead_suppresses_skipped_chapters =
            entered_this_turn && crate::game_loop::source_has_read_ahead(ctx.game, permanent);

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

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CounterPlaced, EventKind::MarkersChanged])
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
