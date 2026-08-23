//! Event trigger qualified by an event-time `while` condition.

use crate::condition_eval::{ExternalEvaluationContext, evaluate_condition_external};
use crate::effect::Condition;
use crate::events::EventKind;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};
use crate::triggers::{Trigger, TriggerEvent};

#[derive(Debug, Clone)]
pub struct ConditionQualifiedTrigger {
    pub trigger: Trigger,
    pub condition: Condition,
    pub surface: String,
    pub stun_counter_reminder_surface: bool,
}

impl ConditionQualifiedTrigger {
    pub fn new(trigger: Trigger, condition: Condition, surface: String) -> Self {
        Self {
            trigger,
            condition,
            surface,
            stun_counter_reminder_surface: false,
        }
    }

    pub fn with_stun_counter_reminder_surface(mut self) -> Self {
        self.stun_counter_reminder_surface = true;
        self
    }
}

impl TriggerMatcher for ConditionQualifiedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.trigger.matches(event, ctx)
            && evaluate_condition_external(
                ctx.game,
                &self.condition,
                &ExternalEvaluationContext {
                    controller: ctx.controller,
                    source: ctx.source_id,
                    filter_source: Some(ctx.source_id),
                    triggering_event: Some(event),
                    trigger_identity: ctx.trigger_identity,
                    ..Default::default()
                },
            )
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

    fn display(&self) -> String {
        let condition = if self.surface.trim().is_empty() {
            crate::runtime_display::describe_condition(&self.condition)
        } else {
            self.surface.trim().to_string()
        };
        format!("{} while {}", self.trigger.display(), condition)
    }
}
