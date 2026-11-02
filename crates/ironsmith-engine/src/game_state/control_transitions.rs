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
            self.update_static_ability_effects()?;
            self.reconcile_continuous_control_changes();
            if !self.continuous_state_is_clean() {
                self.update_static_ability_effects()?;
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
        // A completed instruction observes duration failures even if no player
        // queried a permission during the gap. Such durations never resume.
        let expired = self.effect_store.grant_registry.failed_control_duration_permissions(self);
        self.effect_store.grant_registry.expire_permission_identities(&expired);
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
        // Compare only continuing, unphased battlefield incarnations. Entry,
        // phase-in, and unchanged characteristics are not type-gain events.
        // All differences are computed from the same completed-state pair.
        let mut characteristic_gains = Vec::new();
        for previous in &before {
            let current = &snapshots[&previous.object_id];
            for value in &current.card_types {
                if !previous.card_types.contains(value) {
                    characteristic_gains.push((previous.clone(), current.clone(), ironsmith_core::trigger_model::GainedCharacteristic::CardType(*value)));
                }
            }
            for value in &current.subtypes {
                if !previous.subtypes.contains(value) {
                    characteristic_gains.push((previous.clone(), current.clone(), ironsmith_core::trigger_model::GainedCharacteristic::Subtype(*value)));
                }
            }
            for value in &current.supertypes {
                if !previous.supertypes.contains(value) {
                    characteristic_gains.push((previous.clone(), current.clone(), ironsmith_core::trigger_model::GainedCharacteristic::Supertype(*value)));
                }
            }
        }
        let mut distinct_gains = std::collections::HashSet::new();
        characteristic_gains.retain(|(_, current, characteristic)|
            distinct_gains.insert((current.object_id, *characteristic)));
        // Publish a completed baseline before queueing observations; a later
        // refresh, including one caused while matching, cannot duplicate them.
        self.battlefield_flags_mut().control_event_snapshots = snapshots;
        self.battlefield_flags_mut().control_source_lookback = current_sources;
        for (previous, current, characteristic) in characteristic_gains {
            // A typed fact has no performed-action identity or action receipt.
            // Match after-state observers; control-loss lookback is separate.
            let event = crate::events::CharacteristicGainedEvent {
                permanent: current.object_id, characteristic,
                previous_snapshot: previous, snapshot: current,
            };
            let provenance = self.provenance_graph_mut().alloc_root_event(EventKind::CharacteristicGained);
            self.queue_trigger_event(provenance,
                crate::triggers::TriggerEvent::new_with_provenance(event, provenance));
            let mut receipt = self.effect_store.pending_trigger_events.pop()
                .expect("characteristic transition receipt was queued");
            let mut matched = crate::triggers::TriggerQueue::new();
            crate::game_loop::queue_triggers_from_reported_events(
                self, &mut matched, vec![receipt.clone()], true,
            );
            matched.defer_into_game(self);
            receipt.mark_triggers_captured();
            self.effect_store.pending_trigger_events.push(receipt);
        }
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
            matched.defer_into_game(self);
            receipt.mark_triggers_captured();
            self.effect_store.pending_trigger_events.push(receipt);
        }
    }
}

#[cfg(test)]
mod characteristic_gain_transition_tests {
    use super::*;
    use crate::{Ability, CardDefinitionBuilder, CardId, CardType, Effect, Subtype};
    use crate::types::Supertype;
    use crate::continuous::Modification;
    use crate::effect::Until;
    use crate::effects::{ApplyContinuousEffect, ExecutionContext, execute_effect};
    use crate::target::{ChooseSpec, ObjectFilter};
    use ironsmith_core::trigger_model::{CharacteristicGainTrigger, GainedCharacteristic as C};
    const A: PlayerId = PlayerId(0);
    const B: PlayerId = PlayerId(1);

    fn creature(game: &mut GameState, name: &str) -> ObjectId {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
        game.create_object_from_definition(&definition, A, Zone::Battlefield)
    }
    fn observe(game: &mut GameState, characteristic: C, filter: ObjectFilter) -> ObjectId {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Independent transition observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(crate::triggers::Trigger::characteristic_gained(CharacteristicGainTrigger { filter, characteristic }), vec![Effect::gain_life(1)]))
            .build();
        game.create_object_from_definition(&definition, A, Zone::Battlefield)
    }
    fn change(game: &mut GameState, target: ObjectId, modification: Modification, until: Until) {
        let effect = Effect::new(ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(target), modification, until));
        execute_effect(game, &effect, &mut ExecutionContext::new_default(target, A)).unwrap();
        game.refresh_continuous_state().unwrap();
    }
    fn gains(game: &mut GameState) -> Vec<crate::events::CharacteristicGainedEvent> {
        let events = game.take_pending_trigger_events();
        assert!(events.iter().all(|event| event.downcast::<crate::events::NamedActionEvent>().is_none()),
            "characteristic transitions must not publish custom-action history facts");
        events.into_iter().filter_map(|event|
            event.downcast::<crate::events::CharacteristicGainedEvent>().cloned()).collect()
    }
    #[test]
    fn derived_subtype_gain_notifies_each_independent_observer_once_and_can_recur_after_loss() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let subject = creature(&mut game, "Unrelated actor");
        for _ in 0..2 { observe(&mut game, C::Subtype(Subtype::Rebel), ObjectFilter::creature().you_control()); }
        let named_observer = CardDefinitionBuilder::new(CardId::new(), "Action observer is not a gain observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(crate::triggers::Trigger::from_model(ironsmith_core::trigger_model::Trigger::named_action(
                "characteristic transition subtype rebel".into(), crate::target::PlayerFilter::Any,
                ObjectFilter::default(), None)).unwrap(), vec![Effect::gain_life(100)]))
            .build();
        game.create_object_from_definition(&named_observer, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap(); game.initialize_control_transition_baseline();
        game.take_pending_trigger_entries(); gains(&mut game);
        change(&mut game, subject, Modification::AddSubtypes(vec![Subtype::Rebel]), Until::EndOfTurn);
        let entries = game.take_pending_trigger_entries();
        assert_eq!(entries.len(), 2, "only the two typed observers capture the one actual gain; named actions stay isolated");
        for entry in entries {
            let event = entry.triggering_event.downcast::<crate::events::CharacteristicGainedEvent>().unwrap();
            assert_eq!(event.permanent, subject);
            assert!(event.snapshot.subtypes.contains(&Subtype::Rebel));
            assert!(!event.previous_snapshot.subtypes.contains(&Subtype::Rebel));
        }
        assert_eq!(gains(&mut game).len(), 1);
        game.refresh_continuous_state().unwrap();
        assert!(gains(&mut game).is_empty());
        change(&mut game, subject, Modification::AddSubtypes(vec![Subtype::Rebel]), Until::EndOfTurn);
        assert!(gains(&mut game).is_empty(), "applying the same characteristic twice is not becoming it twice");
        assert!(game.take_pending_trigger_entries().is_empty());
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_characteristics(subject).unwrap().subtypes.contains(&Subtype::Rebel));
        assert!(gains(&mut game).is_empty(), "loss does not emit a gain");
        change(&mut game, subject, Modification::AddSubtypes(vec![Subtype::Rebel]), Until::EndOfTurn);
        assert_eq!(gains(&mut game).len(), 1);
        assert_eq!(game.take_pending_trigger_entries().len(), 2);
    }
    #[test]
    fn grouped_multi_characteristic_change_uses_completed_snapshot_and_current_controller() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let subject = creature(&mut game, "Grouped actor");
        observe(&mut game, C::Supertype(Supertype::Legendary), ObjectFilter::creature().you_control());
        game.refresh_continuous_state().unwrap(); game.initialize_control_transition_baseline();
        gains(&mut game); game.take_pending_trigger_entries();
        game.effect_store.trigger_matching_holds += 1;
        change(&mut game, subject, Modification::AddSupertypes(vec![Supertype::Legendary]), Until::Forever);
        change(&mut game, subject, Modification::AddSubtypes(vec![Subtype::Rebel]), Until::Forever);
        change(&mut game, subject, Modification::AddCardTypes(vec![CardType::Artifact]), Until::Forever);
        assert!(gains(&mut game).is_empty(), "the held operation has not published an intermediate state");
        game.effect_store.trigger_matching_holds -= 1;
        game.refresh_continuous_state().unwrap();
        let events = gains(&mut game);
        assert_eq!(events.len(), 3);
        for event in &events {
            assert_eq!(event.snapshot.controller, A); assert_eq!(event.permanent, subject);
            assert!(event.snapshot.supertypes.contains(&Supertype::Legendary));
            assert!(event.snapshot.subtypes.contains(&Subtype::Rebel));
            assert!(event.snapshot.card_types.contains(&CardType::Artifact));
        }
        assert_eq!(game.take_pending_trigger_entries().len(), 1, "legendary observer fires once, not once per simultaneous gain");
        game.set_current_controller(subject, B).unwrap();
        assert!(gains(&mut game).is_empty(), "control changes alone do not gain an already-present type");
        change(&mut game, subject, Modification::RemoveSupertypes(vec![Supertype::Legendary]), Until::Forever);
        assert!(gains(&mut game).is_empty());
        change(&mut game, subject, Modification::AddSupertypes(vec![Supertype::Legendary]), Until::Forever);
        let regained = gains(&mut game);
        assert_eq!(regained.len(), 1); assert_eq!(regained[0].snapshot.controller, B);
        assert!(game.take_pending_trigger_entries().is_empty(), "you-control filter uses the gain's current controller");
    }
    #[test]
    fn entry_phase_cycle_and_new_incarnation_do_not_gain_existing_characteristics() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let definition = CardDefinitionBuilder::new(CardId::new(), "Already all three")
            .card_types(vec![CardType::Creature, CardType::Artifact])
            .subtypes(vec![Subtype::Rebel]).supertypes(vec![Supertype::Legendary])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
        game.refresh_continuous_state().unwrap(); game.initialize_control_transition_baseline();
        let subject = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap(); assert!(gains(&mut game).is_empty());
        game.phase_out(subject); game.refresh_continuous_state().unwrap();
        game.phase_in(subject); game.refresh_continuous_state().unwrap();
        assert!(gains(&mut game).is_empty());
        game.move_object_by_effect(subject, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(gains(&mut game).is_empty());
    }
}
