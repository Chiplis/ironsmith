//! "Whenever [filter] blocks or becomes blocked" trigger.

use crate::events::EventKind;
use crate::events::combat::{CreatureBecameBlockedEvent, CreatureBlockedEvent};
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BlocksOrBecomesBlockedTrigger {
    pub filter: ObjectFilter,
}

impl BlocksOrBecomesBlockedTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for BlocksOrBecomesBlockedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
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
        format!(
            "Whenever {} blocks or becomes blocked",
            self.filter.description()
        )
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
        let trigger = BlocksOrBecomesBlockedTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("blocks or becomes blocked"));
    }
}
