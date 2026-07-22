use super::*;

#[derive(Debug, Clone, Default)]
pub(crate) struct PreparedEtbChoices {
    pub(crate) chosen_color: Option<crate::color::Color>,
    pub(crate) chosen_basic_land_type: Option<crate::types::Subtype>,
    pub(crate) chosen_land_type: Option<crate::types::Subtype>,
    pub(crate) chosen_creature_type: Option<crate::types::Subtype>,
    pub(crate) chosen_card_type: Option<crate::types::CardType>,
    pub(crate) chosen_player: Option<PlayerId>,
    pub(crate) chosen_named_option: Option<String>,
    pub(crate) noted_life_total: Option<i32>,
    pub(crate) power_toughness_choices:
        Vec<(i32, i32, Vec<crate::static_abilities::StaticAbility>)>,
    pub(crate) battle_protector: Option<PlayerId>,
    pub(crate) discard_hand: bool,
    pub(crate) as_enters_counters: Vec<(crate::object::CounterType, u32)>,
    pub(crate) as_enters_tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    pub(crate) transfer_as_enters_source_links: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedEtbEntry {
    pub(crate) result: crate::events::processing::EtbEventResult,
    pub(crate) choices: PreparedEtbChoices,
}

fn as_enters_effect_program_from_ability(
    ability: &crate::ability::Ability,
) -> Option<(crate::resolution::ResolutionProgram, bool)> {
    let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
        program,
        also_turns_face_up,
        ..
    } = &static_ability.compiled_model()?.payload
    else {
        return None;
    };
    Some((program.clone(), *also_turns_face_up))
}

#[derive(Debug, Clone, Default)]
struct AsEntersProgramExecution {
    ran: bool,
    tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
}

fn merge_retained_tagged_objects(
    destination: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    source: &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
) {
    for (tag, snapshots) in source {
        let retained = destination.entry(tag.clone()).or_default();
        for snapshot in snapshots {
            if retained
                .iter()
                .all(|existing| existing.stable_id != snapshot.stable_id)
            {
                retained.push(snapshot.clone());
            }
        }
    }
}

impl GameState {
    /// CR 400.4a: an instant or sorcery card cannot enter the battlefield.
    ///
    /// This is a zone-change rule, not a replacement effect, so callers must
    /// check it before proposing ETB replacements or collecting entry choices.
    pub(crate) fn card_cannot_enter_battlefield(&self, object_id: ObjectId) -> bool {
        self.object(object_id).is_some_and(|object| {
            object.kind == crate::object::ObjectKind::Card
                && object
                    .card_types
                    .iter()
                    .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery))
        })
    }

    fn execute_as_enters_effect_programs_from_abilities(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        abilities: &[crate::ability::Ability],
        turns_face_up_only: bool,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<AsEntersProgramExecution, crate::game_loop::GameLoopError> {
        let programs = abilities
            .iter()
            .filter_map(as_enters_effect_program_from_ability)
            .filter_map(|(program, also_turns_face_up)| {
                (!turns_face_up_only || also_turns_face_up).then_some(program)
            })
            .collect::<Vec<_>>();
        if programs.is_empty() {
            return Ok(AsEntersProgramExecution::default());
        }

        let mut execution = AsEntersProgramExecution {
            ran: true,
            ..AsEntersProgramExecution::default()
        };
        let optional_costs_paid = self
            .object(source)
            .map(|object| object.optional_costs_paid.clone())
            .unwrap_or_default();
        for program in programs {
            let provenance = self.provenance_graph_mut().alloc_root(
                crate::provenance::ProvenanceNodeKind::EffectExecution { source, controller },
            );
            let mut context =
                crate::effects::ExecutionContext::new(source, controller, decision_maker)
                    .with_optional_costs_paid(optional_costs_paid.clone())
                    .with_cause(crate::events::cause::EventCause::from_effect(
                        source, controller,
                    ))
                    .with_provenance(provenance);
            let _ = crate::game_loop::execute_resolution_program(
                self,
                &mut context,
                controller,
                source,
                &program,
                None,
                &[],
            )?;
            let awaiting_choice = context.decision_maker.awaiting_choice();
            merge_retained_tagged_objects(&mut execution.tagged_objects, &context.tagged_objects);
            if awaiting_choice {
                return Ok(execution);
            }
        }
        Ok(execution)
    }

    pub(crate) fn execute_as_enters_effect_programs_for_turn_face_up(
        &mut self,
        source: ObjectId,
        controller: PlayerId,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<(), crate::game_loop::GameLoopError> {
        let abilities = self
            .object(source)
            .map(|object| object.abilities_vec())
            .unwrap_or_default();
        let execution = self.execute_as_enters_effect_programs_from_abilities(
            source,
            controller,
            &abilities,
            true,
            decision_maker,
        )?;
        if let Some(object) = self.object_mut(source) {
            merge_retained_tagged_objects(
                &mut object.cast_tagged_objects,
                &execution.tagged_objects,
            );
        }
        Ok(())
    }

    pub fn move_object(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
    ) -> Option<ObjectId> {
        self.move_object_with_snapshot(old_id, new_zone, cause, None)
    }

    pub(crate) fn move_object_with_snapshot(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        lki_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    ) -> Option<ObjectId> {
        let pre_event_lookback_source_snapshots = if self
            .may_have_triggered_abilities_for_event_kind(crate::events::EventKind::ZoneChange)
        {
            self.trigger_source_lookback_snapshots()
        } else {
            Vec::new()
        };
        self.move_object_with_snapshot_and_pre_event_lookback(
            old_id,
            new_zone,
            cause,
            lki_snapshot,
            &pre_event_lookback_source_snapshots,
        )
    }

    pub(crate) fn move_object_with_snapshot_and_pre_event_lookback(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        lki_snapshot: Option<crate::snapshot::ObjectSnapshot>,
        pre_event_lookback_source_snapshots: &[crate::snapshot::ObjectSnapshot],
    ) -> Option<ObjectId> {
        // CR 311.2/312.2: planar cards remain in the command zone even if an
        // effect attempts to move them. Turning them face down is handled by
        // the Planechase procedure because it creates a new command-zone object.
        if self.is_planar_card(old_id) && new_zone != Zone::Command {
            return Some(old_id);
        }
        if self.is_vanguard_card(old_id) && new_zone != Zone::Command {
            return Some(old_id);
        }
        // CR 314.2: scheme cards remain in the command zone. Setting one in
        // motion or turning it face down changes its status, not its zone.
        if self.is_scheme_card(old_id) && new_zone != Zone::Command {
            return Some(old_id);
        }
        // CR 315.3: conspiracy cards remain in command. Turning an agenda
        // conspiracy face up changes only its status.
        if self.is_conspiracy_card(old_id) && new_zone != Zone::Command {
            return Some(old_id);
        }
        if new_zone == Zone::Battlefield && self.card_cannot_enter_battlefield(old_id) {
            return None;
        }
        let was_face_down = self.is_face_down(old_id);
        let preserved_exile_viewers = if self
            .objects
            .get(&old_id)
            .is_some_and(|obj| obj.zone == Zone::Exile)
        {
            self.exile_tracking_mut()
                .face_down_exile_viewers
                .remove(&old_id)
        } else {
            None
        };
        // Capture a full pre-move snapshot for LKI-based trigger matching.
        let pre_move_snapshot = lki_snapshot.or_else(|| {
            self.objects
                .get(&old_id)
                .map(|obj| self.cached_object_snapshot_with_calculated_characteristics(obj))
        });
        if self
            .objects
            .get(&old_id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
            && new_zone != Zone::Battlefield
        {
            self.release_phase_out_holds_for_source(old_id);
        }
        if let Some(snapshot) = pre_move_snapshot.as_ref() {
            for entry in &mut self.stack {
                if entry.is_ability
                    && entry
                        .triggering_event
                        .as_ref()
                        .and_then(|event| event.object_id())
                        .is_some_and(|object_id| object_id == old_id)
                {
                    entry.tagged_objects.insert(
                        crate::tag::TagKey::from("triggering"),
                        vec![snapshot.clone()],
                    );
                    entry
                        .tagged_objects
                        .entry(crate::tag::TagKey::from("__it__"))
                        .or_insert_with(|| vec![snapshot.clone()]);
                }
                for tagged_snapshots in entry.tagged_objects.values_mut() {
                    for tagged_snapshot in tagged_snapshots {
                        if (tagged_snapshot.object_id == old_id
                            || tagged_snapshot.stable_id == snapshot.stable_id)
                            && tagged_snapshot.zone == snapshot.zone
                        {
                            *tagged_snapshot = snapshot.clone();
                        }
                    }
                }
                if entry.is_ability
                    && (entry.object_id == old_id
                        || entry
                            .source_stable_id
                            .is_some_and(|id| id == snapshot.stable_id))
                {
                    let should_update_source_lki = entry
                        .source_snapshot
                        .as_ref()
                        .is_none_or(|source_snapshot| source_snapshot.zone == snapshot.zone);
                    if !should_update_source_lki {
                        continue;
                    }
                    entry.source_stable_id = Some(snapshot.stable_id);
                    entry
                        .source_name
                        .get_or_insert_with(|| snapshot.name.to_string());
                    entry.source_snapshot = Some(snapshot.clone());
                }
            }
        }

        let old_object = ObjectStore::into_owned_object(self.objects.remove(&old_id)?);
        self.turn_store.forecast_revealed_hand_cards.remove(&old_id);
        let hidden_card_info = self.auxiliary_tracking_mut().hidden_cards.remove(&old_id);
        self.auxiliary_tracking_mut()
            .sector_designations
            .remove(&old_id);
        self.stable_id_index.remove(&old_object.stable_id);
        self.commander_tracking_mut()
            .declined_command_zone_moves
            .remove(&old_id);
        let old_zone = old_object.zone;
        let owner = old_object.owner;

        let preserves_exile_grants_for_adventure_stack_cast = old_zone == Zone::Exile
            && new_zone == Zone::Stack
            && crate::decision::spell_has_adventure_half(self, &old_object);
        if old_zone != new_zone && !preserves_exile_grants_for_adventure_stack_cast {
            self.effect_store
                .grant_registry
                .remove_stable_card_grants_for_zone(old_object.stable_id, old_zone);
        }
        if old_zone == Zone::Stack && new_zone != Zone::Exile {
            self.effect_store
                .grant_registry
                .remove_stable_card_grants_for_zone(old_object.stable_id, Zone::Exile);
        }

        if let Some(target) = old_object.attached_to {
            match target {
                AttachmentTarget::Object(id) => {
                    if let Some(parent) = self.object_mut(id) {
                        parent.attachments.retain(|existing| *existing != old_id);
                    }
                }
                AttachmentTarget::Player(id) => {
                    if let Some(player) = self.player_mut(id) {
                        player.attachments.retain(|existing| *existing != old_id);
                    }
                }
            }
        }

        // Remove from old zone index
        self.remove_from_zone_index(old_id, old_zone, owner);

        // Clear state from old zone's extension maps
        if old_zone == Zone::Battlefield {
            self.clear_battlefield_state(old_id);
            self.clear_player_control_from_source(old_object.stable_id);
        }
        if old_zone == Zone::Exile {
            self.clear_exile_state(old_id);
        }
        if old_zone == Zone::Stack {
            self.exile_tracking_mut()
                .cast_origin_snapshots
                .remove(&old_id);
        }

        if old_zone == Zone::Battlefield
            && new_zone != Zone::Battlefield
            && let Some(merged) = self.merged_permanent(old_object.stable_id).cloned()
        {
            let component_destinations = self
                .commander_tracking_mut()
                .pending_merged_component_destinations
                .remove(&old_object.stable_id);
            let mut result_object_ids = Vec::with_capacity(merged.components.len());
            for (index, component) in merged.components.iter().enumerate() {
                let component_zone = component_destinations
                    .as_ref()
                    .and_then(|destinations| destinations.get(index))
                    .copied()
                    .unwrap_or(new_zone);
                let new_component_id =
                    self.create_merged_component_object(component, component_zone)?;
                result_object_ids.push(new_component_id);
            }
            self.commander_tracking_mut()
                .merged_permanents
                .remove(&old_object.stable_id);

            use crate::events::zones::ZoneChangeEvent;
            use crate::triggers::TriggerEvent;

            let event = ZoneChangeEvent::with_results(
                old_id,
                result_object_ids.clone(),
                old_zone,
                new_zone,
                cause,
                pre_move_snapshot,
            );
            let event_provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::ZoneChange);
            self.queue_trigger_event(
                event_provenance,
                TriggerEvent::new_with_provenance(event, event_provenance)
                    .with_lookback_source_snapshots(pre_event_lookback_source_snapshots.to_vec()),
            );
            self.record_zone_change_results(old_id, result_object_ids.clone());
            if old_zone != new_zone {
                for result_object_id in &result_object_ids {
                    self.record_ui_zone_transition(old_id, *result_object_id, old_zone, new_zone);
                }
            }

            #[cfg(debug_assertions)]
            self.debug_assert_zone_consistency();
            self.reconcile_ring_bearers();
            return result_object_ids.first().copied();
        }

        if old_zone == Zone::Battlefield
            && new_zone != Zone::Battlefield
            && let Some(melded) = self.melded_permanent(old_object.stable_id).cloned()
        {
            let mut result_object_ids = Vec::with_capacity(melded.components.len());
            for component in &melded.components {
                let new_component_id = self.create_meld_component_object(component, new_zone)?;
                result_object_ids.push(new_component_id);
            }
            self.commander_tracking_mut()
                .melded_permanents
                .remove(&old_object.stable_id);

            use crate::events::zones::ZoneChangeEvent;
            use crate::triggers::TriggerEvent;

            let event = ZoneChangeEvent::with_results(
                old_id,
                result_object_ids.clone(),
                old_zone,
                new_zone,
                cause,
                pre_move_snapshot,
            );
            let event_provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::ZoneChange);
            self.queue_trigger_event(
                event_provenance,
                TriggerEvent::new_with_provenance(event, event_provenance)
                    .with_lookback_source_snapshots(pre_event_lookback_source_snapshots.to_vec()),
            );
            self.record_zone_change_results(old_id, result_object_ids.clone());
            if old_zone != new_zone {
                for result_object_id in &result_object_ids {
                    self.record_ui_zone_transition(old_id, *result_object_id, old_zone, new_zone);
                }
            }

            #[cfg(debug_assertions)]
            self.debug_assert_zone_consistency();

            self.reconcile_ring_bearers();

            return result_object_ids.first().copied();
        }

        // Create new object with new ID (zone change = new object per rule 400.7)
        let new_id = self.new_object_id();
        let mut new_object = old_object;
        new_object.id = new_id;
        new_object.zone = new_zone;
        if old_zone == Zone::Stack && new_zone != Zone::Stack {
            new_object.end_splice_cast_overlay();
        }
        if old_zone == Zone::Stack
            && new_zone == Zone::Battlefield
            && matches!(new_object.kind, crate::object::ObjectKind::SpellCopy)
        {
            new_object.kind = crate::object::ObjectKind::Token;
        }
        // Counters are tied to the object instance, not to the physical card.
        // `move_object` always creates the new object for the destination.
        new_object.counters.clear();

        // Reset zone-specific state on the object
        new_object.attached_to = None;
        new_object.attachments.clear();
        // Casting-contribution state should not persist across arbitrary zone changes.
        // Preserve it only for Stack -> Battlefield (a spell resolving into a permanent).
        let preserve_face_down_overlay =
            new_zone == Zone::Battlefield && new_object.face_down_cast_state.is_some();
        let preserve_bestow_overlay =
            new_zone == Zone::Battlefield && new_object.bestow_cast_state.is_some();
        let preserve_prototype_overlay = matches!(new_zone, Zone::Stack | Zone::Battlefield)
            && new_object.prototype_cast_state.is_some();
        let preserve_temporary_static_ability_grants =
            old_zone == Zone::Stack && new_zone == Zone::Battlefield;
        let preserve_cast_tags =
            new_zone == Zone::Stack || (old_zone == Zone::Stack && new_zone == Zone::Battlefield);
        let preserve_optional_costs_paid = old_zone == Zone::Stack && new_zone == Zone::Battlefield;
        let preserve_x_value = old_zone == Zone::Stack && new_zone == Zone::Battlefield;
        if !preserve_prototype_overlay {
            new_object.end_prototype_cast_overlay();
        }
        if !preserve_face_down_overlay && !preserve_bestow_overlay {
            new_object.keyword_payment_contributions_to_cast.clear();
            new_object.bestow_cast_state = None;
            new_object.face_down_cast_state = None;
        }
        if !preserve_x_value {
            new_object.x_value = None;
        }
        if !preserve_cast_tags {
            new_object.cast_tagged_objects.clear();
        }
        if !preserve_temporary_static_ability_grants {
            new_object.temporary_static_ability_grants.clear();
        }
        if !preserve_optional_costs_paid {
            new_object.optional_costs_paid = crate::cost::OptionalCostsPaid::default();
        }
        let ends_stack_text_or_effect_overlay = old_zone == Zone::Stack
            && new_zone != Zone::Stack
            && new_object
                .cast_alternative_method
                .as_deref()
                .is_some_and(|method| {
                    method.overload_effects().is_some()
                        || method.cleave_effects().is_some()
                        || method.awaken_effects().is_some()
                });
        if ends_stack_text_or_effect_overlay
            && let Some(card_id) = new_object.card
            && let Some(handles) = self.object_store.card_shared.get(&card_id)
        {
            new_object.restore_printed_spell_effect(handles);
        }
        new_object.cast_alternative_method = None;

        if old_zone == Zone::Stack
            && new_zone != Zone::Stack
            && new_object.subtypes.contains(&Subtype::Adventure)
            && let Some(front_def) = self.linked_face_definition_by_name_or_id(
                new_object.other_face_name.as_deref(),
                new_object.other_face,
            )
        {
            let handles = self.object_store.shared_handles_for_definition(&front_def);
            new_object.apply_definition_face_with_shared(&front_def, &handles);
        }
        if old_zone == Zone::Exile
            && new_zone == Zone::Battlefield
            && new_object.linked_face_layout == LinkedFaceLayout::TransformLike
            && let Some(front_def) =
                self.default_face_definition_for_transform_like_return(&new_object)
        {
            let handles = self.object_store.shared_handles_for_definition(&front_def);
            new_object.apply_definition_face_with_shared(&front_def, &handles);
        }

        // Set battlefield state for new permanents
        if new_zone == Zone::Battlefield {
            self.set_summoning_sick(new_id);
        }

        self.add_object(new_object);
        if let Some(mut info) = hidden_card_info {
            let audit_info = info.clone();
            info.zone = new_zone;
            self.auxiliary_tracking_mut()
                .hidden_cards
                .insert(new_id, info);
            self.push_hidden_info_operation(HiddenInfoOperation::HiddenMove {
                owner: audit_info.owner,
                old_object_id: old_id,
                new_object_id: new_id,
                from: old_zone,
                to: new_zone,
                slot: audit_info.slot,
                commitment: audit_info.commitment,
            });
        }

        if new_zone == Zone::Battlefield
            && (was_face_down
                || self
                    .object(new_id)
                    .is_some_and(|obj| obj.face_down_cast_state.is_some()))
        {
            self.set_face_down(new_id);
        }
        if old_zone == Zone::Exile && new_zone == Zone::Exile && was_face_down {
            self.set_face_down(new_id);
            if let Some(viewers) = preserved_exile_viewers {
                for viewer in viewers {
                    self.grant_face_down_exile_view(new_id, viewer);
                }
            }
        }

        if old_zone != new_zone {
            self.record_ui_zone_transition(old_id, new_id, old_zone, new_zone);
        }

        // Record entry timestamp per Rule 613.7d when entering the battlefield
        if new_zone == Zone::Battlefield {
            self.effect_store.continuous_effects.record_entry(new_id);
            self.handle_day_night_object_entered(new_id);
        }

        // Queue zone change event for triggers.
        if old_zone != new_zone {
            use crate::events::zones::ZoneChangeEvent;
            use crate::triggers::TriggerEvent;

            // For LTB-style moves we keep the pre-move object ID; for all others use
            // the destination object ID so ETB/"this enters" matching remains stable.
            let event_object_id = if old_zone == Zone::Battlefield {
                old_id
            } else {
                new_id
            };
            let event = ZoneChangeEvent::with_cause(
                event_object_id,
                old_zone,
                new_zone,
                cause,
                pre_move_snapshot.clone(),
            );
            let mut event = event;
            if old_zone == Zone::Battlefield {
                event.result_objects = vec![new_id];
                if let Some(snapshot) = pre_move_snapshot.as_ref() {
                    for attachment_id in &snapshot.attachments {
                        if let Some(attachment) = self.object(*attachment_id) {
                            event = event.with_object_tag(
                                crate::tag::TagKey::from("attached_source"),
                                crate::snapshot::ObjectSnapshot::from_object(attachment, self),
                            );
                        }
                    }
                }
            }
            let event_provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::ZoneChange);
            self.queue_trigger_event(
                event_provenance,
                TriggerEvent::new_with_provenance(event, event_provenance)
                    .with_lookback_source_snapshots(pre_event_lookback_source_snapshots.to_vec()),
            );
        }
        self.record_zone_change_results(old_id, vec![new_id]);

        // Validate zone consistency in debug builds
        #[cfg(debug_assertions)]
        self.debug_assert_zone_consistency();

        self.reconcile_ring_bearers();

        Some(new_id)
    }

    fn default_face_definition_for_transform_like_return(
        &self,
        object: &Object,
    ) -> Option<crate::cards::CardDefinition> {
        let other_def = self.linked_face_definition_by_name_or_id(
            object.other_face_name.as_deref(),
            object.other_face,
        )?;
        if other_def.card.linked_face_layout != LinkedFaceLayout::TransformLike {
            return None;
        }
        let current_def =
            self.linked_face_definition_by_name_or_id(Some(&object.name), object.card)?;
        if current_def.card.linked_face_layout != LinkedFaceLayout::TransformLike {
            return None;
        }

        (current_def.card.id.0 > other_def.card.id.0).then_some(other_def)
    }

    pub fn move_object_by_effect(&mut self, old_id: ObjectId, new_zone: Zone) -> Option<ObjectId> {
        self.move_object(old_id, new_zone, crate::events::cause::EventCause::effect())
    }

    pub fn move_object_by_game_rule(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
    ) -> Option<ObjectId> {
        self.move_object(
            old_id,
            new_zone,
            crate::events::cause::EventCause::from_game_rule(),
        )
    }

    /// Put an object its owner owns into the ante zone (CR 407.4).
    ///
    /// Ante-specific card effects should use this entrypoint so the
    /// owner-only restriction is enforced centrally.
    pub fn ante_owned_object(
        &mut self,
        owner: PlayerId,
        object_id: ObjectId,
    ) -> Result<ObjectId, String> {
        let Some(object) = self.object(object_id) else {
            return Err("cannot ante a missing object".to_string());
        };
        if object.owner != owner {
            return Err("a player can ante only an object they own".to_string());
        }
        if object.zone == Zone::Ante {
            return Ok(object_id);
        }
        self.move_object_by_game_rule(object_id, Zone::Ante)
            .ok_or_else(|| "the object could not be moved to ante".to_string())
    }

    /// Select a random card from a player's library and ante it (CR 407.2).
    pub fn ante_random_library_card(&mut self, owner: PlayerId) -> Result<ObjectId, String> {
        let mut candidates = self
            .player(owner)
            .ok_or_else(|| "cannot ante for a missing player".to_string())?
            .library
            .clone();
        if candidates.is_empty() {
            return Err("cannot ante from an empty library".to_string());
        }
        self.shuffle_slice(&mut candidates);
        self.ante_owned_object(owner, candidates[0])
    }

    /// Transfer ownership of every card in ante to the winning player at the
    /// end of the game (CR 407.2). Returns the number of changed owners and is
    /// deliberately idempotent so duplicate terminal-result observations are
    /// harmless.
    pub fn finalize_ante_ownership(&mut self, winner: PlayerId) -> usize {
        if self.player(winner).is_none() {
            return 0;
        }
        let ante_ids = self.ante.clone();
        let mut changed = 0;
        for id in ante_ids {
            if self.object(id).is_some_and(|object| object.owner != winner) {
                if let Some(object) = self.object_mut(id) {
                    object.owner = winner;
                }
                changed += 1;
            }
        }
        changed
    }

    pub fn move_object_by_sba(&mut self, old_id: ObjectId, new_zone: Zone) -> Option<ObjectId> {
        self.move_object(
            old_id,
            new_zone,
            crate::events::cause::EventCause::from_sba(),
        )
    }

    pub(crate) fn move_object_by_sba_with_snapshot(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        snapshot: Option<crate::snapshot::ObjectSnapshot>,
    ) -> Option<ObjectId> {
        self.move_object_with_snapshot(
            old_id,
            new_zone,
            crate::events::cause::EventCause::from_sba(),
            snapshot,
        )
    }

    /// Move an object to the battlefield with ETB replacement effect processing.
    ///
    /// This processes replacement effects that modify how a permanent enters the battlefield:
    /// - "Enters tapped" effects (from the permanent itself or other sources)
    /// - "Enters with N counters" effects
    /// - "If this would enter the battlefield, exile it instead"
    ///
    /// For moves TO the battlefield, this should be used instead of `move_object`
    /// to ensure replacement effects are properly applied.
    pub fn move_object_with_etb_processing(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
    ) -> Option<EntersResult> {
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        self.move_object_with_etb_processing_with_dm(old_id, new_zone, &mut dm)
    }

    /// Move an object to the battlefield with ETB replacement processing and decisions.
    pub fn move_object_with_etb_processing_with_dm(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<EntersResult> {
        self.move_object_with_etb_processing_with_dm_and_cause(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
        )
    }

    /// Move an object to the battlefield with ETB replacement processing and an explicit cause.
    pub fn move_object_with_etb_processing_with_dm_and_cause(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<EntersResult> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            cause,
            decision_maker,
            true,
            Vec::new(),
            None,
            None,
        )
    }

    pub fn move_object_with_etb_processing_with_initial_counters_with_dm(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<EntersResult> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            true,
            initial_enters_with_counters,
            None,
            None,
        )
    }

    pub(crate) fn move_object_with_etb_processing_with_initial_counters_and_controller_with_dm(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<EntersResult> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            true,
            initial_enters_with_counters,
            entering_controller,
            None,
        )
    }

    pub fn move_object_with_etb_processing_without_aura_attachment_choice(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<EntersResult> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            new_zone,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            false,
            Vec::new(),
            None,
            None,
        )
    }

    /// Resolve every choice and action that forms part of an object's entry
    /// before the destination object is created.
    ///
    /// The returned record is independent of the source-zone object ID and can
    /// therefore be collected for every member of a simultaneous-entry batch
    /// before any member is committed.
    pub(crate) fn prepare_etb_entry_with_controller_and_dm(
        &mut self,
        old_id: ObjectId,
        mut result: crate::events::processing::EtbEventResult,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<PreparedEtbEntry> {
        if result.prevented {
            return Some(PreparedEtbEntry {
                result,
                choices: PreparedEtbChoices::default(),
            });
        }
        if let Some(choices) = result.prepared_choices.clone() {
            return Some(PreparedEtbEntry { result, choices });
        }

        let prospective_source = result
            .enters_as_copy_of
            .and_then(|copy_source| self.object(copy_source))
            .or_else(|| self.object(old_id));
        let prospective_controller = entering_controller
            .or(result.controller_override)
            .or_else(|| self.current_controller(old_id))
            .or_else(|| self.object(old_id).map(|object| object.owner))?;
        let mut prospective_card_types = prospective_source
            .map(|object| object.card_types.clone())
            .unwrap_or_default();
        for card_type in &result.added_card_types {
            if !prospective_card_types.contains(card_type) {
                prospective_card_types.push(*card_type);
            }
        }
        let mut prospective_subtypes = prospective_source
            .map(|object| object.subtypes.clone())
            .unwrap_or_default();
        for subtype in &result.added_subtypes {
            if !prospective_subtypes.contains(subtype) {
                prospective_subtypes.push(*subtype);
            }
        }
        let mut prospective_abilities = prospective_source
            .map(|object| object.abilities_vec())
            .unwrap_or_default();
        for ability in &result.added_abilities {
            if !prospective_abilities.contains(ability) {
                prospective_abilities.push(ability.clone());
            }
        }

        // Execute arbitrary as-enters setup in the same transactional clone
        // used by the rest of ETB preparation. Effects aimed at the source's
        // counters act on the source-zone object, so convert only their net
        // additions into counters on the prospective battlefield object.
        let counters_before = self
            .object(old_id)
            .map(|object| object.counters.clone())
            .unwrap_or_default();
        let as_enters_execution = self
            .execute_as_enters_effect_programs_from_abilities(
                old_id,
                prospective_controller,
                &prospective_abilities,
                false,
                decision_maker,
            )
            .ok()?;
        if decision_maker.awaiting_choice() {
            return None;
        }
        let counters_after = self
            .object(old_id)
            .map(|object| object.counters.clone())
            .unwrap_or_default();
        let mut as_enters_counters = Vec::new();
        for (counter_type, after) in &counters_after {
            let before = counters_before.get(counter_type).copied().unwrap_or(0);
            if *after > before {
                as_enters_counters.push((*counter_type, *after - before));
            }
        }
        if let Some(source) = self.object_mut(old_id) {
            source.counters = counters_before;
        }

        let battle_protector = if prospective_card_types.contains(&crate::types::CardType::Battle) {
            let legal = self.legal_battle_protectors_for(
                prospective_controller,
                prospective_subtypes.contains(&Subtype::Siege),
            );
            if legal.len() <= 1 {
                legal.first().copied()
            } else {
                let options = legal
                    .iter()
                    .enumerate()
                    .map(|(index, player)| {
                        crate::decisions::context::SelectableOption::new(
                            index,
                            self.player(*player)
                                .map(|player| player.name.clone())
                                .unwrap_or_else(|| format!("Player {}", player.0)),
                        )
                    })
                    .collect();
                let context = crate::decisions::context::SelectOptionsContext::new(
                    prospective_controller,
                    Some(old_id),
                    "Choose a player to protect this battle",
                    options,
                    1,
                    1,
                );
                let selected = decision_maker
                    .decide_options(self, &context)
                    .into_iter()
                    .find_map(|index| legal.get(index).copied());
                if decision_maker.awaiting_choice() {
                    return None;
                }
                selected.or_else(|| legal.first().copied())
            }
        } else {
            None
        };

        let mut choices = PreparedEtbChoices {
            battle_protector,
            as_enters_counters,
            as_enters_tagged_objects: as_enters_execution.tagged_objects,
            transfer_as_enters_source_links: as_enters_execution.ran,
            ..PreparedEtbChoices::default()
        };
        for ability in prospective_abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(spec) = static_ability.color_choice_as_enters() {
                let mut options = vec![
                    crate::color::Color::White,
                    crate::color::Color::Blue,
                    crate::color::Color::Black,
                    crate::color::Color::Red,
                    crate::color::Color::Green,
                ];
                if let Some(excluded) = spec.excluded {
                    options.retain(|color| *color != excluded);
                }
                if !options.is_empty() {
                    let choice_spec = crate::decisions::specs::ManaColorsSpec::restricted(
                        old_id,
                        1,
                        true,
                        options.clone(),
                    );
                    let mut chosen = crate::decisions::make_decision(
                        self,
                        decision_maker,
                        prospective_controller,
                        Some(old_id),
                        choice_spec,
                    );
                    if decision_maker.awaiting_choice() {
                        return None;
                    }
                    choices.chosen_color = chosen.pop().filter(|color| options.contains(color));
                }
            }
            if static_ability.basic_land_type_choice_as_enters().is_some() {
                let options = [
                    crate::types::Subtype::Plains,
                    crate::types::Subtype::Island,
                    crate::types::Subtype::Swamp,
                    crate::types::Subtype::Mountain,
                    crate::types::Subtype::Forest,
                ];
                let display_options = options
                    .iter()
                    .enumerate()
                    .map(|(idx, subtype)| {
                        crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return None;
                }
                choices.chosen_basic_land_type = chosen
                    .pop()
                    .filter(|idx| *idx < options.len())
                    .map(|idx| options[idx]);
            }
            if static_ability.land_type_choice_as_enters().is_some() {
                let options = crate::types::Subtype::all_land_types();
                let display_options = options
                    .iter()
                    .enumerate()
                    .map(|(idx, subtype)| {
                        crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return None;
                }
                choices.chosen_land_type = chosen
                    .pop()
                    .filter(|idx| *idx < options.len())
                    .map(|idx| options[idx]);
            }
            if static_ability.creature_type_choice_as_enters().is_some() {
                let options = crate::effects::BecomeCreatureTypeChoiceEffect::all_creature_types();
                let display_options = options
                    .iter()
                    .enumerate()
                    .map(|(idx, subtype)| {
                        crate::decisions::spec::DisplayOption::new(idx, subtype.to_string())
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return None;
                }
                choices.chosen_creature_type = chosen
                    .pop()
                    .filter(|idx| *idx < options.len())
                    .map(|idx| options[idx]);
            }
            if static_ability.player_choice_as_enters().is_some() {
                let options = self
                    .players
                    .iter()
                    .filter(|player| player.is_in_game())
                    .map(|player| player.id)
                    .collect::<Vec<_>>();
                if !options.is_empty() {
                    let display_options = options
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, player_id)| {
                            self.player(*player_id).map(|player| {
                                crate::decisions::spec::DisplayOption::new(
                                    idx,
                                    player.name.to_string(),
                                )
                            })
                        })
                        .collect::<Vec<_>>();
                    let choice_spec =
                        crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                    let mut chosen = crate::decisions::make_decision(
                        self,
                        decision_maker,
                        prospective_controller,
                        Some(old_id),
                        choice_spec,
                    );
                    if decision_maker.awaiting_choice() {
                        return None;
                    }
                    choices.chosen_player = chosen
                        .pop()
                        .filter(|idx| *idx < options.len())
                        .map(|idx| options[idx]);
                }
            }
            if let Some(spec) = static_ability.reveal_from_hand_choice_as_enters() {
                let filter_ctx = self.filter_context_for(prospective_controller, Some(old_id));
                let candidates = self
                    .player(prospective_controller)
                    .map(|player| player.hand.clone())
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|candidate_id| {
                        self.object(candidate_id)
                            .filter(|object| spec.filter.matches(object, &filter_ctx, self))
                            .map(|object| {
                                crate::decisions::context::SelectableObject::new(
                                    candidate_id,
                                    object.name.to_string(),
                                )
                            })
                    })
                    .collect::<Vec<_>>();
                if !candidates.is_empty() {
                    let min = if spec.optional {
                        0
                    } else {
                        spec.count.min.min(candidates.len())
                    };
                    let max = spec
                        .count
                        .max
                        .unwrap_or(candidates.len())
                        .min(candidates.len());
                    let context = crate::decisions::context::SelectObjectsContext::new(
                        prospective_controller,
                        Some(old_id),
                        "Reveal cards from your hand",
                        candidates.clone(),
                        min,
                        Some(max),
                    );
                    let candidate_ids = candidates
                        .iter()
                        .map(|candidate| candidate.id)
                        .collect::<Vec<_>>();
                    let selected = if min == candidates.len() && max == candidates.len() {
                        candidate_ids.clone()
                    } else {
                        decision_maker.decide_objects(self, &context)
                    };
                    if decision_maker.awaiting_choice() {
                        return None;
                    }
                    let revealed = selected
                        .into_iter()
                        .filter(|selected| candidate_ids.contains(selected))
                        .take(max)
                        .collect::<Vec<_>>();
                    if revealed.len() >= min && !revealed.is_empty() {
                        for viewer_idx in 0..self.players.len() {
                            let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
                            let view_ctx = crate::decisions::context::ViewCardsContext::new(
                                viewer,
                                prospective_controller,
                                Some(old_id),
                                Zone::Hand,
                                "Reveal cards from hand",
                            )
                            .with_public(true);
                            decision_maker.view_cards(self, viewer, &revealed, &view_ctx);
                        }
                    }
                }
            }
            if let Some(spec) = static_ability.card_name_choice_as_enters() {
                if spec.reveal_opponents_hands {
                    let opponent_ids = self
                        .players
                        .iter()
                        .filter(|player| player.is_in_game() && player.id != prospective_controller)
                        .map(|player| player.id)
                        .collect::<Vec<_>>();
                    for opponent_id in opponent_ids {
                        let cards = self
                            .player(opponent_id)
                            .map(|player| player.hand.clone())
                            .unwrap_or_default();
                        for viewer_idx in 0..self.players.len() {
                            let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
                            let mut view_ctx =
                                crate::decisions::context::ViewCardsContext::look_at_hand(
                                    viewer,
                                    opponent_id,
                                    Some(old_id),
                                );
                            view_ctx.description = "Reveal that player's hand".to_string();
                            view_ctx.public = true;
                            decision_maker.view_cards(self, viewer, &cards, &view_ctx);
                        }
                    }
                }
                let choice_ctx = crate::decisions::context::TextInputContext::new(
                    prospective_controller,
                    Some(old_id),
                    "Choose a card name",
                )
                .with_placeholder("Enter a card name")
                .require_known_value(true);
                let chosen_name = decision_maker.decide_text(self, &choice_ctx);
                if decision_maker.awaiting_choice() {
                    return None;
                }
                let chosen_name = chosen_name.trim();
                if !chosen_name.is_empty() {
                    let mut registry = CardRegistry::new();
                    registry.ensure_cards_loaded([chosen_name]);
                    let canonical_name = registry
                        .get(chosen_name)
                        .map(|definition| definition.name().to_string())
                        .unwrap_or_else(|| chosen_name.to_string());
                    let legal = !spec.require_nonland_from_revealed_opponents
                        || self
                            .players
                            .iter()
                            .filter(|player| {
                                player.is_in_game() && player.id != prospective_controller
                            })
                            .flat_map(|player| player.hand.iter().copied())
                            .filter_map(|object_id| self.object(object_id))
                            .any(|object| {
                                !object.is_land()
                                    && object.name.eq_ignore_ascii_case(&canonical_name)
                            });
                    if legal {
                        choices.chosen_named_option = Some(canonical_name);
                    }
                }
            }
            if let Some(spec) = static_ability.named_option_choice_as_enters()
                && !spec.options.is_empty()
            {
                let display_options = spec
                    .options
                    .iter()
                    .enumerate()
                    .map(|(idx, option)| {
                        crate::decisions::spec::DisplayOption::new(idx, option.clone())
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return None;
                }
                if let Some(option) = chosen
                    .pop()
                    .filter(|idx| *idx < spec.options.len())
                    .map(|idx| spec.options[idx].clone())
                {
                    choices.chosen_card_type = match option.as_str() {
                        "artifact" => Some(crate::types::CardType::Artifact),
                        "creature" => Some(crate::types::CardType::Creature),
                        "enchantment" => Some(crate::types::CardType::Enchantment),
                        "instant" => Some(crate::types::CardType::Instant),
                        "sorcery" => Some(crate::types::CardType::Sorcery),
                        "planeswalker" => Some(crate::types::CardType::Planeswalker),
                        "land" => Some(crate::types::CardType::Land),
                        _ => None,
                    };
                    choices.chosen_named_option = Some(option);
                }
            }
            if static_ability.life_total_note_as_enters().is_some() {
                choices.noted_life_total = self
                    .player(prospective_controller)
                    .map(|player| player.life);
            }
            if static_ability.id() == crate::static_abilities::StaticAbilityId::DiscardHandAsEnters
            {
                choices.discard_hand = true;
            }
            if let Some(spec) = static_ability.power_toughness_choice_as_enters_or_turns_face_up()
                && !spec.options.is_empty()
            {
                let display_options = spec
                    .options
                    .iter()
                    .enumerate()
                    .map(|(idx, option)| {
                        crate::decisions::spec::DisplayOption::new(
                            idx,
                            format!("{}/{}", option.power, option.toughness),
                        )
                    })
                    .collect::<Vec<_>>();
                let choice_spec =
                    crate::decisions::specs::ChoiceSpec::single(old_id, display_options);
                let mut chosen = crate::decisions::make_decision(
                    self,
                    decision_maker,
                    prospective_controller,
                    Some(old_id),
                    choice_spec,
                );
                if decision_maker.awaiting_choice() {
                    return None;
                }
                if let Some(option) = chosen
                    .pop()
                    .filter(|idx| *idx < spec.options.len())
                    .map(|idx| spec.options[idx].clone())
                {
                    choices.power_toughness_choices.push((
                        option.power,
                        option.toughness,
                        option.abilities,
                    ));
                }
            }
        }

        result.prepared_choices = Some(choices.clone());
        Some(PreparedEtbEntry { result, choices })
    }

    /// Commit an ETB proposal whose replacement choices were already resolved.
    ///
    /// Batch-entry callers use this after every entrant's immutable proposal has
    /// been collected, so no entrant can make another entrant's replacement
    /// effects visible before the simultaneous event is committed.
    pub(crate) fn commit_prepared_etb_with_controller_and_dm(
        &mut self,
        old_id: ObjectId,
        prepared_entry: PreparedEtbEntry,
        entering_controller: Option<PlayerId>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<EntersResult> {
        self.move_object_with_etb_processing_with_dm_and_cause_internal(
            old_id,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            decision_maker,
            true,
            Vec::new(),
            entering_controller,
            Some(prepared_entry),
        )
    }

    fn move_object_with_etb_processing_with_dm_and_cause_internal(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        choose_aura_attachment: bool,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        entering_controller: Option<PlayerId>,
        prepared_entry: Option<PreparedEtbEntry>,
    ) -> Option<EntersResult> {
        if new_zone == Zone::Battlefield && self.card_cannot_enter_battlefield(old_id) {
            return None;
        }
        if new_zone == Zone::Battlefield {
            let mut working = self.clone();
            let outcome = working.move_object_with_etb_processing_with_dm_and_cause_body(
                old_id,
                new_zone,
                cause,
                decision_maker,
                choose_aura_attachment,
                initial_enters_with_counters,
                entering_controller,
                prepared_entry,
            );
            if decision_maker.awaiting_choice() {
                return None;
            }
            *self = working;
            return outcome;
        }

        self.move_object_with_etb_processing_with_dm_and_cause_body(
            old_id,
            new_zone,
            cause,
            decision_maker,
            choose_aura_attachment,
            initial_enters_with_counters,
            entering_controller,
            prepared_entry,
        )
    }

    fn move_object_with_etb_processing_with_dm_and_cause_body(
        &mut self,
        old_id: ObjectId,
        new_zone: Zone,
        cause: crate::events::cause::EventCause,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
        choose_aura_attachment: bool,
        initial_enters_with_counters: Vec<(crate::object::CounterType, u32)>,
        entering_controller: Option<PlayerId>,
        prepared_entry: Option<PreparedEtbEntry>,
    ) -> Option<EntersResult> {
        let old_zone = self.object(old_id)?.zone;

        // Only process ETB replacement for moves TO the battlefield
        if new_zone != Zone::Battlefield {
            let new_id = self.move_object(old_id, new_zone, cause.clone())?;
            return Some(EntersResult {
                new_id,
                enters_tapped: false,
            });
        }

        // Process through ETB replacement effects
        let prepared_entry = if let Some(prepared_entry) = prepared_entry {
            prepared_entry
        } else {
            let result = crate::events::processing::process_etb_with_event_and_dm_with_initial_counters_and_controller(
                self,
                old_id,
                old_zone,
                decision_maker,
                initial_enters_with_counters,
                entering_controller,
            );
            self.prepare_etb_entry_with_controller_and_dm(
                old_id,
                result,
                entering_controller,
                decision_maker,
            )?
        };
        let PreparedEtbEntry { result, choices } = prepared_entry;

        // If ETB was prevented or redirected to a different zone
        if result.prevented {
            if let Some(dest) = result.new_destination {
                // Move to the alternate destination
                let new_id = self.move_object(old_id, dest, cause.clone())?;
                return Some(EntersResult {
                    new_id,
                    enters_tapped: false,
                });
            }
            return None;
        }

        let prospective_aura_entry = choose_aura_attachment
            && (old_zone != Zone::Stack || result.enters_as_copy_of.is_some())
            && (self
                .object(old_id)
                .is_some_and(|object| object.subtypes.contains(&Subtype::Aura))
                || result.enters_as_copy_of.is_some_and(|copy_id| {
                    self.object(copy_id)
                        .is_some_and(|object| object.subtypes.contains(&Subtype::Aura))
                })
                || result.added_subtypes.contains(&Subtype::Aura));
        // CR 303.4g is not a second zone change. If attachment proves
        // impossible, restore this exact pre-entry state.
        let aura_entry_checkpoint = prospective_aura_entry.then(|| self.clone());

        if choices.discard_hand {
            let controller = entering_controller
                .or(result.controller_override)
                .or_else(|| self.current_controller(old_id))
                .or_else(|| self.object(old_id).map(|object| object.owner))?;
            let hand = self
                .player(controller)
                .map(|player| player.hand.clone())
                .unwrap_or_default();
            for card_id in hand {
                if card_id == old_id {
                    continue;
                }
                let provenance = self
                    .provenance_graph_mut()
                    .alloc_root_event(crate::events::EventKind::Discard);
                crate::events::processing::execute_discard(
                    self,
                    card_id,
                    controller,
                    cause.clone(),
                    false,
                    provenance,
                    decision_maker,
                );
                if decision_maker.awaiting_choice() {
                    return None;
                }
            }
        }

        // Proceed with normal battlefield entry
        let new_id = self.move_object(old_id, Zone::Battlefield, cause.clone())?;
        if let Some(object) = self.object_mut(new_id) {
            merge_retained_tagged_objects(
                &mut object.cast_tagged_objects,
                &choices.as_enters_tagged_objects,
            );
        }
        // As-enters effect programs execute against the pre-move object id;
        // migrate any choices they recorded to the battlefield id.
        if new_id != old_id {
            let choice_store = self.choice_store_mut();
            if let Some(color) = choice_store.chosen_colors.remove(&old_id) {
                choice_store.chosen_colors.insert(new_id, color);
            }
            if let Some(land_type) = choice_store.chosen_land_types.remove(&old_id) {
                choice_store.chosen_land_types.insert(new_id, land_type);
            }
            if let Some(creature_type) = choice_store.chosen_creature_types.remove(&old_id) {
                choice_store
                    .chosen_creature_types
                    .insert(new_id, creature_type);
            }
            if let Some(card_type) = choice_store.chosen_card_types.remove(&old_id) {
                choice_store.chosen_card_types.insert(new_id, card_type);
            }
            if let Some(player) = choice_store.chosen_players.remove(&old_id) {
                choice_store.chosen_players.insert(new_id, player);
            }
            if let Some(names) = choice_store.chosen_named_options.remove(&old_id) {
                choice_store.chosen_named_options.insert(new_id, names);
            }
        }
        if choices.transfer_as_enters_source_links {
            self.transfer_exiled_with_source_links(old_id, new_id);
            let imprinted_cards = self.get_imprinted_cards(old_id).to_vec();
            self.clear_imprinted_cards(old_id);
            for imprinted_card in imprinted_cards {
                self.imprint_card(new_id, imprinted_card);
            }
        }
        if let Some(controller) = entering_controller.or(result.controller_override) {
            self.set_current_controller(new_id, controller);
        }

        // Apply "enters as copy" before tapped/counter modifications. Ordinary
        // enter-as-copy effects replace the object's copiable values. A copy
        // with an explicit duration instead becomes a locked layer-1 effect,
        // preserving the underlying permanent so it can revert when it expires.
        let temporary_copy_duration = result.copy_duration.clone();
        if let Some(copy_source_id) = result.enters_as_copy_of {
            if let Some(duration) = temporary_copy_duration.clone() {
                let effects = self.all_continuous_effects();
                let copiable_values = crate::continuous::copiable_values_with_effects(
                    copy_source_id,
                    self.objects_map(),
                    &effects,
                    &self.battlefield,
                    self.commander_objects(),
                    self,
                );
                if let Some(mut copiable_values) = copiable_values {
                    let controller = self.current_controller(new_id)?;
                    if let Some(name) = &result.copy_name_override {
                        copiable_values.name = name.clone();
                    }
                    for card_type in &result.added_card_types {
                        if !copiable_values.card_types.contains(card_type) {
                            copiable_values.card_types.push(*card_type);
                        }
                    }
                    copiable_values
                        .supertypes
                        .retain(|supertype| !result.removed_supertypes.contains(supertype));
                    for subtype in &result.added_subtypes {
                        if !copiable_values.subtypes.contains(subtype) {
                            copiable_values.subtypes.push(*subtype);
                        }
                    }
                    for ability in &result.added_abilities {
                        let abilities = std::sync::Arc::make_mut(&mut copiable_values.abilities);
                        if !abilities.contains(ability) {
                            abilities.push(ability.clone());
                        }
                    }
                    if let Some((power, toughness)) = result.set_base_power_toughness {
                        copiable_values.power = Some(power);
                        copiable_values.toughness = Some(toughness);
                    }

                    let modification = crate::continuous::Modification::CopyOf {
                        target_id: copy_source_id,
                        copiable_values: Box::new(copiable_values),
                        preserve_source_abilities: false,
                        name_override: None,
                        name_override_surface: None,
                        add_supertypes: Vec::new(),
                    };
                    let expires_end_of_turn = matches!(
                        &duration,
                        crate::effect::Until::EndOfTurn
                            | crate::effect::Until::YourNextTurn
                            | crate::effect::Until::YourNextUpkeep
                            | crate::effect::Until::ControllersNextUntapStep
                    )
                    .then_some(self.turn.turn_number)
                    .unwrap_or(u32::MAX);
                    let effect = crate::continuous::ContinuousEffect::new(
                        new_id,
                        controller,
                        crate::continuous::EffectTarget::Specific(new_id),
                        modification,
                    )
                    .until(duration)
                    .with_expires_end_of_turn(expires_end_of_turn)
                    .with_source_type(
                        crate::continuous::EffectSourceType::Resolution {
                            locked_targets: vec![new_id],
                        },
                    );
                    self.effect_store.continuous_effects.add_effect(effect);
                    self.refresh_continuous_state();
                }
            } else {
                let copy_source = self.object(copy_source_id).cloned();
                let effects = self.all_continuous_effects();
                let copiable_values = crate::continuous::copiable_values_with_effects(
                    copy_source_id,
                    self.objects_map(),
                    &effects,
                    &self.battlefield,
                    self.commander_objects(),
                    self,
                );
                if let (Some(source_obj), Some(new_obj)) = (copy_source, self.object_mut(new_id)) {
                    new_obj.copy_copiable_values_from(&source_obj);
                    if let Some(values) = copiable_values.as_ref() {
                        new_obj.copy_copiable_values_from_values(values);
                    }
                    if let Some(name) = &result.copy_name_override {
                        new_obj.name = name.clone().into();
                    }
                }
            }
        }
        if temporary_copy_duration.is_none() {
            if !result.added_card_types.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                for card_type in &result.added_card_types {
                    if !new_obj.card_types.contains(card_type) {
                        new_obj.card_types.push(*card_type);
                    }
                }
            }
            if !result.removed_supertypes.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                new_obj
                    .supertypes
                    .retain(|supertype| !result.removed_supertypes.contains(supertype));
            }
            if !result.added_subtypes.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                for subtype in &result.added_subtypes {
                    if !new_obj.subtypes.contains(subtype) {
                        new_obj.subtypes.push(*subtype);
                    }
                }
            }
            if !result.added_abilities.is_empty()
                && let Some(new_obj) = self.object_mut(new_id)
            {
                for ability in &result.added_abilities {
                    if !new_obj.abilities.contains(ability) {
                        new_obj.abilities_mut().push(ability.clone());
                    }
                }
            }
            if let Some((power, toughness)) = result.set_base_power_toughness
                && let Some(new_obj) = self.object_mut(new_id)
            {
                new_obj.base_power = Some(crate::card::PtValue::Fixed(power));
                new_obj.base_toughness = Some(crate::card::PtValue::Fixed(toughness));
            }
        }

        // Publish the choices collected against the prospective permanent only
        // after the destination object exists. No decision is made in this
        // section, so neither synchronous nor suspended callers can observe a
        // battlefield permanent whose mandatory entry choice is unresolved.
        if let Some(color) = choices.chosen_color {
            self.set_chosen_color(new_id, color);
        }
        if let Some(subtype) = choices.chosen_basic_land_type {
            self.set_chosen_basic_land_type(new_id, subtype);
        }
        if let Some(subtype) = choices.chosen_land_type {
            self.set_chosen_land_type(new_id, subtype);
        }
        if let Some(subtype) = choices.chosen_creature_type {
            self.set_chosen_creature_type(new_id, subtype);
        }
        if let Some(card_type) = choices.chosen_card_type {
            self.set_chosen_card_type(new_id, card_type);
        }
        if let Some(player) = choices.chosen_player {
            self.set_chosen_player(new_id, player);
        }
        if let Some(option) = choices.chosen_named_option.clone() {
            self.set_chosen_named_option(new_id, option);
        }
        if let Some(life_total) = choices.noted_life_total {
            self.object_annotations_mut()
                .noted_life_totals
                .insert(new_id, life_total);
        }
        for (power, toughness, abilities) in &choices.power_toughness_choices {
            if let Some(object) = self.object_mut(new_id) {
                object.base_power = Some(crate::card::PtValue::Fixed(*power));
                object.base_toughness = Some(crate::card::PtValue::Fixed(*toughness));
                for granted in abilities {
                    let ability = crate::ability::Ability::static_ability(granted.clone());
                    if !object.abilities.contains(&ability) {
                        object.abilities_mut().push(ability);
                    }
                }
                self.mark_continuous_state_dirty();
            }
        }

        // Apply enters tapped
        if result.enters_tapped {
            self.tap(new_id);
        }

        // Apply enters with counters
        for (counter_type, count) in result
            .enters_with_counters
            .iter()
            .chain(&choices.as_enters_counters)
        {
            if let Some(obj) = self.object_mut(new_id) {
                *obj.counters.entry(*counter_type).or_insert(0) += count;
            }
        }

        if let Some(protector) = choices.battle_protector {
            let _ = self.set_battle_protector(new_id, protector);
        }

        if !result.paid_labels.is_empty() {
            if let Some(obj) = self.object_mut(new_id) {
                for label in &result.paid_labels {
                    obj.optional_costs_paid.mark_label_paid(label);
                }
            }
        }

        for linked_old_id in &result.linked_exile_with_entering {
            if self.object(*linked_old_id).is_none() {
                continue;
            }
            let Some(exiled_id) = self.move_object(*linked_old_id, Zone::Exile, cause.clone())
            else {
                continue;
            };
            self.add_exiled_with_source_link(new_id, exiled_id);
            self.record_zone_change_results(*linked_old_id, vec![exiled_id]);
        }

        // If this is an Aura entering from a non-stack zone, choose what to attach to
        if choose_aura_attachment
            && (old_zone != Zone::Stack || result.enters_as_copy_of.is_some())
            && let Some(obj) = self.object(new_id)
            && obj.subtypes.contains(&Subtype::Aura)
            && obj.attached_to.is_none()
            && let Some(filter) = obj.aura_attach_filter_owned()
        {
            let chooser = self.current_controller(new_id).unwrap_or(obj.owner);
            let filter_ctx = self.filter_context_for(chooser, Some(new_id));
            let chosen_target = match filter {
                AuraAttachmentFilter::Object(filter) => {
                    let mut candidates = Vec::new();
                    for (id, candidate) in &self.objects {
                        if *id == new_id || candidate.zone != Zone::Battlefield {
                            continue;
                        }
                        if filter.matches(candidate, &filter_ctx, self) {
                            candidates.push(crate::decisions::context::SelectableObject::new(
                                *id,
                                candidate.name.to_string(),
                            ));
                        }
                    }

                    if candidates.is_empty() {
                        None
                    } else {
                        let fallback_target = candidates.first().map(|candidate| candidate.id);
                        let ctx = crate::decisions::context::SelectObjectsContext::new(
                            chooser,
                            Some(new_id),
                            "Attach Aura to",
                            candidates,
                            1,
                            Some(1),
                        );
                        decision_maker
                            .decide_objects(self, &ctx)
                            .first()
                            .copied()
                            .or(fallback_target)
                            .map(AttachmentTarget::Object)
                    }
                }
                AuraAttachmentFilter::Player(filter) => {
                    let candidates = self
                        .players
                        .iter()
                        .filter(|player| {
                            player.is_in_game() && filter.matches_player(player.id, &filter_ctx)
                        })
                        .map(|player| (player.id, player.name.to_string()))
                        .collect::<Vec<_>>();
                    if candidates.is_empty() {
                        None
                    } else if candidates.len() == 1 {
                        Some(AttachmentTarget::Player(candidates[0].0))
                    } else {
                        let choice_spec = crate::decisions::specs::ChoiceSpec::single(
                            new_id,
                            candidates
                                .iter()
                                .enumerate()
                                .map(|(idx, (_, name))| {
                                    crate::decisions::spec::DisplayOption::new(idx, name.clone())
                                })
                                .collect(),
                        );
                        let mut chosen = crate::decisions::make_decision(
                            self,
                            decision_maker,
                            chooser,
                            Some(new_id),
                            choice_spec,
                        );
                        chosen
                            .pop()
                            .and_then(|idx| candidates.get(idx).map(|(player_id, _)| *player_id))
                            .map(AttachmentTarget::Player)
                            .or_else(|| Some(AttachmentTarget::Player(candidates[0].0)))
                    }
                }
            };

            let attached = chosen_target.is_some_and(|target| {
                if !self.attach_object_to_target(new_id, target) {
                    return false;
                }
                self.effect_store
                    .continuous_effects
                    .record_attachment(new_id);
                true
            });
            if !attached && let Some(checkpoint) = aura_entry_checkpoint {
                *self = checkpoint;
                if old_zone == Zone::Stack {
                    let graveyard_id = self.move_object(old_id, Zone::Graveyard, cause)?;
                    return Some(EntersResult {
                        new_id: graveyard_id,
                        enters_tapped: false,
                    });
                }
                return Some(EntersResult {
                    new_id: old_id,
                    enters_tapped: false,
                });
            }
        }

        Some(EntersResult {
            new_id,
            enters_tapped: result.enters_tapped,
        })
    }

    /// Removes an object from the game completely (e.g., tokens ceasing to exist).
    /// This does NOT create a new object - the object is simply gone.
    pub fn remove_object(&mut self, id: ObjectId) {
        if let Some(obj) = self.objects.remove(&id).map(ObjectStore::into_owned_object) {
            if let Some(target) = obj.attached_to {
                match target {
                    AttachmentTarget::Object(parent_id) => {
                        if let Some(parent) = self.object_mut(parent_id) {
                            parent.attachments.retain(|existing| *existing != id);
                        }
                    }
                    AttachmentTarget::Player(player_id) => {
                        if let Some(player) = self.player_mut(player_id) {
                            player.attachments.retain(|existing| *existing != id);
                        }
                    }
                }
            }
            self.stable_id_index.remove(&obj.stable_id);
            self.auxiliary_tracking_mut()
                .sector_designations
                .remove(&id);
            {
                let commander_tracking = self.commander_tracking_mut();
                commander_tracking.melded_permanents.remove(&obj.stable_id);
                commander_tracking.merged_permanents.remove(&obj.stable_id);
                commander_tracking
                    .pending_merged_component_destinations
                    .remove(&obj.stable_id);
                commander_tracking.declined_command_zone_moves.remove(&id);
            }
            self.remove_from_zone_index(id, obj.zone, obj.owner);
        }
    }

    /// Removes an object ID from its zone index.
    fn remove_from_zone_index(&mut self, id: ObjectId, zone: Zone, owner: PlayerId) {
        let mut removed = false;
        match zone {
            Zone::Battlefield => {
                let before = self.battlefield.len();
                self.battlefield.retain(|&x| x != id);
                removed = self.battlefield.len() != before;
            }
            Zone::Command => {
                let before = self.command_zone.len();
                self.command_zone.retain(|&x| x != id);
                removed = self.command_zone.len() != before;
            }
            Zone::Exile => {
                let before = self.exile.len();
                self.exile.retain(|&x| x != id);
                removed = self.exile.len() != before;
            }
            Zone::Ante => {
                let before = self.ante.len();
                self.ante.retain(|&x| x != id);
                removed = self.ante.len() != before;
            }
            Zone::Library => {
                let was_top = self
                    .player(owner)
                    .and_then(|player| player.library.last().copied())
                    == Some(id);
                if let Some(player) = self.player_mut(owner) {
                    let before = player.library.len();
                    player.library.retain(|&x| x != id);
                    removed = player.library.len() != before;
                }
                if was_top {
                    self.bump_library_top_revision(owner);
                }
            }
            Zone::Hand => {
                if let Some(player) = self.player_mut(owner) {
                    let before = player.hand.len();
                    player.hand.retain(|&x| x != id);
                    removed = player.hand.len() != before;
                }
            }
            Zone::Graveyard => {
                if let Some(player) = self.player_mut(owner) {
                    let before = player.graveyard.len();
                    player.graveyard.retain(|&x| x != id);
                    removed = player.graveyard.len() != before;
                }
            }
            Zone::OutsideGame => {
                if let Some(player) = self.player_mut(owner) {
                    let before = player.sideboard.len();
                    player.sideboard.retain(|&x| x != id);
                    removed = player.sideboard.len() != before;
                }
            }
            Zone::Stack => {}
        }
        if removed {
            self.bump_zone_revision(zone);
        }
    }

    // =========================================================================
    // Zone Consistency Validation (Debug Only)
    // =========================================================================

    /// Validate that zone indexes are consistent with the canonical objects HashMap.
    ///
    /// This checks that:
    /// - Every ID in denormalized zone indexes (battlefield, exile, etc.) exists in objects
    /// - Every object's zone field matches exactly one denormalized index
    /// - No ID appears in multiple zone indexes
    ///
    /// Only runs in debug builds or paranoid invariant builds to avoid release performance impact.
    #[cfg(any(debug_assertions, feature = "paranoid-invariants"))]
    pub fn validate_zone_consistency(&self) -> Result<(), String> {
        use std::collections::HashSet;

        let mut seen_ids: HashSet<ObjectId> = HashSet::new();

        // Check battlefield
        for &id in &self.battlefield {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Battlefield => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in battlefield index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in battlefield index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check exile
        for &id in &self.exile {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Exile => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in exile index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in exile index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check command zone
        for &id in &self.command_zone {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Command => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in command zone index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in command zone index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check ante
        for &id in &self.ante {
            if seen_ids.contains(&id) {
                return Err(format!("Object #{} appears in multiple zone indexes", id.0));
            }
            seen_ids.insert(id);

            match self.objects.get(&id) {
                Some(obj) if obj.zone == Zone::Ante => {}
                Some(obj) => {
                    return Err(format!(
                        "Object #{} in ante index has zone {}",
                        id.0, obj.zone
                    ));
                }
                None => {
                    return Err(format!(
                        "Object #{} in ante index doesn't exist in objects",
                        id.0
                    ));
                }
            }
        }

        // Check player zones
        for player in &self.players {
            // Library
            for &id in &player.library {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Library => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s library has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s library doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }

            // Hand
            for &id in &player.hand {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Hand => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s hand has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s hand doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }

            // Graveyard
            for &id in &player.graveyard {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::Graveyard => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s graveyard has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s graveyard doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }

            // Sideboard / outside the game
            for &id in &player.sideboard {
                if seen_ids.contains(&id) {
                    return Err(format!("Object #{} appears in multiple zone indexes", id.0));
                }
                seen_ids.insert(id);

                match self.objects.get(&id) {
                    Some(obj) if obj.zone == Zone::OutsideGame => {}
                    Some(obj) => {
                        return Err(format!(
                            "Object #{} in {}'s sideboard has zone {}",
                            id.0, player.name, obj.zone
                        ));
                    }
                    None => {
                        return Err(format!(
                            "Object #{} in {}'s sideboard doesn't exist in objects",
                            id.0, player.name
                        ));
                    }
                }
            }
        }

        // Check that all objects with non-Stack zones are in exactly one index
        for (&id, obj) in &self.objects {
            if obj.zone == Zone::Stack {
                // Stack objects are managed via StackEntry, not indexed
                continue;
            }
            if !seen_ids.contains(&id) {
                return Err(format!(
                    "Object #{} with zone {} is not in any zone index",
                    id.0, obj.zone
                ));
            }
        }

        Ok(())
    }

    /// Debug assertion for zone consistency. Panics if zones are inconsistent.
    #[cfg(any(debug_assertions, feature = "paranoid-invariants"))]
    pub fn debug_assert_zone_consistency(&self) {
        if let Err(e) = self.validate_zone_consistency() {
            panic!("Zone consistency violation: {}", e);
        }
    }

    /// Gets a reference to an object by ID.
    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.object_store.object(id)
    }

    /// Gets a mutable reference to an object by ID.
    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.mark_continuous_state_dirty();
        self.stamp_object_modified(id);
        self.object_store.object_mut(id)
    }

    pub(crate) fn objects_map(&self) -> &ObjectMap {
        self.object_store.objects_map()
    }

    pub fn attachment_target_exists_on_battlefield(&self, target: AttachmentTarget) -> bool {
        match target {
            AttachmentTarget::Object(id) => self
                .object(id)
                .is_some_and(|object| object.zone == Zone::Battlefield),
            AttachmentTarget::Player(id) => {
                self.player(id).is_some_and(|player| player.is_in_game())
            }
        }
    }

    pub fn detach_object_from_current_target(&mut self, attachment_id: ObjectId) -> bool {
        let lookback_source_snapshots = self.trigger_source_lookback_snapshots();
        let attachment_snapshot = self
            .object(attachment_id)
            .map(|object| self.cached_object_snapshot_with_calculated_characteristics(object));
        self.mark_continuous_state_dirty();
        let Some(current_target) = self
            .object(attachment_id)
            .and_then(|object| object.attached_to)
        else {
            return false;
        };

        match current_target {
            AttachmentTarget::Object(id) => {
                if let Some(parent) = self.object_mut(id) {
                    parent
                        .attachments
                        .retain(|existing| *existing != attachment_id);
                }
            }
            AttachmentTarget::Player(id) => {
                if let Some(player) = self.player_mut(id) {
                    player
                        .attachments
                        .retain(|existing| *existing != attachment_id);
                }
            }
        }

        if let Some(object) = self.object_mut(attachment_id) {
            object.attached_to = None;
        }

        if let Some(snapshot) = attachment_snapshot {
            let provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::ObjectBecameUnattached);
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::ObjectBecameUnattachedEvent::new(
                    attachment_id,
                    current_target,
                    snapshot.controller,
                    Some(snapshot),
                ),
                provenance,
            )
            .with_lookback_source_snapshots(lookback_source_snapshots);
            self.queue_trigger_event(provenance, event);
        }

        true
    }

    pub fn attach_object_to_target(
        &mut self,
        attachment_id: ObjectId,
        target: AttachmentTarget,
    ) -> bool {
        self.mark_continuous_state_dirty();
        if !self
            .object(attachment_id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
            || !self.attachment_target_exists_on_battlefield(target)
        {
            return false;
        }

        self.detach_object_from_current_target(attachment_id);

        if let Some(object) = self.object_mut(attachment_id) {
            object.attached_to = Some(target);
        } else {
            return false;
        }

        match target {
            AttachmentTarget::Object(id) => {
                if let Some(parent) = self.object_mut(id)
                    && !parent.attachments.contains(&attachment_id)
                {
                    parent.attachments.push(attachment_id);
                }
            }
            AttachmentTarget::Player(id) => {
                if let Some(player) = self.player_mut(id)
                    && !player.attachments.contains(&attachment_id)
                {
                    player.attachments.push(attachment_id);
                }
            }
        }

        true
    }

    // =========================================================================
    // Counter Management
    // =========================================================================

    /// Add counters to an object and return a CounterPlaced event for trigger checking.
    ///
    /// This method adds the counters and returns the event that should be used
    /// to check for triggers (like saga chapter abilities).
    ///
    /// Returns None if the object doesn't exist.
    pub fn add_counters(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
    ) -> Option<crate::triggers::TriggerEvent> {
        self.mark_continuous_state_dirty();
        let obj = self.object_mut(id)?;
        obj.add_counters(counter_type, amount);
        if amount > 0 {
            self.effect_store
                .continuous_effects
                .record_counter_change(id, counter_type);
        }
        self.record_counter_ui_effect_event("counters_added", id, counter_type, amount);

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::CounterPlaced);
        Some(crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::CounterPlacedEvent::new(id, counter_type, amount),
            event_provenance,
        ))
    }

    /// Remove counters from an object.
    ///
    /// Returns the actual number of counters removed and a trigger event.
    /// The actual removed amount may be less than requested if there weren't enough.
    pub fn remove_counters(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<(u32, crate::triggers::TriggerEvent)> {
        self.mark_continuous_state_dirty();
        let location_snapshot = self.object(id).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, self,
            )
        });
        let obj = self.object_mut(id)?;
        let removed = obj.remove_counters(counter_type, amount);
        let count_after = obj.counters.get(&counter_type).copied().unwrap_or(0);

        if removed == 0 {
            return None;
        }
        self.record_counter_ui_effect_event("counters_removed", id, counter_type, removed);

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        let mut event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::removed(
                counter_type,
                id,
                removed,
                source,
                source_controller,
            )
            .with_count_after(count_after),
            event_provenance,
        );
        if let Some(snapshot) = location_snapshot {
            event = event.with_lookback_source_snapshots(vec![snapshot]);
        }

        Some((removed, event))
    }

    /// Add counters with full tracking (source, controller) for the unified marker system.
    ///
    /// Returns a MarkersChangedEvent for trigger checking.
    pub fn add_counters_with_source(
        &mut self,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<crate::triggers::TriggerEvent> {
        self.mark_continuous_state_dirty();
        if amount == 0 {
            return None;
        }

        let obj = self.object_mut(id)?;
        obj.add_counters(counter_type, amount);
        self.effect_store
            .continuous_effects
            .record_counter_change(id, counter_type);
        self.record_counter_ui_effect_event("counters_added", id, counter_type, amount);

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        Some(crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::added(
                counter_type,
                id,
                amount,
                source,
                source_controller,
            ),
            event_provenance,
        ))
    }

    /// Record a UI-only counter change event for battlefield objects.
    fn record_counter_ui_effect_event(
        &mut self,
        kind: &str,
        id: ObjectId,
        counter_type: crate::object::CounterType,
        amount: u32,
    ) {
        if amount == 0 {
            return;
        }
        let Some(stable_id) = self
            .object(id)
            .filter(|obj| obj.zone == Zone::Battlefield)
            .map(|obj| obj.stable_id)
        else {
            return;
        };
        self.record_ui_effect_event(
            kind,
            None,
            None,
            vec![stable_id],
            Some(i64::from(amount)),
            Some(counter_type.description().into_owned()),
        );
    }

    /// Get the number of counters of a specific type on an object.
    pub fn counter_count(&self, id: ObjectId, counter_type: crate::object::CounterType) -> u32 {
        self.object(id)
            .and_then(|obj| obj.counters.get(&counter_type).copied())
            .unwrap_or(0)
    }

    /// Add counters to a player and emit a unified marker event when applicable.
    ///
    /// Counter types with dedicated rules fields and generic player counter types share this path.
    pub fn add_player_counters_with_source(
        &mut self,
        player_id: PlayerId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<crate::triggers::TriggerEvent> {
        if amount == 0 {
            return None;
        }

        if matches!(counter_type, crate::object::CounterType::Poison)
            && !self.can_get_poison_counters(player_id)
        {
            return None;
        }

        let cause = match (source, source_controller) {
            (Some(source), Some(controller)) => {
                crate::events::cause::EventCause::from_effect(source, controller)
            }
            _ => crate::events::cause::EventCause::effect(),
        };
        let amount = crate::events::processing::process_player_counters_with_event(
            self,
            player_id,
            counter_type,
            amount,
            cause,
        );
        if amount == 0 {
            return None;
        }

        if matches!(counter_type, crate::object::CounterType::Poison) {
            let current = self.player(player_id)?.poison_counters;
            self.write_shared_poison(player_id, current.saturating_add(amount));
        } else {
            self.player_mut(player_id)?
                .add_counters(counter_type, amount);
        }

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        Some(crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::added(
                counter_type,
                player_id,
                amount,
                source,
                source_controller,
            ),
            event_provenance,
        ))
    }

    /// Remove counters from a player and emit a unified marker event when applicable.
    ///
    /// Returns the actual number removed and the corresponding event.
    pub fn remove_player_counters_with_source(
        &mut self,
        player_id: PlayerId,
        counter_type: crate::object::CounterType,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Option<(u32, crate::triggers::TriggerEvent)> {
        if amount == 0 {
            return None;
        }

        let removed = if matches!(counter_type, crate::object::CounterType::Poison) {
            let current = self.player(player_id)?.poison_counters;
            let removed = current.min(amount);
            self.write_shared_poison(player_id, current.saturating_sub(removed));
            removed
        } else {
            self.player_mut(player_id)?
                .remove_counters(counter_type, amount)
        };

        if removed == 0 {
            return None;
        }

        let event_provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::MarkersChanged);
        Some((
            removed,
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::MarkersChangedEvent::removed(
                    counter_type,
                    player_id,
                    removed,
                    source,
                    source_controller,
                ),
                event_provenance,
            ),
        ))
    }

    /// Check if an object has any counters of a specific type.
    pub fn has_counters(&self, id: ObjectId, counter_type: crate::object::CounterType) -> bool {
        self.counter_count(id, counter_type) > 0
    }

    // =========================================================================
    // Calculated Characteristics (with continuous effects applied)
    // =========================================================================

    /// Calculate all characteristics for an object, applying continuous effects.
    ///
    /// This includes effects from:
    /// - Registered continuous effects (from resolved spells/abilities)
    /// - Static abilities on permanents (generated dynamically)
    pub fn all_continuous_effects(&self) -> Vec<ContinuousEffect> {
        if self.continuous_state_is_clean() {
            return self
                .cached_continuous_effects_snapshot_arc()
                .as_ref()
                .clone();
        }
        crate::static_ability_processor::get_all_continuous_effects(self)
    }

    /// Combine registered and cached static-ability continuous effects.
    ///
    /// Unlike `all_continuous_effects`, this does not regenerate static-ability
    /// effects dynamically. Callers must only use this after
    /// `refresh_continuous_state` (or `update_static_ability_effects`) for the
    /// current state.
    pub(crate) fn cached_continuous_effects_snapshot(&self) -> Vec<ContinuousEffect> {
        self.cached_continuous_effects_snapshot_arc()
            .as_ref()
            .clone()
    }

    pub(crate) fn cached_continuous_effects_snapshot_arc(&self) -> Arc<Vec<ContinuousEffect>> {
        let revision = self.effect_store.continuous_effects.revision();
        if let Some((cached_revision, effects)) =
            self.runtime_cache.effects_snapshot.borrow().as_ref()
            && *cached_revision == revision
        {
            return Arc::clone(effects);
        }

        let mut effects: Vec<ContinuousEffect> = self
            .effect_store
            .continuous_effects
            .effects_sorted()
            .into_iter()
            .cloned()
            .collect();
        effects.reserve(
            self.effect_store
                .continuous_effects
                .static_ability_effects()
                .len(),
        );
        effects.extend(
            self.effect_store
                .continuous_effects
                .static_ability_effects()
                .iter()
                .cloned(),
        );
        let effects = Arc::new(effects);
        *self.runtime_cache.effects_snapshot.borrow_mut() = Some((revision, Arc::clone(&effects)));
        effects
    }

    /// Calculate all characteristics for an object using precomputed continuous effects.
    ///
    /// This avoids rebuilding/allocating the full effect list when multiple
    /// characteristic lookups happen in the same operation.
    pub fn calculated_characteristics_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Option<crate::continuous::CalculatedCharacteristics> {
        if let Some(chars) = self.face_down_conspiracy_characteristics(id) {
            return Some(chars);
        }
        if let Some(chars) = crate::continuous::in_progress_characteristics(id) {
            return Some(chars);
        }
        crate::continuous::calculate_characteristics_with_effects(
            id,
            &self.objects,
            effects,
            &self.battlefield,
            self.commander_objects(),
            self,
        )
    }

    pub(crate) fn calculated_characteristics_batch_with_effects(
        &self,
        ids: &[ObjectId],
        effects: &[ContinuousEffect],
    ) -> HashMap<ObjectId, crate::continuous::CalculatedCharacteristics> {
        let mut calculated = crate::continuous::calculate_characteristics_batch_with_effects(
            ids,
            &self.objects,
            effects,
            &self.battlefield,
            self.commander_objects(),
            self,
        );
        for id in ids {
            if let Some(chars) = self.face_down_conspiracy_characteristics(*id) {
                calculated.insert(*id, chars);
            }
        }
        calculated
    }

    /// Precompute calculated characteristics for a set of objects in one batch.
    ///
    /// This is useful for external snapshot builders that are about to inspect
    /// many battlefield objects and want to avoid repeated one-object layer
    /// calculations. The cache is transient and automatically invalidated by
    /// continuous-effect revision changes.
    pub fn prewarm_calculated_characteristics(&self, ids: &[ObjectId]) {
        if !self.continuous_state_is_clean() {
            return;
        }

        let effects_revision = self.effect_store.continuous_effects.revision();
        let missing: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| {
                !self
                    .runtime_cache
                    .characteristics_cache
                    .contains_valid_entry(*id, effects_revision)
            })
            .collect();
        if missing.is_empty() {
            return;
        }

        let effects = self.cached_continuous_effects_snapshot();
        let calculated = self.calculated_characteristics_batch_with_effects(&missing, &effects);
        for id in missing {
            self.runtime_cache.characteristics_cache.insert(
                id,
                effects_revision,
                calculated.get(&id).cloned(),
            );
        }
    }

    pub fn calculated_characteristics_arc(
        &self,
        id: ObjectId,
    ) -> Option<Arc<crate::continuous::CalculatedCharacteristics>> {
        if self
            .object(id)
            .is_some_and(|object| object.zone == Zone::Battlefield && self.is_phased_out(id))
        {
            return None;
        }
        if let Some(chars) = crate::continuous::in_progress_characteristics(id) {
            return Some(Arc::new(chars));
        }
        let effects_revision = self.effect_store.continuous_effects.revision();
        if self.continuous_state_is_clean() {
            if let Some(cached) = self
                .runtime_cache
                .characteristics_cache
                .get(id, effects_revision)
            {
                self.runtime_cache
                    .work_counters
                    .bump_characteristics_cache_hits();
                #[cfg(feature = "paranoid-invariants")]
                self.assert_cached_characteristics_fresh(id, cached.as_deref());
                return cached;
            }
        }

        let all_effects = self.all_continuous_effects();
        self.runtime_cache
            .work_counters
            .bump_characteristics_full_recomputes();
        self.runtime_cache
            .work_counters
            .add_effects_considered(all_effects.len() as u64);
        if self.continuous_state_is_clean() {
            let mut scope = self.battlefield.clone();
            if !scope.contains(&id) {
                scope.push(id);
            }
            let missing: Vec<_> = scope
                .into_iter()
                .filter(|candidate| {
                    !self
                        .runtime_cache
                        .characteristics_cache
                        .contains_valid_entry(*candidate, effects_revision)
                })
                .collect();
            if !missing.is_empty() {
                let calculated_batch =
                    self.calculated_characteristics_batch_with_effects(&missing, &all_effects);
                let mut calculated = None;
                for candidate in missing {
                    let cached = self.runtime_cache.characteristics_cache.insert(
                        candidate,
                        effects_revision,
                        calculated_batch.get(&candidate).cloned(),
                    );
                    if candidate == id {
                        calculated = cached;
                    }
                }
                return calculated;
            }
        }

        let calculated = self.calculated_characteristics_with_effects(id, &all_effects);
        if self.continuous_state_is_clean() {
            return self.runtime_cache.characteristics_cache.insert(
                id,
                effects_revision,
                calculated,
            );
        }
        calculated.map(Arc::new)
    }

    pub fn calculated_characteristics(
        &self,
        id: ObjectId,
    ) -> Option<crate::continuous::CalculatedCharacteristics> {
        self.calculated_characteristics_arc(id)
            .map(|chars| chars.as_ref().clone())
    }

    #[cfg(feature = "paranoid-invariants")]
    fn assert_cached_characteristics_fresh(
        &self,
        id: ObjectId,
        cached: Option<&crate::continuous::CalculatedCharacteristics>,
    ) {
        if id.0 % 16 != 0 || !self.continuous_state_is_clean() {
            return;
        }
        let effects = self.cached_continuous_effects_snapshot();
        let recomputed = self.calculated_characteristics_with_effects(id, &effects);
        assert_eq!(
            format!("{cached:?}"),
            format!("{:?}", recomputed.as_ref()),
            "stale calculated-characteristics cache entry for object #{}",
            id.0
        );
    }

    /// Return the object's current characteristics in its zone.
    ///
    /// This view reflects continuous effects across all zones and expands
    /// semantic subtype implications like changeling.
    pub fn current_characteristics(&self, id: ObjectId) -> Option<CalculatedCharacteristics> {
        let object = self.object(id)?;
        if object.zone == Zone::Battlefield && self.is_phased_out(id) {
            return None;
        }
        let mut chars =
            self.calculated_characteristics(id)
                .unwrap_or_else(|| CalculatedCharacteristics {
                    name: object.name.clone(),
                    mana_cost: object.mana_cost_owned(),
                    compiled_card_text: object.compiled_card_text.clone(),
                    power: object.power(),
                    toughness: object.toughness(),
                    card_types: object.card_types.clone(),
                    subtypes: object.subtypes.clone(),
                    supertypes: object.supertypes.clone(),
                    world_supertype_since: object
                        .supertypes
                        .contains(&crate::types::Supertype::World)
                        .then_some(0),
                    colors: object.colors(),
                    loyalty: object.base_loyalty,
                    abilities: object.abilities.clone().into(),
                    static_abilities: object
                        .abilities
                        .iter()
                        .filter_map(|ability| match &ability.kind {
                            AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                            _ => None,
                        })
                        .chain(object.level_granted_abilities().iter().cloned())
                        .chain(
                            object
                                .temporary_static_ability_grants
                                .iter()
                                .filter(|grant| !grant.is_expired(self.turn.turn_number))
                                .filter_map(|grant| grant.materialize()),
                        )
                        .collect::<Vec<_>>()
                        .into(),
                    ability_gain_prohibitions: Vec::new(),
                    aura_attach_filter: object.aura_attach_filter_owned(),
                    controller: self.controller_of(object),
                });

        let has_changeling = chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::Changeling);
        let can_have_creature_subtypes = chars.card_types.iter().any(|card_type| {
            matches!(
                card_type,
                crate::types::CardType::Creature | crate::types::CardType::Kindred
            )
        });
        if object.zone != crate::zone::Zone::Battlefield
            && has_changeling
            && can_have_creature_subtypes
        {
            for subtype in crate::types::Subtype::all_creature_types() {
                if !chars.subtypes.contains(subtype) {
                    chars.subtypes.push(*subtype);
                }
            }
        }

        Some(chars)
    }

    /// Return the object's current name in its zone.
    pub fn current_name(&self, id: ObjectId) -> Option<String> {
        Some(self.current_characteristics(id)?.name.to_owned_string())
    }

    /// Return the object's current controller in its zone.
    pub fn current_controller(&self, id: ObjectId) -> Option<PlayerId> {
        self.current_controller_excluding_change_effect(id, None)
    }

    pub(crate) fn current_controller_excluding_change_effect(
        &self,
        id: ObjectId,
        skipped_effect: Option<ContinuousEffectId>,
    ) -> Option<PlayerId> {
        let object = self.object(id)?;
        if self.is_face_up_planar_object(id) {
            return if self.grand_melee().is_some() {
                self.planar_controller_of_face(id)
            } else {
                self.planar_controller()
            };
        }
        if self.is_vanguard_card(id) {
            return Some(object.owner);
        }
        if self.is_face_up_scheme(id) {
            return Some(object.owner);
        }
        if self.is_conspiracy_card(id) {
            return Some(object.owner);
        }
        if skipped_effect.is_none()
            && self.continuous_state_is_clean()
            && let Some(controller) = self.cached_current_controller(id, object)
        {
            return Some(controller);
        }

        let mut effects = self.controller_change_effects_for_uncached_lookup();
        effects.sort_by(|a, b| {
            let layer_cmp = a.modification.layer().cmp(&b.modification.layer());
            if layer_cmp != std::cmp::Ordering::Equal {
                return layer_cmp;
            }
            a.timestamp.cmp(&b.timestamp)
        });
        Some(self.controller_from_change_effects(id, object, &effects, skipped_effect))
    }

    fn cached_current_controller(&self, id: ObjectId, object: &Object) -> Option<PlayerId> {
        {
            let needs_rebuild = self
                .runtime_cache
                .controller_cache
                .borrow()
                .as_ref()
                .is_none_or(|cache| !cache.matches_state(self));
            if needs_rebuild {
                let change_effects = Arc::new(self.controller_change_effects_for_cached_lookup());
                *self.runtime_cache.controller_cache.borrow_mut() = Some(ControllerCache {
                    revision: self.effect_store.continuous_effects.revision(),
                    turn_number: self.turn.turn_number,
                    active_player: self.turn.active_player,
                    phase: self.turn.phase,
                    step: self.turn.step,
                    change_effects,
                    resolved: RefCell::new(FxMap::default()),
                });
            }
        }

        if let Some(controller) = self
            .runtime_cache
            .controller_cache
            .borrow()
            .as_ref()
            .and_then(|cache| cache.resolved.borrow().get(&id).copied())
        {
            return Some(controller);
        }

        let change_effects = {
            let cache = self.runtime_cache.controller_cache.borrow();
            Arc::clone(&cache.as_ref()?.change_effects)
        };
        let controller = self.controller_from_change_effects(id, object, &change_effects, None);
        if let Some(cache) = self.runtime_cache.controller_cache.borrow().as_ref() {
            cache.resolved.borrow_mut().insert(id, controller);
        }
        Some(controller)
    }

    fn controller_change_effects_for_cached_lookup(&self) -> Vec<ContinuousEffect> {
        let mut effects: Vec<_> = self
            .cached_continuous_effects_snapshot_arc()
            .iter()
            .filter(|effect| matches!(effect.modification, Modification::ChangeController(_)))
            .cloned()
            .collect();
        effects.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        effects
    }

    fn controller_change_effects_for_uncached_lookup(&self) -> Vec<ContinuousEffect> {
        if self.continuous_state_is_clean() {
            self.controller_change_effects_for_cached_lookup()
        } else {
            self.effect_store
                .continuous_effects
                .effects_sorted()
                .into_iter()
                .filter(|effect| matches!(effect.modification, Modification::ChangeController(_)))
                .cloned()
                .collect()
        }
    }

    fn controller_from_change_effects(
        &self,
        id: ObjectId,
        object: &Object,
        effects: &[ContinuousEffect],
        skipped_effect: Option<ContinuousEffectId>,
    ) -> PlayerId {
        let mut controller = object.owner;
        for effect in effects
            .iter()
            .filter(|effect| matches!(effect.modification, Modification::ChangeController(_)))
        {
            if skipped_effect == Some(effect.id) {
                continue;
            }
            if let EffectSourceType::Resolution { locked_targets } = &effect.source_type
                && !locked_targets.contains(&id)
            {
                continue;
            }
            let can_apply = match &effect.applies_to {
                EffectTarget::Specific(target) => *target == id,
                EffectTarget::Source => effect.source == id,
                EffectTarget::AllPermanents => object.zone == Zone::Battlefield,
                EffectTarget::AttachedTo(source) => {
                    self.object(*source)
                        .and_then(|source| source.attached_to)
                        .and_then(|target| target.object_id())
                        == Some(id)
                }
                EffectTarget::AllCreatures | EffectTarget::Filter(_) => true,
            };
            if !can_apply {
                continue;
            }

            if !crate::continuous::continuous_effect_duration_and_condition_are_active(effect, self)
            {
                continue;
            }
            let applies = match &effect.applies_to {
                EffectTarget::Specific(target) => *target == id,
                EffectTarget::Source => effect.source == id,
                EffectTarget::AllPermanents => object.zone == Zone::Battlefield,
                EffectTarget::AllCreatures => {
                    object.zone == Zone::Battlefield && self.current_is_creature(id)
                }
                EffectTarget::Filter(filter) => filter.matches(
                    object,
                    &self.filter_context_for(effect.controller, Some(effect.source)),
                    self,
                ),
                EffectTarget::AttachedTo(source) => {
                    self.object(*source)
                        .and_then(|source| source.attached_to)
                        .and_then(|target| target.object_id())
                        == Some(id)
                }
            };
            if applies && let Modification::ChangeController(new_controller) = effect.modification {
                controller = new_controller;
            }
        }
        controller
    }

    /// Return the object's current controller, falling back to its owner if the
    /// object cannot be evaluated through continuous effects.
    pub fn controller_of(&self, object: &Object) -> PlayerId {
        self.current_controller(object.id).unwrap_or(object.owner)
    }

    /// Return the object's current controller by object id.
    pub fn controller_of_id(&self, id: ObjectId) -> Option<PlayerId> {
        let object = self.object(id)?;
        Some(self.controller_of(object))
    }

    /// Set an object's controller as derived state rather than object storage.
    pub fn set_current_controller(&mut self, id: ObjectId, controller: PlayerId) {
        let Some(owner) = self.object(id).map(|object| object.owner) else {
            return;
        };
        if self.current_controller(id) == Some(controller) {
            return;
        }
        self.set_summoning_sick(id);
        if owner == controller {
            return;
        }
        let effect = ContinuousEffect::new(
            id,
            controller,
            EffectTarget::Specific(id),
            Modification::ChangeController(controller),
        )
        .until(Until::Forever);
        self.effect_store.continuous_effects.add_effect(effect);
        self.refresh_continuous_state();
    }

    /// Return the object's current card types in its zone.
    pub fn current_card_types(&self, id: ObjectId) -> Option<Vec<crate::types::CardType>> {
        Some(self.current_characteristics(id)?.card_types.to_vec())
    }

    /// Return the object's current subtypes in its zone.
    pub fn current_subtypes(&self, id: ObjectId) -> Option<Vec<crate::types::Subtype>> {
        Some(self.current_characteristics(id)?.subtypes.to_vec())
    }

    /// Return the object's current supertypes in its zone.
    pub fn current_supertypes(&self, id: ObjectId) -> Option<Vec<crate::types::Supertype>> {
        Some(self.current_characteristics(id)?.supertypes.to_vec())
    }

    /// Return the object's current colors in its zone.
    pub fn current_colors(&self, id: ObjectId) -> Option<crate::color::ColorSet> {
        Some(self.current_characteristics(id)?.colors)
    }

    /// Return the object's current power in its zone, if any.
    pub fn current_power(&self, id: ObjectId) -> Option<i32> {
        self.current_characteristics(id)?.power
    }

    /// Return the object's current toughness in its zone, if any.
    pub fn current_toughness(&self, id: ObjectId) -> Option<i32> {
        self.current_characteristics(id)?.toughness
    }

    /// Return the abilities an object currently has in its zone.
    pub fn current_abilities(&self, id: ObjectId) -> Option<Vec<Ability>> {
        Some(self.current_characteristics(id)?.abilities.to_vec())
    }

    /// Return a specific current ability by index.
    pub fn current_ability(&self, id: ObjectId, ability_index: usize) -> Option<Ability> {
        self.current_abilities(id)?.get(ability_index).cloned()
    }

    /// Return a specific current activated ability by index.
    pub fn current_activated_ability(
        &self,
        id: ObjectId,
        ability_index: usize,
    ) -> Option<ActivatedAbility> {
        let ability = self.current_ability(id, ability_index)?;
        match ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        }
    }

    /// Check if an object has a specific static ability using precomputed effects.
    pub fn object_has_ability_with_effects(
        &self,
        id: ObjectId,
        ability: &StaticAbility,
        effects: &[ContinuousEffect],
    ) -> bool {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.static_abilities.contains(ability))
            .unwrap_or(false)
    }

    /// Check if an object has a specific card type using precomputed effects.
    pub fn object_has_card_type_with_effects(
        &self,
        id: ObjectId,
        card_type: crate::types::CardType,
        effects: &[ContinuousEffect],
    ) -> bool {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.card_types.contains(&card_type))
            .unwrap_or(false)
    }

    /// Get calculated subtypes using precomputed effects.
    pub fn calculated_subtypes_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Vec<crate::types::Subtype> {
        self.calculated_characteristics_with_effects(id, effects)
            .map(|c| c.subtypes.to_vec())
            .unwrap_or_default()
    }

    /// Get calculated toughness using precomputed effects.
    pub fn calculated_toughness_with_effects(
        &self,
        id: ObjectId,
        effects: &[ContinuousEffect],
    ) -> Option<i32> {
        self.calculated_characteristics_with_effects(id, effects)
            .and_then(|c| c.toughness)
    }

    /// Get the calculated power of a creature (with continuous effects applied).
    pub fn calculated_power(&self, id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(id).and_then(|c| c.power)
    }

    /// Get the calculated toughness of a creature (with continuous effects applied).
    pub fn calculated_toughness(&self, id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(id)
            .and_then(|c| c.toughness)
    }

    /// Check if an object has a specific static ability (with continuous effects applied).
    pub fn object_has_ability(&self, id: ObjectId, ability: &StaticAbility) -> bool {
        self.calculated_characteristics(id)
            .map(|c| c.static_abilities.contains(ability))
            .unwrap_or(false)
    }

    /// Check if an object has a static ability with the given ID.
    pub fn object_has_static_ability_id(
        &self,
        id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        self.current_has_static_ability_id(id, ability_id)
    }

    /// Check if an object currently has a static ability with the given ID.
    pub fn current_has_static_ability_id(
        &self,
        id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        if self.is_suspected(id)
            && matches!(
                ability_id,
                crate::static_abilities::StaticAbilityId::Menace
                    | crate::static_abilities::StaticAbilityId::CantBlock
            )
        {
            return true;
        }

        if let Some(chars) = self.calculated_characteristics(id) {
            return chars
                .static_abilities
                .iter()
                .any(|ability| ability.id() == ability_id && ability.is_active(self, id));
        }

        self.object(id).is_some_and(|object| {
            object.abilities.iter().any(|ability| {
                matches!(&ability.kind, crate::ability::AbilityKind::Static(static_ability)
                    if ability.functions_in(&object.zone)
                        && static_ability.id() == ability_id
                        && static_ability.is_active(self, id))
            })
        })
    }

    /// Get the calculated subtypes of an object (with continuous effects applied).
    pub fn calculated_subtypes(&self, id: ObjectId) -> Vec<crate::types::Subtype> {
        self.calculated_characteristics(id)
            .map(|c| c.subtypes.to_vec())
            .unwrap_or_default()
    }

    /// Get the calculated card types of an object (with continuous effects applied).
    pub fn calculated_card_types(&self, id: ObjectId) -> Vec<crate::types::CardType> {
        self.calculated_characteristics(id)
            .map(|c| c.card_types.to_vec())
            .unwrap_or_default()
    }

    /// Check if an object has a specific card type (with continuous effects applied).
    pub fn object_has_card_type(&self, id: ObjectId, card_type: crate::types::CardType) -> bool {
        self.current_card_types(id)
            .is_some_and(|card_types| card_types.contains(&card_type))
    }

    /// Check if an object currently has a specific card type.
    pub fn current_has_card_type(&self, id: ObjectId, card_type: crate::types::CardType) -> bool {
        self.object_has_card_type(id, card_type)
    }

    /// Check if an object currently has a specific subtype.
    pub fn current_has_subtype(&self, id: ObjectId, subtype: crate::types::Subtype) -> bool {
        self.current_subtypes(id)
            .is_some_and(|subtypes| subtypes.contains(&subtype))
    }

    /// Check if an object currently has a specific supertype.
    pub fn current_has_supertype(&self, id: ObjectId, supertype: crate::types::Supertype) -> bool {
        self.current_supertypes(id)
            .is_some_and(|supertypes| supertypes.contains(&supertype))
    }

    /// Check if an object is currently a creature.
    pub fn current_is_creature(&self, id: ObjectId) -> bool {
        self.current_has_card_type(id, crate::types::CardType::Creature)
    }

    // =========================================================================
    // "Can't" Effect Tracking (Rule 614.17)
    // =========================================================================

    /// Update the CantEffectTracker by scanning static abilities on the battlefield.
    ///
    /// Per Rule 614.17, "can't" effects are not replacement effects - they must
    /// be checked BEFORE attempting an action or event. This function scans all
    /// permanents on the battlefield and populates the tracker based on their
    /// static abilities.
    ///
    /// Call this after:
    /// - State-based actions are checked
    /// - Before processing any event that might be affected by "can't" effects
    /// - After any permanent enters or leaves the battlefield
    pub fn update_cant_effects(&mut self) {
        use crate::ability::AbilityKind;
        use crate::static_abilities::StaticAbility;

        // Clear existing tracker
        self.effect_store.cant_effects.clear();
        self.effect_store
            .mana_spend_effects
            .retain_effect_permissions(self.turn.turn_number);
        self.battlefield_flags_mut().damage_persists.clear();
        let vanguard_hand_modifiers = self
            .vanguard
            .as_ref()
            .map(|state| state.hand_modifiers.clone())
            .unwrap_or_default();
        for player in &mut self.players {
            player.max_hand_size = 7_i32.saturating_add(
                vanguard_hand_modifiers
                    .get(&player.id)
                    .copied()
                    .unwrap_or(0),
            );
            player.land_plays_per_turn = 1;
        }

        let all_effects = if self.continuous_state_is_clean() {
            self.cached_continuous_effects_snapshot_arc()
        } else {
            Arc::new(self.all_continuous_effects())
        };
        if self.cant_effects_static_scan_can_stay_empty(all_effects.as_slice()) {
            return;
        }

        // If no continuous effect can add or remove a restriction-producing
        // static ability, only printed/temporary sources with such an ability
        // need inspection. This avoids deriving every permanent merely because
        // one source (for example Mycosynth Lattice) changes a player rule.
        let effects_can_change_cant_abilities = all_effects
            .iter()
            .any(Self::continuous_effect_requires_cant_update);
        let abilities_to_apply: Vec<(StaticAbility, ObjectId, PlayerId)> =
            if !effects_can_change_cant_abilities {
                self.objects
                    .iter()
                    .flat_map(|(&object_id, object)| {
                        let zone = object.zone;
                        let controller = self.controller_of(object);
                        let mut abilities = object
                            .abilities
                            .iter()
                            .filter_map(|ability| match &ability.kind {
                                AbilityKind::Static(static_ability)
                                    if ability.functions_in(&zone)
                                        && Self::static_ability_requires_cant_update(
                                            static_ability,
                                        )
                                        && static_ability.is_active(self, object_id) =>
                                {
                                    Some((static_ability.clone(), object_id, controller))
                                }
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        if zone == Zone::Battlefield {
                            abilities.extend(
                                object
                                    .level_granted_abilities()
                                    .into_iter()
                                    .filter(|static_ability| {
                                        Self::static_ability_requires_cant_update(static_ability)
                                            && static_ability.is_active(self, object_id)
                                    })
                                    .map(|static_ability| (static_ability, object_id, controller)),
                            );
                            abilities.extend(
                                object
                                    .temporary_static_ability_grants
                                    .iter()
                                    .filter(|grant| !grant.is_expired(self.turn.turn_number))
                                    .filter_map(|grant| grant.materialize())
                                    .filter(|static_ability| {
                                        Self::static_ability_requires_cant_update(static_ability)
                                            && static_ability.is_active(self, object_id)
                                    })
                                    .map(|static_ability| (static_ability, object_id, controller)),
                            );
                        }
                        abilities
                    })
                    .collect()
            } else {
                // Ability-changing effects require the fully layered view so
                // grants and removals are reflected in restriction tracking.
                let battlefield_ids: Vec<_> = self
                    .objects
                    .iter()
                    .filter_map(|(&object_id, object)| {
                        (object.zone == Zone::Battlefield).then_some(object_id)
                    })
                    .collect();
                if self.continuous_state_is_clean() {
                    self.prewarm_calculated_characteristics(&battlefield_ids);
                }
                self.objects
                    .iter()
                    .filter_map(|(&object_id, object)| {
                        let zone = object.zone;
                        let controller = self.controller_of(object);
                        match zone {
                            Zone::Battlefield => Some(
                                if self.continuous_state_is_clean() {
                                    self.calculated_characteristics_arc(object_id)
                                } else {
                                    self.calculated_characteristics_with_effects(
                                        object_id,
                                        all_effects.as_slice(),
                                    )
                                    .map(Arc::new)
                                }
                                .map(|chars| {
                                    chars
                                        .static_abilities
                                        .iter()
                                        .filter(|static_ability| {
                                            static_ability.is_active(self, object_id)
                                        })
                                        .cloned()
                                        .map(|static_ability| {
                                            (static_ability, object_id, controller)
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default(),
                            ),
                            _ => Some(
                                object
                                    .abilities
                                    .iter()
                                    .filter_map(|ability| {
                                        if let AbilityKind::Static(static_ability) = &ability.kind {
                                            if ability.functions_in(&zone)
                                                && static_ability.is_active(self, object_id)
                                            {
                                                Some((
                                                    static_ability.clone(),
                                                    object_id,
                                                    controller,
                                                ))
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            ),
                        }
                    })
                    .flatten()
                    .collect()
            };

        // Now apply each ability's restrictions using the trait method
        for (static_ability, permanent_id, controller) in abilities_to_apply {
            static_ability.apply_restrictions(self, permanent_id, controller);
        }

        // Apply active restriction effects from spells/abilities.
        let current_turn = self.turn.turn_number;
        let mut retained_restrictions = Vec::new();
        let mut active_restrictions = Vec::new();
        for effect in &self.effect_store.restriction_effects {
            if effect.is_active(self, current_turn) {
                retained_restrictions.push(effect.clone());
                active_restrictions.push(effect.clone());
            } else if effect.is_pending()
                || (matches!(
                    effect.duration,
                    crate::effect::Until::ControllersNextUntapStep
                ) && !effect.is_expired(current_turn))
            {
                retained_restrictions.push(effect.clone());
            }
        }
        self.effect_store.restriction_effects = retained_restrictions;

        let mut active_goad = Vec::new();
        for effect in &self.effect_store.goad_effects {
            if effect.is_active(self, current_turn) {
                active_goad.push(effect.clone());
            }
        }
        self.effect_store.goad_effects = active_goad;

        let mut restriction_tracker = CantEffectTracker::default();
        for effect in active_restrictions {
            effect.restriction.apply_with_tagged_objects(
                self,
                &mut restriction_tracker,
                effect.controller,
                Some(effect.source),
                effect.iterated_player,
                &effect.tagged_objects,
            );
        }
        self.effect_store.cant_effects.merge(restriction_tracker);

        // "Can't be regenerated" restrictions disable both new and existing shields.
        let cant_be_regenerated: Vec<_> = self
            .effect_store
            .cant_effects
            .cant_be_regenerated
            .iter()
            .copied()
            .collect();
        for object_id in cant_be_regenerated {
            self.effect_store
                .replacement_effects
                .remove_one_shot_effects_from_source(object_id);
            self.clear_regeneration_shields(object_id);
        }
    }

    fn cant_effects_static_scan_can_stay_empty(&self, all_effects: &[ContinuousEffect]) -> bool {
        use crate::ability::AbilityKind;

        if !self.effect_store.restriction_effects.is_empty()
            || !self.effect_store.goad_effects.is_empty()
        {
            return false;
        }

        if all_effects
            .iter()
            .any(Self::continuous_effect_requires_cant_update)
        {
            return false;
        }

        self.objects.values().all(|object| {
            let printed_abilities_are_irrelevant = object.abilities.iter().all(|ability| {
                if !ability.functions_in(&object.zone) {
                    return true;
                }
                match &ability.kind {
                    AbilityKind::Static(static_ability) => {
                        !Self::static_ability_requires_cant_update(static_ability)
                    }
                    _ => true,
                }
            });
            if !printed_abilities_are_irrelevant || object.zone != Zone::Battlefield {
                return printed_abilities_are_irrelevant;
            }

            let level_abilities_are_irrelevant = object
                .level_granted_abilities()
                .iter()
                .all(|ability| !Self::static_ability_requires_cant_update(ability));
            let temporary_abilities_are_irrelevant = object
                .temporary_static_ability_grants
                .iter()
                .filter(|grant| !grant.is_expired(self.turn.turn_number))
                .filter_map(|grant| grant.materialize())
                .all(|ability| !Self::static_ability_requires_cant_update(&ability));

            level_abilities_are_irrelevant && temporary_abilities_are_irrelevant
        })
    }

    fn continuous_effect_requires_cant_update(effect: &ContinuousEffect) -> bool {
        Self::modification_requires_cant_update(&effect.modification)
    }

    fn modification_requires_cant_update(modification: &Modification) -> bool {
        // Exhaustive on purpose: answering `false` for a variant that can add
        // or remove cant-relevant static abilities silently skips restriction
        // tracking (e.g. an aura's "doesn't untap" never taking effect).
        match modification {
            Modification::CopyOf { .. }
            | Modification::ChangeText { .. }
            | Modification::SetTextBox(_)
            | Modification::CopyStaticAbilityVariants { .. }
            // Restriction modifications materialize as cant-relevant static
            // abilities in calculated characteristics.
            | Modification::CantBeBlocked
            | Modification::CantAttack
            | Modification::CantBlock
            | Modification::DoesntUntap
            // Removals can strip cant-relevant statics granted by other
            // effects; rerun the scan rather than reason about ordering.
            | Modification::RemoveAbility(_)
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana => true,
            Modification::AddAbility(static_ability) => {
                Self::static_ability_requires_cant_update(static_ability)
            }
            Modification::AddAbilityGeneric(ability) => Self::ability_requires_cant_update(ability),
            // Replacing the ability list can remove a printed restriction even
            // when none of the replacement abilities creates one.
            Modification::SetAbilities(_) => true,
            // Activated/triggered ability additions and pure characteristic
            // changes cannot introduce cant-relevant statics.
            Modification::CopyActivatedAbilities { .. }
            | Modification::CopyTriggeredAbilities { .. }
            | Modification::AddCombatDamageDrawAbility
            | Modification::ChangeController(_)
            | Modification::SetName(_)
            | Modification::AddCardTypes(_)
            | Modification::RemoveCardTypes(_)
            | Modification::SetCardTypes(_)
            | Modification::AddSubtypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
            | Modification::RemoveSubtypes(_)
            | Modification::RemoveAllSubtypesOfFamily(_)
            | Modification::SetSubtypes(_)
            | Modification::SetAuraAttachmentFilter(_)
            | Modification::AddSupertypes(_)
            | Modification::RemoveSupertypes(_)
            | Modification::RemoveAllCreatureTypes
            | Modification::AddColors(_)
            | Modification::RemoveColors(_)
            | Modification::SetColors(_)
            | Modification::MakeColorless
            | Modification::SetPower { .. }
            | Modification::SetToughness { .. }
            | Modification::SetPowerToughness { .. }
            | Modification::ModifyPower(_)
            | Modification::ModifyToughness(_)
            | Modification::ModifyPowerToughness { .. }
            | Modification::ModifyPowerToughnessByColorCount { .. }
            | Modification::SwitchPowerToughness => false,
        }
    }

    fn ability_requires_cant_update(ability: &crate::ability::Ability) -> bool {
        match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability) => {
                Self::static_ability_requires_cant_update(static_ability)
            }
            _ => false,
        }
    }

    fn static_ability_requires_cant_update(
        static_ability: &crate::static_abilities::StaticAbility,
    ) -> bool {
        use crate::static_abilities::StaticAbilityId;

        !matches!(
            static_ability.id(),
            StaticAbilityId::Flying
                | StaticAbilityId::FirstStrike
                | StaticAbilityId::DoubleStrike
                | StaticAbilityId::Deathtouch
                | StaticAbilityId::Flash
                | StaticAbilityId::Haste
                | StaticAbilityId::Intimidate
                | StaticAbilityId::Lifelink
                | StaticAbilityId::Menace
                // Protection legality is evaluated directly by targeting,
                // attachment, blocking, and damage paths; it does not
                // populate CantEffectTracker through apply_restrictions.
                | StaticAbilityId::Protection
                | StaticAbilityId::Reach
                | StaticAbilityId::Trample
                | StaticAbilityId::Vigilance
                | StaticAbilityId::Fear
                | StaticAbilityId::Skulk
                | StaticAbilityId::Prowess
                | StaticAbilityId::Flanking
                | StaticAbilityId::UmbraArmor
                | StaticAbilityId::Landwalk
                | StaticAbilityId::Shadow
                | StaticAbilityId::Horsemanship
                | StaticAbilityId::Wither
                | StaticAbilityId::Infect
                | StaticAbilityId::Changeling
                | StaticAbilityId::Partner
                | StaticAbilityId::PartnerWith
                | StaticAbilityId::DoctorsCompanion
                | StaticAbilityId::Assist
                | StaticAbilityId::ReadAhead
                | StaticAbilityId::Anthem
                | StaticAbilityId::GrantAbility
                | StaticAbilityId::GrantObjectAbilityForFilter
                | StaticAbilityId::EquipmentGrant
                | StaticAbilityId::AttachedAbilityGrant
                | StaticAbilityId::CharacteristicDefiningPT
                | StaticAbilityId::SetBasePowerToughnessForFilter
                | StaticAbilityId::AddCardTypes
                | StaticAbilityId::RemoveCardTypes
                | StaticAbilityId::SetCardTypes
                | StaticAbilityId::AddSubtypes
                | StaticAbilityId::AddAllSubtypesOfFamily
                | StaticAbilityId::SetLandSubtypes
                | StaticAbilityId::SetCreatureSubtypes
                | StaticAbilityId::AddColors
                | StaticAbilityId::SetColors
                | StaticAbilityId::SetName
                | StaticAbilityId::MakeColorless
                | StaticAbilityId::AddSupertypes
                | StaticAbilityId::RemoveSupertypes
                | StaticAbilityId::CostReduction
                | StaticAbilityId::ActivatedAbilityCostReduction
                | StaticAbilityId::ActivatedAbilityCostIncrease
                | StaticAbilityId::ThisSpellCostReduction
                | StaticAbilityId::ThisSpellCostReductionManaCost
                | StaticAbilityId::CostIncrease
                | StaticAbilityId::CostReductionManaCost
                | StaticAbilityId::CostIncreaseManaCost
                | StaticAbilityId::CostIncreasePerAdditionalTarget
                | StaticAbilityId::CostIncreaseManaCostPerAdditionalTarget
                | StaticAbilityId::Affinity
                | StaticAbilityId::AffinityForArtifacts
                | StaticAbilityId::Delve
                | StaticAbilityId::Convoke
                | StaticAbilityId::Improvise
                | StaticAbilityId::BlackManaMayBePaidWithLife
                | StaticAbilityId::MinimumSpellTotalMana
        )
    }

    pub fn keep_damage_marked(&mut self, object: ObjectId) {
        self.battlefield_flags_mut().damage_persists.insert(object);
    }

    pub fn damage_persists_on(&self, object: ObjectId) -> bool {
        self.battlefield_flags.damage_persists.contains(&object)
    }
}
