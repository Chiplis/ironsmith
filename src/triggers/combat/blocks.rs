//! "Whenever [filter] blocks" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBlockedEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BlocksTrigger {
    pub filter: ObjectFilter,
}

impl BlocksTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for BlocksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBlockedEvent>() else {
            return false;
        };
        if let Some(obj) = ctx.game.object(e.blocker) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!("Whenever {} blocks", self.filter.description())
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
        let trigger = BlocksTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("blocks"));
    }
}
