//! "Whenever [filter] blocks or becomes blocked" trigger.

use crate::events::EventKind;
use crate::events::combat::{CreatureBecameBlockedEvent, CreatureBlockedEvent};
use crate::filter::ObjectFilterExt as _;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BlocksOrBecomesBlockedTrigger {
    pub filter: ObjectFilter,
    /// When present, match one concrete blocking pair and require the
    /// participant opposite `filter` to satisfy this filter. `None` retains
    /// the aggregate legacy "[filter] blocks or becomes blocked" behavior.
    pub other_filter: Option<ObjectFilter>,
}

impl BlocksOrBecomesBlockedTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            other_filter: None,
        }
    }

    pub fn with_other(filter: ObjectFilter, other_filter: ObjectFilter) -> Self {
        Self {
            filter,
            other_filter: Some(other_filter),
        }
    }
}

fn matches_event_object(
    filter: &ObjectFilter,
    snapshot: Option<&ObjectSnapshot>,
    object_id: ObjectId,
    ctx: &TriggerContext<'_>,
) -> bool {
    snapshot.map_or_else(
        || {
            ctx.game
                .object(object_id)
                .is_some_and(|object| filter.matches(object, &ctx.filter_ctx, ctx.game))
        },
        |snapshot| filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game),
    )
}

fn with_indefinite_article(description: String) -> String {
    let trimmed = description.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("another ")
        || lower.starts_with("the ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("each ")
        || lower.starts_with("one or more ")
    {
        return trimmed.to_string();
    }
    let article = if trimmed
        .chars()
        .next()
        .is_some_and(|first| matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

impl TriggerMatcher for BlocksOrBecomesBlockedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if let Some(other_filter) = &self.other_filter {
            let Some(e) = event.downcast::<CreatureBlockedEvent>() else {
                return false;
            };
            let subject_blocks =
                matches_event_object(&self.filter, e.blocker_snapshot.as_ref(), e.blocker, ctx)
                    && matches_event_object(
                        other_filter,
                        e.attacker_snapshot.as_ref(),
                        e.attacker,
                        ctx,
                    );
            let subject_becomes_blocked =
                matches_event_object(&self.filter, e.attacker_snapshot.as_ref(), e.attacker, ctx)
                    && matches_event_object(
                        other_filter,
                        e.blocker_snapshot.as_ref(),
                        e.blocker,
                        ctx,
                    );
            return subject_blocks || subject_becomes_blocked;
        }
        match event.kind() {
            EventKind::CreatureBlocked => {
                let Some(e) = event.downcast::<CreatureBlockedEvent>() else {
                    return false;
                };
                if let Some(obj) = ctx.game.object(e.blocker) {
                    self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
                } else {
                    false
                }
            }
            EventKind::CreatureBecameBlocked => {
                let Some(e) = event.downcast::<CreatureBecameBlockedEvent>() else {
                    return false;
                };
                if let Some(obj) = ctx.game.object(e.attacker) {
                    self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn display(&self) -> String {
        if let Some(other_filter) = &self.other_filter {
            return format!(
                "Whenever {} blocks or becomes blocked by {}",
                self.filter.description(),
                with_indefinite_article(other_filter.description())
            );
        }
        format!(
            "Whenever {} blocks or becomes blocked",
            self.filter.description()
        )
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        self.other_filter
            .as_ref()
            .map(|_| vec![EventKind::CreatureBlocked])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = BlocksOrBecomesBlockedTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("blocks or becomes blocked"));
    }

    #[test]
    fn paired_display_preserves_the_opposite_filter() {
        let mut other = ObjectFilter::creature();
        other.toughness = Some(crate::filter::Comparison::LessThanOrEqual(3));
        let trigger =
            BlocksOrBecomesBlockedTrigger::with_other(ObjectFilter::tagged("enchanted"), other);
        assert_eq!(
            trigger.display(),
            "Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or less"
        );
    }
}
