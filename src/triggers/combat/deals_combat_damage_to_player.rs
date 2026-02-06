//! "Whenever [filter] deals combat damage to a player" trigger.

use crate::events::DamageEvent;
use crate::events::EventKind;
use crate::game_event::DamageTarget;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DealsCombatDamageToPlayerTrigger {
    pub filter: ObjectFilter,
}

impl DealsCombatDamageToPlayerTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for DealsCombatDamageToPlayerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };
        // Must be combat damage to a player
        if !e.is_combat || !matches!(e.target, DamageTarget::Player(_)) {
            return false;
        }
        if let Some(obj) = ctx.game.object(e.source) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} deals combat damage to a player",
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
        let trigger = DealsCombatDamageToPlayerTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("deals combat damage"));
    }
}
