//! "Whenever [filter] becomes blocked" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureBecameBlockedEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesBlockedTrigger {
    pub filter: ObjectFilter,
}

impl BecomesBlockedTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for BecomesBlockedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureBecameBlocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureBecameBlockedEvent>() else {
            return false;
        };
        if let Some(obj) = ctx.game.object(e.attacker) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!("Whenever {} becomes blocked", self.filter.description())
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
        let trigger = BecomesBlockedTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("becomes blocked"));
    }
}
