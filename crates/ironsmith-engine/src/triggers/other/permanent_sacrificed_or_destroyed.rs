//! Passive permanent sacrifice and successful-destruction triggers.

use crate::events::EventKind;
use crate::events::permanents::{DestroyEvent, SacrificeEvent};
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::zone::Zone;

fn passive_subject(filter: &ObjectFilter) -> String {
    let text = filter.description();
    if text.starts_with("a ") || text.starts_with("an ") {
        text
    } else {
        let article = if matches!(
            text.chars().next().map(|ch| ch.to_ascii_lowercase()),
            Some('a' | 'e' | 'i' | 'o' | 'u')
        ) {
            "an"
        } else {
            "a"
        };
        format!("{article} {text}")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentSacrificedTrigger {
    pub filter: ObjectFilter,
}

impl TriggerMatcher for PermanentSacrificedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(sacrifice) = event.downcast::<SacrificeEvent>() else {
            return false;
        };
        sacrifice.snapshot.as_ref().is_some_and(|snapshot| {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Sacrifice])
    }

    fn uses_snapshot(&self) -> bool {
        true
    }

    fn display(&self) -> String {
        format!("Whenever {} is sacrificed", passive_subject(&self.filter))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentDestroyedTrigger {
    pub filter: ObjectFilter,
}

impl TriggerMatcher for PermanentDestroyedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(destroy) = event.downcast::<DestroyEvent>() else {
            return false;
        };
        if destroy.final_zone != Some(Zone::Graveyard) {
            return false;
        }
        destroy.snapshot.as_ref().is_some_and(|snapshot| {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        })
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Destroy])
    }

    fn uses_snapshot(&self) -> bool {
        true
    }

    fn display(&self) -> String {
        format!("Whenever {} is destroyed", passive_subject(&self.filter))
    }
}
