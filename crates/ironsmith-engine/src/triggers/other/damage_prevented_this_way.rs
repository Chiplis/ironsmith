//! "Whenever damage [from a <quality> source] is prevented this way" — the
//! delayed triggered ability a prevention spell creates (CR 603.7). The
//! registration links it to the shield created just before it, so only
//! that shield's prevention events (CR 615.5, 615.13) reach this matcher's
//! delayed owner; the matcher itself qualifies the prevented damage's
//! source.

use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct DamagePreventedThisWayTrigger {
    pub source_filter: Option<ObjectFilter>,
}

impl TriggerMatcher for DamagePreventedThisWayTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(prevented) = event.downcast::<crate::events::DamagePreventedEvent>() else {
            return false;
        };
        let Some(filter) = &self.source_filter else {
            return true;
        };
        prevented.applications.iter().any(|application| {
            ctx.game
                .object(application.damage_source)
                .is_some_and(|source| filter.matches(source, &ctx.filter_ctx, ctx.game))
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::DamagePrevented])
    }

    fn display(&self) -> String {
        match &self.source_filter {
            Some(filter) => format!(
                "Whenever damage from {} is prevented this way",
                filter.description()
            ),
            None => "Whenever damage is prevented this way".to_string(),
        }
    }

    fn event_value_amount(&self, event: &TriggerEvent, _ctx: &TriggerContext) -> Option<i32> {
        event
            .downcast::<crate::events::DamagePreventedEvent>()
            .map(|prevented| prevented.amount as i32)
    }
}
