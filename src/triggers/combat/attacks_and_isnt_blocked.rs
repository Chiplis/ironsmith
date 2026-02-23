//! "Whenever [filter] attacks and isn't blocked" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedAndUnblockedEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks and isn't blocked.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksAndIsntBlockedTrigger {
    pub filter: ObjectFilter,
}

impl AttacksAndIsntBlockedTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for AttacksAndIsntBlockedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttackedAndUnblocked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedAndUnblockedEvent>() else {
            return false;
        };
        let Some(obj) = ctx.game.object(e.attacker) else {
            return false;
        };
        self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} attacks and isn't blocked",
            self.filter.description()
        )
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}
