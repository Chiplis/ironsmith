//! Trigger matcher for state-triggered abilities.

use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// A trigger whose condition is checked during state-based action scans.
#[derive(Debug, Clone, PartialEq)]
pub struct StateTrigger {
    pub description: String,
    pub objects: Option<crate::target::ObjectFilter>,
}

impl StateTrigger {
    pub fn matching_objects(filter: crate::target::ObjectFilter, description: String) -> Self {
        Self { description, objects: Some(filter) }
    }

    pub fn new(description: String) -> Self {
        Self { description, objects: None }
    }
}

impl TriggerMatcher for StateTrigger {
    fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        false
    }

    fn display(&self) -> String {
        self.description.clone()
    }
}
