//! One arm of a union trigger that functions only while its source is in one
//! of the arm's own zones (CR 113.6).

use crate::events::EventKind;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};
use crate::triggers::{Trigger, TriggerEvent};
use crate::zone::Zone;

#[derive(Debug, Clone)]
pub struct ZoneGatedTrigger {
    pub trigger: Trigger,
    pub zones: Vec<Zone>,
}

impl ZoneGatedTrigger {
    pub fn new(trigger: Trigger, zones: Vec<Zone>) -> Self {
        Self { trigger, zones }
    }

    fn source_in_gate(&self, ctx: &TriggerContext) -> bool {
        ctx.filter_ctx.source_snapshot.as_ref().map(|source| source.zone)
            .or_else(|| ctx.game.object(ctx.source_id).map(|source| source.zone))
            .is_some_and(|zone| self.zones.contains(&zone))
    }
}

impl TriggerMatcher for ZoneGatedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.source_in_gate(ctx) && self.trigger.matches(event, ctx)
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        self.trigger.trigger_count(event)
    }

    fn trigger_count_with_context(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        if self.source_in_gate(ctx) {
            self.trigger.trigger_count_with_context(event, ctx)
        } else {
            0
        }
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        self.source_in_gate(ctx)
            .then(|| self.trigger.event_value_amount(event, ctx))
            .flatten()
    }

    fn uses_snapshot(&self) -> bool {
        self.trigger.uses_snapshot()
    }

    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        self.trigger.looks_back_for_source(event)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        self.trigger.subscribed_kinds()
    }

    fn source_must_match_event_object(&self, event_kind: EventKind) -> bool {
        self.trigger.source_must_match_event_object(event_kind)
    }

    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        self.trigger.simultaneous_trigger_key(event)
    }

    /// The gate is a functional-zone fact, not rules text.
    fn display(&self) -> String {
        self.trigger.display()
    }
}
