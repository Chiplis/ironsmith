use super::*;

impl GameState {
    /// Advances to the next turn.
    ///
    /// Turn order rules:
    /// 1. If there are extra turns queued, the first one is taken instead of normal turn order
    /// 2. If the next player should skip their turn, they are skipped (and removed from skip list)
    /// 3. Otherwise, proceed to the next player in turn order
    pub fn next_turn(&mut self) {
        // Check for extra turns first (Time Walk, etc.)
        let next_player = if !self.turn_store.extra_turns.is_empty() {
            // Take the first extra turn from the queue
            self.turn_store.extra_turns.remove(0)
        } else {
            // Find next player in turn order
            let current_index = self
                .turn_store
                .turn_order
                .iter()
                .position(|&p| p == self.turn.active_player)
                .unwrap_or(0);

            let mut next_index = (current_index + 1) % self.turn_store.turn_order.len();
            let start_index = next_index;

            // Find next valid player (skip players who left or should skip their turn)
            loop {
                let candidate = self.turn_store.turn_order[next_index];

                // Check if player is still in game
                let is_in_game = self.player(candidate).is_some_and(|p| p.is_in_game());

                if is_in_game {
                    // Check if this player should skip their turn
                    if self.turn_store.skip_next_turn.remove(&candidate) {
                        // Player skips this turn, continue to next player
                        next_index = (next_index + 1) % self.turn_store.turn_order.len();
                        if next_index == start_index {
                            // Wrapped around - all players are skipping (shouldn't happen)
                            break;
                        }
                        continue;
                    }
                    // Found a valid player
                    break;
                }

                // Player has left, skip to next
                next_index = (next_index + 1) % self.turn_store.turn_order.len();
                if next_index == start_index {
                    // All other players have left
                    break;
                }
            }

            self.turn_store.turn_order[next_index]
        };

        // Reset turn state
        self.turn.active_player = next_player;
        self.turn.priority_player = Some(next_player);
        self.turn.turn_number += 1;
        self.turn.phase = Phase::Beginning;
        self.turn.step = Some(Step::Untap);
        self.turn_store.tracked_draw_step_player = None;
        self.turn_store.cards_drawn_this_draw_step = 0;
        self.turn_store.combat_phases_started_this_turn = 0;
        self.turn_store.skip_current_turn_combat_phases.clear();
        self.turn_store.skip_current_turn_main_phases.clear();
        self.turn_store.no_combat_damage_this_turn.clear();
        self.turn_store.no_combat_damage_this_combat.clear();

        // Clear turn-based tracking
        self.turn_store.entered_battlefield_last_turn = self
            .turn_store
            .turn_history
            .entered_battlefield_snapshots_this_turn();
        self.turn_store.spells_cast_last_turn_total =
            self.turn_store.turn_history.clear_for_new_turn();
        let spells_cast_last_turn = self.turn_store.spells_cast_last_turn_total;
        if self.has_day_night && self.is_night {
            if spells_cast_last_turn >= 2 {
                self.set_daytime(true);
            }
        } else if self.has_day_night && spells_cast_last_turn == 0 {
            self.set_daytime(false);
        }
        self.turn_store.grant_cast_uses_this_turn.clear();
        self.battlefield_flags_mut()
            .saddled_until_end_of_turn
            .clear();
        {
            let transients = self.combat_transients_mut();
            transients.ninjutsu_attack_targets.clear();
            transients.sneak_attack_targets.clear();
            transients.combat_damage_player_batch_hits.clear();
            transients.speed_increase_triggered_this_turn.clear();
        }

        // Activate any pending player-control effects for the new active player.
        self.activate_pending_player_control(next_player);

        // Begin turn for the player
        if let Some(player) = self.player_mut(next_player) {
            player.begin_turn();
        }
        self.record_turn_start_hand_sizes();
    }

    pub fn record_turn_start_hand_sizes(&mut self) {
        self.turn_store.hand_sizes_at_turn_start = self
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| (player.id, player.hand.len()))
            .collect();
    }

    pub fn mark_combat_phase_started(&mut self) {
        self.turn_store.combat_phases_started_this_turn = self
            .turn_store
            .combat_phases_started_this_turn
            .saturating_add(1);
    }

    /// Add a player-control effect.
    pub fn add_player_control(
        &mut self,
        controller: PlayerId,
        target: PlayerId,
        start: PlayerControlStart,
        duration: PlayerControlDuration,
        source: Option<StableId>,
    ) {
        if matches!(duration, PlayerControlDuration::UntilSourceLeaves)
            && source.is_some_and(|stable| !self.is_source_on_battlefield(stable))
        {
            return;
        }

        let current_turn = self.turn.turn_number;
        let aux = self.auxiliary_tracking_mut();
        aux.player_control_timestamp = aux.player_control_timestamp.saturating_add(1);
        let mut effect = PlayerControlEffect {
            controller,
            target,
            start,
            duration,
            source,
            timestamp: aux.player_control_timestamp,
            active: matches!(start, PlayerControlStart::Immediate),
            expires_on_turn: None,
        };

        if effect.active && matches!(duration, PlayerControlDuration::UntilEndOfTurn) {
            effect.expires_on_turn = Some(current_turn);
        }

        aux.player_control_effects.push(effect);
    }

    /// Add a player-control effect for the currently resolving instruction.
    ///
    /// The returned token should be passed to `remove_scoped_player_control`
    /// when the instruction finishes. Interactive prompts may intentionally
    /// leave the scope present in the partial game state so UI snapshots can
    /// route the pending decision to the controlling player.
    pub fn add_scoped_player_control(
        &mut self,
        controller: PlayerId,
        target: PlayerId,
        source: Option<ObjectId>,
    ) -> u64 {
        let source = source.and_then(|id| self.object(id).map(|obj| obj.stable_id));
        let aux = self.auxiliary_tracking_mut();
        aux.player_control_timestamp = aux.player_control_timestamp.saturating_add(1);
        let timestamp = aux.player_control_timestamp;
        aux.scoped_player_control_effects
            .push(ScopedPlayerControlEffect {
                controller,
                target,
                source,
                timestamp,
            });
        timestamp
    }

    /// Remove a resolving-scope player-control effect.
    pub fn remove_scoped_player_control(&mut self, token: u64) {
        self.auxiliary_tracking_mut()
            .scoped_player_control_effects
            .retain(|effect| effect.timestamp != token);
    }

    /// Return the controlling player for the given player, if any effect applies.
    pub fn controlling_player_for(&self, player: PlayerId) -> PlayerId {
        let mut best: Option<(PlayerId, u64)> = None;
        for effect in &self.auxiliary_tracking.player_control_effects {
            if !effect.active || effect.target != player {
                continue;
            }
            if matches!(effect.duration, PlayerControlDuration::UntilSourceLeaves)
                && effect
                    .source
                    .is_some_and(|stable| !self.is_source_on_battlefield(stable))
            {
                continue;
            }
            if best.is_none_or(|(_, timestamp)| effect.timestamp > timestamp) {
                best = Some((effect.controller, effect.timestamp));
            }
        }

        for effect in &self.auxiliary_tracking.scoped_player_control_effects {
            if effect.target != player {
                continue;
            }
            if effect
                .source
                .is_some_and(|stable| !self.is_source_on_battlefield(stable))
            {
                continue;
            }
            if best.is_none_or(|(_, timestamp)| effect.timestamp > timestamp) {
                best = Some((effect.controller, effect.timestamp));
            }
        }

        best.map(|(controller, _)| controller).unwrap_or(player)
    }

    /// Activate pending player-control effects for the current active player.
    pub fn activate_pending_player_control(&mut self, active_player: PlayerId) {
        let current_turn = self.turn.turn_number;
        for effect in &mut self.auxiliary_tracking_mut().player_control_effects {
            if effect.active {
                continue;
            }
            if !matches!(effect.start, PlayerControlStart::NextTurn) {
                continue;
            }
            if effect.target != active_player {
                continue;
            }

            effect.active = true;
            if matches!(effect.duration, PlayerControlDuration::UntilEndOfTurn) {
                effect.expires_on_turn = Some(current_turn);
            }
        }
    }

    /// Cleanup player-control effects that expire at end of turn.
    pub fn cleanup_player_control_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        let battlefield_sources: HashSet<StableId> = self
            .battlefield
            .iter()
            .filter_map(|&id| self.object(id).map(|obj| obj.stable_id))
            .collect();
        self.auxiliary_tracking_mut()
            .player_control_effects
            .retain(|effect| {
                if matches!(effect.duration, PlayerControlDuration::UntilEndOfTurn)
                    && effect.expires_on_turn == Some(current_turn)
                {
                    return false;
                }
                if matches!(effect.duration, PlayerControlDuration::UntilSourceLeaves)
                    && effect
                        .source
                        .is_some_and(|stable| !battlefield_sources.contains(&stable))
                {
                    return false;
                }
                true
            });
    }

    /// Add a combat-choice control effect that lasts until end of turn.
    pub fn add_combat_choice_control(
        &mut self,
        controller: PlayerId,
        choose_attackers: bool,
        choose_blockers: bool,
    ) {
        let current_turn = self.turn.turn_number;
        let aux = self.auxiliary_tracking_mut();
        aux.combat_choice_control_timestamp = aux.combat_choice_control_timestamp.saturating_add(1);
        aux.combat_choice_control_effects
            .push(CombatChoiceControlEffect {
                controller,
                choose_attackers,
                choose_blockers,
                expires_on_turn: current_turn,
                timestamp: aux.combat_choice_control_timestamp,
            });
    }

    fn combat_choice_controller_for(&self, choose_attackers: bool) -> Option<PlayerId> {
        let mut best: Option<&CombatChoiceControlEffect> = None;
        for effect in &self.auxiliary_tracking.combat_choice_control_effects {
            if effect.expires_on_turn != self.turn.turn_number {
                continue;
            }
            if choose_attackers && !effect.choose_attackers {
                continue;
            }
            if !choose_attackers && !effect.choose_blockers {
                continue;
            }
            if best.is_none_or(|current| effect.timestamp > current.timestamp) {
                best = Some(effect);
            }
        }
        best.map(|effect| effect.controller)
    }

    pub fn combat_choice_controller_for_attackers(&self) -> Option<PlayerId> {
        self.combat_choice_controller_for(true)
    }

    pub fn combat_choice_controller_for_blockers(&self) -> Option<PlayerId> {
        self.combat_choice_controller_for(false)
    }

    pub fn cleanup_combat_choice_control_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        self.auxiliary_tracking_mut()
            .combat_choice_control_effects
            .retain(|effect| effect.expires_on_turn != current_turn);
    }

    pub(super) fn clear_player_control_from_source(&mut self, stable_id: StableId) {
        self.auxiliary_tracking_mut()
            .player_control_effects
            .retain(|effect| {
                !(matches!(effect.duration, PlayerControlDuration::UntilSourceLeaves)
                    && effect.source == Some(stable_id))
            });
    }

    fn is_source_on_battlefield(&self, stable_id: StableId) -> bool {
        self.find_object_by_stable_id(stable_id)
            .and_then(|id| self.object(id))
            .is_some_and(|obj| obj.zone == Zone::Battlefield)
    }

    /// Empties all players' mana pools.
    /// Called at the end of each step and phase per MTG rules.
    /// Players covered by a "don't lose unspent mana" effect (Upwelling,
    /// Kruphix, Omnath) keep the retained portion of their pool.
    pub fn empty_mana_pools(&mut self) {
        let retention: Vec<Option<HashSet<Option<crate::color::Color>>>> = self
            .players
            .iter()
            .map(|player| {
                self.effect_store
                    .cant_effects
                    .retained_mana_scopes(player.id)
                    .cloned()
            })
            .collect();
        for (player, scopes) in self.players.iter_mut().zip(retention) {
            let Some(scopes) = scopes else {
                player.mana_pool.empty();
                player.restricted_mana.clear();
                player.clear_mana_source_provenance();
                continue;
            };
            if scopes.contains(&None) {
                continue;
            }
            let pool = &mut player.mana_pool;
            if !scopes.contains(&Some(crate::color::Color::White)) {
                pool.white = 0;
            }
            if !scopes.contains(&Some(crate::color::Color::Blue)) {
                pool.blue = 0;
            }
            if !scopes.contains(&Some(crate::color::Color::Black)) {
                pool.black = 0;
            }
            if !scopes.contains(&Some(crate::color::Color::Red)) {
                pool.red = 0;
            }
            if !scopes.contains(&Some(crate::color::Color::Green)) {
                pool.green = 0;
            }
            pool.colorless = 0;
            player.restricted_mana.clear();
            player.trim_mana_source_provenance_to_pool();
        }
    }

    /// Clears turn-scoped activated ability tracking.
    /// Called at the beginning of each turn.
    pub fn clear_activated_abilities_tracking(&mut self) {
        self.turn_store
            .turn_history
            .activated_abilities_this_turn
            .clear();
        self.turn_store
            .turn_history
            .loyalty_abilities_activated_this_turn
            .clear();
    }

    /// Record that a creature has attacked this turn.
    pub fn mark_creature_attacked_this_turn(&mut self, creature: ObjectId) {
        self.turn_store
            .turn_history
            .creatures_attacked_this_turn
            .insert(creature);
        *self
            .turn_store
            .turn_history
            .creature_attack_counts_this_turn
            .entry(creature)
            .or_insert(0) += 1;
        self.mark_continuous_state_dirty();
    }

    /// Check whether a creature has attacked this turn.
    pub fn creature_attacked_this_turn(&self, creature: ObjectId) -> bool {
        self.turn_store
            .turn_history
            .creatures_attacked_this_turn
            .contains(&creature)
    }

    /// Count how many times a creature has attacked this turn.
    pub fn creature_attack_count_this_turn(&self, creature: ObjectId) -> u32 {
        self.turn_store
            .turn_history
            .creature_attack_counts_this_turn
            .get(&creature)
            .copied()
            .unwrap_or(0)
    }

    /// Record an explicit combat damage assignment for the next combat damage step.
    pub fn set_combat_damage_assignment(
        &mut self,
        attacker: ObjectId,
        recipient: ObjectId,
        amount: u32,
    ) {
        self.turn_store
            .combat_damage_assignments
            .entry(attacker)
            .or_default()
            .insert(recipient, amount);
    }

    /// Consume explicit damage assignments for an attacker.
    pub fn take_combat_damage_assignments(&mut self, attacker: ObjectId) -> HashMap<ObjectId, u32> {
        self.turn_store
            .combat_damage_assignments
            .remove(&attacker)
            .unwrap_or_default()
    }

    /// Suppress an object's combat-damage assignment for the requested duration.
    pub fn suppress_combat_damage_assignment(
        &mut self,
        source: ObjectId,
        until: crate::effect::Until,
    ) {
        match until {
            crate::effect::Until::EndOfTurn => {
                self.turn_store.no_combat_damage_this_turn.insert(source);
            }
            crate::effect::Until::EndOfCombat => {
                self.turn_store.no_combat_damage_this_combat.insert(source);
            }
            _ => debug_assert!(false, "unsupported assignment suppression duration"),
        }
    }

    /// Whether this object currently assigns no combat damage.
    pub fn combat_damage_assignment_is_suppressed(&self, source: ObjectId) -> bool {
        self.turn_store.no_combat_damage_this_turn.contains(&source)
            || self
                .turn_store
                .no_combat_damage_this_combat
                .contains(&source)
    }

    /// Check whether an object performed a specific keyword action this turn.
    pub fn object_performed_keyword_action_this_turn(
        &self,
        object_id: ObjectId,
        action: KeywordActionKind,
    ) -> bool {
        let stable_id = self
            .object(object_id)
            .map(|object| object.stable_id.object_id())
            .unwrap_or(object_id);

        self.turn_store
            .turn_history
            .event_records
            .iter()
            .chain(self.turn_store.turn_history.staged_event_records.iter())
            .filter_map(|record| record.event.downcast::<crate::events::KeywordActionEvent>())
            .any(|event| {
                event.action == action && (event.source == object_id || event.source == stable_id)
            })
    }

    /// Check whether an object was exerted this turn.
    pub fn object_exerted_this_turn(&self, object_id: ObjectId) -> bool {
        self.object_performed_keyword_action_this_turn(object_id, KeywordActionKind::Exert)
    }

    pub fn creature_blocked_this_turn(&self, creature: ObjectId) -> bool {
        self.turn_store
            .turn_history
            .creature_blocked_this_turn(creature)
    }

    pub fn creature_was_blocked_by_this_turn(&self, attacker: ObjectId, blocker: ObjectId) -> bool {
        self.turn_store
            .turn_history
            .creature_was_blocked_by_this_turn(attacker, blocker)
    }

    /// Record that a specific trigger fired this turn.
    pub fn record_trigger_fired(
        &mut self,
        source_object_id: ObjectId,
        trigger_id: TriggerIdentity,
    ) {
        *self
            .turn_store
            .turn_history
            .triggers_fired_this_turn
            .entry((source_object_id, trigger_id))
            .or_insert(0) += 1;
        self.turn_store
            .turn_history
            .turn_counters
            .increment_trigger_identity(trigger_id);
    }

    /// Get how many times this trigger fired this turn.
    pub fn trigger_fire_count_this_turn(
        &self,
        source_object_id: ObjectId,
        trigger_id: TriggerIdentity,
    ) -> u32 {
        self.turn_store
            .turn_history
            .triggers_fired_this_turn
            .get(&(source_object_id, trigger_id))
            .copied()
            .unwrap_or(0)
    }

    /// Record that a specific triggered ability resolved this turn.
    pub fn record_triggered_ability_resolved(
        &mut self,
        source_object_id: ObjectId,
        trigger_id: TriggerIdentity,
    ) {
        *self
            .turn_store
            .turn_history
            .triggered_abilities_resolved_this_turn
            .entry((source_object_id, trigger_id))
            .or_insert(0) += 1;
        self.turn_store.turn_history.turn_counters.increment_named(
            triggered_ability_resolution_turn_counter_name(source_object_id, trigger_id),
        );
    }

    /// Get how many times this triggered ability resolved this turn.
    pub fn triggered_ability_resolution_count_this_turn(
        &self,
        source_object_id: ObjectId,
        trigger_id: TriggerIdentity,
    ) -> u32 {
        self.turn_store
            .turn_history
            .triggered_abilities_resolved_this_turn
            .get(&(source_object_id, trigger_id))
            .copied()
            .unwrap_or_else(|| {
                self.named_turn_counter(&triggered_ability_resolution_turn_counter_name(
                    source_object_id,
                    trigger_id,
                ))
            })
    }

    /// Record an event kind occurrence this turn.
    pub fn record_trigger_event_kind(&mut self, event_kind: EventKind) {
        self.turn_store
            .turn_history
            .turn_counters
            .increment_event_kind(event_kind);
    }

    /// Get event kind occurrence count this turn.
    pub fn trigger_event_kind_count_this_turn(&self, event_kind: EventKind) -> u32 {
        self.turn_store
            .turn_history
            .turn_counters
            .get(&TurnCounterKey::EventKind(event_kind))
    }

    /// Record the attack target captured while paying a Ninjutsu cost.
    pub fn record_ninjutsu_attack_target(
        &mut self,
        source: ObjectId,
        target: crate::combat_state::AttackTarget,
    ) {
        self.combat_transients_mut()
            .ninjutsu_attack_targets
            .entry(source)
            .or_default()
            .push(target);
    }

    /// Return the most recent Ninjutsu attack target for a source without consuming it.
    pub fn last_ninjutsu_attack_target(
        &self,
        source: ObjectId,
    ) -> Option<&crate::combat_state::AttackTarget> {
        self.combat_transients
            .ninjutsu_attack_targets
            .get(&source)
            .and_then(|targets| targets.last())
    }

    /// Consume the most recent Ninjutsu attack target for a source.
    pub fn pop_ninjutsu_attack_target(
        &mut self,
        source: ObjectId,
    ) -> Option<crate::combat_state::AttackTarget> {
        let transients = self.combat_transients_mut();
        let (popped, remove_entry) = {
            let targets = transients.ninjutsu_attack_targets.get_mut(&source)?;
            let popped = targets.pop();
            (popped, targets.is_empty())
        };
        if remove_entry {
            transients.ninjutsu_attack_targets.remove(&source);
        }
        popped
    }

    /// Forget any pending Ninjutsu attack targets for a source.
    pub fn clear_ninjutsu_attack_targets_for(&mut self, source: ObjectId) {
        self.combat_transients_mut()
            .ninjutsu_attack_targets
            .remove(&source);
    }

    /// Record the attack target captured while paying a Sneak cost.
    pub fn record_sneak_attack_target(
        &mut self,
        source: ObjectId,
        target: crate::combat_state::AttackTarget,
    ) {
        self.combat_transients_mut()
            .sneak_attack_targets
            .entry(source)
            .or_default()
            .push(target);
    }

    /// Return the most recent Sneak attack target for a source without consuming it.
    pub fn last_sneak_attack_target(
        &self,
        source: ObjectId,
    ) -> Option<&crate::combat_state::AttackTarget> {
        self.combat_transients
            .sneak_attack_targets
            .get(&source)
            .and_then(|targets| targets.last())
    }

    /// Consume the most recent Sneak attack target for a source.
    pub fn pop_sneak_attack_target(
        &mut self,
        source: ObjectId,
    ) -> Option<crate::combat_state::AttackTarget> {
        let transients = self.combat_transients_mut();
        let (popped, remove_entry) = {
            let targets = transients.sneak_attack_targets.get_mut(&source)?;
            let popped = targets.pop();
            (popped, targets.is_empty())
        };
        if remove_entry {
            transients.sneak_attack_targets.remove(&source);
        }
        popped
    }

    /// Forget any pending Sneak attack targets for a source.
    pub fn clear_sneak_attack_targets_for(&mut self, source: ObjectId) {
        self.combat_transients_mut()
            .sneak_attack_targets
            .remove(&source);
    }

    /// Clear combat-damage player hits tracked for the current trigger batch.
    pub fn clear_combat_damage_player_batch_hits(&mut self) {
        self.combat_transients_mut()
            .combat_damage_player_batch_hits
            .clear();
    }

    /// Record a combat-damage player hit for the current trigger batch.
    pub fn record_combat_damage_player_batch_hit(&mut self, source: ObjectId, player: PlayerId) {
        self.combat_transients_mut()
            .combat_damage_player_batch_hits
            .push((source, player));
    }

    /// Return combat-damage player hits already seen in the current trigger batch.
    pub fn combat_damage_player_batch_hits(&self) -> &[(ObjectId, PlayerId)] {
        &self.combat_transients.combat_damage_player_batch_hits
    }

    /// Increment an arbitrary named turn counter.
    pub fn increment_named_turn_counter(&mut self, name: impl Into<String>) {
        self.turn_store
            .turn_history
            .turn_counters
            .increment_named(name);
    }

    /// Get an arbitrary named turn counter value.
    pub fn named_turn_counter(&self, name: &str) -> u32 {
        self.turn_store
            .turn_history
            .turn_counters
            .get(&TurnCounterKey::Named(name.to_string()))
    }

    /// Records that an activated ability was used.
    /// Used for OncePerTurn timing restrictions.
    pub fn record_ability_activation(&mut self, source: ObjectId, ability_index: usize) {
        let exhaust_controller = self.object(source).and_then(|object| {
            object
                .abilities
                .get(ability_index)
                .and_then(|ability| match &ability.kind {
                    crate::ability::AbilityKind::Activated(activated)
                        if activated.is_exhaust_ability() =>
                    {
                        Some(self.controller_of(object))
                    }
                    _ => None,
                })
        });
        self.turn_store
            .turn_history
            .activated_abilities_this_turn
            .insert((source, ability_index));
        self.turn_store
            .turn_history
            .turn_counters
            .increment_named(activated_ability_turn_counter_name(source, ability_index));
        if let Some(controller) = exhaust_controller {
            self.turn_store
                .exhaust_abilities_activated
                .insert((source, ability_index));
            self.turn_store
                .turn_history
                .turn_counters
                .increment_named(exhaust_ability_turn_counter_name(controller));
        }
    }

    /// Check if an activated ability has been used this turn.
    pub fn ability_activated_this_turn(&self, source: ObjectId, ability_index: usize) -> bool {
        self.turn_store
            .turn_history
            .activated_abilities_this_turn
            .contains(&(source, ability_index))
    }

    /// Get how many times an activated ability has been used this turn.
    pub fn ability_activation_count_this_turn(
        &self,
        source: ObjectId,
        ability_index: usize,
    ) -> u32 {
        self.named_turn_counter(&activated_ability_turn_counter_name(source, ability_index))
    }

    /// Records that a loyalty ability of this permanent was activated this turn.
    pub fn record_loyalty_ability_activation(&mut self, source: ObjectId) {
        self.turn_store
            .turn_history
            .loyalty_abilities_activated_this_turn
            .insert(source);
    }

    /// Check if any loyalty ability of this permanent has been activated this turn.
    pub fn loyalty_ability_activated_this_turn(&self, source: ObjectId) -> bool {
        self.turn_store
            .turn_history
            .loyalty_abilities_activated_this_turn
            .contains(&source)
    }

    /// Check if an exhaust ability has already been activated by this object instance.
    pub fn exhaust_ability_activated(&self, source: ObjectId, ability_index: usize) -> bool {
        self.turn_store
            .exhaust_abilities_activated
            .contains(&(source, ability_index))
    }

    /// Count exhaust activations by this player during the current turn.
    pub fn exhaust_ability_activation_count_this_turn(&self, player: PlayerId) -> u32 {
        self.named_turn_counter(&exhaust_ability_turn_counter_name(player))
    }

    /// Record that a specific activated ability resolved this turn.
    pub fn record_activated_ability_resolved(&mut self, source: ObjectId, ability_index: usize) {
        *self
            .turn_store
            .turn_history
            .activated_abilities_resolved_this_turn
            .entry((source, ability_index))
            .or_insert(0) += 1;
        self.turn_store.turn_history.turn_counters.increment_named(
            activated_ability_resolution_turn_counter_name(source, ability_index),
        );
    }

    /// Get how many times this activated ability resolved this turn.
    pub fn activated_ability_resolution_count_this_turn(
        &self,
        source: ObjectId,
        ability_index: usize,
    ) -> u32 {
        self.turn_store
            .turn_history
            .activated_abilities_resolved_this_turn
            .get(&(source, ability_index))
            .copied()
            .unwrap_or_else(|| {
                self.named_turn_counter(&activated_ability_resolution_turn_counter_name(
                    source,
                    ability_index,
                ))
            })
    }

    /// Record that a mode index was chosen for an activated modal ability.
    pub fn record_ability_mode_choice(
        &mut self,
        source: ObjectId,
        ability_index: usize,
        mode_index: usize,
        this_turn: bool,
    ) {
        if this_turn {
            self.turn_store
                .turn_history
                .chosen_modes_by_ability_this_turn
                .entry((source, ability_index))
                .or_default()
                .insert(mode_index);
        } else {
            self.choice_store_mut()
                .chosen_modes_by_ability
                .entry((source, ability_index))
                .or_default()
                .insert(mode_index);
        }
    }

    /// Check whether a given mode index has already been chosen for an activated ability.
    pub fn ability_mode_was_chosen(
        &self,
        source: ObjectId,
        ability_index: usize,
        mode_index: usize,
        this_turn: bool,
    ) -> bool {
        let target_map = if this_turn {
            &self
                .turn_store
                .turn_history
                .chosen_modes_by_ability_this_turn
        } else {
            &self.choice_store.chosen_modes_by_ability
        };
        target_map
            .get(&(source, ability_index))
            .is_some_and(|modes| modes.contains(&mode_index))
    }

    /// Check whether an activated modal ability still has an unchosen mode available.
    pub fn ability_has_unchosen_mode(
        &self,
        source: ObjectId,
        ability_index: usize,
        total_mode_count: usize,
        this_turn: bool,
    ) -> bool {
        if total_mode_count == 0 {
            return false;
        }
        let target_map = if this_turn {
            &self
                .turn_store
                .turn_history
                .chosen_modes_by_ability_this_turn
        } else {
            &self.choice_store.chosen_modes_by_ability
        };
        let chosen_count = target_map
            .get(&(source, ability_index))
            .map_or(0, HashSet::len);
        chosen_count < total_mode_count
    }

    /// Returns the active player.
    pub fn active_player(&self) -> Option<&Player> {
        self.player(self.turn.active_player)
    }

    /// Returns a mutable reference to the active player.
    pub fn active_player_mut(&mut self) -> Option<&mut Player> {
        self.player_mut(self.turn.active_player)
    }

    /// Pushes a spell or ability onto the stack.
    pub fn push_to_stack(&mut self, mut entry: StackEntry) {
        if !entry.is_ability {
            let tag = crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG);
            let spent_sources = self
                .object(entry.object_id)
                .and_then(|source| source.cast_tagged_objects.get(&tag))
                .filter(|snapshots| !snapshots.is_empty())
                .cloned();
            if let Some(spent_sources) = spent_sources {
                entry.tagged_objects.entry(tag).or_insert(spent_sources);
            }
        }
        if entry.source_snapshot.is_none()
            && let Some(source) = self.object(entry.object_id)
        {
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    source, self,
                );
            entry.source_stable_id.get_or_insert(snapshot.stable_id);
            entry
                .source_name
                .get_or_insert_with(|| snapshot.name.to_string());
            entry.source_snapshot = Some(snapshot);
        }
        self.stack.push(entry);
        self.update_replacement_effects();
    }

    /// Pops and returns the top item from the stack.
    pub fn pop_from_stack(&mut self) -> Option<StackEntry> {
        self.stack.pop()
    }

    /// Returns true if the stack is empty.
    pub fn stack_is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Returns the number of players still in the game.
    pub fn players_in_game(&self) -> usize {
        self.players.iter().filter(|p| p.is_in_game()).count()
    }

    /// Returns true when the match is using commander designations.
    pub fn is_commander_game(&self) -> bool {
        self.players
            .iter()
            .any(|player| !player.commanders.is_empty())
    }

    /// Returns true if this player's turn-one draw-step draw should be skipped.
    pub fn should_skip_first_turn_draw(&self, player_id: PlayerId) -> bool {
        self.turn.turn_number == 1
            && self.turn.active_player == player_id
            && self.turn_store.turn_order.first().copied() == Some(player_id)
            && !self.is_commander_game()
    }

    // =========================================================================
    // Object Dual-Identity Helpers (id vs stable_id)
    // =========================================================================
    //
    // Objects have two identifiers:
    // - `id`: Changes on each zone change (per MTG rule 400.7)
    // - `stable_id`: Stable identifier that persists across zone changes
    //
    // Commander tracking uses the original ObjectId, which becomes the stable_id
    // after zone changes. These helpers abstract over this complexity.

    /// Check if an object is a commander (by current ID or stable_id).
    ///
    /// This handles the dual-identity nature of objects where zone changes
    /// create new IDs but stable_id persists.
    pub fn is_commander(&self, obj_id: ObjectId) -> bool {
        self.commander_identity(obj_id).is_some()
    }

    /// Find an object by its stable_id (stable identifier).
    ///
    /// Returns the current ObjectId of the object with the given stable_id,
    /// or None if no such object exists.
    pub fn find_object_by_stable_id(&self, stable_id: StableId) -> Option<ObjectId> {
        let id = *self.stable_id_index.get(&stable_id)?;
        self.objects
            .get(&id)
            .filter(|o| o.stable_id == stable_id)
            .map(|o| o.id)
    }

    /// Check if a player controls any of their own commanders on the battlefield.
    ///
    /// This checks if the player controls a permanent that is designated as
    /// one of their own commanders.
    pub fn player_controls_own_commander(&self, player_id: PlayerId) -> bool {
        let commanders = if let Some(player) = self.player(player_id) {
            player.get_commanders().to_vec()
        } else {
            return false;
        };

        // Check if any of the player's commanders are on the battlefield
        // under their control
        for &commander_id in &commanders {
            // A commander might have a different ObjectId now due to zone changes.
            // We check both the current ID and the stable_id (which persists across zone changes).
            for &bf_id in &self.battlefield {
                if let Some(obj) = self.object(bf_id)
                    && self.controller_of(obj) == player_id
                {
                    // Check if this is the commander by current ID
                    if bf_id == commander_id {
                        return true;
                    }
                    // Also check stable_id in case the commander moved zones
                    if obj.stable_id == StableId::from(commander_id) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a player controls ANY commander on the battlefield.
    ///
    /// This checks if the player controls a permanent that is designated as
    /// a commander by ANY player (including opponents' commanders that were stolen).
    /// Used for cards like Akroma's Will which say "if you control a commander".
    pub fn player_controls_a_commander(&self, player_id: PlayerId) -> bool {
        // Collect all commander IDs from all players
        let all_commanders: Vec<ObjectId> = self
            .players
            .iter()
            .flat_map(|p| p.get_commanders().iter().copied())
            .collect();

        // Check if any commander is on the battlefield under this player's control
        for &commander_id in &all_commanders {
            for &bf_id in &self.battlefield {
                if let Some(obj) = self.object(bf_id)
                    && self.controller_of(obj) == player_id
                {
                    // Check if this is a commander by current ID or stable_id
                    if bf_id == commander_id || obj.stable_id == StableId::from(commander_id) {
                        return true;
                    }
                }
            }
        }

        false
    }

    // =========================================================================
    // FilterContext Factory Methods
    // =========================================================================

    /// Create a FilterContext for a controller and optional source.
    ///
    /// This factory method ensures consistent FilterContext construction across
    /// the codebase. It properly populates:
    /// - `you` - the controller
    /// - `source` - the source object (if any)
    /// - `active_player` - the current active player
    /// - `opponents` - all opponents of the controller
    /// - `your_commanders` - the controller's commander IDs
    ///
    /// Use `filter_context_for_combat()` if you also need combat context.
    pub fn filter_context_for(
        &self,
        controller: PlayerId,
        source: Option<ObjectId>,
    ) -> crate::target::FilterContext {
        let opponents = self
            .players
            .iter()
            .filter(|p| p.id != controller && p.is_in_game())
            .map(|p| p.id)
            .collect();

        let your_commanders = self
            .player(controller)
            .map(|p| p.commanders.clone())
            .unwrap_or_default();

        let mut tagged_objects = std::collections::HashMap::new();
        let mut tagged_players = std::collections::HashMap::new();
        if let Some(source_id) = source
            && let Some(source_obj) = self.object(source_id)
        {
            tagged_objects.extend(source_obj.cast_tagged_objects.clone());
            let source_is_aura = source_obj.subtypes.contains(&crate::types::Subtype::Aura)
                || (source_obj
                    .card_types
                    .contains(&crate::types::CardType::Enchantment)
                    && source_obj.aura_attach_filter.is_some());
            let source_is_equipment = source_obj
                .subtypes
                .contains(&crate::types::Subtype::Equipment);
            if let Some(attached_target) = source_obj.attached_to {
                match attached_target {
                    AttachmentTarget::Object(attached_id) => {
                        if let Some(attached_obj) = self.object(attached_id) {
                            let attached_snapshot =
                                crate::snapshot::ObjectSnapshot::from_object(attached_obj, self);
                            if source_is_aura {
                                tagged_objects.insert(
                                    crate::tag::TagKey::from("enchanted"),
                                    vec![attached_snapshot.clone()],
                                );
                            }
                            if source_is_equipment {
                                tagged_objects.insert(
                                    crate::tag::TagKey::from("equipped"),
                                    vec![attached_snapshot],
                                );
                            }
                        }
                    }
                    AttachmentTarget::Player(attached_player) => {
                        if source_is_aura {
                            tagged_players.insert(
                                crate::tag::TagKey::from("enchanted"),
                                vec![attached_player],
                            );
                        }
                    }
                }
            }
        }

        crate::target::FilterContext {
            you: Some(controller),
            source,
            caster: None,
            active_player: Some(self.turn.active_player),
            opponents,
            teammates: Vec::new(), // Team formats are not modeled yet.
            defending_player: None,
            attacking_player: None,
            your_commanders,
            iterated_player: None,
            x_value: None,
            chosen_player: source.and_then(|source_id| self.chosen_player(source_id)),
            target_players: Vec::new(),
            target_objects: Vec::new(),
            tagged_objects,
            tagged_players,
            effect_outcomes: std::collections::HashMap::new(),
        }
    }

    /// Create a FilterContext with combat context.
    ///
    /// This extends `filter_context_for()` with combat-specific fields:
    /// - `defending_player` - the player being attacked
    /// - `attacking_player` - the player who declared attackers
    pub fn filter_context_for_combat(
        &self,
        controller: PlayerId,
        source: Option<ObjectId>,
        defending_player: Option<PlayerId>,
        attacking_player: Option<PlayerId>,
    ) -> crate::target::FilterContext {
        let mut ctx = self.filter_context_for(controller, source);
        ctx.defending_player = defending_player;
        ctx.attacking_player = attacking_player;
        ctx
    }

    /// Get the combined color identity of a player's commanders.
    ///
    /// This returns the union of color identities of all the player's commanders.
    /// Used for cards like Arcane Signet and Command Tower.
    /// If the player has no commanders, returns COLORLESS (producing colorless mana).
    pub fn get_commander_color_identity(&self, player_id: PlayerId) -> crate::color::ColorSet {
        let commanders = if let Some(player) = self.player(player_id) {
            player.get_commanders().to_vec()
        } else {
            return crate::color::ColorSet::COLORLESS;
        };

        let mut identity = crate::color::ColorSet::COLORLESS;

        for &commander_id in &commanders {
            // Try to find the commander object - it might be on battlefield,
            // in command zone, or elsewhere
            if let Some(obj) = self.object(commander_id) {
                identity = identity.union(obj.color_identity());
            } else {
                // Commander might have moved zones and have a different ID.
                // Search through all objects for one with matching stable_id
                for obj in self.objects.values() {
                    if obj.stable_id == StableId::from(commander_id) {
                        identity = identity.union(obj.color_identity());
                        break;
                    }
                }
            }
        }

        identity
    }

    // =========================================================================
    // Battlefield State Extension Map Helpers
    // =========================================================================

    /// Check if a permanent is tapped.
    pub fn is_tapped(&self, id: ObjectId) -> bool {
        self.battlefield_flags.tapped_permanents.contains(&id)
    }

    /// Tap a permanent.
    pub fn tap(&mut self, id: ObjectId) {
        if self.battlefield_flags_mut().tapped_permanents.insert(id) {
            self.mark_tapped_state_changed(id);
        }
    }

    /// Untap a permanent.
    pub fn untap(&mut self, id: ObjectId) {
        let changed = self.battlefield_flags_mut().tapped_permanents.remove(&id);
        if !changed {
            return;
        }

        self.mark_continuous_state_dirty();
        let removed_continuous = self
            .effect_store
            .continuous_effects
            .remove_effects_from_source_with_duration(id, crate::effect::Until::SourceUntaps);
        let restriction_count = self.effect_store.restriction_effects.len();
        self.effect_store.restriction_effects.retain(|effect| {
            !(effect.source == id && effect.duration == crate::effect::Until::SourceUntaps)
        });
        let removed_restrictions = self.effect_store.restriction_effects.len() != restriction_count;
        let goad_count = self.effect_store.goad_effects.len();
        self.effect_store.goad_effects.retain(|effect| {
            !(effect.source == id && effect.duration == crate::effect::Until::SourceUntaps)
        });
        let removed_goad = self.effect_store.goad_effects.len() != goad_count;

        if changed || removed_continuous || removed_restrictions || removed_goad {
            self.update_cant_effects();
        }
    }

    fn mark_tapped_state_changed(&mut self, id: ObjectId) {
        if self.tapped_state_change_can_stay_local(id) {
            self.mark_object_characteristics_dirty(id);
        } else {
            self.mark_continuous_state_dirty();
        }
    }

    fn tapped_state_change_can_stay_local(&self, id: ObjectId) -> bool {
        self.characteristic_extension_change_can_stay_local(
            id,
            Self::continuous_effect_reads_tapped_state,
        )
    }

    fn characteristic_extension_change_can_stay_local(
        &self,
        id: ObjectId,
        effect_reads_state: impl Fn(&ContinuousEffect, ObjectId) -> bool,
    ) -> bool {
        if !self.continuous_state_is_clean() {
            return false;
        }

        let Some(object) = self.object(id) else {
            return false;
        };
        if object.zone != Zone::Battlefield {
            return false;
        }
        if object
            .abilities
            .iter()
            .any(|ability| matches!(&ability.kind, crate::ability::AbilityKind::Static(_)))
        {
            return false;
        }
        if self
            .current_characteristics(id)
            .is_some_and(|chars| !chars.static_abilities.is_empty())
        {
            return false;
        }

        !self
            .cached_continuous_effects_snapshot_arc()
            .iter()
            .any(|effect| effect_reads_state(effect, id))
    }

    fn continuous_effect_reads_tapped_state(effect: &ContinuousEffect, changed: ObjectId) -> bool {
        if effect.source == changed {
            return true;
        }
        if Self::effect_target_reads_tapped_state(&effect.applies_to) {
            return true;
        }
        effect
            .condition
            .as_ref()
            .is_some_and(Self::condition_reads_tapped_state)
    }

    fn effect_target_reads_tapped_state(target: &EffectTarget) -> bool {
        match target {
            EffectTarget::Filter(filter) => Self::filter_reads_tapped_state(filter),
            _ => false,
        }
    }

    fn condition_reads_tapped_state(condition: &crate::ConditionExpr) -> bool {
        match condition {
            crate::ConditionExpr::TargetIsTapped
            | crate::ConditionExpr::SourceIsTapped
            | crate::ConditionExpr::EquippedCreatureTapped
            | crate::ConditionExpr::EquippedCreatureUntapped
            | crate::ConditionExpr::SourceIsUntapped => true,
            crate::ConditionExpr::SourceMatches(filter)
            | crate::ConditionExpr::AttachedToSourceMatches(filter)
            | crate::ConditionExpr::TargetMatches(filter)
            | crate::ConditionExpr::TaggedObjectMatches(_, filter)
            | crate::ConditionExpr::TaggedObjectMatchedLastKnown(_, filter)
            | crate::ConditionExpr::PlayerTaggedObjectMatches { filter, .. } => {
                Self::filter_reads_tapped_state(filter)
            }
            crate::ConditionExpr::MatchingObjectAttachedToMatchingObject {
                attachment,
                attached_to,
            } => {
                Self::filter_reads_tapped_state(attachment)
                    || Self::filter_reads_tapped_state(attached_to)
            }
            crate::ConditionExpr::SourceCrewedByExactly { filter, .. } => {
                Self::filter_reads_tapped_state(filter)
            }
            crate::ConditionExpr::CountComparison { count, .. }
            | crate::ConditionExpr::CountParity { count, .. } => {
                Self::anthem_count_reads_tapped_state(count)
            }
            crate::ConditionExpr::Not(inner) => Self::condition_reads_tapped_state(inner),
            crate::ConditionExpr::And(left, right) | crate::ConditionExpr::Or(left, right) => {
                Self::condition_reads_tapped_state(left)
                    || Self::condition_reads_tapped_state(right)
            }
            _ => false,
        }
    }

    fn anthem_count_reads_tapped_state(count: &AnthemCountExpression) -> bool {
        match count {
            AnthemCountExpression::MatchingFilter(filter)
            | AnthemCountExpression::GreatestManaValueAmong(filter)
            | AnthemCountExpression::AttachedToSource(filter)
            | AnthemCountExpression::AttachedToAffected(filter)
            | AnthemCountExpression::CountersAmong(filter, _)
            | AnthemCountExpression::DistinctCounterTypesAmong(filter)
            | AnthemCountExpression::BasicLandTypesAmong(filter)
            | AnthemCountExpression::CreatureTypesAmong(filter) => {
                Self::filter_reads_tapped_state(filter)
            }
            _ => false,
        }
    }

    fn filter_reads_tapped_state(filter: &crate::target::ObjectFilter) -> bool {
        filter.tapped
            || filter.untapped
            || filter
                .targets_object
                .as_deref()
                .is_some_and(Self::filter_reads_tapped_state)
            || filter
                .targets_only_object
                .as_deref()
                .is_some_and(Self::filter_reads_tapped_state)
            || filter
                .attached_to_object
                .as_deref()
                .is_some_and(Self::filter_reads_tapped_state)
            || filter
                .no_shared_creature_types_with
                .iter()
                .any(Self::filter_reads_tapped_state)
            || filter.any_of.iter().any(Self::filter_reads_tapped_state)
    }

    pub(super) fn mark_face_down_state_changed(&mut self, id: ObjectId) {
        self.effect_store.continuous_effects.record_face_change(id);
        if self.face_down_state_change_can_stay_local(id) {
            self.mark_object_characteristics_dirty(id);
        } else {
            self.mark_continuous_state_dirty();
        }
    }

    fn face_down_state_change_can_stay_local(&self, id: ObjectId) -> bool {
        self.characteristic_extension_change_can_stay_local(
            id,
            Self::continuous_effect_reads_face_down_state,
        )
    }

    fn continuous_effect_reads_face_down_state(
        effect: &ContinuousEffect,
        changed: ObjectId,
    ) -> bool {
        if effect.source == changed {
            return true;
        }
        if Self::effect_target_reads_face_down_state(&effect.applies_to) {
            return true;
        }
        effect
            .condition
            .as_ref()
            .is_some_and(Self::condition_reads_face_down_state)
    }

    fn effect_target_reads_face_down_state(target: &EffectTarget) -> bool {
        match target {
            EffectTarget::Filter(filter) => Self::filter_reads_face_down_state(filter),
            _ => false,
        }
    }

    fn condition_reads_face_down_state(condition: &crate::ConditionExpr) -> bool {
        match condition {
            crate::ConditionExpr::SourceIsFaceDown => true,
            crate::ConditionExpr::SourceMatches(filter)
            | crate::ConditionExpr::AttachedToSourceMatches(filter)
            | crate::ConditionExpr::TargetMatches(filter)
            | crate::ConditionExpr::TaggedObjectMatches(_, filter)
            | crate::ConditionExpr::TaggedObjectMatchedLastKnown(_, filter)
            | crate::ConditionExpr::PlayerTaggedObjectMatches { filter, .. } => {
                Self::filter_reads_face_down_state(filter)
            }
            crate::ConditionExpr::MatchingObjectAttachedToMatchingObject {
                attachment,
                attached_to,
            } => {
                Self::filter_reads_face_down_state(attachment)
                    || Self::filter_reads_face_down_state(attached_to)
            }
            crate::ConditionExpr::SourceCrewedByExactly { filter, .. } => {
                Self::filter_reads_face_down_state(filter)
            }
            crate::ConditionExpr::CountComparison { count, .. }
            | crate::ConditionExpr::CountParity { count, .. } => {
                Self::anthem_count_reads_face_down_state(count)
            }
            crate::ConditionExpr::Not(inner) => Self::condition_reads_face_down_state(inner),
            crate::ConditionExpr::And(left, right) | crate::ConditionExpr::Or(left, right) => {
                Self::condition_reads_face_down_state(left)
                    || Self::condition_reads_face_down_state(right)
            }
            _ => false,
        }
    }

    fn anthem_count_reads_face_down_state(count: &AnthemCountExpression) -> bool {
        match count {
            AnthemCountExpression::MatchingFilter(filter)
            | AnthemCountExpression::GreatestManaValueAmong(filter)
            | AnthemCountExpression::AttachedToSource(filter)
            | AnthemCountExpression::AttachedToAffected(filter)
            | AnthemCountExpression::CountersAmong(filter, _)
            | AnthemCountExpression::DistinctCounterTypesAmong(filter)
            | AnthemCountExpression::BasicLandTypesAmong(filter)
            | AnthemCountExpression::CreatureTypesAmong(filter) => {
                Self::filter_reads_face_down_state(filter)
            }
            _ => false,
        }
    }

    fn filter_reads_face_down_state(filter: &crate::target::ObjectFilter) -> bool {
        filter.face_down.is_some()
            || filter
                .targets_object
                .as_deref()
                .is_some_and(Self::filter_reads_face_down_state)
            || filter
                .targets_only_object
                .as_deref()
                .is_some_and(Self::filter_reads_face_down_state)
            || filter
                .attached_to_object
                .as_deref()
                .is_some_and(Self::filter_reads_face_down_state)
            || filter
                .no_shared_creature_types_with
                .iter()
                .any(Self::filter_reads_face_down_state)
            || filter.any_of.iter().any(Self::filter_reads_face_down_state)
    }

    pub(super) fn mark_summoning_sickness_changed(&mut self, id: ObjectId) {
        if self.summoning_sickness_change_can_stay_local(id) {
            self.mark_object_characteristics_dirty(id);
        } else {
            self.mark_continuous_state_dirty();
        }
    }

    fn summoning_sickness_change_can_stay_local(&self, id: ObjectId) -> bool {
        if !self.continuous_state_is_clean() {
            return false;
        }

        let Some(object) = self.object(id) else {
            return false;
        };
        if object.zone != Zone::Battlefield {
            return false;
        }

        !self
            .cached_continuous_effects_snapshot_arc()
            .iter()
            .any(|effect| Self::continuous_effect_reads_summoning_sickness_state(effect, id))
    }

    fn continuous_effect_reads_summoning_sickness_state(
        effect: &ContinuousEffect,
        _changed: ObjectId,
    ) -> bool {
        if Self::effect_target_reads_summoning_sickness_state(&effect.applies_to) {
            return true;
        }
        effect
            .condition
            .as_ref()
            .is_some_and(Self::condition_reads_summoning_sickness_state)
    }

    fn effect_target_reads_summoning_sickness_state(target: &EffectTarget) -> bool {
        match target {
            EffectTarget::Filter(filter) => Self::filter_reads_summoning_sickness_state(filter),
            _ => false,
        }
    }

    fn condition_reads_summoning_sickness_state(condition: &crate::ConditionExpr) -> bool {
        match condition {
            crate::ConditionExpr::SourceMatches(filter)
            | crate::ConditionExpr::AttachedToSourceMatches(filter)
            | crate::ConditionExpr::TargetMatches(filter)
            | crate::ConditionExpr::TaggedObjectMatches(_, filter)
            | crate::ConditionExpr::TaggedObjectMatchedLastKnown(_, filter)
            | crate::ConditionExpr::PlayerTaggedObjectMatches { filter, .. }
            | crate::ConditionExpr::SourceCrewedByExactly { filter, .. } => {
                Self::filter_reads_summoning_sickness_state(filter)
            }
            crate::ConditionExpr::MatchingObjectAttachedToMatchingObject {
                attachment,
                attached_to,
            } => {
                Self::filter_reads_summoning_sickness_state(attachment)
                    || Self::filter_reads_summoning_sickness_state(attached_to)
            }
            crate::ConditionExpr::CountComparison { count, .. }
            | crate::ConditionExpr::CountParity { count, .. } => {
                Self::anthem_count_reads_summoning_sickness_state(count)
            }
            crate::ConditionExpr::Not(inner) => {
                Self::condition_reads_summoning_sickness_state(inner)
            }
            crate::ConditionExpr::And(left, right) | crate::ConditionExpr::Or(left, right) => {
                Self::condition_reads_summoning_sickness_state(left)
                    || Self::condition_reads_summoning_sickness_state(right)
            }
            _ => false,
        }
    }

    fn anthem_count_reads_summoning_sickness_state(count: &AnthemCountExpression) -> bool {
        match count {
            AnthemCountExpression::MatchingFilter(filter)
            | AnthemCountExpression::GreatestManaValueAmong(filter)
            | AnthemCountExpression::AttachedToSource(filter)
            | AnthemCountExpression::AttachedToAffected(filter)
            | AnthemCountExpression::CountersAmong(filter, _)
            | AnthemCountExpression::DistinctCounterTypesAmong(filter)
            | AnthemCountExpression::BasicLandTypesAmong(filter)
            | AnthemCountExpression::CreatureTypesAmong(filter) => {
                Self::filter_reads_summoning_sickness_state(filter)
            }
            _ => false,
        }
    }

    fn filter_reads_summoning_sickness_state(filter: &crate::target::ObjectFilter) -> bool {
        filter.entered_since_your_last_turn_ended
            || filter
                .targets_object
                .as_deref()
                .is_some_and(Self::filter_reads_summoning_sickness_state)
            || filter
                .targets_only_object
                .as_deref()
                .is_some_and(Self::filter_reads_summoning_sickness_state)
            || filter
                .no_shared_creature_types_with
                .iter()
                .any(Self::filter_reads_summoning_sickness_state)
            || filter
                .any_of
                .iter()
                .any(Self::filter_reads_summoning_sickness_state)
    }

    pub(super) fn mark_source_designation_changed(
        &mut self,
        id: ObjectId,
        condition_reads_state: fn(&crate::ConditionExpr) -> bool,
    ) {
        if self.source_designation_change_can_stay_local(id, condition_reads_state) {
            self.mark_object_characteristics_dirty(id);
        } else {
            self.mark_continuous_state_dirty();
        }
    }

    fn source_designation_change_can_stay_local(
        &self,
        id: ObjectId,
        condition_reads_state: fn(&crate::ConditionExpr) -> bool,
    ) -> bool {
        self.characteristic_extension_change_can_stay_local(id, |effect, _| {
            effect.condition.as_ref().is_some_and(condition_reads_state)
        })
    }

    pub(super) fn condition_reads_monstrous_state(condition: &crate::ConditionExpr) -> bool {
        Self::condition_matches_or_nested(condition, |condition| {
            matches!(condition, crate::ConditionExpr::SourceIsMonstrous)
        })
    }

    pub(super) fn condition_reads_saddled_state(condition: &crate::ConditionExpr) -> bool {
        Self::condition_matches_or_nested(condition, |condition| {
            matches!(condition, crate::ConditionExpr::SourceIsSaddled)
        })
    }

    pub(super) fn condition_reads_devoured_count(condition: &crate::ConditionExpr) -> bool {
        Self::condition_matches_or_nested(condition, |condition| {
            matches!(
                condition,
                crate::ConditionExpr::SourceDevouredCreaturesOrMore(_)
            )
        })
    }

    pub(super) fn condition_reads_suspected_state(condition: &crate::ConditionExpr) -> bool {
        Self::condition_matches_or_nested(condition, |condition| {
            matches!(condition, crate::ConditionExpr::SourceSuspected)
        })
    }

    fn condition_matches_or_nested(
        condition: &crate::ConditionExpr,
        direct_match: fn(&crate::ConditionExpr) -> bool,
    ) -> bool {
        if direct_match(condition) {
            return true;
        }
        match condition {
            crate::ConditionExpr::Not(inner) => {
                Self::condition_matches_or_nested(inner, direct_match)
            }
            crate::ConditionExpr::And(left, right) | crate::ConditionExpr::Or(left, right) => {
                Self::condition_matches_or_nested(left, direct_match)
                    || Self::condition_matches_or_nested(right, direct_match)
            }
            _ => false,
        }
    }
}
