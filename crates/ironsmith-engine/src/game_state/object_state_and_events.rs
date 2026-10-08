use super::*;

/// Authored installations have distinct existing duplicate contracts.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AuthoredAbilityDuplicatePolicy {
    PreserveAll,
    SuppressEquivalentKindAndZones,
    SuppressEqualAbility,
}

impl GameState {
    /// Install an authored ability on this exact object incarnation.
    /// Callers retain their authored duplicate policy: all occurrences,
    /// equivalent kind/functional zones, or full ability equality. Temporary
    /// grants use their separate timestamp and expiry owner.
    pub(crate) fn install_authored_object_ability(
        &mut self,
        object_id: ObjectId,
        ability: crate::ability::Ability,
        duplicate_policy: AuthoredAbilityDuplicatePolicy,
    ) -> bool {
        let Some(object) = self.object_mut(object_id) else {
            return false;
        };
        let duplicate = match duplicate_policy {
            AuthoredAbilityDuplicatePolicy::PreserveAll => false,
            AuthoredAbilityDuplicatePolicy::SuppressEquivalentKindAndZones => {
                let fingerprint = |ability: &crate::ability::Ability| {
                    format!("{:?}|{:?}", ability.kind, ability.functional_zones)
                };
                let new_fingerprint = fingerprint(&ability);
                object
                    .abilities
                    .iter()
                    .any(|existing| fingerprint(existing) == new_fingerprint)
            }
            AuthoredAbilityDuplicatePolicy::SuppressEqualAbility => {
                object.abilities.contains(&ability)
            }
        };
        if duplicate {
            return false;
        }
        object.abilities_mut().push(ability);
        true
    }

    /// Check if a creature has summoning sickness.
    pub fn is_summoning_sick(&self, id: ObjectId) -> bool {
        self.battlefield_flags.summoning_sick.contains(&id)
    }

    /// Set summoning sickness on a creature.
    pub fn set_summoning_sick(&mut self, id: ObjectId) {
        self.turn_store
            .came_under_control_since_last_upkeep
            .insert(id);
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
            self.object_store.changes.record(id);
            *self
                .battlefield_flags_mut()
                .damage_marked
                .entry(id)
                .or_insert(0) += amount;
        }
    }

    /// Set the exact damage marked on an object.
    pub fn set_damage_marked(&mut self, id: ObjectId, amount: u32) {
        self.object_store.changes.record(id);
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
        self.object_store.changes.record(id);
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
        for id in self
            .battlefield_flags
            .dealt_deathtouch_damage_since_sba
            .iter()
            .copied()
        {
            self.object_store.changes.record(id);
        }
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

    /// The object dealt damage to anything this turn (active voice).
    pub fn source_dealt_damage_this_turn(&self, source: ObjectId) -> bool {
        let stable_id = self.object(source).map(|obj| obj.stable_id);
        self.turn_store
            .turn_history
            .source_dealt_damage_this_turn(source, stable_id)
    }

    /// Whether this exact source object has dealt positive damage to `player`
    /// at any earlier point in the game.
    ///
    /// The raw object ID comparison is intentional. A card that leaves a zone
    /// and returns is a new object and must not inherit the earlier object's
    /// damage history through its stable card identity.
    pub fn source_dealt_damage_to_player_this_game(
        &self,
        source: ObjectId,
        player: PlayerId,
    ) -> bool {
        self.players.iter().any(|involved| {
            self.action_history_for_player(involved.id).any(|record| {
                record
                    .event
                    .downcast::<crate::events::DamageEvent>()
                    .is_some_and(|event| {
                        event.source == source
                            && event.amount > 0
                            && matches!(
                                event.target,
                                crate::events::DamageTarget::Player(target) if target == player
                            )
                    })
            })
        })
    }

    /// Whether this exact source object has dealt positive damage to this
    /// exact object at any earlier point in the game.
    pub fn source_dealt_damage_to_object_this_game(
        &self,
        source: ObjectId,
        object: ObjectId,
    ) -> bool {
        self.players.iter().any(|involved| {
            self.action_history_for_player(involved.id).any(|record| {
                record
                    .event
                    .downcast::<crate::events::DamageEvent>()
                    .is_some_and(|event| {
                        event.source == source
                            && event.amount > 0
                            && matches!(
                                event.target,
                                crate::events::DamageTarget::Object(target) if target == object
                            )
                    })
            })
        })
    }

    /// Clear damage from an object.
    pub fn clear_damage(&mut self, id: ObjectId) {
        self.object_store.changes.record(id);
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

        for id in self.battlefield_flags.damage_marked.keys().copied() {
            self.object_store.changes.record(id);
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

    /// Whether this permanent's entry lore instruction already completed.
    pub fn has_processed_saga_entry_lore(&self, id: ObjectId) -> bool {
        self.battlefield_flags
            .saga_entry_lore_processed
            .contains(&id)
    }

    /// Record completed entry processing independently of how many counters it placed.
    pub fn mark_saga_entry_lore_processed(&mut self, id: ObjectId) {
        if self
            .object(id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
            && self
                .battlefield_flags_mut()
                .saga_entry_lore_processed
                .insert(id)
        {
            self.object_store.changes.record(id);
        }
    }

    /// Check if a creature is monstrous.
    pub fn is_monstrous(&self, id: ObjectId) -> bool {
        self.battlefield_flags.monstrous.contains(&id)
    }

    /// Whether this permanent has dealt damage since it entered.
    pub fn has_dealt_damage_since_entered(&self, id: ObjectId) -> bool {
        self.battlefield_flags.dealt_damage_since_entered.contains(&id)
    }

    /// Record that a battlefield permanent dealt (unprevented) damage.
    pub fn mark_dealt_damage_since_entered(&mut self, id: ObjectId) {
        if !self
            .object(id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
        {
            return;
        }
        if self.battlefield_flags_mut().dealt_damage_since_entered.insert(id) {
            self.mark_source_designation_changed(id, Self::condition_reads_damage_dealt_state);
        }
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

    /// The permanents this object devoured as it entered, as last known.
    pub fn devoured_objects(&self, id: ObjectId) -> &[crate::snapshot::ObjectSnapshot] {
        self.battlefield_flags
            .devoured_objects
            .get(&id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Record the permanents this object devoured as it entered.
    pub fn set_devoured_objects(
        &mut self,
        id: ObjectId,
        objects: Vec<crate::snapshot::ObjectSnapshot>,
    ) {
        if objects.is_empty() {
            self.battlefield_flags_mut().devoured_objects.remove(&id);
        } else {
            self.battlefield_flags_mut()
                .devoured_objects
                .insert(id, objects);
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
    pub fn set_suspected(&mut self, id: ObjectId) -> bool {
        self.update_cant_effects();
        let Some(object) = self
            .object(id)
            .filter(|object| object.zone == Zone::Battlefield)
        else {
            return false;
        };
        if self.is_phased_out(id)
            || self
                .effect_store
                .cant_effects
                .cant_become_suspected
                .contains(&id)
        {
            return false;
        }
        let controller = self.current_controller(id).unwrap_or(object.owner);
        if self.battlefield_flags_mut().suspected.insert(id) {
            // CR 701.60c, 613.7: the designation grants abilities in layer six
            // with the timestamp of suspecting, so later ability loss wins.
            for ability in [
                crate::static_abilities::StaticAbility::menace(),
                crate::static_abilities::StaticAbility::cant_block(),
            ] {
                let effect = crate::continuous::ContinuousEffect::new(
                    id,
                    controller,
                    crate::continuous::EffectTarget::Specific(id),
                    crate::continuous::Modification::AddAbility(ability),
                )
                .with_source_type(crate::continuous::EffectSourceType::Resolution {
                    locked_targets: vec![id],
                })
                .with_condition(crate::ConditionExpr::SourceSuspected)
                .until(crate::effect::Until::ThisLeavesTheBattlefield);
                self.effect_store.continuous_effects.add_effect(effect);
            }
            self.mark_source_designation_changed(id, Self::condition_reads_suspected_state);
            return true;
        }
        false
    }

    /// Clear the suspected designation from a permanent.
    pub fn clear_suspected(&mut self, id: ObjectId) -> bool {
        if self.is_phased_out(id)
            || !self
                .object(id)
                .is_some_and(|object| object.zone == Zone::Battlefield)
        {
            return false;
        }
        let removed = self.battlefield_flags_mut().suspected.remove(&id);
        if removed {
            self.mark_source_designation_changed(id, Self::condition_reads_suspected_state);
        }
        removed
    }

    /// Check if a permanent is prepared.
    pub fn is_prepared(&self, id: ObjectId) -> bool {
        self.battlefield_flags.prepared.contains(&id)
    }

    /// Return all currently prepared permanents.
    pub(crate) fn prepared_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.battlefield_flags.prepared.iter().copied()
    }

    /// Mark a permanent as prepared, putting a copy of its prepare spell into
    /// exile. Returns true if this changed game state.
    ///
    /// A permanent cannot become prepared twice: the second attempt is a no-op
    /// rather than a second prepare spell copy.
    pub fn set_prepared(&mut self, id: ObjectId) -> bool {
        if !self.battlefield_flags_mut().prepared.insert(id) {
            return false;
        }
        if let Some(definition) = self.prepare_spell_definition(id)
            && let Some(controller) = self.object(id).map(|object| self.controller_of(object))
        {
            let copy_id = self.create_object_from_definition(&definition, controller, Zone::Exile);
            self.cast_permission_flags_mut()
                .prepared_spell_copies
                .insert(id, copy_id);
            self.cast_permission_flags_mut()
                .prepared_spell_sources
                .insert(copy_id, id);
        }
        self.mark_object_characteristics_dirty(id);
        true
    }

    /// Clear the prepared designation from a permanent, and with it the prepare
    /// spell copy waiting in exile.
    ///
    /// Used when the permanent leaves the battlefield or an effect unprepares
    /// it. A copy that has already left exile is being cast, so it is left
    /// alone; [`Self::unprepare_for_cast`] is that path.
    pub fn clear_prepared(&mut self, id: ObjectId) -> bool {
        // Read before writing: every object leaving the battlefield calls this,
        // and `battlefield_flags_mut` would clone the copy-on-write flag block
        // (and invalidate the batched layer rebuild) for objects that were
        // never prepared.
        if !self.battlefield_flags.prepared.contains(&id) {
            return false;
        }
        self.battlefield_flags_mut().prepared.remove(&id);
        if let Some(copy_id) = self.unlink_prepared_spell_copy(id)
            && self
                .object(copy_id)
                .is_some_and(|object| object.zone == Zone::Exile)
        {
            self.remove_object(copy_id);
        }
        self.mark_object_characteristics_dirty(id);
        true
    }

    /// The prepared permanent a prepare spell copy belongs to, if any.
    pub fn prepared_spell_source(&self, copy_id: ObjectId) -> Option<ObjectId> {
        self.cast_permission_flags
            .prepared_spell_sources
            .get(&copy_id)
            .copied()
    }

    /// Whether an exiled object is a prepare spell copy its controller may cast.
    pub fn is_prepared_spell_copy(&self, copy_id: ObjectId) -> bool {
        self.cast_permission_flags
            .prepared_spell_sources
            .contains_key(&copy_id)
    }

    /// Drop the designation as the prepare spell copy is cast (CR: the creature
    /// stops being prepared as the copy is cast). The copy is already on its way
    /// to the stack, so it is not removed here.
    pub fn unprepare_for_cast(&mut self, copy_id: ObjectId) {
        let Some(source) = self.prepared_spell_source(copy_id) else {
            return;
        };
        self.battlefield_flags_mut().prepared.remove(&source);
        self.unlink_prepared_spell_copy(source);
        self.mark_object_characteristics_dirty(source);
    }

    fn unlink_prepared_spell_copy(&mut self, id: ObjectId) -> Option<ObjectId> {
        let copy_id = self
            .cast_permission_flags_mut()
            .prepared_spell_copies
            .remove(&id)?;
        self.cast_permission_flags_mut()
            .prepared_spell_sources
            .remove(&copy_id);
        Some(copy_id)
    }

    /// Check if a Case permanent has become solved.
    pub fn is_harnessed(&self, id: ObjectId) -> bool {
        self.battlefield_flags.harnessed.contains(&id)
    }

    pub fn harness(&mut self, id: ObjectId) -> bool {
        if !self
            .object(id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
        {
            return false;
        }
        let changed = self.battlefield_flags_mut().harnessed.insert(id);
        if changed {
            self.mark_continuous_state_dirty();
        }
        changed
    }

    pub fn is_case_solved(&self, id: ObjectId) -> bool {
        self.battlefield_flags.solved_cases.contains(&id)
    }

    /// A Class permanent's level (CR 716.2). Classes are level 1 unless they
    /// gained a level; this is a designation, not level counters (716.4).
    pub fn class_level(&self, id: ObjectId) -> u32 {
        self.battlefield_flags
            .class_levels
            .get(&id)
            .copied()
            .unwrap_or(1)
    }

    /// Set a Class permanent's level. Returns true if this changed game state.
    pub fn set_class_level(&mut self, id: ObjectId, level: u32) -> bool {
        let previous = self.class_level(id);
        if previous == level {
            return false;
        }
        if level <= 1 {
            self.battlefield_flags_mut().class_levels.remove(&id);
        } else {
            self.battlefield_flags_mut().class_levels.insert(id, level);
        }
        self.mark_source_designation_changed(id, Self::condition_reads_class_level);
        true
    }

    /// Mark a Case permanent solved. Returns true if this changed game state.
    pub fn solve_case(&mut self, id: ObjectId) -> bool {
        let changed = self.battlefield_flags_mut().solved_cases.insert(id);
        if changed {
            self.mark_source_designation_changed(id, Self::condition_reads_case_solved);
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

    /// Flip a flip-card permanent, updating every applicable component of a
    /// merged permanent as required by CR 730.2h.
    pub fn flip_permanent(&mut self, id: ObjectId) -> bool {
        if self.is_flipped(id) {
            return false;
        }

        let merged_stable_id = self.object(id).and_then(|object| {
            self.commander_tracking
                .merged_permanents
                .contains_key(&object.stable_id)
                .then_some(object.stable_id)
        });
        if let Some(stable_id) = merged_stable_id {
            let Some(mut merged) = self
                .commander_tracking
                .merged_permanents
                .get(&stable_id)
                .cloned()
            else {
                return false;
            };
            let mut changed = false;
            for component in &mut merged.components {
                if component.flipped || !is_flip_card_layout(component.object.linked_face_layout) {
                    continue;
                }
                let Some(other_definition) = self.linked_face_definition_by_name_or_id(
                    component.object.other_face_name.as_deref(),
                    component.object.other_face,
                ) else {
                    continue;
                };
                component.object.apply_flipped_face(&other_definition);
                component.flipped = true;
                changed = true;
            }
            if !changed {
                return false;
            }
            self.commander_tracking_mut()
                .merged_permanents
                .insert(stable_id, merged);
            self.refresh_merged_permanent_characteristics(id);
            self.flip(id);
            return true;
        }

        let Some(object) = self.object(id) else {
            return false;
        };
        // Only a flip card can flip; a double-faced or split card isn't one.
        if !is_flip_card_layout(object.linked_face_layout) {
            return false;
        }
        let Some(other_definition) = self.linked_face_definition_by_name_or_id(
            object.other_face_name.as_deref(),
            object.other_face,
        ) else {
            return false;
        };
        if let Some(object) = self.object_mut(id) {
            object.apply_flipped_face(&other_definition);
        }
        self.flip(id);
        true
    }

    /// Check if a permanent is face-down.
    pub fn is_face_down(&self, id: ObjectId) -> bool {
        self.battlefield_flags.face_down.contains(&id)
    }

    /// Eligibility for turning an existing permanent face down (CR 712.11).
    /// This is deliberately separate from `set_face_down`: a double-faced
    /// card can enter face down through manifest, cloak, or a face-down cast.
    pub fn can_turn_face_down_permanent(&self, id: ObjectId) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        if object.zone != Zone::Battlefield || self.is_face_down(id) || self.is_phased_out(id) {
            return false;
        }
        let double_faced = |object: &crate::object::Object| {
            matches!(object.linked_face_layout, LinkedFaceLayout::TransformLike)
        };
        if double_faced(object) {
            return false;
        }
        !self
            .commander_tracking
            .merged_permanents
            .get(&object.stable_id)
            .is_some_and(|merged| {
                merged
                    .components
                    .iter()
                    .any(|component| double_faced(&component.object))
            })
    }

    /// Set an object face down.
    ///
    /// CR 730.2f turns every face-up component of a merged permanent face
    /// down. CR 730.2j forbids the action if the face-up merged permanent
    /// contains a double-faced card.
    pub fn set_face_down(&mut self, id: ObjectId) -> bool {
        let merged_stable_id = self.object(id).and_then(|object| {
            self.commander_tracking
                .merged_permanents
                .contains_key(&object.stable_id)
                .then_some(object.stable_id)
        });
        if let Some(stable_id) = merged_stable_id {
            let contains_double_faced_card = self
                .commander_tracking
                .merged_permanents
                .get(&stable_id)
                .is_some_and(|merged| {
                    merged.components.iter().any(|component| {
                        component.object.kind == crate::object::ObjectKind::Card
                            && component.object.linked_face_layout
                                == LinkedFaceLayout::TransformLike
                    })
                });
            if contains_double_faced_card {
                return false;
            }
            if let Some(merged) = self
                .commander_tracking_mut()
                .merged_permanents
                .get_mut(&stable_id)
            {
                for component in &mut merged.components {
                    component.face_down = true;
                }
            }
        }

        // Store the underlying characteristics on the object so layer 1b can
        // apply after any layer-1a copy effect. This same reversible overlay is
        // already used by morph-style face-down casting.
        let overlay_changed = self.object_store.object_mut(id).is_some_and(|object| {
            matches!(object.zone, Zone::Battlefield | Zone::Stack)
                && object.apply_face_down_cast_overlay()
        });
        let face_status_changed = self.battlefield_flags_mut().face_down.insert(id);
        if face_status_changed || overlay_changed {
            self.mark_face_down_state_changed(id);
        }
        face_status_changed || overlay_changed
    }

    /// Mark a face-down permanent as manifested.
    pub fn set_manifested(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().manifested.insert(id) {
            self.mark_object_characteristics_dirty(id);
        }
    }

    /// Record cloak on the exact battlefield incarnation. Current ward and
    /// characteristics cannot reconstruct this provenance.
    pub fn set_cloaked(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().cloaked.insert(id) {
            self.mark_object_characteristics_dirty(id);
        }
    }

    pub fn is_cloaked(&self, id: ObjectId) -> bool {
        self.battlefield_flags.cloaked.contains(&id)
    }

    /// Check if a permanent is manifested.
    pub fn is_manifested(&self, id: ObjectId) -> bool {
        self.battlefield_flags.manifested.contains(&id)
    }

    /// Whether an object can currently be turned face up.
    ///
    /// CR 730.2g prohibits turning up a face-down merged permanent that
    /// contains an instant or sorcery card.
    pub fn can_turn_face_up_permanent(&self, id: ObjectId) -> bool {
        if !self.is_face_down(id) || self.is_phased_out(id) {
            return false;
        }
        !self
            .effect_store
            .cant_effects
            .cant_turn_face_up
            .contains(&id)
            && !self.merged_permanent_blocks_turn_face_up(id)
            && !self.manifested_spell_card_blocks_turn_face_up(id)
    }

    /// CR 701.40g, 701.58g: a manifested or cloaked permanent represented by
    /// an instant or sorcery card can't be turned face up. Only a face-down
    /// battlefield permanent can be one (morph and disguise need a permanent
    /// card), so the printed card types under the face-down overlay decide.
    /// An identity the local engine doesn't know yet (a hidden placeholder)
    /// is not blocked here.
    fn manifested_spell_card_blocks_turn_face_up(&self, id: ObjectId) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        if object.zone != Zone::Battlefield || !self.is_face_down(id) {
            return false;
        }
        let card_types = object
            .face_down_cast_state
            .as_deref()
            .map(|state| &state.card_types)
            .unwrap_or(&object.card_types);
        object.kind == crate::object::ObjectKind::Card
            && (card_types.contains(&CardType::Instant) || card_types.contains(&CardType::Sorcery))
    }

    /// Record the public reveal of a face-down object's real identity
    /// (CR 708.9 / 708.10) for the owner.
    pub(crate) fn record_face_down_reveal(&mut self, id: ObjectId, owner: PlayerId) {
        let Some((stable_id, name)) = self
            .object(id)
            .map(|object| (object.stable_id, object.identity_name().to_string()))
        else {
            return;
        };
        self.record_ui_effect_event(
            "reveal",
            Some(owner),
            None,
            vec![stable_id],
            None,
            Some(name),
        );
    }

    /// CR 708.10: at the end of the game, every face-down permanent and spell
    /// is revealed. Iterates in object-id order so every peer records the same
    /// events. Identities this peer can't open are left to the hidden-card
    /// end-of-match disclosure.
    pub(crate) fn reveal_face_down_objects_at_game_end(&mut self) {
        let mut face_down: Vec<ObjectId> = self
            .battlefield_flags
            .face_down
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|object| matches!(object.zone, Zone::Battlefield | Zone::Stack))
                    && !self.is_hidden_card_placeholder(*id)
            })
            .collect();
        face_down.sort_unstable();
        for id in face_down {
            let Some(owner) = self.object(id).map(|object| object.owner) else {
                continue;
            };
            self.record_face_down_reveal(id, owner);
        }
    }

    fn reveal_face_down_permanent_for_failed_turn_face_up(&mut self, id: ObjectId) {
        let Some((stable_id, name)) = self
            .object(id)
            .map(|object| (object.stable_id, object.identity_name().to_string()))
        else {
            return;
        };
        let controller = self.current_controller(id);
        self.record_ui_effect_event(
            "reveal",
            controller,
            None,
            vec![stable_id],
            None,
            Some(name),
        );
    }

    fn merged_permanent_blocks_turn_face_up(&self, id: ObjectId) -> bool {
        let Some(object) = self.object(id) else {
            return true;
        };
        self.commander_tracking
            .merged_permanents
            .get(&object.stable_id)
            .is_some_and(|merged| {
                merged.components.iter().any(|component| {
                    component.object.kind == crate::object::ObjectKind::Card
                        && (component.object.card_types.contains(&CardType::Instant)
                            || component.object.card_types.contains(&CardType::Sorcery))
                })
            })
    }

    fn reveal_merged_permanent_for_failed_turn_face_up(&mut self, id: ObjectId) {
        let Some((stable_id, component_names)) = self.object(id).and_then(|object| {
            self.merged_permanent(object.stable_id).map(|merged| {
                (
                    object.stable_id,
                    merged
                        .components
                        .iter()
                        .map(|component| component.object.name.to_string())
                        .collect::<Vec<_>>(),
                )
            })
        }) else {
            return;
        };
        let controller = self.current_controller(id);
        self.record_ui_effect_event(
            "reveal",
            controller,
            None,
            vec![stable_id],
            None,
            Some(component_names.join(" + ")),
        );
    }

    /// Turn an object face up, including every face-down merged component.
    pub fn set_face_up(
        &mut self,
        id: ObjectId,
    ) -> Result<bool, crate::static_ability_processor::StaticEffectDiscoveryError> {
        let checkpoint = self.clone();
        let result = (|| {
            self.refresh_continuous_state()?;
            let changed = self.set_face_up_with_current_restrictions(id);
            if changed {
                self.refresh_continuous_state()?;
            }
            Ok(changed)
        })();
        if result.is_err() {
            self.restore_execution_checkpoint(checkpoint, false);
        }
        result
    }

    /// Calculate the face-up view for a query, without executing immediate
    /// programmes or rules procedures belonging to an actual game action.
    pub(crate) fn hypothetical_face_up(
        &self,
        id: ObjectId,
    ) -> Result<Option<Self>, crate::static_ability_processor::StaticEffectDiscoveryError> {
        let mut snapshot = self.continuous_query_snapshot()?;
        if !snapshot.set_face_up_with_current_restrictions(id) {
            return Ok(None);
        }
        Ok(Some(snapshot.continuous_query_snapshot()?))
    }

    fn set_face_up_with_current_restrictions(&mut self, id: ObjectId) -> bool {
        if self
            .effect_store
            .cant_effects
            .cant_turn_face_up
            .contains(&id)
        {
            return false;
        }
        if !self.is_face_down(id) || self.is_phased_out(id) {
            return false;
        }
        if self.merged_permanent_blocks_turn_face_up(id) {
            // CR 730.2g requires the failed action to reveal the permanent,
            // leave it face down, and emit no turned-face-up event.
            self.reveal_merged_permanent_for_failed_turn_face_up(id);
            return false;
        }
        if self.manifested_spell_card_blocks_turn_face_up(id) {
            // CR 701.40g / 701.58g: reveal it and leave it face down; no
            // "turned face up" event (callers emit none on `false`).
            self.reveal_face_down_permanent_for_failed_turn_face_up(id);
            return false;
        }
        let merged_stable_id = self.object(id).and_then(|object| {
            self.commander_tracking
                .merged_permanents
                .contains_key(&object.stable_id)
                .then_some(object.stable_id)
        });
        if let Some(stable_id) = merged_stable_id
            && let Some(merged) = self
                .commander_tracking_mut()
                .merged_permanents
                .get_mut(&stable_id)
        {
            for component in &mut merged.components {
                component.face_down = false;
            }
        }
        let _ = self
            .object_store
            .object_mut(id)
            .is_some_and(|object| object.end_face_down_cast_overlay());
        let (face_down_changed, origin_changed) = {
            let flags = self.battlefield_flags_mut();
            (
                flags.face_down.remove(&id),
                flags.manifested.remove(&id) | flags.cloaked.remove(&id),
            )
        };
        if face_down_changed {
            self.mark_face_down_state_changed(id);
        } else if origin_changed {
            self.mark_object_characteristics_dirty(id);
        }
        face_down_changed
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

    /// Return how many times a permanent has mutated since it entered the battlefield.
    pub fn mutation_count(&self, id: ObjectId) -> u32 {
        self.battlefield_flags
            .mutation_count
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    /// Record that a permanent mutated.
    pub fn mark_mutated(&mut self, id: ObjectId) {
        let next = self.mutation_count(id).saturating_add(1);
        self.battlefield_flags_mut().mutation_count.insert(id, next);
    }

    /// Transform a transform-like permanent in place.
    pub fn transform_permanent(
        &mut self,
        id: ObjectId,
    ) -> Result<bool, crate::static_ability_processor::StaticEffectDiscoveryError> {
        let checkpoint = self.clone();
        let result = (|| {
            self.refresh_continuous_state()?;
            let transformed = self.transform_permanent_with_current_restrictions(id);
            if transformed {
                // The new face can introduce a failing discovery graph. Do not
                // return success with its face, timestamp or notification committed.
                self.refresh_continuous_state()?;
            }
            Ok(transformed)
        })();
        if result.is_err() {
            self.restore_execution_checkpoint(checkpoint, false);
        }
        result
    }

    fn transform_permanent_with_current_restrictions(&mut self, id: ObjectId) -> bool {
        // CR 712.15a / 702.26b apply equally to transform and convert.
        if self.is_face_down(id) || self.is_phased_out(id) || !self.can_transform(id) {
            return false;
        }
        let merged_stable_id = self.object(id).and_then(|object| {
            self.commander_tracking
                .merged_permanents
                .contains_key(&object.stable_id)
                .then_some(object.stable_id)
        });
        if let Some(stable_id) = merged_stable_id {
            let Some(mut merged) = self
                .commander_tracking
                .merged_permanents
                .get(&stable_id)
                .cloned()
            else {
                return false;
            };
            let mut transformed = false;
            for component in &mut merged.components {
                if component.object.linked_face_layout != LinkedFaceLayout::TransformLike {
                    continue;
                }
                let Some(other_definition) = self.linked_face_definition_by_name_or_id(
                    component.object.other_face_name.as_deref(),
                    component.object.other_face,
                ) else {
                    continue;
                };
                if other_definition
                    .card
                    .card_types
                    .contains(&CardType::Instant)
                    || other_definition
                        .card
                        .card_types
                        .contains(&CardType::Sorcery)
                {
                    continue;
                }
                component.object.apply_definition_face(&other_definition);
                transformed = true;
            }
            if !transformed {
                return false;
            }
            self.commander_tracking_mut()
                .merged_permanents
                .insert(stable_id, merged);
            self.refresh_merged_permanent_characteristics(id);
            self.mark_transformed(id);
            return true;
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

    fn permanent_has_day_night_keyword(
        &self,
        id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        !self.is_phased_out(id)
            && !self.is_face_down(id)
            && self
                .object(id)
                .is_some_and(|object| object.zone == Zone::Battlefield)
            && self.current_has_static_ability_id(id, ability_id)
    }

    /// Whether a permanent currently has daybound or nightbound, which carry
    /// the "can't transform except due to its daybound/nightbound ability"
    /// restriction (CR 702.145b/e).
    pub(crate) fn permanent_has_day_or_nightbound(&self, id: ObjectId) -> bool {
        use crate::static_abilities::StaticAbilityId::{Daybound, Nightbound};
        self.permanent_has_day_night_keyword(id, Daybound)
            || self.permanent_has_day_night_keyword(id, Nightbound)
    }

    fn permanent_has_transforming_component(&self, id: ObjectId) -> bool {
        let Some(object) = self.object(id) else {
            return false;
        };
        self.commander_tracking
            .merged_permanents
            .get(&object.stable_id)
            .map(|merged| {
                merged.components.iter().any(|component| {
                    component.object.linked_face_layout == LinkedFaceLayout::TransformLike
                })
            })
            .unwrap_or(object.linked_face_layout == LinkedFaceLayout::TransformLike)
    }

    fn object_starts_daytime_if_unset_as_enters(object: &Object) -> bool {
        object.has_static_ability_id(
            crate::static_abilities::StaticAbilityId::DayNightStartsDayAsEnters,
        )
    }

    /// Apply day/nightbound transformations for the current day/night designation.
    pub fn apply_day_nightbound_transformations(&mut self) {
        self.refresh_continuous_state();
        self.apply_day_nightbound_transformations_with_current_restrictions();
    }

    pub(super) fn apply_day_nightbound_transformations_with_current_restrictions(
        &mut self,
    ) -> bool {
        use crate::static_abilities::StaticAbilityId::{Daybound, Nightbound};
        let ids = self.battlefield.clone();
        let mut designation_changed = false;
        if !self.has_day_night {
            // CR 702.145d/g apply immediately, including when an ability is
            // gained or a permanent phases in, not only upon entry.
            let daybound = ids
                .iter()
                .any(|&id| self.permanent_has_day_night_keyword(id, Daybound));
            let nightbound = ids
                .iter()
                .any(|&id| self.permanent_has_day_night_keyword(id, Nightbound));
            if !daybound && !nightbound {
                return false;
            }
            designation_changed = self.set_day_night_designation(daybound);
        }
        let mut transformed = false;
        for id in ids {
            let should_transform = self.object(id).is_some_and(|object| {
                object.zone == Zone::Battlefield
                    && self.permanent_has_transforming_component(id)
                    && ((self.is_night && self.permanent_has_day_night_keyword(id, Daybound))
                        || (!self.is_night && self.permanent_has_day_night_keyword(id, Nightbound)))
            });
            if should_transform && self.transform_permanent_with_current_restrictions(id) {
                transformed = true;
                // CR 702.145b/e, 701.27e: the permanent transforms, so
                // "transforms into" triggers see it and "As this transforms"
                // programs (CR 712.20) run; those need a decision maker and are
                // applied before the next priority (see check_and_apply_sbas_with).
                // Publication waits for all "as transforms" programs. An
                // early pending event can be drained before those choices by
                // the next state-based-action owner.
                self.turn_store.pending_day_night_as_transforms.push(id);
            }
        }
        transformed || designation_changed
    }

    /// Run "As this transforms" programs (CR 712.20) for permanents that
    /// transformed because day became night or night became day.
    pub(crate) fn apply_pending_day_night_as_transforms(
        &mut self,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<(), crate::game_loop::GameLoopError> {
        if self.turn_store.pending_day_night_as_transforms.is_empty() {
            return Ok(());
        }
        let checkpoint = self.clone();
        let result = (|| {
            let (mut completed, observations) =
                crate::effects::with_action_observations(self, |game| {
                    let mut completed = Vec::new();
                    while let Some(id) = game
                        .turn_store
                        .pending_day_night_as_transforms
                        .first()
                        .copied()
                    {
                        if let Some(controller) = game
                            .object(id)
                            .filter(|object| object.zone == Zone::Battlefield)
                            .and_then(|_| game.current_controller(id))
                        {
                            game.execute_as_transforms_effect_programs(
                                id,
                                controller,
                                decision_maker,
                            )?;
                            if decision_maker.awaiting_choice() {
                                return Ok(completed);
                            }
                        }
                        game.turn_store.pending_day_night_as_transforms.remove(0);
                        let provenance = game
                            .provenance_graph_mut()
                            .alloc_root_event(crate::events::EventKind::Transformed);
                        completed.push(crate::triggers::TriggerEvent::new_with_provenance(
                            crate::events::other::TransformedEvent::new(id),
                            provenance,
                        ));
                    }
                    Ok(completed)
                })?;
            if decision_maker.awaiting_choice() {
                return Ok(());
            }
            // All day/night changes and "as transforms" choices belong to
            // one completed operation. Never freeze intermediate faces.
            crate::effects::observe_lifecycle_completions_with_observations(
                self,
                &mut completed,
                &observations,
            )?;
            for event in completed {
                self.queue_trigger_event(event.provenance(), event);
            }
            Ok(())
        })();
        if result.is_err() || decision_maker.awaiting_choice() {
            self.restore_execution_checkpoint(
                checkpoint,
                result.is_ok() && decision_maker.awaiting_choice(),
            );
        }
        result
    }

    /// Apply day/night setup rules for a permanent that just entered the battlefield.
    pub fn handle_day_night_object_entered(&mut self, id: ObjectId) {
        let Some((sets_day_if_unset, daybound, nightbound)) = self.object(id).and_then(|object| {
            (object.zone == Zone::Battlefield).then(|| {
                (
                    Self::object_starts_daytime_if_unset_as_enters(object),
                    self.permanent_has_day_night_keyword(
                        id,
                        crate::static_abilities::StaticAbilityId::Daybound,
                    ),
                    self.permanent_has_day_night_keyword(
                        id,
                        crate::static_abilities::StaticAbilityId::Nightbound,
                    ),
                )
            })
        }) else {
            return;
        };
        let daybound_or_nightbound = daybound || nightbound;

        if !self.has_day_night {
            if sets_day_if_unset || daybound {
                // CR 702.145d: a daybound permanent makes it day.
                self.set_daytime(true);
            } else if nightbound {
                // CR 702.145g: a nightbound permanent makes it night, unless
                // a daybound permanent is on the battlefield (702.145d).
                let daybound_on_battlefield = self.battlefield.iter().any(|&other| {
                    self.permanent_has_day_night_keyword(
                        other,
                        crate::static_abilities::StaticAbilityId::Daybound,
                    )
                });
                self.set_daytime(daybound_on_battlefield);
            }
        }
        // CR 702.145b / 712.14: if it is night, a daybound double-faced
        // permanent enters transformed. Entering transformed isn't
        // transforming, so no transform event, "As this transforms" program or
        // transform count is produced for it.
        if daybound_or_nightbound && self.has_day_night && self.is_night {
            self.enter_daybound_permanent_transformed(id);
        }
        if daybound_or_nightbound {
            self.apply_day_nightbound_transformations();
        }
    }

    /// Put a just-entered daybound permanent back face up (CR 702.145b).
    fn enter_daybound_permanent_transformed(&mut self, id: ObjectId) -> bool {
        let Some(target) = self.object(id) else {
            return false;
        };
        if target.zone != Zone::Battlefield
            || target.linked_face_layout != LinkedFaceLayout::TransformLike
            || !self.permanent_has_day_night_keyword(
                id,
                crate::static_abilities::StaticAbilityId::Daybound,
            )
            || self
                .commander_tracking
                .merged_permanents
                .contains_key(&target.stable_id)
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
        self.mark_continuous_state_dirty();
        // It is a transformed permanent (back face up), so the face parity
        // reads odd; no transform happened, so no event or new timestamp.
        self.battlefield_flags_mut().transform_count.insert(id, 1);
        true
    }

    /// Change the designation without recursively refreshing characteristics.
    fn set_day_night_designation(&mut self, daytime: bool) -> bool {
        let night = !daytime;
        let had_day_night = self.has_day_night;
        let changed = self.is_night != night;
        if had_day_night && !changed {
            return false;
        }
        self.has_day_night = true;
        self.is_night = night;
        // Day/night can affect continuous characteristics even when no
        // daybound permanent needs to transform.
        self.mark_continuous_state_dirty();
        // CR 731.1a: starting a designation is not "day becomes night" or
        // "night becomes day", the transition represented by this event.
        if had_day_night {
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
        true
    }

    /// Set the global day/night designation and transform daybound/nightbound permanents.
    pub fn set_daytime(&mut self, daytime: bool) {
        if self.set_day_night_designation(daytime) {
            self.apply_day_nightbound_transformations();
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

    #[cfg(test)]
    pub(crate) fn phased_out_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.battlefield_flags.phased_out.iter().copied()
    }

    pub(crate) fn directly_phased_out_under(
        &self,
        controller: PlayerId,
    ) -> impl Iterator<Item = ObjectId> + '_ {
        self.battlefield_flags
            .phased_out_under_controller
            .iter()
            .filter(move |(id, phased_controller)| {
                **phased_controller == controller
                    && !self.battlefield_flags.indirectly_phased_out.contains(id)
                    && !self.is_phase_out_held(**id)
            })
            .map(|(id, _)| *id)
    }

    /// Prevent a directly phased-out permanent from phasing in during untap
    /// until the specified source leaves the battlefield.
    pub fn hold_phased_out_until_source_leaves(&mut self, permanent: ObjectId, source: ObjectId) {
        if !self.is_phased_out(permanent) {
            return;
        }
        self.battlefield_flags_mut()
            .phase_out_holds_by_source
            .entry(source)
            .or_default()
            .insert(permanent);
    }

    pub(super) fn is_phase_out_held(&self, permanent: ObjectId) -> bool {
        self.battlefield_flags
            .phase_out_holds_by_source
            .values()
            .any(|held| held.contains(&permanent))
    }

    /// Release every permanent held phased out by a source that is leaving.
    pub(crate) fn release_phase_out_holds_for_source(&mut self, source: ObjectId) {
        let held = self
            .battlefield_flags_mut()
            .phase_out_holds_by_source
            .remove(&source)
            .unwrap_or_default();
        let mut held = held.into_iter().collect::<Vec<_>>();
        held.sort();
        self.phase_in_simultaneously(&held);
    }

    /// Commit an already-chosen "enters attacking" role. The caller owns
    /// eligibility and any pending choice; this does not declare an attacker
    /// or publish declaration triggers.
    pub(crate) fn add_entering_attacker(
        &mut self,
        creature: ObjectId,
        target: crate::combat_state::AttackTarget,
    ) {
        let attacked_permanent = target.attacked_permanent();
        let as_battle = matches!(target, crate::combat_state::AttackTarget::Battle(_));
        self.retain_attacking_role(creature, &target);
        self.combat
            .get_or_insert_with(Default::default)
            .attackers
            .push(crate::combat_state::AttackerInfo { creature, target });
        // Combat roles can change characteristics and replacement eligibility.
        self.mark_continuous_state_dirty();
        // Capture history at the role commit. Waiting until a later refresh
        // can record the next instruction's changed types as the original
        // attacked types (CR 506.4e).
        if let Some(permanent) = attacked_permanent {
            let original_types = crate::combat_state::AttackedPermanentTypes {
                planeswalker: !as_battle
                    || self.object_has_card_type(permanent, crate::types::CardType::Planeswalker),
                battle: as_battle
                    || self.object_has_card_type(permanent, crate::types::CardType::Battle),
            };
            self.combat
                .as_mut()
                .expect("role committed")
                .attacked_permanent_types
                .entry(permanent)
                .or_insert(original_types);
        }
    }

    /// Remove an attacking or blocking permanent from combat (CR 506.4).
    /// Creatures it blocked stay blocked (CR 509.1h).
    pub(crate) fn remove_object_from_combat(&mut self, id: ObjectId) {
        self.retire_attacking_role(id);
        let was_participating = self.mutate_combat_lanes(|combat| combat.remove_combatant(id));
        self.clear_ninjutsu_attack_targets_for(id);
        if was_participating {
            // Combat roles can condition abilities and replacement matchers.
            // Publish their removal to every calculated-characteristic cache.
            self.mark_continuous_state_dirty();
        }
    }

    /// CR 506.4 / 506.4c: stop a planeswalker or battle from being attacked.
    /// Creatures attacking it remain attacking creatures but attack nothing;
    /// they keep the defending player they had at declaration (CR 508.5),
    /// which is `defending_player` when given, else the permanent's current
    /// controller (planeswalker) or protector (battle).
    pub(crate) fn remove_attacked_permanent_from_combat(
        &mut self,
        permanent: ObjectId,
        defending_player: Option<PlayerId>,
    ) {
        let planeswalker_defender = defending_player.or_else(|| self.controller_of_id(permanent));
        let battle_defender = defending_player.or_else(|| self.battle_protector(permanent));
        if self.mutate_combat_lanes(|combat| {
            combat.remove_attacked_permanent(permanent, planeswalker_defender, battle_defender)
        }) {
            self.mark_continuous_state_dirty();
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

    /// Open the madness cast window while the card's madness trigger casts it.
    pub(crate) fn authorize_madness_cast(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut()
            .madness_cast_authorized
            .insert(id);
    }

    /// Close the madness cast window.
    pub(crate) fn revoke_madness_cast(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut()
            .madness_cast_authorized
            .remove(&id);
    }

    /// Open the miracle cast window while the card's miracle trigger casts it.
    pub(crate) fn authorize_miracle_cast(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut()
            .miracle_cast_authorized
            .insert(id);
    }

    /// Close the miracle cast window.
    pub(crate) fn revoke_miracle_cast(&mut self, id: ObjectId) {
        self.cast_permission_flags_mut()
            .miracle_cast_authorized
            .remove(&id);
    }

    /// Whether this card may be cast for its miracle cost now: only while its
    /// miracle trigger resolves (CR 702.94a).
    pub fn miracle_cast_is_authorized(&self, id: ObjectId) -> bool {
        self.cast_permission_flags
            .miracle_cast_authorized
            .contains(&id)
    }

    /// Whether `player` may cast this exiled card for its madness cost now:
    /// only its owner, only while its madness trigger resolves (CR 702.35a).
    pub fn madness_cast_is_authorized(&self, id: ObjectId, player: PlayerId) -> bool {
        self.cast_permission_flags.madness_exiled.contains(&id)
            && self
                .cast_permission_flags
                .madness_cast_authorized
                .contains(&id)
            && self.object(id).is_some_and(|object| object.owner == player)
    }

    /// Check if a card is exiled via foretell.
    pub fn is_foretold(&self, id: ObjectId) -> bool {
        self.cast_permission_flags.foretold_cards.contains_key(&id)
    }

    /// The turn on which a card became foretold.
    pub fn foretold_turn(&self, id: ObjectId) -> Option<u32> {
        self.cast_permission_flags.foretold_cards.get(&id).copied()
    }

    /// Whether a foretold card may be cast for a foretell cost now: only after
    /// the turn it became foretold has ended (CR 702.143a, 702.143d).
    pub fn foretold_card_is_castable(&self, id: ObjectId) -> bool {
        self.foretold_turn(id)
            .is_some_and(|turn| turn < self.turn.turn_number)
    }

    /// Mark a card as exiled via foretell on the current turn.
    pub fn set_foretold(&mut self, id: ObjectId) {
        self.set_foretold_on_turn(id, self.turn.turn_number);
    }

    /// Mark a card as foretold on a specific turn.
    pub fn set_foretold_on_turn(&mut self, id: ObjectId, turn: u32) {
        // Record the public claim alongside the foretell permission.
        self.record_hidden_foretell_obligation(id);
        if self
            .cast_permission_flags_mut()
            .foretold_cards
            .insert(id, turn)
            .is_none()
        {
            self.mark_continuous_state_dirty();
        }
    }

    /// Clear foretell exiled status.
    pub fn clear_foretold(&mut self, id: ObjectId) {
        if self
            .cast_permission_flags_mut()
            .foretold_cards
            .remove(&id)
            .is_some()
        {
            self.mark_continuous_state_dirty();
        }
    }

    /// Check if a card is exiled because its Adventure spell resolved.
    pub fn is_adventure_exiled(&self, id: ObjectId) -> bool {
        self.cast_permission_flags
            .adventure_exiled
            .contains_key(&id)
    }

    /// The player who may play a card exiled by its resolving Adventure
    /// spell: the controller of that spell, not necessarily the card's owner
    /// (CR 715.3d).
    pub fn adventure_exiled_player(&self, id: ObjectId) -> Option<PlayerId> {
        self.cast_permission_flags
            .adventure_exiled
            .get(&id)
            .copied()
    }

    /// Mark a card as exiled because its Adventure spell resolved, playable
    /// by its owner.
    pub fn set_adventure_exiled(&mut self, id: ObjectId) {
        let Some(owner) = self.object(id).map(|object| object.owner) else {
            return;
        };
        self.set_adventure_exiled_for(id, owner);
    }

    /// Mark a card as exiled because an Adventure spell controlled by
    /// `player` resolved (CR 715.3d).
    pub fn set_adventure_exiled_for(&mut self, id: ObjectId, player: PlayerId) {
        self.cast_permission_flags_mut()
            .adventure_exiled
            .insert(id, player);
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

    /// The plotted designation itself grants casting permission, even when the
    /// card has no printed plot ability. Keep this tied to the exile object.
    pub(crate) fn plotted_cast_permission(
        &self,
        id: ObjectId,
        zone: Zone,
        player: PlayerId,
    ) -> Option<crate::grant_registry::GrantedAlternativeCast> {
        use crate::alternative_cast::AlternativeCastingMethod;
        let card = self.object(id)?;
        if zone != Zone::Exile
            || card.zone != Zone::Exile
            || card.owner != player
            || !self.is_plotted_by(id, player)
            || card
                .alternative_casts
                .iter()
                .any(|method| matches!(method, AlternativeCastingMethod::Plot { .. }))
        {
            return None;
        }
        Some(crate::grant_registry::GrantedAlternativeCast {
            constraints: Default::default(),
            permission_identity: None,
            method: AlternativeCastingMethod::Plot {
                cost: crate::mana::ManaCost::new(),
            },
            source_id: id,
            zone,
            usage_limit: None,
            cast_this_way_grants: Vec::new(),
            permanent_this_way_grants: Vec::new(),
            on_use_effects: Vec::new(),
            cast_this_way_filter: None,
        })
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
        self.clear_prepared(id);
        {
            let flags = self.battlefield_flags_mut();
            flags.tapped_permanents.remove(&id);
            flags.summoning_sick.remove(&id);
            flags.controller_at_last_refresh.remove(&id);
            flags.control_event_snapshots.remove(&id);
            flags.damage_marked.remove(&id);
            flags.battle_protectors.remove(&id);
            flags.monstrous.remove(&id);
            flags.dealt_damage_since_entered.remove(&id);
            flags.suspected.remove(&id);
            flags.dealt_deathtouch_damage_since_sba.remove(&id);
            flags.regeneration_shields.remove(&id);
            flags.devoured_counts.remove(&id);
            flags.devoured_objects.remove(&id);
            flags.solved_cases.remove(&id);
            flags.class_levels.remove(&id);
            flags.harnessed.remove(&id);
            flags.renowned.remove(&id);
            flags.saga_entry_lore_processed.remove(&id);
            flags.flipped.remove(&id);
            flags.face_down.remove(&id);
            flags.manifested.remove(&id);
            flags.cloaked.remove(&id);
            flags.fully_unlocked_rooms.remove(&id);
            flags.rooms_with_no_unlocked_door.remove(&id);
            flags.transform_count.remove(&id);
            flags.mutation_count.remove(&id);
            flags.phased_out.remove(&id);
            flags.phased_out_under_controller.remove(&id);
            flags.indirectly_phased_out.remove(&id);
            flags.phase_out_holds_by_source.remove(&id);
            for held in flags.phase_out_holds_by_source.values_mut() {
                held.remove(&id);
            }
            flags
                .phase_out_holds_by_source
                .retain(|_, held| !held.is_empty());
        }
        self.exile_tracking_mut().imprinted_cards.remove(&id);
        self.object_annotations_mut().noted_life_totals.remove(&id);
        {
            let choices = self.choice_store_mut();
            choices.chosen_colors.remove(&id);
            choices.chosen_color_sets.remove(&id);
            choices.chosen_basic_land_types.remove(&id);
            choices.chosen_land_types.remove(&id);
            choices.chosen_creature_types.remove(&id);
            choices.secret_chosen_subtypes.remove(&id);
            choices.chosen_creature_type_sets.remove(&id);
            choices.chosen_card_types.remove(&id);
            choices.chosen_players.remove(&id);
            choices.chosen_player_sets.remove(&id);
            choices.chosen_objects.remove(&id);
            choices.chosen_named_options.remove(&id);
            choices
                .numeric_acquisitions
                .retain(|owner, _| owner.host != id);
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

    /// Return the player currently designated to protect a battle.
    pub fn battle_protector(&self, battle: ObjectId) -> Option<PlayerId> {
        self.battlefield_flags
            .battle_protectors
            .get(&battle)
            .copied()
    }

    /// Return every player who may legally protect the battle right now.
    pub fn legal_battle_protectors(&self, battle: ObjectId) -> Vec<PlayerId> {
        let Some(object) = self.object(battle) else {
            return Vec::new();
        };
        if object.zone != Zone::Battlefield
            || !self
                .current_card_types(battle)
                .is_some_and(|types| types.contains(&crate::types::CardType::Battle))
        {
            return Vec::new();
        }
        let controller = self.current_controller(battle).unwrap_or(object.owner);
        let is_siege = self
            .current_subtypes(battle)
            .is_some_and(|subtypes| subtypes.contains(&crate::types::Subtype::Siege));
        self.legal_battle_protectors_for(controller, is_siege)
    }

    /// Return the players who may protect a battle with the prospective
    /// controller and battle type. Entry processing uses this before the zone
    /// change is committed so an asynchronous protector choice cannot leave a
    /// half-entered permanent behind.
    pub(crate) fn legal_battle_protectors_for(
        &self,
        controller: PlayerId,
        is_siege: bool,
    ) -> Vec<PlayerId> {
        if !is_siege {
            return self
                .player(controller)
                .is_some_and(|player| player.is_in_game())
                .then_some(controller)
                .into_iter()
                .collect();
        }
        // CR 310.11a: a Siege's protector is an opponent of its controller,
        // never a teammate.
        self.players
            .iter()
            .filter(|player| {
                player.id != controller
                    && player.is_in_game()
                    && self.are_opponents(controller, player.id)
            })
            .map(|player| player.id)
            .collect()
    }

    /// Designate a legal protector for a battle.
    pub fn set_battle_protector(&mut self, battle: ObjectId, protector: PlayerId) -> bool {
        if !self.legal_battle_protectors(battle).contains(&protector) {
            return false;
        }
        self.object_store.changes.record(battle);
        self.battlefield_flags_mut()
            .battle_protectors
            .insert(battle, protector);
        true
    }

    /// Ask the battle's controller to choose its protector from the legal set.
    pub fn choose_battle_protector(
        &mut self,
        battle: ObjectId,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> bool {
        let legal = self.legal_battle_protectors(battle);
        let Some(object) = self.object(battle) else {
            return false;
        };
        let controller = self.current_controller(battle).unwrap_or(object.owner);
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
            controller,
            Some(battle),
            "Choose a player to protect this battle",
            options,
            1,
            1,
        );
        let selected = decision_maker
            .decide_options(self, &context)
            .into_iter()
            .find_map(|index| legal.get(index).copied())
            .or_else(|| {
                (!decision_maker.awaiting_choice())
                    .then(|| legal.first().copied())
                    .flatten()
            });
        selected.is_some_and(|protector| self.set_battle_protector(battle, protector))
    }

    /// Seed intrinsic defense/protector state for direct battlefield fixtures.
    pub(crate) fn initialize_intrinsic_battle_state(&mut self, battle: ObjectId) {
        if !self
            .current_card_types(battle)
            .is_some_and(|types| types.contains(&crate::types::CardType::Battle))
        {
            return;
        }
        let printed_defense = self.object(battle).and_then(|object| object.base_defense);
        if let Some(defense) = printed_defense
            && defense > 0
            && self.counter_count(battle, crate::object::CounterType::Defense) == 0
            && let Some(object) = self.object_mut(battle)
        {
            object.add_counters(crate::object::CounterType::Defense, defense);
        }
        if self.battle_protector(battle).is_none()
            && let Some(protector) = self.legal_battle_protectors(battle).first().copied()
        {
            self.battlefield_flags_mut()
                .battle_protectors
                .insert(battle, protector);
        }
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
        if !self
            .combat_transients
            .soulbond_pairs
            .contains_key(&object_id)
        {
            return;
        }
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
        if self.combat_transients.soulbond_pairs.get(&left) == Some(&right)
            && self.combat_transients.soulbond_pairs.get(&right) == Some(&left)
        {
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

    pub(crate) fn soulbond_identity(&self) -> crate::incremental::ChangeCursor {
        self.combat_transients.soulbond_pairs.cursor()
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
        // A prepare spell copy only leaves exile by being cast (or ceasing to
        // exist); either way its permanent stops being prepared. The zone move
        // gives the copy a new ID, so this is the last point where the old
        // copy ID still resolves to its source; unlinking here without
        // dropping the designation would leave the source stuck prepared.
        if let Some(source) = self.prepared_spell_source(id) {
            self.unlink_prepared_spell_copy(source);
            if self.battlefield_flags.prepared.contains(&source) {
                self.battlefield_flags_mut().prepared.remove(&source);
                self.mark_object_characteristics_dirty(source);
            }
        }
        {
            let tracking = self.exile_tracking_mut();
            tracking.plotted_cards.remove(&id);
            tracking.face_down_exile_viewers.remove(&id);
            tracking.face_down_exile_view_sources.remove(&id);
        }
        self.remove_exiled_with_source_link(id);
    }

    /// Allow a player to keep looking at a face-down exiled card.
    pub fn grant_face_down_exile_view(&mut self, id: ObjectId, viewer: PlayerId) {
        self.object_store.changes.record(id);
        self.object_store.render_changes.record(id);
        Arc::make_mut(&mut self.exile_tracking)
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
            .is_some_and(|viewers| {
                viewers.iter().any(|entitled_player| {
                    *entitled_player == viewer
                        || self.controlling_player_for(*entitled_player) == viewer
                })
            })
            || self
                .face_down_exile_source_controller(id)
                .is_some_and(|player| {
                    player == viewer || self.controlling_player_for(player) == viewer
                })
    }

    /// A resolution-created look permission follows the source permanent's
    /// controller, but never a new incarnation of either linked object.
    pub fn grant_face_down_exile_source_controller_view(&mut self, id: ObjectId, source: ObjectId) {
        self.exile_tracking_mut()
            .face_down_exile_view_sources
            .insert(id, source);
        self.remember_face_down_exile_source_controllers();
    }

    fn face_down_exile_source_controller(&self, id: ObjectId) -> Option<PlayerId> {
        if self.object(id)?.zone != Zone::Exile {
            return None;
        }
        let source = *self.exile_tracking.face_down_exile_view_sources.get(&id)?;
        if self.object(source)?.zone != Zone::Battlefield || self.is_phased_out(source) {
            return None;
        }
        self.current_controller(source)
    }

    /// CR 406.3: once entitled, a player keeps permission even after control
    /// changes or the source leaves. This does not require actually looking.
    pub(crate) fn remember_face_down_exile_source_controllers(&mut self) {
        let grants = self
            .exile_tracking
            .face_down_exile_view_sources
            .keys()
            .copied()
            .filter_map(|id| {
                self.face_down_exile_source_controller(id)
                    .map(|player| (id, player))
            })
            .filter(|(id, player)| {
                !self
                    .exile_tracking
                    .face_down_exile_viewers
                    .get(id)
                    .is_some_and(|viewers| viewers.contains(player))
            })
            .collect::<Vec<_>>();
        for (id, player) in grants {
            self.grant_face_down_exile_view(id, player);
        }
    }

    /// CR 406.3 retains a rules player's inspection entitlement without any
    /// requirement that they actually looked. Exact member incarnations and
    /// active acquisition-bound readers supply new entitlements; the durable
    /// map deliberately does not retain the source as a continuing authority.
    pub(crate) fn remember_linked_exile_inspection_entitlements(&mut self) {
        let entitlements = self
            .effect_store
            .grant_registry
            .linked_exile_inspection_entitlements(self);
        for (member, player) in entitlements {
            if !self
                .exile_tracking
                .face_down_exile_viewers
                .get(&member)
                .is_some_and(|viewers| viewers.contains(&player))
            {
                self.grant_face_down_exile_view(member, player);
            }
        }
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

    /// Record several chosen colors for a permanent ("choose two colors").
    pub fn set_chosen_colors(&mut self, permanent_id: ObjectId, colors: crate::color::ColorSet) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_color_sets
            .insert(permanent_id, colors);
    }

    /// Every color chosen for a permanent: a multi-color choice, or the single
    /// chosen color as a one-color set.
    pub fn chosen_colors(&self, permanent_id: ObjectId) -> Option<crate::color::ColorSet> {
        let set = self.choice_store.chosen_color_sets.get(&permanent_id).copied();
        let single = self
            .chosen_color(permanent_id)
            .map(crate::color::ColorSet::from_color);
        match (set, single) {
            (Some(set), Some(single)) => Some(set.union(single)),
            (set, single) => set.or(single),
        }
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
        self.set_chosen_subtype(permanent_id, subtype);
    }

    /// Record a chosen subtype of any family for a source object.
    pub fn set_chosen_subtype(&mut self, permanent_id: ObjectId, subtype: crate::types::Subtype) {
        self.mark_continuous_state_dirty();
        let choices = self.choice_store_mut();
        choices.chosen_creature_types.insert(permanent_id, subtype);
        choices
            .chosen_creature_type_sets
            .entry(permanent_id)
            .or_default()
            .insert(subtype);
    }

    /// Store a hidden choice on this exact object, without publishing it as a characteristic.
    pub fn set_secret_chosen_subtype(
        &mut self,
        source: ObjectId,
        chooser: PlayerId,
        subtype: crate::types::Subtype,
    ) {
        self.choice_store_mut()
            .secret_chosen_subtypes
            .insert(source, (chooser, subtype));
    }

    pub(crate) fn secret_subtype_snapshot(
        &self,
        source: ObjectId,
    ) -> Option<(PlayerId, crate::types::Subtype)> {
        self.choice_store
            .secret_chosen_subtypes
            .get(&source)
            .copied()
    }

    /// A hidden choice can be inspected only by the player who made it.
    pub fn secret_chosen_subtype(
        &self,
        source: ObjectId,
        viewer: PlayerId,
    ) -> Option<crate::types::Subtype> {
        self.choice_store
            .secret_chosen_subtypes
            .get(&source)
            .filter(|(chooser, _)| *chooser == viewer)
            .map(|(_, subtype)| *subtype)
    }

    /// Get a chosen creature type for a permanent, if any.
    pub fn chosen_creature_type(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.chosen_subtype(permanent_id)
    }

    /// Get the chosen subtype of any family for a source object, if any.
    pub fn chosen_subtype(&self, permanent_id: ObjectId) -> Option<crate::types::Subtype> {
        self.choice_store
            .chosen_creature_types
            .get(&permanent_id)
            .copied()
    }

    /// Every subtype selected for this source, including multi-player choices.
    pub fn chosen_subtypes(&self, source_id: ObjectId) -> Option<&HashSet<crate::types::Subtype>> {
        self.choice_store.chosen_creature_type_sets.get(&source_id)
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

    /// Record several chosen players for a permanent ("choose two players").
    pub fn set_chosen_players(&mut self, permanent_id: ObjectId, players: Vec<PlayerId>) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_player_sets
            .insert(permanent_id, players);
    }

    /// Every player chosen by a permanent: a multi-player choice, else the
    /// single chosen player.
    pub fn chosen_players(&self, permanent_id: ObjectId) -> Vec<PlayerId> {
        if let Some(players) = self.choice_store.chosen_player_sets.get(&permanent_id) {
            return players.clone();
        }
        self.chosen_player(permanent_id).into_iter().collect()
    }

    // === Chosen object helpers ===

    /// Record the singular object chosen for a source.
    pub fn set_chosen_object(
        &mut self,
        source_id: ObjectId,
        object: crate::snapshot::ObjectSnapshot,
    ) {
        self.mark_continuous_state_dirty();
        self.choice_store_mut()
            .chosen_objects
            .insert(source_id, object);
    }

    /// Get the object chosen for a source, if any.
    pub fn chosen_object(&self, source_id: ObjectId) -> Option<&crate::snapshot::ObjectSnapshot> {
        self.choice_store.chosen_objects.get(&source_id)
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
                        self.install_authored_object_ability(
                            permanent_id,
                            ability,
                            AuthoredAbilityDuplicatePolicy::SuppressEqualAbility,
                        );
                    }
                    self.mark_continuous_state_dirty();
                }
            }
        }
    }

    pub fn set_number_for_acquisition(
        &mut self,
        owner: crate::source_numbers::NumberChoiceOwner,
        number: u32,
    ) -> Result<(), crate::effects::ExecutionError> {
        let public_group = if let Some(record) = self.choice_store.numeric_acquisitions.get(&owner)
        {
            record.public_group
        } else if let Some(last) = self
            .choice_store
            .numeric_acquisitions
            .iter()
            .filter(|(known, _)| known.host == owner.host)
            .map(|(_, record)| record.public_group)
            .max()
        {
            last.checked_add(1)
                .ok_or(crate::effects::ExecutionError::ResourceLimitExceeded {
                    resource: "numeric choice group sequence",
                    requested: u128::from(last) + 1,
                    maximum: u128::from(u64::MAX),
                })?
        } else {
            0
        };
        // Numeric memory is noncopiable snapshot evidence even when no layer
        // descriptor changes. Invalidate the object snapshot cache as well as
        // calculated characteristics before publishing the completed choice.
        self.bump_mutation_revision();
        self.mark_continuous_state_dirty();
        self.choice_store_mut().numeric_acquisitions.insert(
            owner,
            crate::source_numbers::NumberChoiceRecord {
                number,
                public_group,
            },
        );
        Ok(())
    }
    pub fn numeric_choice_memory(
        &self,
        source: ObjectId,
    ) -> crate::source_numbers::NumberChoiceMemory {
        self.choice_store
            .numeric_acquisitions
            .iter()
            .filter(|(owner, _)| owner.host == source)
            .map(|(owner, number)| (owner.clone(), *number))
            .collect()
    }
    pub fn number_for_acquisition(
        &self,
        owner: &crate::source_numbers::NumberChoiceOwner,
        retained: Option<&crate::snapshot::ObjectSnapshot>,
    ) -> Result<Option<u32>, crate::effects::ExecutionError> {
        if self.object(owner.host).is_some() {
            return Ok(self
                .choice_store
                .numeric_acquisitions
                .get(owner)
                .map(|record| record.number));
        }
        // A later completed choice before departure supersedes the older
        // admission snapshot. Neither path follows a new object incarnation.
        let snapshot = self
            .source_departure_snapshot(owner.host)
            .or_else(|| retained.filter(|snapshot| snapshot.object_id == owner.host));
        let choices=snapshot.and_then(|snapshot|snapshot.numeric_choice_memory.as_deref())
            .ok_or_else(||crate::effects::ExecutionError::IncompleteEvidence(
                "numeric acquisition history is unavailable; the public proof is not an executable acquisition receipt".into()))?;
        Ok(choices.get(owner).map(|record| record.number))
    }

    /// The number chosen for a permanent as it entered ("choose 2, 3, or 4
    /// at random"), stored as its named option.
    pub fn chosen_number(&self, permanent_id: ObjectId) -> Option<i32> {
        self.chosen_named_option(permanent_id)
            .and_then(|option| option.trim().parse::<i32>().ok())
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
        let inserted = {
            let entry = self
                .exile_tracking_mut()
                .exiled_with_source
                .entry(source_id)
                .or_default();
            if entry.contains(&exiled_card_id) {
                false
            } else {
                entry.push(exiled_card_id);
                true
            }
        };
        if inserted {
            let revision = self
                .exile_tracking_mut()
                .exiled_with_source_revisions
                .entry(source_id)
                .or_default();
            *revision = revision.saturating_add(1);
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

    /// End an "exile ... until this leaves the battlefield" duration. The
    /// return is queued for a caller with a player decision channel (see
    /// `process_pending_duration_end_returns`).
    pub fn return_exiled_for_source_leave(&mut self, source_id: ObjectId) {
        let (linked, return_zones) = {
            let tracking = self.exile_tracking_mut();
            if !tracking.return_exiled_when_source_leaves.remove(&source_id) {
                return;
            }
            let linked = tracking
                .exiled_with_source
                .get(&source_id)
                .cloned()
                .unwrap_or_default();
            let return_zones = tracking
                .exiled_with_source_return_zones
                .remove(&source_id)
                .unwrap_or_default();
            (linked, return_zones)
        };
        let returns = linked
            .into_iter()
            .filter_map(|object_id| {
                // The source may also have an unrelated exile ability. Only
                // an actual "exile until" receipt creates this duration;
                // source ownership alone must not invent a return to play.
                let return_zone = return_zones.get(&object_id).copied()?;
                self.object(object_id)
                    .is_some_and(|object| object.zone == Zone::Exile)
                    .then_some((object_id, return_zone))
            })
            .collect::<Vec<_>>();
        self.return_exiled_cards_at_duration_end(source_id, returns);
    }

    /// CR 610.3 / 610.3c: when an "exile ... until" duration ends, the cards
    /// return by a one-shot effect. A battlefield return is an ordinary,
    /// simultaneous (CR 610.3d) battlefield entry under its owner's control,
    /// so it goes through ETB replacement processing: intrinsic loyalty and
    /// defense counters, enters-tapped, as-enters choices, copy choices and
    /// CR 303.4f Aura attachment. Other return zones use the zone-change
    /// replacement pipeline.
    ///
    /// Duration ends aren't resolving effects. Entry choices are asked of the
    /// players through `decision_maker`. A caller without a player decision
    /// channel queues the return instead; the next trigger drain that has one
    /// (and at the latest the priority advance) performs it, before any
    /// player receives priority.
    fn return_exiled_cards_at_duration_end(
        &mut self,
        source_id: ObjectId,
        returns: Vec<(ObjectId, Zone)>,
    ) {
        if !returns.is_empty() {
            self.auxiliary_tracking_mut()
                .pending_duration_end_returns
                .push((source_id, returns));
        }
    }

    fn execute_duration_end_returns(
        &mut self,
        source_id: ObjectId,
        returns: Vec<(ObjectId, Zone)>,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<(), crate::effects::ExecutionError> {
        if returns.is_empty() {
            return Ok(());
        }
        let Some(controller) = returns
            .iter()
            .find_map(|(object_id, _)| self.object(*object_id).map(|object| object.owner))
        else {
            return Ok(());
        };
        let mut ctx = crate::effects::ExecutionContext::new(source_id, controller, decision_maker);
        let mut battlefield_requests = Vec::new();
        for (object_id, return_zone) in returns {
            if return_zone == Zone::Battlefield {
                battlefield_requests.push((
                    object_id,
                    crate::effects::zones::BattlefieldEntryOptions::owner(false),
                ));
            } else {
                let _ = crate::effects::zones::apply_zone_change(
                    self,
                    object_id,
                    Zone::Exile,
                    return_zone,
                    ctx.cause.clone(),
                    &mut *ctx.decision_maker,
                )?;
            }
        }
        let _ = crate::effects::zones::move_to_battlefield_batch_with_options(
            self,
            &mut ctx,
            battlefield_requests,
        )?;
        Ok(())
    }

    pub(crate) fn has_pending_duration_end_returns(&self) -> bool {
        !self
            .auxiliary_tracking
            .pending_duration_end_returns
            .is_empty()
    }

    /// Perform queued exile-until returns (in the order their durations
    /// ended), asking entry choices through `decision_maker`. Cards that left
    /// exile meanwhile are new objects and stay where they are (CR 400.7).
    pub(crate) fn process_pending_duration_end_returns(
        &mut self,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<(), crate::effects::ExecutionError> {
        let checkpoint = self.clone();
        let result = self.process_pending_duration_end_returns_inner(decision_maker);
        if result.is_err() || decision_maker.awaiting_choice() {
            *self = checkpoint;
        }
        result
    }

    fn process_pending_duration_end_returns_inner(
        &mut self,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Result<(), crate::effects::ExecutionError> {
        let pending =
            std::mem::take(&mut self.auxiliary_tracking_mut().pending_duration_end_returns);
        let mut pending = pending.into_iter();
        while let Some((source_id, returns)) = pending.next() {
            let returns = returns
                .into_iter()
                .filter(|(object_id, _)| {
                    self.object(*object_id)
                        .is_some_and(|object| object.zone == Zone::Exile)
                })
                .collect::<Vec<_>>();
            self.execute_duration_end_returns(source_id, returns, &mut *decision_maker)?;
            if decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Track a one-shot exile duration that ends the next time one of the
    /// effect controller's opponents becomes the monarch.
    pub fn track_exiled_until_opponent_becomes_monarch(
        &mut self,
        controller: PlayerId,
        stable_ids: Vec<StableId>,
        return_zone: Zone,
    ) {
        if stable_ids.is_empty() {
            return;
        }
        let group_id = self.create_linked_exile_group(stable_ids, return_zone, true);
        self.exile_tracking_mut()
            .return_exiled_when_opponent_becomes_monarch
            .insert(group_id, controller);
    }

    /// End all qualifying monarch-event exile durations. This is a one-shot
    /// duration ending, not a triggered ability, so the cards return directly
    /// as the designation changes.
    pub fn return_exiled_for_opponent_becoming_monarch(&mut self, monarch: PlayerId) {
        let qualifying_groups = self
            .exile_tracking
            .return_exiled_when_opponent_becomes_monarch
            .iter()
            .filter_map(|(&group_id, &controller)| {
                self.are_opponents(controller, monarch).then_some(group_id)
            })
            .collect::<Vec<_>>();
        // Sorted: returns allocate new object ids, so order must match on every peer.
        let mut qualifying_groups = qualifying_groups;
        qualifying_groups.sort_unstable();

        for group_id in qualifying_groups {
            self.exile_tracking_mut()
                .return_exiled_when_opponent_becomes_monarch
                .remove(&group_id);
            let Some(group) = self.take_linked_exile_group(group_id) else {
                continue;
            };
            let returns = group
                .stable_ids
                .into_iter()
                .filter_map(|stable_id| self.find_object_by_stable_id(stable_id))
                .filter(|object_id| {
                    self.object(*object_id)
                        .is_some_and(|object| object.zone == Zone::Exile)
                })
                .map(|object_id| (object_id, group.return_zone))
                .collect::<Vec<_>>();
            // The effect that exiled them is long gone; any stable reference works.
            let Some(source_id) = returns.first().map(|(object_id, _)| *object_id) else {
                continue;
            };
            self.return_exiled_cards_at_duration_end(source_id, returns);
        }
    }

    /// Get cards exiled by a specific source object ID.
    /// Record the exact resulting incarnation for this proven ability pair.
    pub fn add_linked_exile_pair_member(
        &mut self,
        owner: crate::linked_exile::LinkedExileOwner,
        member: ObjectId,
    ) {
        let members = self
            .exile_tracking_mut()
            .linked_exile_pairs
            .entry(owner)
            .or_default();
        if !members.contains(&member) {
            members.push(member);
        }
        self.remember_linked_exile_inspection_entitlements();
    }

    pub fn linked_exile_pair_members(
        &self,
        owner: &crate::linked_exile::LinkedExileOwner,
    ) -> Result<&[ObjectId], crate::effects::ExecutionError> {
        if self.exile_tracking.linked_exile_pairs_incomplete {
            return Err(crate::effects::ExecutionError::IncompleteEvidence(
                "linked exile ownership was omitted by a source-only state import; full replay required".into()));
        }
        Ok(self
            .exile_tracking
            .linked_exile_pairs
            .get(owner)
            .map(Vec::as_slice)
            .unwrap_or(&[]))
    }

    pub fn get_exiled_with_source_links(&self, source_id: ObjectId) -> &[ObjectId] {
        self.exile_tracking
            .exiled_with_source
            .get(&source_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Monotonic successful-exile revision for one source object identity.
    pub fn exiled_with_source_revision(&self, source_id: ObjectId) -> u64 {
        self.exile_tracking
            .exiled_with_source_revisions
            .get(&source_id)
            .copied()
            .unwrap_or(0)
    }

    /// Record an object identity put onto the battlefield by this source.
    pub fn add_battlefield_put_with_source_link(
        &mut self,
        source_id: ObjectId,
        object_id: ObjectId,
    ) {
        self.exile_tracking_mut()
            .battlefield_put_with_source
            .entry(source_id)
            .or_default()
            .insert(object_id);
    }

    /// Whether this exact battlefield object identity was put there by the
    /// specified source. A later zone change creates a new object ID and
    /// therefore breaks this relationship.
    pub fn was_put_onto_battlefield_with_source(
        &self,
        source_id: ObjectId,
        object_id: ObjectId,
    ) -> bool {
        self.exile_tracking
            .battlefield_put_with_source
            .get(&source_id)
            .is_some_and(|objects| objects.contains(&object_id))
    }

    /// Record that a token instance was created by an effect of a particular
    /// source instance. Both sides use stable identity because the source may
    /// have changed zones before a linked leaves-trigger resolves.
    pub fn add_token_created_with_source_link(
        &mut self,
        source_stable_id: StableId,
        token_stable_id: StableId,
    ) {
        self.object_annotations_mut()
            .token_creation_sources
            .insert(token_stable_id, source_stable_id);
    }

    /// Whether this token instance was created by the specified source
    /// instance.
    pub fn was_token_created_with_source(
        &self,
        source_stable_id: StableId,
        token_stable_id: StableId,
    ) -> bool {
        self.object_annotations
            .token_creation_sources
            .get(&token_stable_id)
            .is_some_and(|created_by| *created_by == source_stable_id)
    }

    pub fn exiled_with_source_entries(&self) -> impl Iterator<Item = (&ObjectId, &Vec<ObjectId>)> {
        self.exile_tracking.exiled_with_source.iter()
    }

    pub fn return_exiled_when_source_leaves_ids(&self) -> impl Iterator<Item = &ObjectId> {
        self.exile_tracking.return_exiled_when_source_leaves.iter()
    }

    pub fn replace_exiled_with_source_links(&mut self, links: HashMap<ObjectId, Vec<ObjectId>>) {
        let tracking = self.exile_tracking_mut();
        tracking.exiled_with_source = links;
        tracking.linked_exile_pairs.clear();
        tracking.linked_exile_pairs_incomplete = true;
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
        tracking.linked_exile_pairs.retain(|_, members| {
            members.retain(|member| *member != exiled_card_id);
            !members.is_empty()
        });
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

    /// Get CR 730 merged-permanent metadata by the permanent's stable identity.
    pub fn merged_permanent(&self, stable_id: StableId) -> Option<&MergedPermanentState> {
        self.commander_tracking.merged_permanents.get(&stable_id)
    }

    /// Remove and return CR 730 merged-permanent metadata.
    pub fn take_merged_permanent(&mut self, stable_id: StableId) -> Option<MergedPermanentState> {
        self.commander_tracking_mut()
            .merged_permanents
            .remove(&stable_id)
    }

    /// Partition a card-only zone replacement across a token merged
    /// permanent (CR 730.3e): card components use the replacement destination,
    /// while token components use the original destination.
    pub(crate) fn prepare_merged_token_card_component_destinations(
        &mut self,
        permanent_id: ObjectId,
        original_destination: Zone,
        replacement_destination: Zone,
    ) {
        let Some(stable_id) = self.object(permanent_id).and_then(|object| {
            (object.kind == crate::object::ObjectKind::Token).then_some(object.stable_id)
        }) else {
            return;
        };
        let Some(merged) = self.merged_permanent(stable_id) else {
            return;
        };
        let destinations = merged
            .components
            .iter()
            .map(|component| {
                if component.object.kind == crate::object::ObjectKind::Card {
                    replacement_destination
                } else {
                    original_destination
                }
            })
            .collect();
        self.commander_tracking_mut()
            .pending_merged_component_destinations
            .insert(stable_id, destinations);
    }

    /// Rebuild the visible characteristics of a merged permanent from its
    /// physical top component plus every component's abilities (CR 730.2a).
    fn refresh_merged_permanent_characteristics(&mut self, permanent_id: ObjectId) -> bool {
        let Some(stable_id) = self.object(permanent_id).map(|object| object.stable_id) else {
            return false;
        };
        let Some(state) = self
            .commander_tracking
            .merged_permanents
            .get(&stable_id)
            .cloned()
        else {
            return false;
        };
        let Some(top) = state
            .components
            .first()
            .map(|component| component.object.clone())
        else {
            return false;
        };

        let mut all_abilities = Vec::new();
        let mut all_alternative_casts = Vec::new();
        let mut all_optional_costs = Vec::new();
        let Some(mut all_temporary_grants) = self.object(permanent_id).map(|object| {
            object
                .temporary_static_ability_grants
                .empty_with_allocator()
        }) else {
            return false;
        };
        let mut merged_text = Vec::new();
        let mut merged_labels = Vec::new();
        let mut labels_aligned = true;
        for component in &state.components {
            all_abilities.extend(component.object.abilities.iter().cloned());
            labels_aligned &=
                component.object.ability_labels.len() == component.object.abilities.len();
            merged_labels.extend(component.object.ability_labels.iter().cloned());
            all_alternative_casts.extend(component.object.alternative_casts.iter().cloned());
            all_optional_costs.extend(component.object.optional_costs.iter().cloned());
            all_temporary_grants.extend_existing(&component.object.temporary_static_ability_grants);
            if !component.object.compiled_card_text.trim().is_empty() {
                merged_text.push(component.object.compiled_card_text.to_string());
            }
        }

        let Some(permanent) = self.object_mut(permanent_id) else {
            return false;
        };
        permanent.kind = top.kind;
        permanent.card = top.card;
        permanent.name = top.name;
        permanent.first_printed_set_name = top.first_printed_set_name;
        permanent.mana_cost = top.mana_cost;
        permanent.color_override = top.color_override;
        permanent.supertypes = top.supertypes;
        permanent.card_types = top.card_types;
        permanent.subtypes = top.subtypes;
        permanent.compiled_card_text = std::sync::Arc::from(merged_text.join("\n"));
        permanent.ability_labels = if labels_aligned {
            merged_labels.into()
        } else {
            Default::default()
        };
        permanent.rules_text_color_identity = top.rules_text_color_identity;
        permanent.other_face = top.other_face;
        permanent.other_face_name = top.other_face_name;
        permanent.linked_face_layout = top.linked_face_layout;
        permanent.base_power = top.base_power;
        permanent.base_toughness = top.base_toughness;
        permanent.base_loyalty = top.base_loyalty;
        permanent.base_defense = top.base_defense;
        permanent.abilities = std::sync::Arc::new(all_abilities);
        permanent.aura_attach_filter = top.aura_attach_filter;
        permanent.bestow_cast_state = top.bestow_cast_state;
        permanent.face_down_cast_state = top.face_down_cast_state;
        permanent.prototype_cast_state = top.prototype_cast_state;
        permanent.alternative_casts = all_alternative_casts.into();
        permanent.optional_costs = all_optional_costs.into();
        permanent.temporary_static_ability_grants = all_temporary_grants;
        true
    }

    /// Capture the front/default characteristics a physical card component
    /// will have after the merged permanent leaves the battlefield (CR 730.3).
    /// Live flips and transforms subsequently update `component.object`, but
    /// never this immutable destination snapshot.
    fn merged_component_destination_object(
        &self,
        object: &Object,
        current_is_secondary_face: bool,
    ) -> Object {
        let mut destination = object.clone();
        // Each component becomes a new object with its printed characteristics
        // (CR 729.3, 400.7): a Clone's copy effect ends (CR 707.2), a
        // face-down card is turned face up (CR 708.9), and prototype/bestow
        // cast characteristics end (CR 718.4, 702.103). Same order as an
        // ordinary battlefield leave.
        destination.end_enters_as_copy_overlay();
        destination.end_prototype_cast_overlay();
        destination.end_bestow_cast_overlay();
        destination.end_face_down_cast_overlay();
        if destination.kind != crate::object::ObjectKind::Card {
            return destination;
        }
        // CR 712.8a: outside the battlefield a transforming DFC has only its
        // front face. Decide by the printed faces rather than transform-count
        // parity, which misses permanents that entered transformed.
        let front_definition = self
            .default_face_definition_for_transform_like_return(&destination)
            .or_else(|| {
                (current_is_secondary_face && destination.name == object.name)
                    .then(|| {
                        self.linked_face_definition_by_name_or_id(
                            destination.other_face_name.as_deref(),
                            destination.other_face,
                        )
                    })
                    .flatten()
            });
        if let Some(front_definition) = front_definition {
            destination.apply_definition_face(&front_definition);
        }
        destination
    }

    /// Merge a mutating creature spell with its legal battlefield target.
    ///
    /// The target remains the same permanent: it does not leave or enter the
    /// battlefield, keeps its counters/controller/tapped state/timestamp, and
    /// gains the abilities of every physical component.  `spell_on_top`
    /// selects the component that supplies the other copiable characteristics.
    pub fn merge_mutating_creature_spell(
        &mut self,
        spell_id: ObjectId,
        target_id: ObjectId,
        spell_on_top: bool,
    ) -> Option<ObjectId> {
        let spell = self.object(spell_id)?.clone();
        let target = self.object(target_id)?.clone();
        if spell.zone != Zone::Stack
            || target.zone != Zone::Battlefield
            || spell.owner != target.owner
            || !self.current_is_creature(target_id)
            || self
                .calculated_subtypes(target_id)
                .contains(&Subtype::Human)
        {
            return None;
        }

        let target_stable_id = target.stable_id;
        let target_face_down = self.is_face_down(target_id);
        let target_flipped = self.is_flipped(target_id);
        let target_destination_object = self.merged_component_destination_object(
            &target,
            target_flipped
                || (target.linked_face_layout == LinkedFaceLayout::TransformLike
                    && self.transform_count(target_id) % 2 == 1),
        );
        let mut state = self
            .commander_tracking
            .merged_permanents
            .get(&target_stable_id)
            .cloned()
            .unwrap_or_else(|| MergedPermanentState {
                components: vec![MergedPermanentComponentState {
                    is_commander: self.is_commander(target_id),
                    object: target.clone(),
                    destination_object: target_destination_object,
                    face_down: target_face_down,
                    flipped: target_flipped,
                }],
            });
        let spell_flipped = self.is_flipped(spell_id);
        let spell_component = MergedPermanentComponentState {
            is_commander: self.is_commander(spell_id),
            object: spell.clone(),
            destination_object: self.merged_component_destination_object(
                &spell,
                spell_flipped
                    || (spell.linked_face_layout == LinkedFaceLayout::TransformLike
                        && self.transform_count(spell_id) % 2 == 1),
            ),
            face_down: self.is_face_down(spell_id),
            flipped: spell_flipped,
        };
        if spell_on_top {
            state.components.insert(0, spell_component);
        } else {
            state.components.push(spell_component);
        }

        let top_face_down = state.components.first()?.face_down;
        self.commander_tracking_mut()
            .merged_permanents
            .insert(target_stable_id, state);
        self.refresh_merged_permanent_characteristics(target_id);

        // Merging adopts the top component's status without turning the
        // permanent face up or down (CR 730.2e), so no face-change event is
        // emitted. The merge itself gives the copiable effect a new timestamp.
        let face_status_changed = {
            let flags = self.battlefield_flags_mut();
            if top_face_down {
                flags.face_down.insert(target_id)
            } else {
                let changed = flags.face_down.remove(&target_id);
                flags.manifested.remove(&target_id);
                flags.cloaked.remove(&target_id);
                changed
            }
        };
        if face_status_changed {
            self.mark_object_characteristics_dirty(target_id);
        }
        self.effect_store
            .continuous_effects
            .record_face_change(target_id);
        self.effect_store
            .continuous_effects
            .retarget_merged_spell(spell_id, target_id);
        self.remove_object(spell_id);
        self.mark_continuous_state_dirty();
        Some(target_id)
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
                zone_change
                    .destination_snapshots
                    .first()
                    .cloned()
                    .or_else(|| {
                        zone_change.objects.first().copied().and_then(|id| {
                            self.object(id)
                                .map(|obj| crate::snapshot::ObjectSnapshot::from_object(obj, self))
                        })
                    })
            })
            .or_else(|| event.snapshot().cloned())
            .or_else(|| {
                // Attack-history characteristics are fixed after attackers
                // are declared, including static bonuses for attacking.
                // Raw object P/T omits those layer-7 modifications.
                event
                    .downcast::<crate::events::combat::CreatureAttackedEvent>()
                    .and_then(|attack| self.object(attack.attacker))
                    .and_then(|object| {
                        crate::snapshot::ObjectSnapshot::capture_for_execution(object, self)
                    })
            })
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

    /// Execute state construction through ordinary mutation/replacement owners
    /// while suppressing action history and trigger publication. Local receipt
    /// journals still serve child programs; enclosing journals/queues are kept
    /// outside the scope. The caller owns world rollback and pending choices.
    pub fn with_suppressed_action_observations<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self.effect_store.action_observations_suppressed;
        let events = std::mem::take(&mut self.effect_store.pending_trigger_events);
        let entries = std::mem::take(&mut self.effect_store.pending_trigger_entries);
        let reflexive = std::mem::take(&mut self.effect_store.pending_reflexive_triggers);
        let observers = std::mem::take(&mut self.effect_store.action_observation_records);
        let original_observers =
            std::mem::take(&mut self.effect_store.retained_original_observation_scopes);
        let results = std::mem::take(&mut self.effect_store.instruction_result_records);
        let matched = std::mem::take(&mut self.effect_store.matched_outcome_events);
        self.effect_store.action_observations_suppressed = true;
        let result = body(self);
        self.effect_store.action_observations_suppressed = previous;
        self.effect_store.pending_trigger_events = events;
        self.effect_store.pending_trigger_entries = entries;
        self.effect_store.pending_reflexive_triggers = reflexive;
        self.effect_store.action_observation_records = observers;
        self.effect_store.retained_original_observation_scopes = original_observers;
        self.effect_store.instruction_result_records = results;
        self.effect_store.matched_outcome_events = matched;
        result
    }

    pub(crate) fn action_observations_suppressed(&self) -> bool {
        self.effect_store.action_observations_suppressed
    }

    pub(crate) fn retain_action_observation(&mut self, event: &crate::triggers::TriggerEvent) {
        if self.effect_store.action_observation_records.is_empty() {
            return;
        }
        for observations in &mut self.effect_store.action_observation_records {
            if let Some(previous) = observations
                .iter_mut()
                .find(|previous| previous.occurrence_key() == event.occurrence_key())
            {
                *previous = if previous.completed_action_provenance().is_some() {
                    event.with_completed_action_receipt(previous)
                } else {
                    event.clone()
                };
            } else {
                observations.push(event.clone());
            }
        }
    }

    pub(crate) fn stage_turn_history_event(&mut self, incoming: &crate::triggers::TriggerEvent) {
        let incoming = self.event_with_retained_trigger_capture(incoming);
        let incoming = &incoming;
        if incoming.ordinary_triggers_captured() || incoming.delayed_triggers_captured() {
            self.retain_trigger_capture_receipt(incoming);
        }
        if self.action_observations_suppressed() {
            self.retain_action_observation(incoming);
            return;
        }
        // An older alias cannot replace an archived completion's frozen payload.
        let event = self
            .turn_store
            .action_history
            .iter()
            .find(|record| {
                record.event.ptr_eq(incoming)
                    && record.event.completed_action_provenance().is_some()
            })
            .map(|record| incoming.with_completed_action_receipt(&record.event))
            .unwrap_or_else(|| incoming.clone());
        self.retain_action_observation(&event);
        if event.completed_action_provenance().is_some() {
            let (object_snapshot, source_snapshot) = self.projected_turn_event_snapshots(&event);
            let record = self
                .turn_store
                .turn_history
                .refresh_completed_action_record(
                    &event,
                    object_snapshot.clone(),
                    source_snapshot.clone(),
                )
                .or_else(|| {
                    self.turn_store
                        .action_history
                        .refresh_completed_action_record(&event, object_snapshot, source_snapshot)
                });
            if let Some(record) = record {
                // Both views share one immutable receipt; replacing it retains
                // chronology and never appends a past action into the new turn.
                self.turn_store.action_history.replace_shared(record);
                self.invalidate_continuous_history_modifiers();
                return;
            }
        }
        // Captured or archived occurrences have already been counted. This
        // applies even after turn rollover clears the local alias map/history.
        if event.triggers_captured()
            || self
                .effect_store
                .matched_outcome_events
                .contains_key(&event.occurrence_key())
            || self
                .turn_store
                .action_history
                .iter()
                .any(|record| record.event.ptr_eq(&event))
        {
            return;
        }
        let (object_snapshot, source_snapshot) = self.projected_turn_event_snapshots(&event);
        self.turn_store
            .turn_history
            .stage_event(&event, object_snapshot, source_snapshot);
        self.invalidate_continuous_history_modifiers();
    }

    pub(crate) fn record_turn_history_event(&mut self, event: &crate::triggers::TriggerEvent) {
        if self.action_observations_suppressed()
            || event.triggers_captured()
            || self
                .turn_store
                .turn_history
                .event_records
                .iter()
                .any(|record| record.event.ptr_eq(event))
            || self
                .turn_store
                .action_history
                .iter()
                .any(|record| record.event.ptr_eq(event))
        {
            return;
        }
        if let Some(mutated) = event.downcast::<crate::events::other::MutatedEvent>() {
            self.mark_mutated(mutated.permanent);
        }
        if let Some(spell_cast) = event.downcast::<crate::events::spells::SpellCastEvent>()
            && let Some(player) = self.player_mut(spell_cast.caster)
        {
            player.spells_cast_this_game = player.spells_cast_this_game.saturating_add(1);
        }
        if let Some(attacked) = event.downcast::<crate::events::combat::CreatureAttackedEvent>()
            && let Some(stable_id) = self
                .object(attacked.attacker)
                .map(|object| object.stable_id)
        {
            self.turn_store
                .creature_last_attacked_turn
                .insert(stable_id, self.turn.turn_number);
        }
        if let Some(blocked) = event.downcast::<crate::events::combat::CreatureBlockedEvent>()
            && let Some(stable_id) = self.object(blocked.blocker).map(|object| object.stable_id)
        {
            self.turn_store
                .creature_last_blocked_turn
                .insert(stable_id, self.turn.turn_number);
        }
        let became_blocked = event
            .downcast::<crate::events::combat::CreatureBecameBlockedEvent>()
            .map(|blocked| blocked.attacker)
            .or_else(|| {
                event
                    .downcast::<crate::events::combat::CreatureBlockedEvent>()
                    .map(|blocked| blocked.attacker)
            });
        if let Some(attacker) = became_blocked
            && let Some(stable_id) = self.object(attacker).map(|object| object.stable_id)
        {
            self.turn_store
                .creature_last_became_blocked_turn
                .insert(stable_id, self.turn.turn_number);
        }
        let (object_snapshot, source_snapshot) = self.projected_turn_event_snapshots(event);
        self.turn_store
            .turn_history
            .record_event(event, object_snapshot, source_snapshot);
        self.invalidate_continuous_history_modifiers();
        let Some(record) = self.turn_store.turn_history.event_records.last_shared() else {
            return;
        };
        self.turn_store.action_history.push_shared(record);
    }

    /// Read only proof for this exact occurrence from the current branch's
    /// immutable receipts. An equal-looking later event is a distinct action.
    pub(crate) fn event_with_retained_trigger_capture(
        &self,
        event: &crate::triggers::TriggerEvent,
    ) -> crate::triggers::TriggerEvent {
        let mut event = event.clone();
        if event.triggers_captured() {
            return event;
        }
        if let Some(receipt) = self
            .effect_store
            .matched_outcome_events
            .get(&event.occurrence_key())
        {
            event = event.with_retained_trigger_capture(receipt);
            if event.triggers_captured() {
                return event;
            }
        }
        for receipt in self.retained_action_observations() {
            event = event.with_retained_trigger_capture(receipt);
            if event.triggers_captured() {
                break;
            }
        }
        event
    }

    pub(crate) fn event_is_pending_for_trigger_matching(
        &self,
        event: &crate::triggers::TriggerEvent,
    ) -> bool {
        self.effect_store
            .pending_trigger_events
            .iter()
            .any(|pending| pending.ptr_eq(event))
    }

    /// Only an actual matching owner publishes observer proof. Retain metadata
    /// on the physical receipt without changing history counts or chronology.
    pub(crate) fn retain_trigger_capture_receipt(
        &mut self,
        incoming: &crate::triggers::TriggerEvent,
    ) {
        if self.action_observations_suppressed() {
            return;
        }
        let event = self.event_with_retained_trigger_capture(incoming);
        let turn_retained = self
            .turn_store
            .turn_history
            .retain_trigger_capture_receipt(&event);
        let archived = self
            .turn_store
            .action_history
            .retain_trigger_capture_receipt(&event)
            .is_some();
        if !turn_retained && !archived {
            let (object_snapshot, source_snapshot) = self.projected_turn_event_snapshots(&event);
            self.turn_store
                .turn_history
                .stage_event(&event, object_snapshot, source_snapshot);
        }
        for observations in &mut self.effect_store.action_observation_records {
            for previous in observations
                .iter_mut()
                .filter(|previous| previous.ptr_eq(&event))
            {
                previous.inherit_trigger_capture(&event);
            }
        }
        if self.effect_store.per_event_trigger_matching {
            self.effect_store
                .matched_outcome_events
                .insert(event.occurrence_key(), event);
        }
    }

    /// Shared history write for native activation completion and notification
    /// publication. Callers retain their existing earlier/later boundaries.
    pub(crate) fn record_land_mana_activation(
        &mut self,
        source: ObjectId,
        activator: PlayerId,
        snapshot: Option<&ObjectSnapshot>,
    ) {
        let is_land = self
            .object(source)
            .map(|object| object.is_land())
            .or_else(|| snapshot.map(|snapshot| snapshot.is_land()))
            .unwrap_or(false);
        if is_land {
            self.turn_store
                .turn_history
                .players_tapped_land_for_mana_this_turn
                .insert(activator);
        }
    }

    /// Allocate one activation notification occurrence from an already bound
    /// payload. Its caller owns queueing and immediate-mana trigger timing.
    pub(crate) fn complete_activation_notification(
        &mut self,
        activation: crate::events::AbilityActivatedEvent,
    ) -> crate::effects::CompletedEffectOutputs {
        if activation.is_mana_ability {
            self.record_land_mana_activation(
                activation.source,
                activation.activator,
                activation.snapshot.as_ref(),
            );
        }
        let provenance = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::AbilityActivated);
        let event = crate::triggers::TriggerEvent::new_with_provenance(activation, provenance);
        crate::effects::CompletedEffectOutputs::aggregate_only(
            crate::effect::EffectOutcome::resolved().with_event(event),
        )
    }

    /// Exact departure evidence survives turn cleanup through the existing
    /// physical event archive. Never follow another stable-card incarnation.
    pub(crate) fn source_departure_snapshot(&self, source: ObjectId) -> Option<&ObjectSnapshot> {
        self.turn_store
            .turn_history
            .source_departure_snapshot(source)
            .or_else(|| {
                self.turn_store
                    .action_history
                    .iter()
                    .rev()
                    .find_map(|record| record.event.source_departure_snapshot(source))
            })
    }

    /// As above, with physical phasing receipts admitted explicitly.
    pub(crate) fn source_last_known_snapshot(&self, source: ObjectId) -> Option<&ObjectSnapshot> {
        self.turn_store
            .turn_history
            .source_last_known_snapshot(source)
            .or_else(|| {
                self.turn_store
                    .action_history
                    .iter()
                    .rev()
                    .find_map(|record| record.event.source_last_known_snapshot(source))
            })
    }

    /// Historical characteristics for an execution source. None means that
    /// current characteristics are authoritative (or no exact evidence exists).
    /// This does not resolve an object for a physical action.
    pub(crate) fn source_characteristic_lki_snapshot<'a>(
        &'a self,
        source: ObjectId,
        retained: Option<&'a ObjectSnapshot>,
    ) -> Option<&'a ObjectSnapshot> {
        if self.is_phased_out(source) {
            return self
                .source_last_known_snapshot(source)
                .or_else(|| retained.filter(|snapshot| snapshot.object_id == source));
        }
        if let Some(current) = self.object(source) {
            // Preserve a caller's explicit old-incarnation binding, while
            // preferring its actual departure over an earlier announcement.
            let previous = retained.filter(|snapshot| {
                snapshot.stable_id == current.stable_id
                    && (snapshot.object_id != current.id || snapshot.zone != current.zone)
            })?;
            return self
                .source_departure_snapshot(previous.object_id)
                .or(Some(previous));
        }
        self.source_departure_snapshot(source)
            .or_else(|| retained.filter(|snapshot| snapshot.object_id == source))
    }

    /// Current projections and retained full-game observations are the receipt
    /// authority, including aliases created before a later turn boundary.
    pub(crate) fn retained_action_observations(
        &self,
    ) -> impl Iterator<Item = &crate::triggers::TriggerEvent> {
        self.turn_store
            .turn_history
            .projected_records()
            .chain(self.turn_store.action_history.iter())
            .map(|record| &record.event)
    }

    /// Freeze public milling characteristics at the outer instruction boundary,
    /// after all players' prepared moves commit. Later instructions never update
    /// the already completed batch, and exact IDs cannot follow a new incarnation.
    pub(crate) fn finalize_milling_event_snapshots(&mut self) {
        let Some(batch) = self
            .auxiliary_tracking
            .simultaneous_action_scope
            .and_then(|scope| scope.batch)
        else {
            return;
        };
        let updates: Vec<_> = self
            .effect_store
            .pending_trigger_events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.simultaneous_batch() == Some(batch))
            .filter_map(|(index, event)| {
                let mut milled = event.downcast::<crate::events::CardMilledEvent>()?.clone();
                // Absence denotes a hidden destination; never reveal it here.
                let previous = milled.snapshot.as_ref()?;
                if let Some(object) = self
                    .object(milled.card)
                    .filter(|object| object.zone == previous.zone)
                    && !self.is_face_down(object.id)
                {
                    milled.snapshot = ObjectSnapshot::capture_for_execution(object, self);
                }
                Some((index, event.with_inner_event(milled)))
            })
            .collect();
        for (index, event) in updates {
            self.effect_store.pending_trigger_events[index] = event;
        }
    }

    /// Freeze actual entry characteristics at the completed original-operation
    /// boundary. Both reported and queued events use this owner. Simultaneous
    /// callers supply the entire batch after timestamps, before added programs.
    pub fn freeze_completed_entry_events<'a>(
        &mut self,
        events: impl IntoIterator<Item = &'a mut crate::triggers::TriggerEvent>,
    ) -> Result<(), crate::effects::ExecutionError> {
        if self.effect_store.trigger_matching_holds > 0
            || self.auxiliary_tracking.simultaneous_action_scope.is_some()
        {
            return Ok(());
        }
        let mut reported = events.into_iter().collect::<Vec<_>>();
        let mut pending = std::mem::take(&mut self.effect_store.pending_trigger_events);
        let result = (|| {
            let mut all = reported
                .iter_mut()
                .map(|event| &mut **event)
                .chain(pending.iter_mut())
                .collect::<Vec<_>>();
            let retained = self
                .retained_action_observations()
                .filter(|receipt| receipt.completed_action_provenance().is_some())
                .map(|receipt| receipt.occurrence_key())
                .collect::<std::collections::HashSet<_>>();
            let entries = all
                .iter()
                .enumerate()
                .filter_map(|(index, event)| {
                    let entry = event.downcast::<crate::events::EnterBattlefieldEvent>()?;
                    // Unobserved departures still need their own retained original
                    // boundary; do not follow a returned card or invent its arrival.
                    (entry.completed_snapshot.is_some()
                        || event.completed_action_provenance().is_some()
                        || retained.contains(&event.occurrence_key())
                        || self
                            .object(entry.object)
                            .is_some_and(|object| object.zone == Zone::Battlefield))
                    .then_some(index)
                })
                .collect::<Vec<_>>();
            if entries.is_empty() {
                return Ok(());
            }
            let completions = entries
                .iter()
                .map(|&index| {
                    let event = &*all[index];
                    let parent = self
                        .provenance_graph()
                        .node(event.provenance())
                        .map(|_| event.provenance());
                    (event.clone(), parent)
                })
                .collect();
            let completed =
                crate::effects::observe_action_completions_retaining_groups(self, completions)?;
            let destinations = completed
                .iter()
                .filter_map(|event| event.snapshot().cloned())
                .collect::<Vec<_>>();
            let emerge_receipts = completed
                .iter()
                .filter_map(|event| {
                    let entry = event.downcast::<crate::events::EnterBattlefieldEvent>()?;
                    entry
                        .emerge_sacrifice
                        .clone()
                        .map(|receipt| (entry.object, receipt))
                })
                .collect::<std::collections::HashMap<_, _>>();
            for (index, event) in entries.into_iter().zip(completed) {
                *all[index] = event;
            }
            // Preserve origin LKI and attach the same exact destination receipt
            // to zone notifications, including reported and queued token events.
            for event in &mut all {
                let Some(zone) = event.downcast::<crate::events::ZoneChangeEvent>() else {
                    continue;
                };
                if zone.to != Zone::Battlefield {
                    continue;
                }
                let mut zone = zone.clone();
                for snapshot in &destinations {
                    if zone.destination_objects().contains(&snapshot.object_id)
                        && zone.destination_snapshot(snapshot.object_id).is_none()
                    {
                        zone.destination_snapshots.push(snapshot.clone());
                    }
                }
                if zone.from == Zone::Stack {
                    for destination in zone.destination_objects().to_vec() {
                        if let Some(receipt) = emerge_receipts.get(&destination) {
                            zone.destination_emerge_sacrifices
                                .insert(destination, receipt.clone());
                        }
                    }
                }
                **event = event.with_inner_event(zone);
                self.stage_turn_history_event(event);
            }
            Ok(())
        })();
        self.effect_store.pending_trigger_events = pending;
        result
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

        if let Some(draw) = event.downcast::<crate::events::CardsDrawnEvent>()
            && draw.snapshots.is_empty()
        {
            let snapshots = draw
                .cards
                .iter()
                .filter_map(|id| crate::snapshot::ObjectSnapshot::from_object_id(self, *id))
                .collect();
            event = event.with_inner_event(draw.clone().with_snapshots(snapshots));
        }
        if let Some(targeted) = event.downcast::<crate::events::BecomesTargetedEvent>() {
            event = event.with_inner_event(targeted.clone().with_participant_snapshots(self));
        }
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
        // Queueing is a scheduling observation. A completed action already has
        // its own immutable causal row; do not replace it with the queue node.
        if event.completed_action_provenance().is_none() {
            event.set_provenance(queued);
        }
        // CR 603.2c: zone changes performed together (one instruction's
        // objects, one state-based-action check) are one simultaneous event.
        if event.simultaneous_batch().is_none()
            && event.kind() == crate::events::EventKind::ZoneChange
            && let Some(batch) = self.simultaneous_action_batch()
        {
            event = event.with_simultaneous_batch(batch);
        }
        self.turn_store
            .turn_history
            .remove_staged_event(initial_provenance);
        self.stage_turn_history_event(&event);
        if !self.effect_store.instruction_result_records.is_empty() {
            let facts = crate::effects::outcome_recording::event_facts(self, &event);
            if let Some(record) = self.effect_store.instruction_result_records.last_mut() {
                record.extend(facts);
            }
        }
        self.effect_store.pending_trigger_events.push(event);
    }

    /// Drop the not-yet-matched "becomes the target" events a stack object
    /// reported for targets it no longer has.
    ///
    /// A copy reports its original targets as it is created, but when new
    /// targets are chosen for it as part of creating it, only the targets it
    /// ends up with ever became its targets (CR 707.10c, 115.7).
    pub(crate) fn drop_pending_stale_becomes_targeted_events(
        &mut self,
        source: ObjectId,
        by_ability: bool,
        stack_ability: Option<ObjectId>,
        final_targets: &[crate::game_state::Target],
    ) {
        use crate::events::spells::BecomesTargetedEvent;

        let mut dropped = Vec::new();
        self.effect_store.pending_trigger_events.retain(|event| {
            let stale = event
                .downcast::<BecomesTargetedEvent>()
                .is_some_and(|targeted| {
                    targeted.source == source
                        && targeted.by_ability == by_ability
                        && targeted.stack_ability == stack_ability
                        && !final_targets.contains(&targeted.target)
                });
            if stale {
                dropped.push(event.provenance());
            }
            !stale
        });
        for provenance in dropped {
            self.turn_store.turn_history.remove_staged_event(provenance);
        }
    }

    pub(crate) fn tag_pending_zone_change_event_for_object(
        &mut self,
        event_object: ObjectId,
        tag: crate::tag::TagKey,
        snapshot: crate::snapshot::ObjectSnapshot,
    ) {
        use crate::events::zones::ZoneChangeEvent;

        let Some((index, mut zone_change)) = self
            .effect_store
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
                matches_object.then(|| (index, zone_change.clone()))
            })
        else {
            return;
        };

        zone_change = zone_change.with_object_tag(tag, snapshot);
        let replacement =
            self.effect_store.pending_trigger_events[index].with_inner_event(zone_change);
        self.effect_store.pending_trigger_events[index] = replacement;
    }

    /// Take all pending trigger events (empties the queue).
    pub fn take_pending_trigger_events(&mut self) -> Vec<crate::triggers::TriggerEvent> {
        std::mem::take(&mut self.effect_store.pending_trigger_events)
    }

    pub(crate) fn defer_trigger_entries(
        &mut self,
        entries: impl IntoIterator<Item = crate::triggers::TriggeredAbilityEntry>,
    ) {
        if self.action_observations_suppressed() {
            return;
        }
        self.effect_store.pending_trigger_entries.extend(entries);
    }

    pub(crate) fn take_pending_trigger_entries(
        &mut self,
    ) -> Vec<crate::triggers::TriggeredAbilityEntry> {
        std::mem::take(&mut self.effect_store.pending_trigger_entries)
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

/// Whether a permanent with this linked-face layout is a flip card (CR 710).
/// Hand-built flip fixtures carry a linked face with no layout at all.
fn is_flip_card_layout(layout: LinkedFaceLayout) -> bool {
    matches!(layout, LinkedFaceLayout::Flip | LinkedFaceLayout::None)
}
