//! "When [filter] enters the battlefield untapped" trigger.

use crate::events::EventKind;
use crate::events::zones::EnterBattlefieldEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching object enters the battlefield untapped.
#[derive(Debug, Clone, PartialEq)]
pub struct EntersBattlefieldUntappedTrigger {
    /// Filter for objects that trigger this ability.
    pub filter: ObjectFilter,
}

impl EntersBattlefieldUntappedTrigger {
    /// Create a new ETB-untapped trigger with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for EntersBattlefieldUntappedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::EnterBattlefield {
            return false;
        }

        let Some(enter_event) = event.downcast::<EnterBattlefieldEvent>() else {
            return false;
        };

        if enter_event.enters_tapped {
            return false;
        }

        let Some(object_id) = event.object_id() else {
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
            "Whenever {} enters the battlefield untapped",
            self.filter.description()
        )
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}
