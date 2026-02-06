//! "Whenever [filter] becomes tapped" trigger.

use crate::events::EventKind;
use crate::events::other::PermanentTappedEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentBecomesTappedTrigger {
    pub filter: ObjectFilter,
}

impl PermanentBecomesTappedTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for PermanentBecomesTappedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::PermanentTapped {
            return false;
        }
        let Some(e) = event.downcast::<PermanentTappedEvent>() else {
            return false;
        };
        if let Some(obj) = ctx.game.object(e.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!("Whenever {} becomes tapped", self.filter.description())
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
        let trigger = PermanentBecomesTappedTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("becomes tapped"));
    }
}
