use crate::events::{EventKind, PermanentPhasedInEvent, PermanentPhasedOutEvent};
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::matcher_trait::SimultaneousTriggerKey;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};
#[derive(Debug, Clone, PartialEq)]
pub struct PhasingChangedTrigger {
    pub filter: ObjectFilter,
    pub phased_in: bool,
    pub one_or_more: bool,
}
impl TriggerMatcher for PhasingChangedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let snapshot = if self.phased_in {
            event
                .downcast::<PermanentPhasedInEvent>()
                .and_then(|event| event.snapshot.as_ref())
        } else {
            event
                .downcast::<PermanentPhasedOutEvent>()
                .and_then(|event| event.snapshot.as_ref())
        };
        snapshot.is_some_and(|snapshot| {
            self.filter
                .matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![if self.phased_in {
            EventKind::PermanentPhasedIn
        } else {
            EventKind::PermanentPhasedOut
        }])
    }
    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        !self.phased_in && event.kind() == EventKind::PermanentPhasedOut
    }
    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        let kind = if self.phased_in {
            EventKind::PermanentPhasedIn
        } else {
            EventKind::PermanentPhasedOut
        };
        (self.one_or_more && event.kind() == kind).then_some(SimultaneousTriggerKey::PhasingBatch {
            phased_in: self.phased_in,
        })
    }
    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        self.matches(event, ctx).then_some(1)
    }
    fn display(&self) -> String {
        let mut filter = self.filter.clone();
        filter.set_plural_object_noun_surface(self.one_or_more);
        format!(
            "Whenever {}{} {} {}",
            if self.one_or_more { "one or more " } else { "" },
            filter.description(),
            if self.one_or_more { "phase" } else { "phases" },
            if self.phased_in { "in" } else { "out" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};
    use crate::triggers::{AnyOfTrigger, Trigger};
    #[test]
    fn alternative_matchers_retain_agreed_grouping_keys_without_collapsing_singular_arms() {
        let grouped = Trigger::new(PhasingChangedTrigger {
            filter: ObjectFilter::permanent(),
            phased_in: false,
            one_or_more: true,
        });
        let singular = Trigger::new(PhasingChangedTrigger {
            filter: ObjectFilter::creature(),
            phased_in: false,
            one_or_more: false,
        });
        let event = TriggerEvent::new(
            PermanentPhasedOutEvent::new(ObjectId::from_raw(1), PlayerId(0), None),
            Default::default(),
        );
        let key = Some(SimultaneousTriggerKey::PhasingBatch { phased_in: false });
        assert_eq!(
            Trigger::either(grouped.clone(), Trigger::this_enters_battlefield())
                .simultaneous_trigger_key(&event),
            key
        );
        assert_eq!(
            AnyOfTrigger {
                branches: vec![grouped.clone(), grouped.clone()]
            }
            .simultaneous_trigger_key(&event),
            key
        );
        assert_eq!(
            Trigger::either(grouped, singular).simultaneous_trigger_key(&event),
            None
        );
    }
}
