//! "Whenever an ability of [filter] is activated" trigger.

use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityActivatedTrigger {
    pub filter: ObjectFilter,
}

impl AbilityActivatedTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for AbilityActivatedTrigger {
    fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        // We don't have an AbilityActivated event currently.
        // This would need to be added for full implementation.
        false
    }

    fn display(&self) -> String {
        format!(
            "Whenever an ability of {} is activated",
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
        let trigger = AbilityActivatedTrigger::new(ObjectFilter::default());
        assert!(trigger.display().contains("activated"));
    }
}
