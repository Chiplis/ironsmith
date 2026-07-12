use super::*;

impl GameState {
    /// Check if a creature has summoning sickness.
    pub fn is_summoning_sick(&self, id: ObjectId) -> bool {
        self.battlefield_flags.summoning_sick.contains(&id)
    }

    /// Set summoning sickness on a creature.
    pub fn set_summoning_sick(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().summoning_sick.insert(id) {
            self.mark_summoning_sickness_changed(id);
        }
    }

    /// Remove summoning sickness from a creature (e.g., haste).
    pub fn remove_summoning_sickness(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().summoning_sick.remove(&id) {
            self.mark_summoning_sickness_changed(id);
        }
    }

    /// Get the damage marked on an object.
    pub fn damage_on(&self, id: ObjectId) -> u32 {
        self.battlefield_flags
            .damage_marked
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    /// Mark damage on an object.
    pub fn mark_damage(&mut self, id: ObjectId, amount: u32) {
        if amount > 0 {
            *self
                .battlefield_flags_mut()
                .damage_marked
                .entry(id)
                .or_insert(0) += amount;
        }
    }

    /// Set the exact damage marked on an object.
    pub fn set_damage_marked(&mut self, id: ObjectId, amount: u32) {
        if amount == 0 {
            self.battlefield_flags_mut().damage_marked.remove(&id);
        } else {
            self.battlefield_flags_mut()
                .damage_marked
                .insert(id, amount);
        }
    }

    /// Record that a creature was dealt nonzero damage by a source with deathtouch.
    pub fn mark_deathtouch_damage_since_sba(&mut self, id: ObjectId) {
        self.battlefield_flags_mut()
            .dealt_deathtouch_damage_since_sba
            .insert(id);
    }

    /// Returns true if the creature was dealt nonzero damage by a source with
    /// deathtouch since the last time state-based actions were checked.
    pub fn has_deathtouch_damage_since_sba(&self, id: ObjectId) -> bool {
        self.battlefield_flags
            .dealt_deathtouch_damage_since_sba
            .contains(&id)
    }

    /// Clears the transient deathtouch-damage tracker used by SBA evaluation.
    pub fn clear_deathtouch_damage_since_sba(&mut self) {
        self.battlefield_flags_mut()
            .dealt_deathtouch_damage_since_sba
            .clear();
    }

    /// Returns true if `creature` was dealt damage by `source` this turn.
    pub fn creature_was_damaged_by_source_this_turn(
        &self,
        creature: ObjectId,
        source: ObjectId,
    ) -> bool {
        self.turn_store
            .turn_history
            .creature_was_damaged_by_source_this_turn(creature, source)
    }

    /// Returns true if `creature` was dealt damage by any source this turn.
    pub fn creature_was_damaged_this_turn(&self, creature: ObjectId) -> bool {
        self.turn_store
            .turn_history
            .creature_was_damaged_this_turn(creature)
    }

    pub fn source_dealt_combat_damage_to_player_this_turn(&self, source: ObjectId) -> bool {
        let stable_id = self.object(source).map(|obj| obj.stable_id);
        self.turn_store
            .turn_history
            .source_dealt_combat_damage_to_player_this_turn(source, stable_id)
    }

    pub fn source_dealt_damage_to_player_this_turn(
        &self,
        source: ObjectId,
        player: PlayerId,
    ) -> bool {
        let stable_id = self.object(source).map(|obj| obj.stable_id);
        self.turn_store
            .turn_history
            .source_dealt_damage_to_player_this_turn(source, stable_id, player)
    }

    /// Clear damage from an object.
    pub fn clear_damage(&mut self, id: ObjectId) {
        self.battlefield_flags_mut().damage_marked.remove(&id);
    }

    /// Get the number of regeneration shields on an object.
    pub fn regeneration_shield_count(&self, id: ObjectId) -> u32 {
        self.battlefield_flags
            .regeneration_shields
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    /// Add regeneration shields to an object.
    pub fn add_regeneration_shield(&mut self, id: ObjectId, count: u32) {
        if count > 0 {
            *self
                .battlefield_flags_mut()
                .regeneration_shields
                .entry(id)
                .or_insert(0) += count;
        }
    }

    /// Use one regeneration shield. Returns true if a shield was used.
    pub fn use_regeneration_shield(&mut self, id: ObjectId) -> bool {
        let mut remove_empty_shield_entry = false;
        let used_shield = if let Some(shields) = self
            .battlefield_flags_mut()
            .regeneration_shields
            .get_mut(&id)
        {
            if *shields > 0 {
                *shields -= 1;
                remove_empty_shield_entry = *shields == 0;
                true
            } else {
                false
            }
        } else {
            false
        };

        if used_shield {
            if remove_empty_shield_entry {
                self.battlefield_flags_mut()
                    .regeneration_shields
                    .remove(&id);
            }
            *self
                .battlefield_flags_mut()
                .regenerated_this_turn
                .entry(id)
                .or_insert(0) += 1;
        }

        used_shield
    }

    /// Get how many times an object regenerated this turn.
    pub fn regenerated_this_turn_count(&self, id: ObjectId) -> u32 {
        self.battlefield_flags
            .regenerated_this_turn
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    /// Clear all per-object regeneration counts for this turn.
    pub fn clear_regenerated_this_turn(&mut self) {
        self.battlefield_flags_mut().regenerated_this_turn.clear();
    }

    /// Clear all regeneration shields from an object.
    pub fn clear_regeneration_shields(&mut self, id: ObjectId) {
        self.battlefield_flags_mut()
            .regeneration_shields
            .remove(&id);
    }

    /// Remove cleanup-step damage and regeneration state in one copy-on-write
    /// mutation. Runtime cost follows the sparse tracker sizes rather than the
    /// number of permanents on the battlefield.
    pub(crate) fn cleanup_damage_and_regeneration_end_of_turn(&mut self) {
        if self.battlefield_flags.damage_marked.is_empty()
            && self.battlefield_flags.regeneration_shields.is_empty()
            && self.battlefield_flags.regenerated_this_turn.is_empty()
        {
            return;
        }

        let BattlefieldFlags {
            damage_marked,
            damage_persists,
            regeneration_shields,
            regenerated_this_turn,
            ..
        } = self.battlefield_flags_mut();
        damage_marked.retain(|object, _| damage_persists.contains(object));
        regeneration_shields.clear();
        regenerated_this_turn.clear();
    }

    /// Check if a creature is monstrous.
    pub fn is_monstrous(&self, id: ObjectId) -> bool {
        self.battlefield_flags.monstrous.contains(&id)
    }

    /// Mark a creature as monstrous.
    pub fn set_monstrous(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().monstrous.insert(id) {
            self.mark_source_designation_changed(id, Self::condition_reads_monstrous_state);
        }
    }

    /// Check if a creature is renowned.
    pub fn is_renowned(&self, id: ObjectId) -> bool {
        self.battlefield_flags.renowned.contains(&id)
    }

    /// Mark a creature as renowned.
    pub fn set_renowned(&mut self, id: ObjectId) {
        self.battlefield_flags_mut().renowned.insert(id);
    }

    /// Return how many permanents this object devoured as it entered.
    pub fn devoured_count(&self, id: ObjectId) -> u32 {
        self.battlefield_flags
            .devoured_counts
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    /// Record how many permanents this object devoured as it entered.
    pub fn set_devoured_count(&mut self, id: ObjectId, count: u32) {
        let changed = if count == 0 {
            self.battlefield_flags_mut()
                .devoured_counts
                .remove(&id)
                .is_some()
        } else {
            self.battlefield_flags_mut()
                .devoured_counts
                .insert(id, count)
                != Some(count)
        };
        if changed {
            self.mark_source_designation_changed(id, Self::condition_reads_devoured_count);
        }
    }

    /// Check if a permanent is suspected.
    pub fn is_suspected(&self, id: ObjectId) -> bool {
        self.battlefield_flags.suspected.contains(&id)
    }

    /// Return all currently suspected permanents.
    pub(crate) fn suspected_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.battlefield_flags.suspected.iter().copied()
    }

    /// Mark a permanent as suspected.
    pub fn set_suspected(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().suspected.insert(id) {
            self.mark_source_designation_changed(id, Self::condition_reads_suspected_state);
        }
    }

    /// Clear the suspected designation from a permanent.
    pub fn clear_suspected(&mut self, id: ObjectId) -> bool {
        let removed = self.battlefield_flags_mut().suspected.remove(&id);
        if removed {
            self.mark_source_designation_changed(id, Self::condition_reads_suspected_state);
        }
        removed
    }

    /// Check if a Case permanent has become solved.
    pub fn is_case_solved(&self, id: ObjectId) -> bool {
        self.battlefield_flags.solved_cases.contains(&id)
    }

    /// Mark a Case permanent solved. Returns true if this changed game state.
    pub fn solve_case(&mut self, id: ObjectId) -> bool {
        let changed = self.battlefield_flags_mut().solved_cases.insert(id);
        if changed {
            self.mark_object_characteristics_dirty(id);
        }
        changed
    }

    /// Check if a permanent is saddled (until end of turn).
    pub fn is_saddled(&self, id: ObjectId) -> bool {
        self.battlefield_flags
            .saddled_until_end_of_turn
            .contains(&id)
    }

    /// Mark a permanent as saddled until end of turn.
    pub fn set_saddled_until_end_of_turn(&mut self, id: ObjectId) {
        if self
            .battlefield_flags_mut()
            .saddled_until_end_of_turn
            .insert(id)
        {
            self.mark_source_designation_changed(id, Self::condition_reads_saddled_state);
        }
    }

    /// Check if a permanent is flipped.
    pub fn is_flipped(&self, id: ObjectId) -> bool {
        self.battlefield_flags.flipped.contains(&id)
    }

    /// Flip a permanent.
    pub fn flip(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.battlefield_flags_mut().flipped.insert(id);
    }

    /// Check if a permanent is face-down.
    pub fn is_face_down(&self, id: ObjectId) -> bool {
        self.battlefield_flags.face_down.contains(&id)
    }

    /// Set a permanent as face-down.
    pub fn set_face_down(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().face_down.insert(id) {
            self.mark_face_down_state_changed(id);
        }
    }

    /// Mark a face-down permanent as manifested.
    pub fn set_manifested(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().manifested.insert(id) {
            self.mark_object_characteristics_dirty(id);
        }
    }

    /// Check if a permanent is manifested.
    pub fn is_manifested(&self, id: ObjectId) -> bool {
        self.battlefield_flags.manifested.contains(&id)
    }

    /// Turn a permanent face-up.
    pub fn set_face_up(&mut self, id: ObjectId) {
        let (face_down_changed, manifested_changed) = {
            let flags = self.battlefield_flags_mut();
            (flags.face_down.remove(&id), flags.manifested.remove(&id))
        };
        if face_down_changed {
            self.mark_face_down_state_changed(id);
        } else if manifested_changed {
            self.mark_object_characteristics_dirty(id);
        }
    }

    /// Return how many times a permanent has transformed since it entered the battlefield.
    pub fn transform_count(&self, id: ObjectId) -> u64 {
        self.battlefield_flags
            .transform_count
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    /// Record that a permanent transformed and refresh its timestamp per CR 613.7g.
    pub fn mark_transformed(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        let next = self
            .battlefield_flags
            .transform_count
            .get(&id)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        self.battlefield_flags_mut()
            .transform_count
            .insert(id, next);
        self.effect_store.continuous_effects.record_entry(id);
        if let Some(stable_id) = self.object(id).map(|o| o.stable_id) {
            self.record_ui_effect_event("transform", None, None, vec![stable_id], None, None);
        }
    }

    /// Transform a transform-like permanent in place.
    pub fn transform_permanent(&mut self, id: ObjectId) -> bool {
        self.refresh_continuous_state();
        self.transform_permanent_with_current_restrictions(id)
    }

    fn transform_permanent_with_current_restrictions(&mut self, id: ObjectId) -> bool {
        if !self.can_transform(id) {
            return false;
        }
        let Some(target) = self.object(id) else {
            return false;
        };
        if target.zone != Zone::Battlefield
            || target.linked_face_layout != LinkedFaceLayout::TransformLike
        {
            return false;
        }
        let Some(other_def) = self.linked_face_definition_by_name_or_id(
            target.other_face_name.as_deref(),
            target.other_face,
        ) else {
            return false;
        };
        if other_def.card.card_types.contains(&CardType::Instant)
            || other_def.card.card_types.contains(&CardType::Sorcery)
        {
            return false;
        }
        let handles = self.object_store.shared_handles_for_definition(&other_def);
        if let Some(obj) = self.object_mut(id) {
            obj.apply_definition_face_with_shared(&other_def, &handles);
        }
        self.mark_transformed(id);
        true
    }

    fn object_has_daybound_keyword(object: &Object) -> bool {
        object.has_static_ability_id(crate::static_abilities::StaticAbilityId::Daybound)
    }

    fn object_has_nightbound_keyword(object: &Object) -> bool {
        object.has_static_ability_id(crate::static_abilities::StaticAbilityId::Nightbound)
    }

    fn object_has_day_or_nightbound_keyword(object: &Object) -> bool {
        Self::object_has_daybound_keyword(object) || Self::object_has_nightbound_keyword(object)
    }

    fn object_starts_daytime_if_unset_as_enters(object: &Object) -> bool {
        object.has_static_ability_id(
            crate::static_abilities::StaticAbilityId::DayNightStartsDayAsEnters,
        )
    }

    /// Apply day/nightbound transformations for the current day/night designation.
    pub fn apply_day_nightbound_transformations(&mut self) {
        if !self.has_day_night {
            return;
        }
        self.refresh_continuous_state();
        self.apply_day_nightbound_transformations_with_current_restrictions();
    }

    pub(super) fn apply_day_nightbound_transformations_with_current_restrictions(
        &mut self,
    ) -> bool {
        if !self.has_day_night {
            return false;
        }
        let ids = self.battlefield.clone();
        let mut transformed = false;
        for id in ids {
            let should_transform = self.object(id).is_some_and(|object| {
                object.zone == Zone::Battlefield
                    && object.linked_face_layout == LinkedFaceLayout::TransformLike
                    && ((self.is_night && Self::object_has_daybound_keyword(object))
                        || (!self.is_night && Self::object_has_nightbound_keyword(object)))
            });
            if should_transform {
                transformed |= self.transform_permanent_with_current_restrictions(id);
            }
        }
        transformed
    }

    /// Apply day/night setup rules for a permanent that just entered the battlefield.
    pub fn handle_day_night_object_entered(&mut self, id: ObjectId) {
        let Some((sets_day_if_unset, daybound_or_nightbound)) =
            self.object(id).and_then(|object| {
                (object.zone == Zone::Battlefield).then(|| {
                    (
                        Self::object_starts_daytime_if_unset_as_enters(object),
                        Self::object_has_day_or_nightbound_keyword(object),
                    )
                })
            })
        else {
            return;
        };

        if !self.has_day_night && (sets_day_if_unset || daybound_or_nightbound) {
            self.set_daytime(true);
        }
        if daybound_or_nightbound {
            self.apply_day_nightbound_transformations();
        }
    }

    /// Set the global day/night designation and transform daybound/nightbound permanents.
    pub fn set_daytime(&mut self, daytime: bool) {
        let night = !daytime;
        let had_day_night = self.has_day_night;
        let changed = self.is_night != night;
        self.has_day_night = true;
        self.is_night = night;
        if !had_day_night || changed {
            self.apply_day_nightbound_transformations();
        }
        if had_day_night && changed {
            self.record_ui_effect_event(
                "day_night",
                None,
                None,
                Vec::new(),
                None,
                Some(if daytime { "day" } else { "night" }.to_string()),
            );
            let provenance = self
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::DayNightChanged);
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::DayNightChangedEvent::new(daytime),
                provenance,
            );
            self.queue_trigger_event(provenance, event);
        }
    }

    pub fn has_day_night(&self) -> bool {
        self.has_day_night
    }

    pub fn is_daytime(&self) -> bool {
        self.has_day_night && !self.is_night
    }

    /// Check if a permanent is phased out.
    pub fn is_phased_out(&self, id: ObjectId) -> bool {
        self.battlefield_flags.phased_out.contains(&id)
    }

    pub(crate) fn phased_out_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.battlefield_flags.phased_out.iter().copied()
    }

    /// Phase out a permanent.
    pub fn phase_out(&mut self, id: ObjectId) {
        let lookback_source_snapshots = self.trigger_source_lookback_snapshots();
        let permanent_snapshot = self
            .object(id)
            .map(|object| self.cached_object_snapshot_with_calculated_characteristics(object));
        self.mark_continuous_state_dirty();
        if self.battlefield_flags_mut().phased_out.insert(id) {
            if let Some(snapshot) = permanent_snapshot {
                self.record_ui_effect_event(
                    "phase_out",
                    None,
                    None,
                    vec![snapshot.stable_id],
                    None,
                    None,
                );
                let provenance = self
                    .provenance_graph_mut()
                    .alloc_root_event(crate::events::EventKind::PermanentPhasedOut);
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::PermanentPhasedOutEvent::new(
                        id,
                        snapshot.controller,
                        Some(snapshot),
                    ),
                    provenance,
                )
                .with_lookback_source_snapshots(lookback_source_snapshots);
                self.queue_trigger_event(provenance, event);
            }
        }
    }

    /// Phase in a permanent.
    pub fn phase_in(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        if self.battlefield_flags_mut().phased_out.remove(&id)
            && let Some(stable_id) = self.object(id).map(|o| o.stable_id)
        {
            self.record_ui_effect_event("phase_in", None, None, vec![stable_id], None, None);
        }
    }

    /// Check if a card is exiled via madness.
    pub fn is_madness_exiled(&self, id: ObjectId) -> bool {
        self.cast_permission_flags.madness_exiled.contains(&id)
    }

    /// Mark a card as exiled via madness.
    pub fn set_madness_exiled(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut().madness_exiled.insert(id);
    }

    /// Clear madness exiled status.
    pub fn clear_madness_exiled(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut().madness_exiled.remove(&id);
    }

    /// Check if a card is exiled via foretell.
    pub fn is_foretold(&self, id: ObjectId) -> bool {
        self.cast_permission_flags.foretold_cards.contains(&id)
    }

    /// Mark a card as exiled via foretell.
    pub fn set_foretold(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut().foretold_cards.insert(id);
    }

    /// Clear foretell exiled status.
    pub fn clear_foretold(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut().foretold_cards.remove(&id);
    }

    /// Check if a card is exiled because its Adventure spell resolved.
    pub fn is_adventure_exiled(&self, id: ObjectId) -> bool {
        self.cast_permission_flags.adventure_exiled.contains(&id)
    }

    /// Mark a card as exiled because its Adventure spell resolved.
    pub fn set_adventure_exiled(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut().adventure_exiled.insert(id);
    }

    /// Clear adventure exiled status.
    pub fn clear_adventure_exiled(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut()
            .adventure_exiled
            .remove(&id);
    }

    /// Check if a card is exiled via plot by the given player.
    pub fn is_plotted_by(&self, id: ObjectId, player: PlayerId) -> bool {
        self.exile_tracking
            .plotted_cards
            .get(&id)
            .is_some_and(|(plotter, _)| *plotter == player)
    }

    pub fn plotted_by(&self, id: ObjectId) -> Option<PlayerId> {
        self.exile_tracking
            .plotted_cards
            .get(&id)
            .map(|(player, _)| *player)
    }

    /// Return the turn number on which a card was plotted.
    pub fn plotted_turn(&self, id: ObjectId) -> Option<u32> {
        self.exile_tracking
            .plotted_cards
            .get(&id)
            .map(|(_, turn)| *turn)
    }

    /// Mark a card as plotted by a player on the current turn.
    pub fn set_plotted(&mut self, id: ObjectId, player: PlayerId) {
        self.set_plotted_on_turn(id, player, self.turn.turn_number);
    }

    pub fn set_plotted_on_turn(&mut self, id: ObjectId, player: PlayerId, turn: u32) {
        self.exile_tracking_mut()
            .plotted_cards
            .insert(id, (player, turn));
    }

    /// Clear plot state for a card.
    pub fn clear_plotted(&mut self, id: ObjectId) {
        self.exile_tracking_mut().plotted_cards.remove(&id);
    }

    /// Track that a player has taken the foretell special action this turn.
    pub fn record_foretell_action(&mut self, player: PlayerId) {
        self.turn_store
            .turn_history
            .foretell_actions_this_turn
            .insert(player);
    }

    /// Check whether the player has already taken the foretell special action this turn.
    pub fn has_foretold_this_turn(&self, player: PlayerId) -> bool {
        self.turn_store
            .turn_history
            .foretell_actions_this_turn
            .contains(&player)
    }

    /// Check if an object is designated as a commander.
    pub fn is_commander_object(&self, id: ObjectId) -> bool {
        self.is_commander(id)
    }

    /// Designate an object as a commander.
    pub fn set_commander(&mut self, id: ObjectId) {
        self.mark_continuous_state_dirty();
        self.commander_tracking_mut().commanders.insert(id);
    }

    /// Clear battlefield state for an object (when leaving battlefield).
    pub fn clear_battlefield_state(&mut self, id: ObjectId) {
        self.clear_soulbond_pair(id);
        {
            let flags = self.battlefield_flags_mut();
            flags.tapped_permanents.remove(&id);
            flags.summoning_sick.remove(&id);
            flags.damage_marked.remove(&id);
            flags.monstrous.remove(&id);
            flags.suspected.remove(&id);
            flags.dealt_deathtouch_damage_since_sba.remove(&id);
            flags.regeneration_shields.remove(&id);
            flags.devoured_counts.remove(&id);
            flags.solved_cases.remove(&id);
            flags.renowned.remove(&id);
            flags.flipped.remove(&id);
            flags.face_down.remove(&id);
            flags.manifested.remove(&id);
            flags.fully_unlocked_rooms.remove(&id);
            flags.transform_count.remove(&id);
            flags.phased_out.remove(&id);
        }
        self.exile_tracking_mut().imprinted_cards.remove(&id);
        self.object_annotations_mut().noted_life_totals.remove(&id);
        {
            let choices = self.choice_store_mut();
            choices.chosen_colors.remove(&id);
            choices.chosen_basic_land_types.remove(&id);
            choices.chosen_land_types.remove(&id);
            choices.chosen_creature_types.remove(&id);
            choices.chosen_card_types.remove(&id);
            choices.chosen_players.remove(&id);
            choices.chosen_named_options.remove(&id);
            choices
                .chosen_modes_by_ability
                .retain(|(source, _), _| *source != id);
        }
        self.turn_store
            .turn_history
            .chosen_modes_by_ability_this_turn
            .retain(|(source, _), _| *source != id);
        // Note: commanders persist across zone changes
    }

    fn soulbond_pair_is_valid(&self, left: ObjectId, right: ObjectId) -> bool {
        if left == right {
            return false;
        }
        let Some(left_obj) = self.object(left) else {
            return false;
        };
        let Some(right_obj) = self.object(right) else {
            return false;
        };
        if left_obj.zone != Zone::Battlefield || right_obj.zone != Zone::Battlefield {
            return false;
        }
        if !self.current_is_creature(left) || !self.current_is_creature(right) {
            return false;
        }
        self.controller_of(left_obj) == self.controller_of(right_obj)
    }

    pub fn clear_soulbond_pair(&mut self, object_id: ObjectId) {
        let transients = self.combat_transients_mut();
        let partner = transients.soulbond_pairs.remove(&object_id);
        if let Some(partner_id) = partner {
            transients.soulbond_pairs.remove(&partner_id);
        }
    }

    pub fn set_soulbond_pair(&mut self, left: ObjectId, right: ObjectId) {
        if !self.soulbond_pair_is_valid(left, right) {
            return;
        }
        self.clear_soulbond_pair(left);
        self.clear_soulbond_pair(right);
        let transients = self.combat_transients_mut();
        transients.soulbond_pairs.insert(left, right);
        transients.soulbond_pairs.insert(right, left);
    }

    pub(crate) fn soulbond_pairs(&self) -> &HashMap<ObjectId, ObjectId> {
        &self.combat_transients.soulbond_pairs
    }

    pub fn soulbond_partner(&self, object_id: ObjectId) -> Option<ObjectId> {
        let partner = self
            .combat_transients
            .soulbond_pairs
            .get(&object_id)
            .copied()?;
        if self
            .combat_transients
            .soulbond_pairs
            .get(&partner)
            .is_none_or(|paired_back| *paired_back != object_id)
        {
            return None;
        }
        self.soulbond_pair_is_valid(object_id, partner)
            .then_some(partner)
    }

    pub(crate) fn soulbond_partner_for_shared_bonus(
        &self,
        object_id: ObjectId,
    ) -> Option<ObjectId> {
        let partner = self
            .combat_transients
            .soulbond_pairs
            .get(&object_id)
            .copied()?;
        if self
            .combat_transients
            .soulbond_pairs
            .get(&partner)
            .is_none_or(|paired_back| *paired_back != object_id)
        {
            return None;
        }
        let left_obj = self.object(object_id)?;
        let right_obj = self.object(partner)?;
        if left_obj.zone != Zone::Battlefield || right_obj.zone != Zone::Battlefield {
            return None;
        }
        if self.controller_of(left_obj) != self.controller_of(right_obj) {
            return None;
        }
        Some(partner)
    }

    pub fn is_soulbond_paired(&self, object_id: ObjectId) -> bool {
        self.soulbond_partner(object_id).is_some()
    }

    /// Clear exile state for an object (when leaving exile).
    pub fn clear_exile_state(&mut self, id: ObjectId) {
        {
            let flags = self.cast_permission_flags_mut();
            flags.madness_exiled.remove(&id);
            flags.foretold_cards.remove(&id);
            flags.adventure_exiled.remove(&id);
        }
        {
            let tracking = self.exile_tracking_mut();
            tracking.plotted_cards.remove(&id);
            tracking.face_down_exile_viewers.remove(&id);
        }
        self.remove_exiled_with_source_link(id);
    }

    /// Allow a player to keep looking at a face-down exiled card.
    pub fn grant_face_down_exile_view(&mut self, id: ObjectId, viewer: PlayerId) {
        self.exile_tracking_mut()
            .face_down_exile_viewers
            .entry(id)
            .or_default()
            .insert(viewer);
    }

    /// Check whether a player may inspect a face-down exiled card.
    pub fn can_player_look_at_face_down_exiled_card(&self, id: ObjectId, viewer: PlayerId) -> bool {
        self.exile_tracking
            .face_down_exile_viewers
            .get(&id)
            .is_some_and(|viewers| viewers.contains(&viewer))
    }

    // === Chosen color helpers ===

    /// Record a chosen color for a permanent.
    pub fn set_chosen_color(&mut self, permanent_id: ObjectId, color: crate::color::Color) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_colors
            .insert(permanent_id, color);
    }

    /// Get a chosen color for a permanent, if any.
    pub fn chosen_color(&self, permanent_id: ObjectId) -> Option<crate::color::Color> {
        self.choice_store.chosen_colors.get(&permanent_id).copied()
    }

    // === Chosen basic land type helpers ===

    /// Record a chosen basic land type for a permanent.
    pub fn set_chosen_basic_land_type(
        &mut self,
        permanent_id: ObjectId,
        subtype: crate::types::Subtype,
    ) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_basic_land_types
            .insert(permanent_id, subtype);
    }

    /// Get a chosen basic land type for a permanent, if any.
    pub fn chosen_basic_land_type(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.choice_store
            .chosen_basic_land_types
            .get(&permanent_id)
            .copied()
    }

    // === Chosen land type helpers ===

    /// Record a chosen land type for a permanent.
    pub fn set_chosen_land_type(&mut self, permanent_id: ObjectId, subtype: crate::types::Subtype) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_land_types
            .insert(permanent_id, subtype);
    }

    /// Get a chosen land type for a permanent, if any.
    pub fn chosen_land_type(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.choice_store
            .chosen_land_types
            .get(&permanent_id)
            .copied()
    }

    // === Chosen creature type helpers ===

    /// Record a chosen creature type for a permanent.
    pub fn set_chosen_creature_type(
        &mut self,
        permanent_id: ObjectId,
        subtype: crate::types::Subtype,
    ) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_creature_types
            .insert(permanent_id, subtype);
    }

    /// Get a chosen creature type for a permanent, if any.
    pub fn chosen_creature_type(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.choice_store
            .chosen_creature_types
            .get(&permanent_id)
            .copied()
    }

    // === Chosen card type helpers ===

    /// Record a chosen card type for a source object.
    pub fn set_chosen_card_type(&mut self, source_id: ObjectId, card_type: crate::types::CardType) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_card_types
            .insert(source_id, card_type);
    }

    /// Get a chosen card type for a source object, if any.
    pub fn chosen_card_type(&self, source_id: ObjectId) -> Option<crate::types::CardType> {
        self.choice_store.chosen_card_types.get(&source_id).copied()
    }

    // === Chosen player helpers ===

    /// Record a chosen player for a permanent.
    pub fn set_chosen_player(&mut self, permanent_id: ObjectId, player: PlayerId) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_players
            .insert(permanent_id, player);
    }

    /// Get a chosen player for a permanent, if any.
    pub fn chosen_player(&self, permanent_id: ObjectId) -> Option<PlayerId> {
        self.choice_store.chosen_players.get(&permanent_id).copied()
    }

    // === Chosen named option helpers ===

    /// Record a chosen named option for a permanent.
    pub fn set_chosen_named_option(&mut self, permanent_id: ObjectId, option: String) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_named_options
            .insert(permanent_id, option);
    }

    pub(crate) fn apply_power_toughness_choice_as_enters_or_turns_face_up(
        &mut self,
        permanent_id: ObjectId,
        controller: PlayerId,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) {
        let abilities = self
            .object(permanent_id)
            .map(|object| object.abilities_vec())
            .unwrap_or_default();
        for ability in abilities {
            let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            let Some(spec) = static_ability.power_toughness_choice_as_enters_or_turns_face_up()
            else {
                continue;
            };
            if spec.options.is_empty() {
                continue;
            }
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
                crate::decisions::specs::ChoiceSpec::single(permanent_id, display_options);
            let mut chosen = crate::decisions::make_decision(
                self,
                decision_maker,
                controller,
                Some(permanent_id),
                choice_spec,
            );
            if let Some(chosen_idx) = chosen.pop().filter(|idx| *idx < spec.options.len()) {
                let option = &spec.options[chosen_idx];
                if let Some(object) = self.object_mut(permanent_id) {
                    object.base_power = Some(crate::card::PtValue::Fixed(option.power));
                    object.base_toughness = Some(crate::card::PtValue::Fixed(option.toughness));
                    for granted in &option.abilities {
                        let ability = crate::ability::Ability::static_ability(granted.clone());
                        if !object.abilities.contains(&ability) {
                            object.abilities_mut().push(ability);
                        }
                    }
                    self.mark_continuous_state_dirty();
                }
            }
        }
    }

    /// Get a chosen named option for a permanent, if any.
    pub fn chosen_named_option(&self, permanent_id: ObjectId) -> Option<&str> {
        self.choice_store
            .chosen_named_options
            .get(&permanent_id)
            .map(String::as_str)
    }

    // === Imprint helpers ===

    /// Imprint a card onto a permanent (used by Chrome Mox, Isochron Scepter, etc.).
    pub fn imprint_card(&mut self, permanent_id: ObjectId, exiled_card_id: ObjectId) {
        self.exile_tracking_mut()
            .imprinted_cards
            .entry(permanent_id)
            .or_default()
            .push(exiled_card_id);
    }

    /// Get the cards imprinted on a permanent.
    pub fn get_imprinted_cards(&self, permanent_id: ObjectId) -> &[ObjectId] {
        self.exile_tracking
            .imprinted_cards
            .get(&permanent_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a permanent has any imprinted cards.
    pub fn has_imprinted_cards(&self, permanent_id: ObjectId) -> bool {
        self.exile_tracking
            .imprinted_cards
            .get(&permanent_id)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Clear imprinted cards when a permanent leaves the battlefield.
    pub fn clear_imprinted_cards(&mut self, permanent_id: ObjectId) {
        self.exile_tracking_mut()
            .imprinted_cards
            .remove(&permanent_id);
    }

    /// Record that `exiled_card_id` was exiled by `source_id`.
    pub fn add_exiled_with_source_link(&mut self, source_id: ObjectId, exiled_card_id: ObjectId) {
        let entry = self
            .exile_tracking_mut()
            .exiled_with_source
            .entry(source_id)
            .or_default();
        if !entry.contains(&exiled_card_id) {
            entry.push(exiled_card_id);
        }
    }

    pub fn add_exiled_with_source_link_returning_to(
        &mut self,
        source_id: ObjectId,
        exiled_card_id: ObjectId,
        return_zone: Zone,
    ) {
        self.add_exiled_with_source_link(source_id, exiled_card_id);
        self.exile_tracking_mut()
            .exiled_with_source_return_zones
            .entry(source_id)
            .or_default()
            .insert(exiled_card_id, return_zone);
    }

    pub fn mark_return_exiled_when_source_leaves(&mut self, source_id: ObjectId) {
        self.exile_tracking_mut()
            .return_exiled_when_source_leaves
            .insert(source_id);
    }

    pub fn return_exiled_for_source_leave(&mut self, source_id: ObjectId) {
        let (linked, return_zones) = {
            let tracking = self.exile_tracking_mut();
            if !tracking.return_exiled_when_source_leaves.remove(&source_id) {
                return;
            }
            let linked = tracking
                .exiled_with_source
                .remove(&source_id)
                .unwrap_or_default();
            let return_zones = tracking
                .exiled_with_source_return_zones
                .remove(&source_id)
                .unwrap_or_default();
            (linked, return_zones)
        };
        for object_id in linked {
            if self
                .object(object_id)
                .is_some_and(|object| object.zone == Zone::Exile)
            {
                let return_zone = return_zones
                    .get(&object_id)
                    .copied()
                    .unwrap_or(Zone::Battlefield);
                self.move_object_by_effect(object_id, return_zone);
            }
        }
    }

    /// Get cards exiled by a specific source object ID.
    pub fn get_exiled_with_source_links(&self, source_id: ObjectId) -> &[ObjectId] {
        self.exile_tracking
            .exiled_with_source
            .get(&source_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn exiled_with_source_entries(&self) -> impl Iterator<Item = (&ObjectId, &Vec<ObjectId>)> {
        self.exile_tracking.exiled_with_source.iter()
    }

    pub fn return_exiled_when_source_leaves_ids(&self) -> impl Iterator<Item = &ObjectId> {
        self.exile_tracking.return_exiled_when_source_leaves.iter()
    }

    pub fn replace_exiled_with_source_links(&mut self, links: HashMap<ObjectId, Vec<ObjectId>>) {
        self.exile_tracking_mut().exiled_with_source = links;
    }

    pub fn replace_return_exiled_when_source_leaves(&mut self, sources: HashSet<ObjectId>) {
        self.exile_tracking_mut().return_exiled_when_source_leaves = sources;
    }

    pub fn transfer_exiled_with_source_links(
        &mut self,
        old_source_id: ObjectId,
        new_source_id: ObjectId,
    ) {
        if old_source_id == new_source_id {
            return;
        }

        let linked = self
            .exile_tracking_mut()
            .exiled_with_source
            .remove(&old_source_id)
            .unwrap_or_default();
        for exiled_card_id in linked {
            self.add_exiled_with_source_link(new_source_id, exiled_card_id);
        }

        if let Some(return_zones) = self
            .exile_tracking_mut()
            .exiled_with_source_return_zones
            .remove(&old_source_id)
        {
            self.exile_tracking_mut()
                .exiled_with_source_return_zones
                .entry(new_source_id)
                .or_default()
                .extend(return_zones);
        }

        if self
            .exile_tracking_mut()
            .return_exiled_when_source_leaves
            .remove(&old_source_id)
        {
            self.exile_tracking_mut()
                .return_exiled_when_source_leaves
                .insert(new_source_id);
        }
    }

    /// Remove an exiled card from all source-link lists.
    pub fn remove_exiled_with_source_link(&mut self, exiled_card_id: ObjectId) {
        let tracking = self.exile_tracking_mut();
        tracking.exiled_with_source.retain(|_, linked| {
            linked.retain(|id| *id != exiled_card_id);
            !linked.is_empty()
        });
        tracking.exiled_with_source_return_zones.retain(|_, zones| {
            zones.remove(&exiled_card_id);
            !zones.is_empty()
        });
    }

    /// Record the component-card identity for a melded permanent.
    pub fn set_melded_permanent(
        &mut self,
        permanent_id: ObjectId,
        components: Vec<MeldComponentState>,
    ) {
        let Some(stable_id) = self
            .object(permanent_id)
            .map(|permanent| permanent.stable_id)
        else {
            return;
        };
        self.commander_tracking_mut()
            .melded_permanents
            .insert(stable_id, MeldedPermanentState { components });
    }

    /// Get meld metadata for a permanent by its stable ID.
    pub fn melded_permanent(&self, stable_id: StableId) -> Option<&MeldedPermanentState> {
        self.commander_tracking.melded_permanents.get(&stable_id)
    }

    /// Remove and return meld metadata for a permanent by stable ID.
    pub fn take_melded_permanent(&mut self, stable_id: StableId) -> Option<MeldedPermanentState> {
        self.commander_tracking_mut()
            .melded_permanents
            .remove(&stable_id)
    }

    /// Record the destination objects created by a zone change.
    pub fn record_zone_change_results(&mut self, source_id: ObjectId, result_ids: Vec<ObjectId>) {
        self.zone_change_result_objects
            .insert(source_id, result_ids);
    }

    /// Return the live object for a prior object id after a zone change, if known.
    pub fn current_object_id_after_zone_change(&self, source_id: ObjectId) -> Option<ObjectId> {
        let mut current = source_id;
        let mut seen = HashSet::new();
        loop {
            if self.objects.contains_key(&current) {
                return Some(current);
            }
            if !seen.insert(current) {
                return None;
            }
            current = self
                .zone_change_result_objects
                .get(&current)
                .and_then(|result_ids| result_ids.first().copied())?;
        }
    }

    /// Take the destination objects created by a zone change.
    pub fn take_zone_change_results(&mut self, source_id: ObjectId) -> Vec<ObjectId> {
        self.zone_change_result_objects
            .remove(&source_id)
            .unwrap_or_default()
    }

    /// Create a linked exile group and return its generated group ID.
    pub fn create_linked_exile_group(
        &mut self,
        mut stable_ids: Vec<StableId>,
        return_zone: Zone,
        return_under_owner_control: bool,
    ) -> u64 {
        // Keep stable order while de-duplicating.
        stable_ids.dedup();

        let tracking = self.exile_tracking_mut();
        tracking.next_linked_exile_group_id = tracking.next_linked_exile_group_id.saturating_add(1);
        let group_id = tracking.next_linked_exile_group_id;
        tracking.linked_exile_groups.insert(
            group_id,
            LinkedExileGroup {
                stable_ids,
                return_zone,
                return_under_owner_control,
            },
        );
        group_id
    }

    /// Take (and clear) a linked exile group.
    pub fn take_linked_exile_group(&mut self, group_id: u64) -> Option<LinkedExileGroup> {
        self.exile_tracking_mut()
            .linked_exile_groups
            .remove(&group_id)
    }

    /// Queue a trigger event to be processed by the game loop.
    /// Use this when effects need to emit events that should generate triggers.
    ///
    /// `parent` is the causal provenance node for this emitted event. If the
    /// event already has a valid provenance, it is preserved.
    fn projected_turn_event_snapshots(
        &self,
        event: &crate::triggers::TriggerEvent,
    ) -> (
        Option<crate::snapshot::ObjectSnapshot>,
        Option<crate::snapshot::ObjectSnapshot>,
    ) {
        let object_snapshot = event
            .downcast::<crate::events::zones::ZoneChangeEvent>()
            .filter(|zone_change| zone_change.to == Zone::Battlefield)
            .and_then(|zone_change| {
                zone_change.objects.first().copied().and_then(|id| {
                    self.object(id)
                        .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, self))
                })
            })
            .or_else(|| event.snapshot().cloned())
            .or_else(|| {
                event.object_id().and_then(|id| {
                    self.object(id)
                        .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, self))
                })
            });
        let source_snapshot = event.source_snapshot().cloned().or_else(|| {
            event.inner().source_object().and_then(|id| {
                self.object(id)
                    .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, self))
            })
        });
        (object_snapshot, source_snapshot)
    }

    pub(crate) fn stage_turn_history_event(&mut self, event: &crate::triggers::TriggerEvent) {
        let (object_snapshot, source_snapshot) = self.projected_turn_event_snapshots(event);
        self.turn_store
            .turn_history
            .stage_event(event, object_snapshot, source_snapshot);
    }

    pub(crate) fn record_turn_history_event(&mut self, event: &crate::triggers::TriggerEvent) {
        let (object_snapshot, source_snapshot) = self.projected_turn_event_snapshots(event);
        self.turn_store
            .turn_history
            .record_event(event, object_snapshot, source_snapshot);
    }

    pub fn queue_trigger_event(
        &mut self,
        parent: ProvNodeId,
        mut event: crate::triggers::TriggerEvent,
    ) {
        use crate::events::DamageEvent;
        use crate::events::DamageTarget;
        use crate::events::permanents::SacrificeEvent;
        use crate::events::zones::ZoneChangeEvent;

        if let Some(damage) = event.downcast::<DamageEvent>()
            && let DamageTarget::Object(object_id) = damage.target
            && let Some(obj) = self.object(object_id)
            && obj.zone == Zone::Battlefield
        {
            self.record_ui_battlefield_transition(
                UiBattlefieldTransitionKind::Damaged,
                obj.stable_id,
            );
        }

        if let Some(sacrifice) = event.downcast::<SacrificeEvent>() {
            let stable_id = sacrifice
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.stable_id)
                .or_else(|| self.object(sacrifice.permanent).map(|obj| obj.stable_id));
            if let Some(stable_id) = stable_id {
                self.record_ui_battlefield_transition(
                    UiBattlefieldTransitionKind::Sacrificed,
                    stable_id,
                );
            }
        }

        if let Some(zone_change) = event.downcast::<ZoneChangeEvent>()
            && zone_change.from == Zone::Battlefield
            && zone_change.to == Zone::Exile
        {
            let stable_id = zone_change
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.stable_id)
                .or_else(|| {
                    zone_change
                        .objects
                        .first()
                        .and_then(|object_id| self.object(*object_id))
                        .map(|obj| obj.stable_id)
                });
            if let Some(stable_id) = stable_id {
                self.record_ui_battlefield_transition(
                    UiBattlefieldTransitionKind::Exiled,
                    stable_id,
                );
            }
        }

        if let Some(mana_added) = event.downcast::<crate::events::ManaAddedEvent>()
            && !mana_added.mana.is_empty()
        {
            let player = mana_added.player;
            let count = mana_added.mana.len() as i64;
            let text: String = mana_added
                .mana
                .iter()
                .map(|symbol| {
                    format!(
                        "{{{}}}",
                        match symbol {
                            crate::mana::ManaSymbol::White => "W".to_string(),
                            crate::mana::ManaSymbol::Blue => "U".to_string(),
                            crate::mana::ManaSymbol::Black => "B".to_string(),
                            crate::mana::ManaSymbol::Red => "R".to_string(),
                            crate::mana::ManaSymbol::Green => "G".to_string(),
                            crate::mana::ManaSymbol::Colorless => "C".to_string(),
                            crate::mana::ManaSymbol::Snow => "S".to_string(),
                            crate::mana::ManaSymbol::X => "X".to_string(),
                            crate::mana::ManaSymbol::Generic(n) => n.to_string(),
                            crate::mana::ManaSymbol::Life(_) => "P".to_string(),
                        }
                    )
                })
                .collect();
            let stable_ids = self
                .object(mana_added.source)
                .map(|obj| vec![obj.stable_id])
                .unwrap_or_default();
            self.record_ui_effect_event(
                "mana_added",
                Some(player),
                None,
                stable_ids,
                Some(count),
                Some(text),
            );
        }

        let initial_provenance = event.provenance();
        if initial_provenance == ProvNodeId::default()
            || self.provenance_graph().node(initial_provenance).is_none()
        {
            let event_provenance = if parent == ProvNodeId::default()
                || self.provenance_graph().node(parent).is_none()
            {
                self.provenance_graph_mut().alloc_root_event(event.kind())
            } else {
                self.alloc_child_event_provenance(parent, event.kind())
            };
            event.set_provenance(event_provenance);
        }

        let queued = self
            .provenance_graph_mut()
            .alloc_child(event.provenance(), ProvenanceNodeKind::TriggerQueued);
        event.set_provenance(queued);
        self.turn_store
            .turn_history
            .remove_staged_event(initial_provenance);
        self.stage_turn_history_event(&event);
        self.effect_store.pending_trigger_events.push(event);
    }

    pub(crate) fn tag_pending_zone_change_event_for_object(
        &mut self,
        event_object: ObjectId,
        tag: crate::tag::TagKey,
        snapshot: crate::snapshot::ObjectSnapshot,
    ) {
        use crate::events::zones::ZoneChangeEvent;

        let Some((index, mut zone_change, provenance, source_snapshot, lookback_source_snapshots)) =
            self.effect_store
                .pending_trigger_events
                .iter()
                .enumerate()
                .rev()
                .find_map(|(index, event)| {
                    let zone_change = event.downcast::<ZoneChangeEvent>()?;
                    let matches_object = zone_change.objects.contains(&event_object)
                        || zone_change.result_objects.contains(&event_object)
                        || zone_change.snapshot.as_ref().is_some_and(|event_snapshot| {
                            event_snapshot.object_id == event_object
                                || event_snapshot.stable_id == snapshot.stable_id
                        });
                    matches_object.then(|| {
                        (
                            index,
                            zone_change.clone(),
                            event.provenance(),
                            event.source_snapshot().cloned(),
                            event.lookback_source_snapshots().to_vec(),
                        )
                    })
                })
        else {
            return;
        };

        zone_change = zone_change.with_object_tag(tag, snapshot);
        let mut replacement =
            crate::triggers::TriggerEvent::new_with_provenance(zone_change, provenance);
        if let Some(source_snapshot) = source_snapshot {
            replacement = replacement.with_source_snapshot(source_snapshot);
        }
        replacement = replacement.with_lookback_source_snapshots(lookback_source_snapshots);
        self.effect_store.pending_trigger_events[index] = replacement;
    }

    /// Take all pending trigger events (empties the queue).
    pub fn take_pending_trigger_events(&mut self) -> Vec<crate::triggers::TriggerEvent> {
        std::mem::take(&mut self.effect_store.pending_trigger_events)
    }

    pub(crate) fn remove_pending_trigger_events_matching_from(
        &mut self,
        start_index: usize,
        mut predicate: impl FnMut(&crate::triggers::TriggerEvent) -> bool,
    ) -> Vec<crate::triggers::TriggerEvent> {
        let mut removed = Vec::new();
        let mut retained = Vec::new();
        for (index, event) in std::mem::take(&mut self.effect_store.pending_trigger_events)
            .into_iter()
            .enumerate()
        {
            if index >= start_index && predicate(&event) {
                self.turn_store
                    .turn_history
                    .remove_staged_event(event.provenance());
                removed.push(event);
            } else {
                retained.push(event);
            }
        }
        self.effect_store.pending_trigger_events = retained;
        removed
    }

    pub fn record_ui_battlefield_transition(
        &mut self,
        kind: UiBattlefieldTransitionKind,
        stable_id: StableId,
    ) {
        if self
            .metadata
            .ui_battlefield_transitions
            .iter()
            .any(|entry| entry.kind == kind && entry.stable_id == stable_id)
        {
            return;
        }
        self.metadata
            .ui_battlefield_transitions
            .push_back(UiBattlefieldTransition { stable_id, kind });
    }

    pub fn take_ui_battlefield_transitions(&mut self) -> Vec<UiBattlefieldTransition> {
        std::mem::take(&mut self.metadata.ui_battlefield_transitions)
            .into_iter()
            .collect()
    }

    pub fn has_ui_battlefield_transitions(&self) -> bool {
        !self.metadata.ui_battlefield_transitions.is_empty()
    }

    pub fn ui_zone_transitions(&self) -> impl Iterator<Item = &UiZoneTransition> {
        self.metadata.ui_zone_transitions.iter()
    }

    pub(super) fn record_ui_zone_transition(
        &mut self,
        old_object_id: ObjectId,
        new_object_id: ObjectId,
        from: Zone,
        to: Zone,
    ) {
        const MAX_UI_ZONE_TRANSITIONS: usize = 128;
        if from == to {
            return;
        }
        let Some(object) = self.object(new_object_id) else {
            return;
        };
        let transition = UiZoneTransition {
            id: self.metadata.next_ui_zone_transition_id,
            old_object_id,
            new_object_id,
            stable_id: object.stable_id,
            owner: object.owner,
            controller: self.controller_of(object),
            from,
            to,
        };
        self.metadata.next_ui_zone_transition_id =
            self.metadata.next_ui_zone_transition_id.saturating_add(1);
        self.metadata.ui_zone_transitions.push_back(transition);
        if self.metadata.ui_zone_transitions.len() > MAX_UI_ZONE_TRANSITIONS {
            while self.metadata.ui_zone_transitions.len() > MAX_UI_ZONE_TRANSITIONS {
                self.metadata.ui_zone_transitions.pop_front();
            }
        }
    }

    pub fn ui_effect_events(&self) -> impl Iterator<Item = &UiEffectEvent> {
        self.metadata.ui_effect_events.iter()
    }

    /// Record a UI-only effect event for the frontend animation layer.
    ///
    /// This has no rules meaning: it is a bounded, append-only feed of
    /// "something visually interesting happened" hints keyed by monotonic id.
    pub fn record_ui_effect_event(
        &mut self,
        kind: &str,
        player: Option<PlayerId>,
        other_player: Option<PlayerId>,
        stable_ids: Vec<StableId>,
        value: Option<i64>,
        text: Option<String>,
    ) {
        const MAX_UI_EFFECT_EVENTS: usize = 64;
        let event = UiEffectEvent {
            id: self.metadata.next_ui_effect_event_id,
            kind: kind.to_string(),
            player,
            other_player,
            stable_ids,
            value,
            text,
        };
        self.metadata.next_ui_effect_event_id =
            self.metadata.next_ui_effect_event_id.saturating_add(1);
        self.metadata.ui_effect_events.push_back(event);
        if self.metadata.ui_effect_events.len() > MAX_UI_EFFECT_EVENTS {
            while self.metadata.ui_effect_events.len() > MAX_UI_EFFECT_EVENTS {
                self.metadata.ui_effect_events.pop_front();
            }
        }
    }

    pub fn provenance_graph(&self) -> &ProvenanceGraph {
        &self.metadata.provenance_graph
    }

    pub fn provenance_graph_mut(&mut self) -> &mut ProvenanceGraph {
        &mut self.metadata.provenance_graph
    }

    /// Ensure a replacement-event envelope has provenance.
    pub fn ensure_event_provenance(&mut self, mut event: Event) -> Event {
        let provenance = event.provenance();
        if provenance == ProvNodeId::default() || self.provenance_graph().node(provenance).is_none()
        {
            let provenance = self.provenance_graph_mut().alloc_root_event(event.kind());
            event.set_provenance(provenance);
        }
        event
    }

    /// Ensure a trigger-event envelope has provenance.
    pub fn ensure_trigger_event_provenance(
        &mut self,
        mut event: crate::triggers::TriggerEvent,
    ) -> crate::triggers::TriggerEvent {
        let provenance = event.provenance();
        if provenance == ProvNodeId::default() || self.provenance_graph().node(provenance).is_none()
        {
            let provenance = self.provenance_graph_mut().alloc_root_event(event.kind());
            event.set_provenance(provenance);
        }
        event
    }

    /// Allocate a provenance child event under `parent` (or a root when parent is unset/invalid).
    pub fn alloc_child_event_provenance(
        &mut self,
        parent: ProvNodeId,
        kind: EventKind,
    ) -> ProvNodeId {
        self.provenance_graph_mut().alloc_child_event(parent, kind)
    }
}
