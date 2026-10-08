//! "Whenever [filter] becomes tapped" trigger.

use crate::events::EventKind;
use crate::events::other::PermanentTappedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentBecomesTappedTrigger {
    pub filter: ObjectFilter,
    pub one_or_more: bool,
}

impl PermanentBecomesTappedTrigger {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: false,
        }
    }
}

impl TriggerMatcher for PermanentBecomesTappedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::PermanentTapped {
            return false;
        }
        let Some(e) = event.downcast::<PermanentTappedEvent>() else {
            return false;
        };
        if let Some(snapshot) = &e.snapshot {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        } else if let Some(obj) = ctx.game.object(e.permanent) {
            self.filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn simultaneous_trigger_key(
        &self,
        event: &TriggerEvent,
    ) -> Option<crate::triggers::matcher_trait::SimultaneousTriggerKey> {
        (self.one_or_more && event.kind() == EventKind::PermanentTapped).then_some(
            crate::triggers::matcher_trait::SimultaneousTriggerKey::TapStateBatch { tapped: true },
        )
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        self.matches(event, ctx).then_some(1)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::PermanentTapped])
    }

    fn display(&self) -> String {
        if self.one_or_more {
            let mut filter = self.filter.clone();
            filter.set_plural_object_noun_surface(true);
            let mut subject = filter.description();
            // A filter without a plural surface still reads as a singular
            // noun phrase ("a creature you control").
            if subject.starts_with("a ") || subject.starts_with("an ") {
                subject = crate::triggers::combat::pluralize_one_or_more_attack_subject(&subject);
            }
            format!("Whenever one or more {subject} become tapped")
        } else {
            format!("Whenever {} becomes tapped", self.filter.description())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = PermanentBecomesTappedTrigger::new(ObjectFilter::creature());
        assert!(trigger.display().contains("becomes tapped"));
    }
}
