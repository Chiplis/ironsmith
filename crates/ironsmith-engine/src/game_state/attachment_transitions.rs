//! Completed attachment transitions share one producer for equip, Aura entry,
//! attachment effects, SBA detachments, and either participant leaving.
use crate::events::{EventKind, ObjectBecameAttachedEvent, ObjectBecameUnattachedEvent};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::object::AttachmentTarget;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

struct Detachment {
    attachment: ObjectId,
    previous_target: AttachmentTarget,
    attachment_snapshot: ObjectSnapshot,
    recipient_snapshot: Option<ObjectSnapshot>,
}
impl GameState {
    fn attachment_target_snapshot(&self, target: AttachmentTarget) -> Option<ObjectSnapshot> {
        let AttachmentTarget::Object(id) = target else {
            return None;
        };
        self.object(id)
            .map(|object| self.cached_object_snapshot_with_calculated_characteristics(object))
    }

    fn capture_detachment(&self, attachment: ObjectId) -> Option<Detachment> {
        let object = self.object(attachment)?;
        let previous_target = object.attached_to?;
        Some(Detachment {
            attachment,
            previous_target,
            attachment_snapshot: self
                .cached_object_snapshot_with_calculated_characteristics(object),
            recipient_snapshot: self.attachment_target_snapshot(previous_target),
        })
    }

    fn commit_detachment(&mut self, detached: Detachment, lookback: Vec<ObjectSnapshot>) {
        self.mark_continuous_state_dirty();
        match detached.previous_target {
            AttachmentTarget::Object(id) => {
                if let Some(parent) = self.object_mut(id) {
                    parent.attachments.retain(|id| *id != detached.attachment);
                }
            }
            AttachmentTarget::Player(id) => {
                if let Some(player) = self.player_mut(id) {
                    player.attachments.retain(|id| *id != detached.attachment);
                }
            }
        }
        if let Some(object) = self.object_mut(detached.attachment) {
            object.attached_to = None;
        }
        let provenance = self
            .provenance_graph_mut()
            .alloc_root_event(EventKind::ObjectBecameUnattached);
        let notification = ObjectBecameUnattachedEvent::new(
            detached.attachment,
            detached.previous_target,
            detached.attachment_snapshot.controller,
            Some(detached.attachment_snapshot),
        )
        .with_previous_target_snapshot(detached.recipient_snapshot);
        let event = TriggerEvent::new_with_provenance(notification, provenance)
            .with_lookback_source_snapshots(lookback);
        self.queue_trigger_event(provenance, event);
    }

    pub fn detach_object_from_current_target(&mut self, attachment: ObjectId) -> bool {
        let Some(detached) = self.capture_detachment(attachment) else {
            return false;
        };
        let lookback = self.trigger_source_lookback_snapshots();
        self.commit_detachment(detached, lookback);
        true
    }

    /// Either participant leaving ends the relation. Capture all participants
    /// before any relation changes so attached continuous effects cannot alter
    /// the characteristics used by another detachment in the same departure.
    pub(crate) fn detach_relations_for_leaving_object(&mut self, object: ObjectId) {
        let Some(source) = self.object(object) else {
            return;
        };
        let mut attachments = source.attachments.clone();
        if source.attached_to.is_some() {
            attachments.push(object);
        }
        let detached = attachments
            .into_iter()
            .filter_map(|attachment| self.capture_detachment(attachment))
            .collect::<Vec<_>>();
        if detached.is_empty() {
            return;
        }
        let lookback = self.trigger_source_lookback_snapshots();
        for detached in detached {
            self.commit_detachment(detached, lookback.clone());
        }
    }

    pub fn attach_object_to_target(
        &mut self,
        attachment: ObjectId,
        target: AttachmentTarget,
    ) -> bool {
        if !self
            .object(attachment)
            .is_some_and(|object| object.zone == Zone::Battlefield)
            || !self.attachment_target_exists(target)
        {
            return false;
        }
        // CR 701.3b: attaching to the same destination does nothing. This is
        // still a valid relation, but no detach/attach event or new timestamp.
        if self
            .object(attachment)
            .is_some_and(|object| object.attached_to == Some(target))
        {
            return true;
        }
        self.detach_object_from_current_target(attachment);
        self.mark_continuous_state_dirty();
        if let Some(object) = self.object_mut(attachment) {
            object.attached_to = Some(target);
        } else {
            return false;
        }
        match target {
            AttachmentTarget::Object(id) => {
                if let Some(parent) = self.object_mut(id)
                    && !parent.attachments.contains(&attachment)
                {
                    parent.attachments.push(attachment);
                }
            }
            AttachmentTarget::Player(id) => {
                if let Some(player) = self.player_mut(id)
                    && !player.attachments.contains(&attachment)
                {
                    player.attachments.push(attachment);
                }
            }
        }
        // Publish completed characteristics after the relation and timestamp
        // are installed. Attach-trigger filters see both participants then.
        self.effect_store
            .continuous_effects
            .record_attachment(attachment);
        if let Some(snapshot) = self
            .object(attachment)
            .map(|object| self.cached_object_snapshot_with_calculated_characteristics(object))
        {
            let recipient = self.attachment_target_snapshot(target);
            let notification = ObjectBecameAttachedEvent::new(
                attachment,
                target,
                snapshot.controller,
                Some(snapshot),
            )
            .with_target_snapshot(recipient);
            let provenance = self
                .provenance_graph_mut()
                .alloc_root_event(EventKind::ObjectBecameAttached);
            self.queue_trigger_event(
                provenance,
                TriggerEvent::new_with_provenance(notification, provenance),
            );
        }
        true
    }
}
