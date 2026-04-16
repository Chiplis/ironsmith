//! "Whenever a counter is removed from [filter]" trigger.

use crate::events::EventKind;
use crate::events::other::MarkersChangedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CounterRemovedFromTrigger {
    pub filter: ObjectFilter,
}

impl CounterRemovedFromTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for CounterRemovedFromTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::MarkersChanged {
            return false;
        }
        let Some(e) = event.downcast::<MarkersChangedEvent>() else {
            return false;
        };
        if !e.is_removed() {
            return false;
        }

        let Some(object_id) = e.object() else {
            return false;
        };

        if let Some(obj) = ctx.game.object(object_id) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!(
            "Whenever a counter is removed from {}",
            self.filter.description()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = CounterRemovedFromTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("counter is removed"));
    }
}
