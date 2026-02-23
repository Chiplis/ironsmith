//! "Whenever a counter is put on [filter]" trigger.

use crate::events::EventKind;
use crate::events::other::CounterPlacedEvent;
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::triggers::CountMode;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct CounterPutOnTrigger {
    pub filter: ObjectFilter,
    pub counter_type: Option<CounterType>,
    pub count_mode: CountMode,
}

impl CounterPutOnTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter_type: None,
            count_mode: CountMode::Each,
        }
    }

    pub fn counter_type(mut self, counter_type: CounterType) -> Self {
        self.counter_type = Some(counter_type);
        self
    }

    pub fn count(mut self, count_mode: CountMode) -> Self {
        self.count_mode = count_mode;
        self
    }
}

impl TriggerMatcher for CounterPutOnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CounterPlaced {
            return false;
        }
        let Some(e) = event.downcast::<CounterPlacedEvent>() else {
            return false;
        };
        if let Some(counter_type) = self.counter_type
            && e.counter_type != counter_type
        {
            return false;
        }
        if let Some(obj) = ctx.game.object(e.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        match self.count_mode {
            CountMode::OneOrMore => 1,
            CountMode::Each => event
                .downcast::<CounterPlacedEvent>()
                .map(|e| e.amount.max(1))
                .unwrap_or(1),
        }
    }

    fn display(&self) -> String {
        fn counter_text(counter_type: CounterType) -> String {
            match counter_type {
                CounterType::PlusOnePlusOne => "+1/+1".to_string(),
                CounterType::MinusOneMinusOne => "-1/-1".to_string(),
                other => format!("{other:?}").to_ascii_lowercase(),
            }
        }

        let counters = match self.counter_type {
            Some(counter_type) => format!("{} counter", counter_text(counter_type)),
            None => "counter".to_string(),
        };

        match self.count_mode {
            CountMode::OneOrMore => format!(
                "Whenever one or more {}s are put on {}",
                counters,
                self.filter.description()
            ),
            CountMode::Each => format!(
                "Whenever a {} is put on {}",
                counters,
                self.filter.description()
            ),
        }
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
        let trigger = CounterPutOnTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("counter is put on"));
    }
}
