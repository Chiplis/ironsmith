//! "Whenever [filter] becomes the target of a spell or ability" trigger.

use crate::events::EventKind;
use crate::events::spells::BecomesTargetedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedObjectTrigger {
    pub filter: ObjectFilter,
}

impl BecomesTargetedObjectTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for BecomesTargetedObjectTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BecomesTargeted {
            return false;
        }
        let Some(e) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        let Some(target_id) = e.target_object() else {
            return false;
        };
        let Some(target) = ctx.game.object(target_id) else {
            return false;
        };
        self.filter.matches(target, &ctx.filter_ctx, ctx.game)
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} becomes the target of a spell or ability",
            self.filter.description()
        )
    }
}
