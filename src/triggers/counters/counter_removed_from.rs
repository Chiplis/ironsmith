//! "Whenever a counter is removed from [filter]" trigger.

use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CounterRemovedFromTrigger {
    pub filter: ObjectFilter,
}

impl CounterRemovedFromTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl TriggerMatcher for CounterRemovedFromTrigger {
    fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        // We don't have a CounterRemoved event currently.
        false
    }

    fn display(&self) -> String {
        format!(
            "Whenever a counter is removed from {}",
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
        let trigger = CounterRemovedFromTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("counter is removed"));
    }
}
