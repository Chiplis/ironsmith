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
                if !self
                    .attachment
                    .matches_snapshot(attachment, &ctx.filter_ctx, ctx.game)
                {
                    return false;
                }
                // "a nonland permanent ... with mana value less than or equal
                // to that Aura's mana value" (Eriette): the recipient filter
                // compares against the matched attachment participant.
                if self.recipient.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == ironsmith_core::tag::TRIGGER_ATTACHMENT_TAG
                }) {
                    let mut filter_ctx = ctx.filter_ctx.clone();
                    filter_ctx.tagged_objects.insert(
                        crate::tag::TagKey::from(ironsmith_core::tag::TRIGGER_ATTACHMENT_TAG),
                        vec![attachment.clone()],
                    );
                    return self
                        .recipient
                        .matches_snapshot(recipient, &filter_ctx, ctx.game);
                }
                self.recipient
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
            {
                // "becomes attached to a creature": a bare noun phrase takes
                // its indefinite article.
                let recipient = self.recipient.description();
                let first = recipient.split_whitespace().next().unwrap_or_default();
                if matches!(
                    first,
                    "a" | "an" | "another" | "each" | "target" | "this" | "that" | "the"
                        | "enchanted" | "equipped" | "it" | "you" | "your"
                ) || recipient.is_empty()
                {
                    recipient
                } else if first.starts_with(['a', 'e', 'i', 'o', 'u']) {
                    format!("an {recipient}")
                } else {
                    format!("a {recipient}")
                }
            }
        )
    }
}

/// Player attachments carry a player identity, independently from the attached
/// permanent's frozen snapshot. No target object is invented for a player.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachmentChangedToPlayerTrigger {
    pub attachment: ObjectFilter,
    pub recipient: crate::PlayerFilter,
    pub attached: bool,
}
impl AttachmentChangedToPlayerTrigger {
    pub(crate) fn participants<'a>(&self, event: &'a TriggerEvent) -> Option<(&'a ObjectSnapshot, crate::PlayerId)> {
        let (snapshot, recipient) = if self.attached {
            let event = event.downcast::<ObjectBecameAttachedEvent>()?;
            (event.snapshot.as_ref()?, &event.target)
        } else {
            let event = event.downcast::<ObjectBecameUnattachedEvent>()?;
            (event.snapshot.as_ref()?, &event.previous_target)
        };
        let crate::object::AttachmentTarget::Player(player) = recipient else { return None; };
        Some((snapshot, *player))
    }
}
impl TriggerMatcher for AttachmentChangedToPlayerTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        use crate::filter::PlayerFilterExt as _;
        self.participants(event).is_some_and(|(attachment, recipient)|
            self.attachment.matches_snapshot(attachment, &ctx.filter_ctx, ctx.game)
                && self.recipient.matches_player(recipient, &ctx.filter_ctx))
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![if self.attached { EventKind::ObjectBecameAttached } else { EventKind::ObjectBecameUnattached }])
    }
    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        !self.attached && event.kind() == EventKind::ObjectBecameUnattached
    }
    fn display(&self) -> String {
        format!("Whenever {} becomes {} {}", self.attachment.description(),
            if self.attached { "attached to" } else { "unattached from" },
            crate::triggers::describe_player_filter_subject(&self.recipient))
    }
}
