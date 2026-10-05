//! Completed lifecycle subjects, including event-time attachment relations.
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::snapshot::ObjectSnapshot;
use crate::target::ObjectFilter;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};

pub(super) fn matches_completed(
    filter: &ObjectFilter,
    snapshot: &ObjectSnapshot,
    ctx: &TriggerContext,
) -> bool {
    let mut filter_ctx = ctx.filter_ctx.clone();
    // An attachment's "equipped/enchanted creature" is the host at this
    // event, even if it was detached or attached elsewhere before dispatch.
    // Empty tags deliberately prevent fallback to today's attachment.
    let host = if snapshot.attachments.contains(&ctx.source_id) {
        vec![snapshot.clone()]
    } else {
        Vec::new()
    };
    filter_ctx
        .tagged_objects
        .insert("equipped".into(), host.clone());
    filter_ctx.tagged_objects.insert("enchanted".into(), host);
    filter.matches_snapshot(snapshot, &filter_ctx, ctx.game)
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermanentMutatesTrigger {
    pub filter: ObjectFilter,
}
impl TriggerMatcher for PermanentMutatesTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(event) = event.downcast::<crate::events::other::MutatedEvent>() else {
            return false;
        };
        event
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| matches_completed(&self.filter, snapshot, ctx))
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::Mutated])
    }
    fn uses_snapshot(&self) -> bool {
        true
    }

    fn display(&self) -> String {
        format!("Whenever {} mutates", self.filter.description())
    }
}
