//! "When this Class becomes level N" trigger.

use crate::events::EventKind;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// CR 716.2a: a Class's level ability sets its level to N. The printed
/// "When this Class becomes level N" triggers on that level change; levels
/// are a designation (CR 716.2b), not level counters.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassBecomesLevelTrigger {
    pub level: u32,
}

impl ClassBecomesLevelTrigger {
    pub fn new(level: u32) -> Self {
        Self { level }
    }
}

impl TriggerMatcher for ClassBecomesLevelTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::KeywordAction {
            return false;
        }
        let Some(e) = event.downcast::<KeywordActionEvent>() else {
            return false;
        };
        if e.action != KeywordActionKind::GainClassLevel || e.amount != self.level {
            return false;
        }
        let stable_source = ctx
            .game
            .object(ctx.source_id)
            .map(|object| object.stable_id.object_id())
            .unwrap_or(ctx.source_id);
        e.source == ctx.source_id || e.source == stable_source
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::KeywordAction])
    }

    fn display(&self) -> String {
        format!("When this Class becomes level {}", self.level)
    }
}
