//! "Whenever a counter is put on [filter]" trigger.

use crate::events::EventKind;
use crate::events::other::CounterPlacedEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CounterPutOnTrigger {
    pub filter: ObjectFilter,
}

impl CounterPutOnTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for CounterPutOnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CounterPlaced {
            return false;
        }
        let Some(e) = event.downcast::<CounterPlacedEvent>() else {
            return false;
        };
        if let Some(obj) = ctx.game.object(e.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!("Whenever a counter is put on {}", self.filter.description())
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = CounterPutOnTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("counter is put on"));
    }
}
