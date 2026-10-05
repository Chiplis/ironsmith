use crate::events::{EventKind, ObjectBecameAttachedEvent, ObjectBecameUnattachedEvent};
use crate::filter::ObjectFilterExt as _;
use crate::snapshot::ObjectSnapshot;
use crate::target::ObjectFilter;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct AttachmentChangedTrigger {
    pub attachment: ObjectFilter,
    pub recipient: ObjectFilter,
    pub attached: bool,
}
impl AttachmentChangedTrigger {
    pub(crate) fn participants<'a>(
        &self,
        event: &'a TriggerEvent,
    ) -> Option<(&'a ObjectSnapshot, &'a ObjectSnapshot)> {
        if self.attached {
            let event = event.downcast::<ObjectBecameAttachedEvent>()?;
            Some((event.snapshot.as_ref()?, event.target_snapshot.as_ref()?))
        } else {
            let event = event.downcast::<ObjectBecameUnattachedEvent>()?;
            Some((
                event.snapshot.as_ref()?,
                event.previous_target_snapshot.as_ref()?,
            ))
        }
    }
}
impl TriggerMatcher for AttachmentChangedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.participants(event)
            .is_some_and(|(attachment, recipient)| {
                self.attachment
                    .matches_snapshot(attachment, &ctx.filter_ctx, ctx.game)
                    && self
                        .recipient
                        .matches_snapshot(recipient, &ctx.filter_ctx, ctx.game)
            })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![if self.attached {
            EventKind::ObjectBecameAttached
        } else {
            EventKind::ObjectBecameUnattached
        }])
    }
    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        !self.attached && event.kind() == EventKind::ObjectBecameUnattached
    }
    fn display(&self) -> String {
        format!(
            "Whenever {} becomes {} {}",
            self.attachment.description(),
            if self.attached {
                "attached to"
            } else {
                "unattached from"
            },
            self.recipient.description()
        )
    }
}
