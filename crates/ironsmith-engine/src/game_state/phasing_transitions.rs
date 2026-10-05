//! One phasing instruction changes all direct and indirect participants before
//! completed notifications are published. This never changes zones or links.
use crate::events::{EventKind, PermanentPhasedInEvent, PermanentPhasedOutEvent};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::AttachmentTarget;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
use std::collections::{BTreeMap, BTreeSet};

fn gather_out(
    game: &GameState,
    id: ObjectId,
    controller: PlayerId,
    indirect: bool,
    out: &mut BTreeMap<ObjectId, (PlayerId, bool)>,
) {
    if out.contains_key(&id) || game.is_phased_out(id) {
        return;
    }
    let Some(object) = game
        .object(id)
        .filter(|object| object.zone == Zone::Battlefield)
    else {
        return;
    };
    out.insert(id, (controller, indirect));
    for attachment in &object.attachments {
        gather_out(game, *attachment, controller, true, out);
    }
}
fn gather_in(game: &GameState, id: ObjectId, into: &mut BTreeSet<ObjectId>) {
    if into.contains(&id) || game.is_phase_out_held(id) || !game.is_phased_out(id) {
        return;
    }
    let Some(object) = game
        .object(id)
        .filter(|object| object.zone == Zone::Battlefield)
    else {
        return;
    };
    into.insert(id);
    for attachment in &object.attachments {
        if game
            .battlefield_flags
            .indirectly_phased_out
            .contains(attachment)
        {
            gather_in(game, *attachment, into);
        }
    }
}
impl GameState {
    pub fn phase_out(&mut self, id: ObjectId) {
        self.phase_out_simultaneously(&[id]);
    }
    pub fn phase_in(&mut self, id: ObjectId) {
        self.phase_in_simultaneously(&[id]);
    }
    pub fn phase_out_simultaneously(&mut self, ids: &[ObjectId]) {
        self.phase_simultaneously(ids, &[]);
    }
    pub fn phase_in_simultaneously(&mut self, ids: &[ObjectId]) {
        self.phase_simultaneously(&[], ids);
    }

    /// CR 502.1 / 702.26: the turn-based exchange phases its incoming and
    /// outgoing sets simultaneously. Other callers supply one instruction.
    pub(crate) fn phase_simultaneously(&mut self, phase_out: &[ObjectId], phase_in: &[ObjectId]) {
        let candidates: BTreeSet<_> = phase_out
            .iter()
            .copied()
            .filter(|id| {
                !self.is_phased_out(*id)
                    && self
                        .object(*id)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
            })
            .collect();
        // CR 702.26h: an attachment selected with its host phases indirectly,
        // regardless of the order they were selected.
        let host_in_set = |id: ObjectId| {
            let mut seen = BTreeSet::new();
            let mut current = id;
            while seen.insert(current) {
                let Some(AttachmentTarget::Object(host)) =
                    self.object(current).and_then(|object| object.attached_to)
                else {
                    return false;
                };
                if candidates.contains(&host) {
                    return true;
                }
                current = host;
            }
            false
        };
        let mut outgoing = BTreeMap::new();
        for id in candidates.iter().copied().filter(|id| !host_in_set(*id)) {
            if let Some(controller) = self.current_controller(id) {
                gather_out(self, id, controller, false, &mut outgoing);
            }
        }
        let mut incoming = BTreeSet::new();
        for id in phase_in {
            gather_in(self, *id, &mut incoming);
        }
        if outgoing.is_empty() && incoming.is_empty() {
            return;
        }

        let lookback = self.trigger_source_lookback_snapshots();
        let before: BTreeMap<ObjectId, ObjectSnapshot> = outgoing
            .keys()
            .filter_map(|id| {
                self.object(*id).map(|object| {
                    (
                        *id,
                        self.cached_object_snapshot_with_calculated_characteristics(object),
                    )
                })
            })
            .collect();
        self.mark_continuous_state_dirty();
        for (&id, &(controller, indirect)) in &outgoing {
            if let Some(snapshot) = before.get(&id) {
                for entry in &mut self.stack {
                    if entry.is_ability && entry.object_id == id {
                        entry.source_snapshot = Some(snapshot.clone());
                    }
                }
            }
            let flags = self.battlefield_flags_mut();
            flags.phased_out.insert(id);
            flags.phased_out_under_controller.insert(id, controller);
            if indirect {
                flags.indirectly_phased_out.insert(id);
            } else {
                flags.indirectly_phased_out.remove(&id);
            }
            self.remove_object_from_combat(id);
            self.remove_attacked_permanent_from_combat(id, None);
        }
        for id in &incoming {
            let flags = self.battlefield_flags_mut();
            flags.phased_out.remove(id);
            flags.phased_out_under_controller.remove(id);
            flags.indirectly_phased_out.remove(id);
            for held in flags.phase_out_holds_by_source.values_mut() {
                held.remove(id);
            }
        }
        self.mark_continuous_state_dirty();
        self.effect_store
            .continuous_effects
            .expire_presence_durations_for_phased_objects(
                &outgoing.keys().copied().collect::<Vec<_>>(),
            );
        self.expire_condition_ended_prevention_shields();
        let kind = if outgoing.is_empty() {
            EventKind::PermanentPhasedIn
        } else {
            EventKind::PermanentPhasedOut
        };
        let batch = self.provenance_graph_mut().alloc_root_event(kind);
        for (&id, snapshot) in &before {
            self.record_ui_effect_event(
                "phase_out",
                None,
                None,
                vec![snapshot.stable_id],
                None,
                None,
            );
            let event = TriggerEvent::new_with_provenance(
                PermanentPhasedOutEvent::new(id, snapshot.controller, Some(snapshot.clone()))
                    .with_complete_source_lookback(),
                batch,
            )
            .with_simultaneous_batch(batch)
            .with_lookback_source_snapshots(lookback.clone());
            self.queue_trigger_event(batch, event);
        }
        for id in incoming {
            if let Some(snapshot) = self
                .object(id)
                .map(|object| self.cached_object_snapshot_with_calculated_characteristics(object))
            {
                self.record_ui_effect_event(
                    "phase_in",
                    None,
                    None,
                    vec![snapshot.stable_id],
                    None,
                    None,
                );
                let event = TriggerEvent::new_with_provenance(
                    PermanentPhasedInEvent::new(id, snapshot.controller, Some(snapshot)),
                    batch,
                )
                .with_simultaneous_batch(batch);
                self.queue_trigger_event(batch, event);
            }
        }
    }
}
