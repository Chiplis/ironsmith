//! "Whenever [filter] is turned face up" trigger.

use crate::events::EventKind;
use crate::events::other::TurnedFaceUpEvent;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentTurnedFaceUpTrigger {
    pub filter: ObjectFilter,
}

impl PermanentTurnedFaceUpTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for PermanentTurnedFaceUpTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::TurnedFaceUp {
            return false;
        }
        let Some(e) = event.downcast::<TurnedFaceUpEvent>() else {
            return false;
        };
        if let Some(obj) = ctx.game.object(e.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn display(&self) -> String {
        format!("Whenever {} is turned face up", self.filter.description())
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
        let trigger = PermanentTurnedFaceUpTrigger::new(ObjectFilter::permanent().you_control());
        assert_eq!(
            trigger.display(),
            "Whenever a permanent you control is turned face up"
        );
    }
}
