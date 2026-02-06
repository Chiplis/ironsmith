//! Custom trigger for complex conditions.

use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// A custom trigger for complex conditions that don't fit standard patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomTrigger {
    pub id: &'static str,
    pub description: String,
}

impl CustomTrigger {
    pub fn new(id: &'static str, description: String) -> Self {
        Self { id, description }
    }
}

impl TriggerMatcher for CustomTrigger {
    fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        // Custom triggers need special handling based on their ID
        false
    }

    fn display(&self) -> String {
        self.description.clone()
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
        let trigger = CustomTrigger::new("test", "When something special happens".to_string());
        assert_eq!(trigger.display(), "When something special happens");
    }
}
