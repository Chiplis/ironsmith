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

/// Surface marker for a triggered ability gated by a level-up range.
pub const LEVEL_RANGE_SURFACE_PREFIX: &str = "__ironsmith_level_range:";

impl TriggerMatcher for ConditionQualifiedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.trigger.matches(event, ctx)
            && evaluate_condition_external(
                ctx.game,
                &self.condition,
                &ExternalEvaluationContext {
                    controller: ctx.controller,
                    source: ctx.source_id,
                    defending_player: event
                        .downcast::<crate::events::combat::CreatureAttackedEvent>()
                        .and_then(|attack| {
                            crate::combat_state::defending_player_for_attack_event(
                                ctx.game,
                                attack.target,
                                attack.attacker,
                            )
                        }),
                    filter_source: Some(ctx.source_id),
                    triggering_event: Some(event),
                    iterated_player: event.trigger_player().or_else(|| event.player()),
                    trigger_identity: ctx.trigger_identity,
                    ..Default::default()
                },
            )
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        self.trigger.trigger_count(event)
    }

    fn trigger_count_with_context(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        if self.matches(event, ctx) {
            self.trigger.trigger_count_with_context(event, ctx)
        } else {
            0
        }
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        self.matches(event, ctx)
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

    fn display(&self) -> String {
        // A level-range gate (CR 711.2a) is presentation structure: the
        // ability is printed under its LEVEL header, not with a qualifier.
        if self.surface.starts_with(LEVEL_RANGE_SURFACE_PREFIX) {
            return self.trigger.display();
        }
        let condition = if self.surface.trim().is_empty() {
            crate::runtime_display::describe_condition(&self.condition)
        } else {
            self.surface.trim().to_string()
        };
        // An adverbial qualification ("from anywhere other than exile",
        // "during your turn") reads directly after the event.
        // "by spending four or more mana to activate it" and "enters
        // transformed" are likewise part of the event's own wording.
        // "Whenever a spell or ability an opponent controls destroys a
        // noncreature permanent you control": the destroying agent reads as
        // the subject of the active verb.
        let event = self.trigger.display();
        if let Some(agent) = condition.strip_prefix("by ")
            && let Some(object) = event
                .strip_prefix("Whenever ")
                .and_then(|rest| rest.strip_suffix(" is destroyed"))
        {
            return format!("Whenever {agent} destroys {object}");
        }
        if condition.starts_with("from ")
            || condition.starts_with("during ")
            || condition.starts_with("by ")
            || condition == "transformed"
        {
            return format!("{} {}", self.trigger.display(), condition);
        }
        format!("{} while {}", self.trigger.display(), condition)
    }
}
