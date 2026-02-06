//! "Whenever [filter] attacks you or a planeswalker you control" trigger.

use crate::events::EventKind;
use crate::events::combat::{AttackEventTarget, CreatureAttackedEvent};
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks you or your planeswalkers.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksYouTrigger {
    pub filter: ObjectFilter,
}

impl AttacksYouTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for AttacksYouTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };

        // Check if attack target is the controller or their planeswalker
        let attacks_controller = match &e.target {
            AttackEventTarget::Player(p) => *p == ctx.controller,
            AttackEventTarget::Planeswalker(pw) => ctx
                .game
                .object(*pw)
                .is_some_and(|o| o.controller == ctx.controller),
        };

        if !attacks_controller {
            return false;
        }

        if let Some(obj) = ctx.game.object(e.attacker) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!(
            "Whenever {} attacks you or a planeswalker you control",
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
        let trigger = AttacksYouTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("attacks you"));
    }
}
