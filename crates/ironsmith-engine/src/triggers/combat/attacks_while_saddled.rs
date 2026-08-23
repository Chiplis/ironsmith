//! "Whenever [filter] attacks while saddled" trigger.

use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks while saddled.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksWhileSaddledTrigger {
    pub filter: ObjectFilter,
}

impl AttacksWhileSaddledTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for AttacksWhileSaddledTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        if !ctx.game.is_saddled(e.attacker) {
            return false;
        }
        let Some(obj) = ctx.game.object(e.attacker) else {
            return false;
        };
        self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} attacks while saddled",
            self.filter.description()
        )
    }
}
