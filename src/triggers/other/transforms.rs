//! "When this permanent transforms" trigger.

use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct TransformsTrigger;

impl TriggerMatcher for TransformsTrigger {
    fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        // We don't have a Transform event currently.
        false
    }

    fn display(&self) -> String {
        "When this permanent transforms".to_string()
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
        let trigger = TransformsTrigger;
        assert!(trigger.display().contains("transforms"));
    }
}
