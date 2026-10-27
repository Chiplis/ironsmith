//! A one-shot duration starts with its actual stack announcement or trigger,
//! independently of provenance-node allocation order (CR 610.3a-b/610.4b-c).
use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct DurationWindowStart(usize);

impl GameState {
    pub(crate) fn duration_window_now(&self) -> DurationWindowStart {
        DurationWindowStart(self.turn_store.action_history.len())
    }

    pub(crate) fn retain_duration_window(&mut self, owner: LibraryTopAnnouncement) {
        if matches!(owner, LibraryTopAnnouncement::Land(_)) {
            return;
        }
        let start = self.duration_window_now();
        self.auxiliary_tracking_mut()
            .duration_windows
            .entry(owner)
            .or_insert(start);
    }

    pub(crate) fn retain_stack_duration_window(
        &mut self,
        object: ObjectId,
        start: DurationWindowStart,
    ) {
        self.auxiliary_tracking_mut()
            .duration_windows
            .insert(LibraryTopAnnouncement::Cast(object), start);
    }

    pub(crate) fn duration_window_for_owner(
        &self,
        owner: LibraryTopAnnouncement,
    ) -> Option<DurationWindowStart> {
        self.auxiliary_tracking
            .duration_windows
            .get(&owner)
            .copied()
    }

    pub(crate) fn duration_window_for_stack_entry(
        &self,
        entry: &StackEntry,
    ) -> Option<DurationWindowStart> {
        // A copied ability has its own physical stack stand-in. Its window
        // starts at copy insertion, rather than its original trigger event.
        if let Some(start) =
            self.duration_window_for_owner(LibraryTopAnnouncement::Cast(entry.object_id))
        {
            return Some(start);
        }
        if let Some(event) = &entry.triggering_event {
            // A trigger-match annotation may point at a queued node which
            // itself owns the chronological observation. Check each exact
            // identity before following another annotation; node allocation
            // order never substitutes for completed history.
            let mut event_provenance = event.provenance();
            loop {
                if let Some((index, _)) = self
                    .turn_store
                    .action_history
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, record)| record.event.provenance() == event_provenance)
                {
                    return Some(DurationWindowStart(index + 1));
                }
                let node = self.provenance_graph().node(event_provenance)?;
                if !matches!(
                    node.kind,
                    crate::provenance::ProvenanceNodeKind::TriggerMatched { .. }
                        | crate::provenance::ProvenanceNodeKind::TriggerQueued
                ) {
                    return None;
                }
                event_provenance = node.parent?;
            }
        }
        self.duration_window_for_owner(LibraryTopAnnouncement::Activation(entry.provenance))
    }

    pub(crate) fn duration_event_occurred(
        &self,
        start: Option<DurationWindowStart>,
        mut matches: impl FnMut(&crate::triggers::TriggerEvent) -> bool,
    ) -> Result<bool, crate::effects::ExecutionError> {
        let Some(DurationWindowStart(cursor)) = start else {
            return Ok(false);
        };
        if cursor > self.turn_store.action_history.len() {
            return Err(crate::effects::ExecutionError::IncompleteEvidence(
                "duration window is outside this native history branch".into(),
            ));
        }
        Ok(self
            .turn_store
            .action_history
            .iter()
            .skip(cursor)
            .any(|record| matches(&record.event)))
    }
}

#[cfg(test)]
#[path = "duration_window_tests.rs"]
mod tests;
