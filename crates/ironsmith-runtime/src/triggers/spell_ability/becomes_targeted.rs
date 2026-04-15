//! "Whenever this permanent becomes the target of a spell or ability" trigger.

use crate::events::EventKind;
use crate::events::spells::BecomesTargetedEvent;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedTrigger;

impl TriggerMatcher for BecomesTargetedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::BecomesTargeted {
            return false;
        }
        let Some(e) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        e.target == ctx.source_id
    }

    fn display(&self) -> String {
        "Whenever this permanent becomes the target of a spell or ability".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = BecomesTargetedTrigger;
        assert!(trigger.display().contains("becomes the target"));
    }
}
