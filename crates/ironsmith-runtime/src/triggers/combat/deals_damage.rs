//! "Whenever [filter] deals damage" trigger.

use crate::events::DamageEvent;
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DealsDamageTrigger {
    pub filter: ObjectFilter,
    pub combat_only: bool,
}

impl DealsDamageTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            combat_only: false,
        }
    }

    pub fn combat_only(filter: ObjectFilter) -> Self {
        Self {
            filter,
            combat_only: true,
        }
    }
}

impl TriggerMatcher for DealsDamageTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if self.combat_only && !e.is_combat {
            return false;
        }
        if let Some(obj) = ctx.game.object(e.source) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        if self.combat_only {
            format!("Whenever {} deals combat damage", self.filter.description())
        } else {
            format!("Whenever {} deals damage", self.filter.description())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = DealsDamageTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("deals damage"));
    }
}
