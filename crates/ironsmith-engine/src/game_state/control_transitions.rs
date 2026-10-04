//! A single completed-state owner for explicit, static and expired control
//! effects. Entry and phasing do not manufacture control-change events.
use super::*;

impl GameState {
    pub(crate) fn control_transition_boundary_is_held(&self) -> bool {
        self.effect_store.trigger_matching_holds > 0
            || self.auxiliary_tracking.simultaneous_action_scope.is_some()
            || self.simultaneous_event_lookback().is_some()
    }
    /// Establish the actual derived state before the next instruction, without
    /// invoking turn procedures or rebuilding unrelated replacement rules.
    pub(crate) fn establish_control_transition_boundary(
        &mut self,
    ) -> Result<(), crate::static_ability_processor::StaticEffectDiscoveryError> {
        let checkpoint = self.clone();
        let result = (|| {
            self.try_update_static_ability_effects(Default::default())?;
            self.reconcile_continuous_control_changes();
            if !self.continuous_state_is_clean() {
                self.try_update_static_ability_effects(Default::default())?;
                self.reconcile_continuous_control_changes();
            }
            self.publish_completed_control_transitions();
            Ok(())
        })();
        if result.is_err() {
            self.restore_execution_checkpoint(checkpoint, false);
        }
        result
    }

    /// Rebase only the observation baseline after a saved game has restored
    /// its actual continuous effects and verified every effective controller.
    /// Rebuilding current facts is not a new game event and must preserve the
    /// serialized summoning-sickness and soulbond state.
    pub fn initialize_control_transition_baseline(&mut self) {
        let snapshots = self.current_control_snapshots();
        let controllers = self
            .battlefield
            .iter()
            .copied()
            .filter_map(|id| {
                self.current_controller(id)
                    .map(|controller| (id, controller))
            })
            .collect();
        let sources = self
            .current_trigger_source_snapshots()
            .into_iter()
            .filter(|snapshot| {
                snapshot.zone != Zone::Battlefield || !self.is_phased_out(snapshot.object_id)
            })
            .collect();
        let flags = self.battlefield_flags_mut();
        flags.control_event_snapshots = snapshots;
        flags.control_source_lookback = sources;
        flags.controller_at_last_refresh = controllers;
        flags.control_transition_pending = false;
    }

    fn current_control_snapshots(&self) -> HashMap<ObjectId, ObjectSnapshot> {
        self.battlefield
            .iter()
            .copied()
            .filter(|id| !self.is_phased_out(*id))
            .filter_map(|id| {
                self.object(id).map(|object| {
                    (
                        id,
                        self.cached_object_snapshot_with_calculated_characteristics(object),
                    )
                })
            })
            .collect()
    }

    pub(crate) fn publish_completed_control_transitions(&mut self) {
        // A coordinated operation may hold matching until all its mutations
        // finish. Preserve the old baseline across that entire operation.
        if self.control_transition_boundary_is_held() {
            self.battlefield_flags_mut().control_transition_pending = true;
            return;
        }
        let snapshots = self.current_control_snapshots();
        let mut before = self
            .battlefield_flags
            .control_event_snapshots
            .values()
            .filter(|snapshot| snapshots.contains_key(&snapshot.object_id))
            .cloned()
            .collect::<Vec<_>>();
        before.sort_by_key(|snapshot| snapshot.object_id);
        // An observer departing in this same operation belongs to the old
        // source set. A separate completed instruction already advances this
        // baseline, so a previously departed observer is not resurrected.
        let lookback = self.battlefield_flags.control_source_lookback.clone();
        let current_sources = self
            .current_trigger_source_snapshots()
            .into_iter()
            .filter(|snapshot| {
                snapshot.zone != Zone::Battlefield || !self.is_phased_out(snapshot.object_id)
            })
            .collect();
        let transitions = before
            .iter()
            .filter_map(|previous| {
                let current = snapshots.get(&previous.object_id)?;
                (previous.controller != current.controller)
                    .then(|| (previous.clone(), current.clone()))
            })
            .collect::<Vec<_>>();
        // Publish a completed baseline before queueing observations; a later
        // refresh, including one caused while matching, cannot duplicate them.
        self.battlefield_flags_mut().control_event_snapshots = snapshots;
        self.battlefield_flags_mut().control_source_lookback = current_sources;
        for (previous, current) in transitions {
            let event = crate::events::ControlChangedEvent::new(
                current.object_id,
                previous.controller,
                current.controller,
            )
            .with_snapshots(previous.clone(), current.clone())
            .with_complete_source_lookback();
            let provenance = self
                .provenance_graph_mut()
                .alloc_root_event(EventKind::ControlChanged);
            let notification =
                crate::triggers::TriggerEvent::new_with_provenance(event, provenance)
                    .with_lookback_source_snapshots(lookback.clone());
            self.record_ui_effect_event(
                "control_change",
                Some(current.controller),
                Some(previous.controller),
                vec![current.stable_id],
                None,
                None,
            );
            self.queue_trigger_event(provenance, notification);
            // Both ordinary gain observers and lookback loss observers must
            // see this completed transition before a later instruction can
            // remove an ability. Keep its physical history receipt afterward.
            let mut receipt = self
                .effect_store
                .pending_trigger_events
                .pop()
                .expect("control receipt was queued");
            let mut matched = crate::triggers::TriggerQueue::new();
            crate::game_loop::queue_triggers_from_reported_events(
                self,
                &mut matched,
                vec![receipt.clone()],
                true,
            );
            self.defer_trigger_entries(matched.take_all());
            receipt.mark_triggers_captured();
            self.effect_store.pending_trigger_events.push(receipt);
        }
    }
}
