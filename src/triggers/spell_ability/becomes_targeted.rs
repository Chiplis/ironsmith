//! "Whenever this permanent becomes the target of a spell or ability" trigger.

use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedTrigger;

impl TriggerMatcher for BecomesTargetedTrigger {
    fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        // We don't have a BecomesTargeted event currently.
        // Ward abilities use this trigger type.
        false
    }

    fn display(&self) -> String {
        "Whenever this permanent becomes the target of a spell or ability".to_string()
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
        let trigger = BecomesTargetedTrigger;
        assert!(trigger.display().contains("becomes the target"));
    }
}
