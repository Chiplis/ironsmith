use super::*;

impl GameState {
    /// Whether `player` has taken the special action that lets them ignore the
    /// attached-object rule restrictions from `source` this turn.
    pub fn player_ignores_attached_static_restrictions_this_turn(
        &self,
        source: ObjectId,
        player: PlayerId,
    ) -> bool {
        self.turn_store
            .turn_history
            .players_ignoring_attached_static_restrictions_this_turn
            .contains(&(source, player))
    }

    /// Whether the current controller of the source's attached object has paid
    /// to ignore the source's rule restrictions this turn.
    pub fn attached_static_restrictions_are_ignored_this_turn(&self, source: ObjectId) -> bool {
        let Some(crate::object::AttachmentTarget::Object(attached_id)) =
            self.object(source).and_then(|object| object.attached_to)
        else {
            return false;
        };
        self.controller_of_id(attached_id).is_some_and(|player| {
            self.player_ignores_attached_static_restrictions_this_turn(source, player)
        })
    }

    /// Suppress the source's rule-restriction abilities through the current
    /// turn boundary without disabling unrelated abilities of that source.
    pub(crate) fn player_ignores_attached_static_restrictions_until_end_of_turn(
        &mut self,
        source: ObjectId,
        player: PlayerId,
    ) -> bool {
        let inserted = self
            .turn_store
            .turn_history
            .players_ignoring_attached_static_restrictions_this_turn
            .insert((source, player));
        if inserted {
            self.mark_continuous_state_dirty();
            self.bump_mutation_revision();
        }
        inserted
    }

    /// Add one step directly before the next occurrence of `before` this turn.
    pub fn add_step_before(&mut self, step: Step, before: Step) {
        self.add_step_at(step, AddedStepPlacement::BeforeStep(before));
    }

    /// Add one step directly after the next occurrence of `after` this turn.
    pub fn add_step_after(&mut self, step: Step, after: Step) {
        self.add_step_at(step, AddedStepPlacement::AfterStep(after));
    }

    /// Apply a CR 500.10a “you get” addition before a named step.
    pub fn add_step_before_for_controller(
        &mut self,
        controller: PlayerId,
        step: Step,
        before: Step,
    ) -> bool {
        self.add_step_for_controller_at(controller, step, AddedStepPlacement::BeforeStep(before))
    }

    /// Apply a CR 500.10a “you get” addition after a named step.
    pub fn add_step_after_for_controller(
        &mut self,
        controller: PlayerId,
        step: Step,
        after: Step,
    ) -> bool {
        self.add_step_for_controller_at(controller, step, AddedStepPlacement::AfterStep(after))
    }

    /// Add a phase containing only `step` directly after `after` this turn.
    pub fn add_step_after_phase(&mut self, step: Step, after: Phase) {
        self.add_step_at(step, AddedStepPlacement::AfterPhase(after));
    }

    /// Apply a CR 500.10a “you get” step addition.
    ///
    /// Such an addition does nothing during a turn other than the effect
    /// controller's turn.
    pub fn add_step_after_phase_for_controller(
        &mut self,
        controller: PlayerId,
        step: Step,
        after: Phase,
    ) -> bool {
        self.add_step_for_controller_at(controller, step, AddedStepPlacement::AfterPhase(after))
    }

    fn add_step_for_controller_at(
        &mut self,
        controller: PlayerId,
        step: Step,
        placement: AddedStepPlacement,
    ) -> bool {
        if !self.is_active_player(controller) {
            return false;
        }
        self.add_step_at(step, placement);
        true
    }

    fn add_step_at(&mut self, step: Step, placement: AddedStepPlacement) {
        self.normalize_additional_phase_metadata();
        let creation_order = self.turn_store.next_turn_schedule_order;
        self.turn_store.next_turn_schedule_order = creation_order.saturating_add(1);
        self.turn_store.added_steps.push(AddedStep {
            step,
            placement,
            turn_number: self.turn.turn_number,
            creation_order,
        });
    }

    /// Schedule one independently consumable skip of `player`'s next `step`.
    pub fn skip_next_step(&mut self, player: PlayerId, step: Step) {
        let player = self.team_turn_representative(player);
        *self
            .turn_store
            .skipped_steps
            .entry((player, step))
            .or_default() += 1;
    }

    pub fn pending_step_skips(&self, player: PlayerId, step: Step) -> u32 {
        let player = self.team_turn_representative(player);
        self.turn_store
            .skipped_steps
            .get(&(player, step))
            .copied()
            .unwrap_or(0)
    }

    /// Consume exactly one applicable step skip.
    pub fn consume_step_skip(&mut self, player: PlayerId, step: Step) -> bool {
        let player = self.team_turn_representative(player);
        let key = (player, step);
        let Some(remaining) = self.turn_store.skipped_steps.get_mut(&key) else {
            return false;
        };
        *remaining = remaining.saturating_sub(1);
        if *remaining == 0 {
            self.turn_store.skipped_steps.remove(&key);
        }
        true
    }

    /// Remove and return additions at one boundary, newest-created first.
    pub(crate) fn take_added_steps(&mut self, placement: AddedStepPlacement) -> Vec<ScheduledStep> {
        self.take_added_step_records(placement)
            .into_iter()
            .map(|addition| ScheduledStep {
                phase: addition.step.containing_phase(),
                step: addition.step,
                isolated_phase: matches!(placement, AddedStepPlacement::AfterPhase(_)),
            })
            .collect()
    }

    fn take_added_step_records(&mut self, placement: AddedStepPlacement) -> Vec<AddedStep> {
        let turn_number = self.turn.turn_number;
        let mut selected = Vec::new();
        for index in (0..self.turn_store.added_steps.len()).rev() {
            let addition = self.turn_store.added_steps[index];
            if addition.turn_number == turn_number && addition.placement == placement {
                self.turn_store.added_steps.remove(index);
                selected.push(addition);
            }
        }
        selected
    }

    /// Add an I019 phase group to the shared creation-ordered phase schedule.
    ///
    /// All phases in one effect share a creation sequence so their written
    /// order remains stable, while later-created groups run first.
    pub(crate) fn add_additional_phase_group(&mut self, phases: impl IntoIterator<Item = Phase>) {
        let phases = phases.into_iter().collect::<Vec<_>>();
        if phases.is_empty() {
            return;
        }
        self.normalize_additional_phase_metadata();
        let creation_order = self.turn_store.next_turn_schedule_order;
        self.turn_store.next_turn_schedule_order = creation_order.saturating_add(1);
        let count = phases.len();
        self.turn_store.additional_phases.splice(0..0, phases);
        self.turn_store
            .additional_phase_orders
            .splice(0..0, std::iter::repeat_n(creation_order, count));
        self.turn_store
            .additional_phase_only_steps
            .splice(0..0, std::iter::repeat_n(None, count));
    }

    /// Merge CR 500.10 single-step phases into the same schedule used by
    /// additional full phases, ordered by their shared creation sequence.
    pub(crate) fn queue_added_step_phases_after(&mut self, phase: Phase) {
        let additions = self.take_added_step_records(AddedStepPlacement::AfterPhase(phase));
        if additions.is_empty() {
            return;
        }
        self.normalize_additional_phase_metadata();
        for addition in additions {
            self.turn_store
                .additional_phases
                .push(addition.step.containing_phase());
            self.turn_store
                .additional_phase_orders
                .push(addition.creation_order);
            self.turn_store
                .additional_phase_only_steps
                .push(Some(addition.step));
        }

        let mut entries = self
            .turn_store
            .additional_phases
            .drain(..)
            .zip(self.turn_store.additional_phase_orders.drain(..))
            .zip(self.turn_store.additional_phase_only_steps.drain(..))
            .map(|((phase, order), only_step)| (phase, order, only_step))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| right.1.cmp(&left.1));
        for (phase, order, only_step) in entries {
            self.turn_store.additional_phases.push(phase);
            self.turn_store.additional_phase_orders.push(order);
            self.turn_store.additional_phase_only_steps.push(only_step);
        }
    }

    /// Pop the next full or CR 500.10 synthetic phase.
    pub(crate) fn pop_additional_phase(&mut self) -> Option<(Phase, Option<Step>)> {
        self.normalize_additional_phase_metadata();
        if self.turn_store.additional_phases.is_empty() {
            return None;
        }
        let phase = self.turn_store.additional_phases.remove(0);
        self.turn_store.additional_phase_orders.remove(0);
        let only_step = self.turn_store.additional_phase_only_steps.remove(0);
        Some((phase, only_step))
    }

    /// Backfill metadata for legacy callers that still populate the public
    /// phase vector directly. Such entries are treated as one creation group.
    fn normalize_additional_phase_metadata(&mut self) {
        let phase_count = self.turn_store.additional_phases.len();
        self.turn_store
            .additional_phase_orders
            .truncate(phase_count);
        self.turn_store
            .additional_phase_only_steps
            .truncate(phase_count);

        let missing_orders =
            phase_count.saturating_sub(self.turn_store.additional_phase_orders.len());
        if missing_orders > 0 {
            let creation_order = self.turn_store.next_turn_schedule_order;
            self.turn_store.next_turn_schedule_order = creation_order.saturating_add(1);
            self.turn_store
                .additional_phase_orders
                .extend(std::iter::repeat_n(creation_order, missing_orders));
        }
        self.turn_store
            .additional_phase_only_steps
            .resize(phase_count, None);
    }

    /// Return the next player in turn order who is still in the game.
    ///
    /// This is the common CR 800.4 routing primitive for priority and rule
    /// choices after a multiplayer participant leaves.
    pub fn next_player_in_game_after(&self, player: PlayerId) -> Option<PlayerId> {
        let len = self.turn_store.turn_order.len();
        if len == 0 {
            return None;
        }
        let current_index = self
            .turn_store
            .turn_order
            .iter()
            .position(|candidate| *candidate == player)
            .unwrap_or(0);
        (1..=len)
            .map(|offset| self.turn_store.turn_order[(current_index + offset) % len])
            .find(|candidate| {
                self.player(*candidate)
                    .is_some_and(|candidate| candidate.is_in_game())
            })
    }

    /// Player who receives priority when a new priority window opens.
    ///
    /// Normally this is the active player. If that player left during their
    /// turn, the turn continues but priority starts with the next player still
    /// in the game (CR 800.4a, 800.4j).
    pub fn priority_recipient_for_new_window(&self) -> Option<PlayerId> {
        if self.grand_melee.is_some() {
            let eligible = self.priority_players_for_current_turn();
            if eligible.contains(&self.turn.active_player) {
                return Some(self.turn.active_player);
            }
            return self
                .turn_store
                .turn_order
                .iter()
                .position(|player| *player == self.turn.active_player)
                .and_then(|index| {
                    (1..=self.turn_store.turn_order.len())
                        .map(|offset| {
                            self.turn_store.turn_order
                                [(index + offset) % self.turn_store.turn_order.len()]
                        })
                        .find(|player| eligible.contains(player))
                });
        }
        if let Some(active_team) = self.active_team_index() {
            return self
                .primary_player_for_team(active_team)
                .or_else(|| self.next_team_turn_representative_after(self.turn.active_player));
        }
        self.player(self.turn.active_player)
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .or_else(|| self.next_player_in_game_after(self.turn.active_player))
    }

    pub fn reset_priority_for_new_window(&mut self) {
        self.turn.priority_player = self.priority_recipient_for_new_window();
    }

    /// Current player information while the player remains in the game, or a
    /// frozen pre-departure snapshot afterward (CR 800.4i).
    pub fn player_last_known_information(&self, player: PlayerId) -> Option<&Player> {
        let current = self.players.get(player.index())?;
        if current.has_left_game {
            self.turn_store
                .departed_player_history
                .get(&player)
                .map(|history| &history.player_lki)
                .or(Some(current))
        } else {
            Some(current)
        }
    }

    /// Full-game committed action/event records involving `player`.
    ///
    /// Unlike turn-scoped history, this remains queryable after the player
    /// leaves and does not expire at their would-be next-turn boundary.
    pub fn action_history_for_player(
        &self,
        player: PlayerId,
    ) -> impl Iterator<Item = &TurnEventRecord> {
        self.turn_store
            .action_history_by_player
            .get(&player)
            .into_iter()
            .flat_map(|records| records.iter())
    }

    /// Actions from a player's most recent turn.
    ///
    /// For departed players this remains available only until their next turn
    /// after leaving would have begun, as required by CR 800.4i.
    pub fn last_turn_history_for_player(&self, player: PlayerId) -> Option<&TurnHistory> {
        if self
            .players
            .get(player.index())
            .is_some_and(|current| current.has_left_game)
        {
            return self
                .turn_store
                .departed_player_history
                .get(&player)
                .filter(|history| self.turn.turn_number < history.last_turn_expires_before_turn)
                .and_then(|history| history.last_turn_history.as_ref());
        }
        self.turn_store.last_turn_history_by_player.get(&player)
    }

    /// Turn number at which `player`'s next non-skipped turn would begin if
    /// they remained in the game. This snapshots the shared CR 800.4i/800.4m
    /// boundary before the leave-game procedure removes their future turns.
    pub(crate) fn next_turn_number_if_player_stayed(&self, player: PlayerId) -> u32 {
        if self.turn_store.turn_order.is_empty() {
            return self.turn.turn_number;
        }

        if let Some(shared) = self.shared_team_turns()
            && let Some(target_team) = self.team_index_for(player)
            && let Some(mut simulated_team) = self.active_team_index()
        {
            let mut simulated_turn_number = self.turn.turn_number;
            let mut simulated_extra_turns = self.turn_store.extra_turns.clone();
            let mut simulated_skip_next_turn = self.turn_store.skip_next_turn.clone();
            let max_iterations = shared
                .team_order()
                .len()
                .saturating_mul(16)
                .saturating_add(simulated_extra_turns.len().saturating_mul(2))
                .saturating_add(16)
                .max(1);

            for _ in 0..max_iterations {
                let mut normal_anchor = simulated_team;
                let candidate_team = loop {
                    let candidate = if let Some(extra_turn) = simulated_extra_turns.pop() {
                        let Some(team) = self.team_index_for(extra_turn) else {
                            continue;
                        };
                        team
                    } else {
                        let current_index = shared
                            .team_order()
                            .iter()
                            .position(|team| *team == normal_anchor)
                            .unwrap_or(0);
                        let team =
                            shared.team_order()[(current_index + 1) % shared.team_order().len()];
                        normal_anchor = team;
                        team
                    };

                    let skipped = simulated_skip_next_turn
                        .iter()
                        .copied()
                        .find(|player| self.team_index_for(*player) == Some(candidate));
                    if let Some(skipped) = skipped {
                        simulated_skip_next_turn.remove(&skipped);
                        continue;
                    }
                    break candidate;
                };

                simulated_turn_number = simulated_turn_number.saturating_add(1);
                simulated_team = candidate_team;
                if candidate_team == target_team {
                    return simulated_turn_number;
                }
            }

            return self.turn.turn_number.saturating_add(1);
        }

        let mut simulated_active = self.turn.active_player;
        let mut simulated_turn_number = self.turn.turn_number;
        let mut simulated_extra_turns = self.turn_store.extra_turns.clone();
        let mut simulated_skip_next_turn = self.turn_store.skip_next_turn.clone();
        let max_iterations = self
            .turn_store
            .turn_order
            .len()
            .saturating_mul(16)
            .saturating_add(simulated_extra_turns.len().saturating_mul(2))
            .saturating_add(16)
            .max(1);

        for _ in 0..max_iterations {
            let current_index = self
                .turn_store
                .turn_order
                .iter()
                .position(|candidate| *candidate == simulated_active)
                .unwrap_or(0);
            let mut normal_index = (current_index + 1) % self.turn_store.turn_order.len();
            let next_player = loop {
                let candidate = if let Some(extra_turn) = simulated_extra_turns.pop() {
                    extra_turn
                } else {
                    let candidate = self.turn_store.turn_order[normal_index];
                    normal_index = (normal_index + 1) % self.turn_store.turn_order.len();
                    candidate
                };
                if !self
                    .player(candidate)
                    .is_some_and(|candidate| candidate.is_in_game())
                {
                    continue;
                }
                if simulated_skip_next_turn.remove(&candidate) {
                    continue;
                }
                break candidate;
            };

            simulated_turn_number = simulated_turn_number.saturating_add(1);
            simulated_active = next_player;
            if next_player == player {
                return simulated_turn_number;
            }
        }

        self.turn.turn_number.saturating_add(1)
    }

    /// Perform the immediate multiplayer leave-game procedure (CR 800.4).
    ///
    /// Owned objects cease to exist without a zone change, control effects end,
    /// noncard stack objects cease to exist, and remaining objects controlled by
    /// the departing player are exiled. Runtime effects, queued choices, combat
    /// state, and future turns that can no longer involve that player are also
    /// pruned in the same atomic procedure.
    pub fn leave_game(&mut self, player: PlayerId) -> bool {
        if self
            .player(player)
            .is_none_or(|candidate| candidate.has_left_game)
        {
            return false;
        }

        let departing_turn_boundary = self.next_turn_number_if_player_stayed(player);
        let departing_team = self.team_index_for(player);
        let was_active_player = self.is_active_player(player);
        let had_priority = self.turn.priority_player == Some(player);
        let priority_team = had_priority.then(|| self.priority_team_index()).flatten();
        let Some(mut player_lki) = self.players.get(player.index()).cloned() else {
            return false;
        };
        player_lki.has_left_game = true;
        let last_turn_history = if was_active_player {
            Some(self.turn_store.turn_history.clone())
        } else {
            self.turn_store
                .last_turn_history_by_player
                .get(&player)
                .cloned()
        };
        self.turn_store.departed_player_history.insert(
            player,
            DepartedPlayerHistory {
                player_lki,
                last_turn_history,
                last_turn_expires_before_turn: departing_turn_boundary,
            },
        );
        if let Some(candidate) = self.player_mut(player) {
            candidate.has_left_game = true;
        }
        self.handle_grand_melee_player_departure(player);
        if was_active_player
            && let Some(team) = departing_team
            && let Some(primary) = self.primary_player_for_team(team)
        {
            self.turn.active_player = primary;
        }

        // Planechase transfers the planar controller, communal ownership, and
        // control of planar-card abilities before CR 800.4a removes objects.
        self.prepare_planechase_player_departure(player);

        // CR 800.4a first removes every object the player owns. This is not a
        // zone change, so remove_object deliberately emits no zone-change event.
        let owned_objects = self
            .objects_map()
            .values()
            // CR 800.4n is an explicit exception to CR 800.4a: ante cards
            // remain in the game when their owner leaves a multiplayer game.
            .filter(|object| object.owner == player && object.zone != Zone::Ante)
            .map(|object| (object.id, object.stable_id))
            .collect::<Vec<_>>();
        let removed_ids = owned_objects
            .iter()
            .map(|(id, _)| *id)
            .collect::<HashSet<_>>();
        let removed_stable_ids = owned_objects
            .iter()
            .map(|(_, stable_id)| *stable_id)
            .collect::<HashSet<_>>();
        // Scheme state needs owner information that CR 800.4a removes below.
        self.handle_archenemy_player_departure(player);
        // CR 702.106e reveals hidden agendas before their owner's objects leave.
        self.handle_conspiracy_player_departure(player);
        for (object_id, _) in &owned_objects {
            self.remove_object(*object_id);
        }
        self.handle_planechase_player_departure(player, &removed_ids);
        self.handle_vanguard_player_departure(player);
        self.prune_grand_melee_stacks_for_departure(player, &removed_ids);

        // End every effect that gives the departing player control. Other
        // resolved continuous effects survive until their ordinary duration;
        // turn-relative ones expire when this player's next turn would have
        // begun (CR 800.4m).
        self.effect_store
            .continuous_effects
            .prepare_for_departing_player(player, departing_turn_boundary.saturating_sub(1));
        self.effect_store
            .grant_registry
            .prepare_for_departing_player(player, departing_turn_boundary.saturating_sub(1));
        self.effect_store
            .delayed_triggers
            .retain(|trigger| trigger.controller != player);
        self.effect_store
            .pending_trigger_entries
            .retain(|trigger| trigger.controller != player);
        self.effect_store
            .active_state_trigger_conditions
            .retain(|key| !removed_stable_ids.contains(&key.source_stable_id));
        self.effect_store
            .granted_mana_abilities
            .retain(|ability| ability.controller != player);
        self.effect_store
            .temporary_spell_cost_reductions
            .retain(|effect| effect.player != player);
        for effect in self
            .effect_store
            .temporary_spell_cost_reductions
            .iter_mut()
            .filter(|effect| {
                effect.duration_controller == player
                    && matches!(
                        effect.duration,
                        Until::YourNextTurn
                            | Until::YourNextTurnEnd
                            | Until::YourNextUpkeep
                            | Until::ControllersNextUntapStep
                    )
            })
        {
            effect.duration = Until::YourNextTurnEnd;
            effect.expires_end_of_turn = departing_turn_boundary.saturating_sub(1);
        }
        self.effect_store
            .temporary_spell_ability_grants
            .retain(|effect| effect.player != player);
        for effect in self
            .effect_store
            .restriction_effects
            .iter_mut()
            .filter(|effect| {
                effect.controller == player
                    && matches!(
                        effect.duration,
                        Until::YourNextTurn
                            | Until::YourNextTurnEnd
                            | Until::YourNextUpkeep
                            | Until::ControllersNextUntapStep
                    )
            })
        {
            effect.duration = Until::YourNextTurnEnd;
            effect.expires_end_of_turn = departing_turn_boundary.saturating_sub(1);
        }
        for effect in self.effect_store.goad_effects.iter_mut().filter(|effect| {
            effect.goaded_by == player
                && matches!(
                    effect.duration,
                    Until::YourNextTurn
                        | Until::YourNextTurnEnd
                        | Until::YourNextUpkeep
                        | Until::ControllersNextUntapStep
                )
        }) {
            effect.duration = Until::YourNextTurnEnd;
            effect.expires_end_of_turn = departing_turn_boundary.saturating_sub(1);
        }
        self.effect_store
            .mana_spend_effects
            .permissions
            .retain(|permission| permission.controller != player);
        let live_replacement_effects = self
            .effect_store
            .replacement_effects
            .effects()
            .iter()
            .map(|effect| effect.id)
            .collect::<HashSet<_>>();
        if let Some(choice) = self.effect_store.pending_replacement_choice.as_mut() {
            choice
                .applicable_effects
                .retain(|effect| live_replacement_effects.contains(effect));
        }
        if self
            .effect_store
            .pending_replacement_choice
            .as_ref()
            .is_some_and(|choice| choice.applicable_effects.is_empty())
        {
            self.effect_store.pending_replacement_choice = None;
        }

        {
            let choices = self.choice_store_mut();
            choices
                .chosen_modes_by_ability
                .retain(|(source, _), _| !removed_ids.contains(source));
            choices
                .chosen_colors
                .retain(|source, _| !removed_ids.contains(source));
            choices
                .chosen_basic_land_types
                .retain(|source, _| !removed_ids.contains(source));
            choices
                .chosen_land_types
                .retain(|source, _| !removed_ids.contains(source));
            choices
                .chosen_creature_types
                .retain(|source, _| !removed_ids.contains(source));
            choices
                .chosen_card_types
                .retain(|source, _| !removed_ids.contains(source));
            choices
                .chosen_players
                .retain(|source, _| !removed_ids.contains(source));
            choices
                .chosen_named_options
                .retain(|source, _| !removed_ids.contains(source));
        }

        {
            let aux = self.auxiliary_tracking_mut();
            aux.player_control_effects
                .retain(|effect| effect.controller != player && effect.target != player);
            aux.scoped_player_control_effects
                .retain(|effect| effect.controller != player && effect.target != player);
            aux.combat_choice_control_effects
                .retain(|effect| effect.controller != player);
        }

        // Rebuild static effects after owned sources leave and control effects
        // end, before checking which remaining objects are still controlled by
        // a player outside the game.
        self.mark_continuous_state_dirty();
        self.refresh_continuous_state();

        // Ability copies and other noncard stack objects controlled by the
        // departing player cease to exist. A remaining card spell they control
        // is exiled in the next step.
        let remaining_stack_objects_controlled = self
            .stack
            .iter()
            .filter(|entry| !entry.is_ability && entry.controller == player)
            .map(|entry| entry.object_id)
            .collect::<HashSet<_>>();
        self.stack.retain(|entry| entry.controller != player);

        // After control effects end, exile remaining battlefield/stack objects
        // whose current controller is no longer in the game (including CR
        // 800.4c's default-controller-already-left case).
        let controlled_by_absent_player = self
            .objects_map()
            .values()
            .filter(|object| matches!(object.zone, Zone::Battlefield | Zone::Stack))
            .filter_map(|object| {
                (remaining_stack_objects_controlled.contains(&object.id)
                    || self
                        .current_controller(object.id)
                        .is_some_and(|controller| {
                            !self
                                .player(controller)
                                .is_some_and(|candidate| candidate.is_in_game())
                        }))
                .then_some(object.id)
            })
            .collect::<Vec<_>>();
        for object_id in controlled_by_absent_player {
            let _ = self.move_object_by_game_rule(object_id, Zone::Exile);
        }
        let stack_object_ids = self
            .objects_map()
            .values()
            .filter(|object| object.zone == Zone::Stack)
            .map(|object| object.id)
            .collect::<HashSet<_>>();
        self.stack
            .retain(|entry| entry.is_ability || stack_object_ids.contains(&entry.object_id));

        // Remove future turn and per-player step state. An active player's turn
        // itself continues without that player (CR 800.4j); only priority moves.
        self.turn_store
            .extra_turns
            .retain(|candidate| *candidate != player);
        self.turn_store.skip_next_turn.remove(&player);
        self.turn_store
            .skipped_steps
            .retain(|(candidate, _), _| *candidate != player);
        self.turn_store.skip_next_combat_phases.remove(&player);
        self.turn_store
            .skip_current_turn_combat_phases
            .remove(&player);
        self.turn_store
            .skip_current_turn_main_phases
            .remove(&player);
        self.turn_store.hand_sizes_at_turn_start.remove(&player);
        self.turn_store
            .combat_damage_assignments
            .retain(|source, assignments| {
                if removed_ids.contains(source) {
                    return false;
                }
                assignments.retain(|recipient, _| !removed_ids.contains(recipient));
                true
            });
        if self.turn_store.tracked_draw_step_player == Some(player) {
            self.turn_store.tracked_draw_step_player = None;
            self.turn_store.cards_drawn_this_draw_step = 0;
        }

        if let Some(combat) = self.combat.as_mut() {
            combat
                .attackers
                .retain(|attacker| !removed_ids.contains(&attacker.creature));
            combat.blockers.retain(|attacker, blockers| {
                if removed_ids.contains(attacker) {
                    return false;
                }
                blockers.retain(|blocker| !removed_ids.contains(blocker));
                true
            });
            combat.damage_assignment_order.retain(|attacker, blockers| {
                if removed_ids.contains(attacker) {
                    return false;
                }
                blockers.retain(|blocker| !removed_ids.contains(blocker));
                true
            });
            combat.attacking_bands.retain_mut(|band| {
                band.retain(|creature| !removed_ids.contains(creature));
                !band.is_empty()
            });
            combat
                .had_to_attack_this_combat
                .retain(|creature| !removed_ids.contains(creature));
        }

        // Rule choices pass to the next player in turn order (800.4h). Ordinary
        // object-controlled choices already name the surviving object's
        // controller; this only repairs a pending affected-player choice.
        let replacement_choice_needs_reroute = self
            .effect_store
            .pending_replacement_choice
            .as_ref()
            .is_some_and(|choice| choice.player == player);
        if replacement_choice_needs_reroute {
            if let Some(next) = self.next_player_in_game_after(player) {
                if let Some(choice) = self.effect_store.pending_replacement_choice.as_mut() {
                    choice.player = next;
                }
            } else {
                self.effect_store.pending_replacement_choice = None;
            }
        }
        if had_priority {
            self.turn.priority_player = priority_team
                .and_then(|team| self.primary_player_for_team(team))
                .or_else(|| self.next_player_in_game_after(player));
        }

        let active_player_still_in_game = self
            .player(self.turn.active_player)
            .filter(|candidate| candidate.is_in_game())
            .map(|candidate| candidate.id);
        if self.monarch == Some(player) {
            let successor = if let Some(active) = active_player_still_in_game {
                self.can_become_monarch(active).then_some(active)
            } else {
                let len = self.turn_store.turn_order.len();
                let start = self
                    .turn_store
                    .turn_order
                    .iter()
                    .position(|candidate| *candidate == player)
                    .unwrap_or(0);
                (1..=len)
                    .map(|offset| self.turn_store.turn_order[(start + offset) % len])
                    .find(|candidate| {
                        self.player(*candidate)
                            .is_some_and(|candidate| candidate.is_in_game())
                            && self.can_become_monarch(*candidate)
                    })
            };
            self.set_monarch(successor);
        }
        if self.initiative == Some(player) {
            let successor =
                active_player_still_in_game.or_else(|| self.next_player_in_game_after(player));
            self.set_initiative(successor);
        }

        self.mark_continuous_state_dirty();
        self.refresh_continuous_state();
        self.synchronize_focused_grand_melee_lane();
        true
    }

    /// Keep a Forecast source publicly revealed while it remains in hand and
    /// the current upkeep continues (CR 702.57b).
    pub fn reveal_hand_card_until_upkeep_ends(&mut self, object_id: ObjectId) -> bool {
        if !self
            .object(object_id)
            .is_some_and(|object| object.zone == Zone::Hand)
        {
            return false;
        }
        self.turn_store
            .forecast_revealed_hand_cards
            .insert(object_id)
    }

    pub fn is_hand_card_revealed_until_upkeep_ends(&self, object_id: ObjectId) -> bool {
        self.turn_store
            .forecast_revealed_hand_cards
            .contains(&object_id)
            && self
                .object(object_id)
                .is_some_and(|object| object.zone == Zone::Hand)
    }

    pub(crate) fn clear_forecast_revealed_hand_cards(&mut self) {
        self.turn_store.forecast_revealed_hand_cards.clear();
    }

    /// Advances to the next turn.
    ///
    /// Turn order rules:
    /// 1. If there are extra turns queued, the most recently created one is considered first
    /// 2. If any candidate turn should be skipped, it is skipped (and removed from the skip list)
    /// 3. Otherwise, proceed to the next player in turn order
    pub fn next_turn(&mut self) {
        if self.grand_melee.is_some() {
            self.next_grand_melee_turn();
            return;
        }
        self.next_turn_single_lane();
    }

    pub(crate) fn next_turn_single_lane(&mut self) {
        let completed_turn_players = self.turn_players();
        let mut normal_anchor = self.turn.active_player;
        let current_index = self
            .turn_store
            .turn_order
            .iter()
            .position(|&player| player == self.turn.active_player)
            .unwrap_or(0);
        let mut normal_index = (current_index + 1) % self.turn_store.turn_order.len();
        let next_player = loop {
            let candidate = if let Some(extra_turn) = self.turn_store.extra_turns.pop() {
                self.team_turn_representative(extra_turn)
            } else if self.shared_team_turns_enabled() {
                let player = self
                    .next_team_turn_representative_after(normal_anchor)
                    .expect("a shared-turn game must retain an in-game team");
                normal_anchor = player;
                player
            } else {
                let player = self.turn_store.turn_order[normal_index];
                normal_index = (normal_index + 1) % self.turn_store.turn_order.len();
                player
            };

            if !self
                .player(candidate)
                .is_some_and(|player| player.is_in_game())
            {
                continue;
            }
            if self.consume_team_turn_skip(candidate) {
                continue;
            }
            break candidate;
        };

        // Reset turn state
        self.turn.active_player = next_player;
        self.turn.priority_player = Some(next_player);
        self.turn.turn_number += 1;
        self.refresh_range_of_influence_snapshot();
        self.turn.phase = Phase::Beginning;
        self.turn.step = Some(Step::Untap);
        self.turn_store.tracked_draw_step_player = None;
        self.turn_store.cards_drawn_this_draw_step = 0;
        self.turn_store.combat_phases_started_this_turn = 0;
        self.turn_store.additional_phases.clear();
        self.turn_store.additional_phase_orders.clear();
        self.turn_store.additional_phase_only_steps.clear();
        self.turn_store.phase_schedule_continuation = None;
        self.turn_store.additional_phase_continuation = None;
        self.turn_store.skip_current_turn_combat_phases.clear();
        self.turn_store.skip_current_turn_main_phases.clear();
        self.turn_store.added_steps.clear();
        self.turn_store.pending_added_steps.clear();
        self.turn_store.active_added_step = None;
        self.turn_store.added_step_continuation = None;
        self.turn_store.no_combat_damage_this_turn.clear();
        self.turn_store.no_combat_damage_this_combat.clear();
        self.clear_forecast_revealed_hand_cards();
        self.set_planar_controller(next_player);
        self.reset_planar_rolls_for_turn();

        for history in self.turn_store.departed_player_history.values_mut() {
            if self.turn.turn_number >= history.last_turn_expires_before_turn {
                history.last_turn_history = None;
            }
        }

        // Clear turn-based tracking
        self.turn_store.entered_battlefield_last_turn = self
            .turn_store
            .turn_history
            .entered_battlefield_snapshots_this_turn();
        let max_spells_cast_by_completed_teammate = completed_turn_players
            .iter()
            .map(|player| self.turn_store.turn_history.spells_cast_by_player(*player))
            .max()
            .unwrap_or(0);
        self.turn_store.spells_cast_last_turn_total =
            self.turn_store.turn_history.total_spells_cast_this_turn();
        let completed_turn_history = std::mem::take(&mut self.turn_store.turn_history);
        for player in completed_turn_players {
            self.turn_store
                .last_turn_history_by_player
                .insert(player, completed_turn_history.clone());
        }
        self.turn_store.previous_turn_history = completed_turn_history;
        let spells_cast_last_turn = self.turn_store.spells_cast_last_turn_total;
        if self.has_day_night && self.is_night {
            if max_spells_cast_by_completed_teammate >= 2 {
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
        for player in self.turn_players() {
            self.activate_pending_player_control(player);
        }

        // Begin the shared turn independently for each active player.
        for player in self.turn_players() {
            if let Some(player) = self.player_mut(player) {
                player.begin_turn();
            }
        }
        self.record_turn_start_hand_sizes();

        // Turn-relative durations can change control exactly at this boundary.
        // Reconcile them before the untap step establishes which permanents
        // have been continuously controlled since this turn began (CR 302.6).
        self.reconcile_continuous_control_changes();
        self.activate_restrictions_starting_this_turn();
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

    /// Players entitled to private information visible to `player` under
    /// CR 722.4. In-game information is shared with that player's controller;
    /// outside-the-game information remains visible only to `player`.
    pub fn private_information_viewers_for(
        &self,
        player: PlayerId,
        zone: crate::zone::Zone,
    ) -> Vec<PlayerId> {
        let controller = self.controlling_player_for(player);
        if zone == crate::zone::Zone::OutsideGame || controller == player {
            return vec![player];
        }
        // Put the rules player last so single-window frontends retain that
        // identity while still publishing an audit/open event for both.
        vec![controller, player]
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
            if !effect.active
                || (effect.target != player
                    && !(self.shared_team_turns_enabled()
                        && self.are_teammates(effect.target, player)))
            {
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
            if effect.target != player
                && !(self.shared_team_turns_enabled() && self.are_teammates(effect.target, player))
            {
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

        let controller = best.map(|(controller, _)| controller).unwrap_or(player);
        if self
            .player(controller)
            .is_some_and(|candidate| candidate.is_in_game())
        {
            controller
        } else {
            // Rule choices that still need a player after the named player has
            // left pass to the next player in turn order (CR 800.4h). Object-
            // controlled choices normally arrive here already attributed to
            // that object's surviving controller (CR 800.4g).
            self.next_player_in_game_after(player).unwrap_or(player)
        }
    }

    /// Activate pending player-control effects for the current active player.
    pub fn activate_pending_player_control(&mut self, active_player: PlayerId) {
        let current_turn = self.turn.turn_number;
        let active_players = if self.shared_team_turns_enabled() {
            self.active_players()
        } else {
            vec![active_player]
        };
        for effect in &mut self.auxiliary_tracking_mut().player_control_effects {
            if effect.active {
                continue;
            }
            if !matches!(effect.start, PlayerControlStart::NextTurn) {
                continue;
            }
            if !active_players.contains(&effect.target) {
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
    /// Kruphix, Omnath) keep the retained portion of their pool. Individual
    /// mana units can also carry a retention duration (for example,
    /// Firebending); those units do not cause unrelated mana of the same color
    /// to persist.
    pub fn empty_mana_pools(&mut self) {
        let ending_combat = matches!(
            (self.turn.phase, self.turn.step),
            (Phase::Combat, Some(Step::EndCombat))
        );
        let ending_turn = matches!(
            (self.turn.phase, self.turn.step),
            (Phase::Ending, Some(Step::Cleanup))
        );
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
            let scopes = scopes.unwrap_or_default();

            // Expiration belongs to the duration itself, even when a separate
            // global retention effect is currently keeping the same mana. If
            // we left the marker attached, an expired Firebending unit could
            // start retaining mana again after the global effect ended.
            if ending_combat || ending_turn {
                for unit in &mut player.mana_source_provenance {
                    if (ending_combat
                        && unit.retention
                            == Some(ironsmith_core::ManaRetentionDuration::EndOfCombat))
                        || (ending_turn
                            && unit.retention
                                == Some(ironsmith_core::ManaRetentionDuration::EndOfTurn))
                    {
                        unit.retention = None;
                    }
                }
            }
            if scopes.contains(&None) {
                continue;
            }

            let globally_retained = |symbol: crate::mana::ManaSymbol| match symbol {
                crate::mana::ManaSymbol::White => {
                    scopes.contains(&Some(crate::color::Color::White))
                }
                crate::mana::ManaSymbol::Blue => scopes.contains(&Some(crate::color::Color::Blue)),
                crate::mana::ManaSymbol::Black => {
                    scopes.contains(&Some(crate::color::Color::Black))
                }
                crate::mana::ManaSymbol::Red => scopes.contains(&Some(crate::color::Color::Red)),
                crate::mana::ManaSymbol::Green => {
                    scopes.contains(&Some(crate::color::Color::Green))
                }
                _ => false,
            };

            let original_pool = player.mana_pool.clone();
            player.mana_source_provenance.retain(|unit| {
                globally_retained(unit.symbol)
                    || match unit.retention {
                        Some(ironsmith_core::ManaRetentionDuration::EndOfCombat) => !ending_combat,
                        Some(ironsmith_core::ManaRetentionDuration::EndOfTurn) => true,
                        None => false,
                    }
            });

            let mut retained_unit_pool = crate::player::ManaPool::default();
            let mut retained_restricted = std::collections::HashMap::new();
            for unit in &player.mana_source_provenance {
                retained_unit_pool.add(unit.symbol, 1);
                if unit.restricted {
                    *retained_restricted
                        .entry((unit.symbol, unit.source))
                        .or_insert(0usize) += 1;
                }
            }

            for symbol in [
                crate::mana::ManaSymbol::White,
                crate::mana::ManaSymbol::Blue,
                crate::mana::ManaSymbol::Black,
                crate::mana::ManaSymbol::Red,
                crate::mana::ManaSymbol::Green,
                crate::mana::ManaSymbol::Colorless,
            ] {
                let retained = if globally_retained(symbol) {
                    original_pool.amount(symbol)
                } else {
                    retained_unit_pool
                        .amount(symbol)
                        .min(original_pool.amount(symbol))
                };
                let current = player.mana_pool.amount(symbol);
                if current > retained {
                    let _ = player.mana_pool.remove(symbol, current - retained);
                }
            }

            player.restricted_mana.retain(|unit| {
                if globally_retained(unit.symbol) {
                    return true;
                }
                let Some(remaining) = retained_restricted.get_mut(&(unit.symbol, unit.source))
                else {
                    return false;
                };
                if *remaining == 0 {
                    return false;
                }
                *remaining -= 1;
                true
            });
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

    /// Return the player entitled to choose `source`'s combat-damage division.
    pub fn combat_damage_assignment_player(&self, source: ObjectId) -> Option<PlayerId> {
        let combat = self.combat.as_ref()?;
        crate::combat_state::combat_damage_assignment_player(self, combat, source)
    }

    /// Record an assignment only when it was submitted by the rules-defined chooser.
    pub fn set_combat_damage_assignment_for_player(
        &mut self,
        assigning_player: PlayerId,
        source: ObjectId,
        recipient: ObjectId,
        amount: u32,
    ) -> Result<(), String> {
        let expected = self
            .combat_damage_assignment_player(source)
            .ok_or_else(|| format!("object {} is not assigning combat damage", source.0))?;
        if assigning_player != expected {
            return Err(format!(
                "player {} cannot assign combat damage for object {}; player {} chooses",
                assigning_player.0, source.0, expected.0
            ));
        }
        self.set_combat_damage_assignment(source, recipient, amount);
        Ok(())
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

    /// Clear combat-damage object hits tracked for the current trigger batch.
    pub fn clear_combat_damage_object_batch_hits(&mut self) {
        self.combat_transients_mut()
            .combat_damage_object_batch_hits
            .clear();
    }

    /// Record a combat-damage object hit for the current trigger batch.
    pub fn record_combat_damage_object_batch_hit(&mut self, source: ObjectId, object: ObjectId) {
        self.combat_transients_mut()
            .combat_damage_object_batch_hits
            .push((source, object));
    }

    /// Return combat-damage object hits already seen in the current trigger batch.
    pub fn combat_damage_object_batch_hits(&self) -> &[(ObjectId, ObjectId)] {
        &self.combat_transients.combat_damage_object_batch_hits
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

    /// Returns the rules-active player. The scheduler deliberately retains a
    /// departed player's id as the progression anchor for the rest of that
    /// turn, but CR 800.4j says the turn then has no active player.
    pub fn active_player_id(&self) -> Option<PlayerId> {
        self.player(self.turn.active_player)
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
    }

    /// Returns the active player.
    pub fn active_player(&self) -> Option<&Player> {
        self.active_player_id().and_then(|id| self.player(id))
    }

    /// Returns a mutable reference to the active player.
    pub fn active_player_mut(&mut self) -> Option<&mut Player> {
        let id = self.active_player_id()?;
        self.player_mut(id)
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
        self.record_grand_melee_stack_provenance(entry.provenance);
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
        if self.turn.turn_number == 1
            && let Some(profile) = self.two_headed_giant()
        {
            return profile.team_index(player_id) == Some(profile.starting_team());
        }
        self.turn.turn_number == 1
            && !self.shared_team_turns_enabled()
            && self.turn.active_player == player_id
            && self.turn_store.turn_order.first().copied() == Some(player_id)
            && self.players.len() == 2
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
        let eligible_player = |player: PlayerId| {
            self.source_is_exempt_from_range(source)
                || self.player_is_within_range(controller, player)
        };
        let opponents = self
            .players
            .iter()
            .filter(|p| {
                p.is_in_game() && self.are_opponents(controller, p.id) && eligible_player(p.id)
            })
            .map(|p| p.id)
            .collect();
        let teammates = self
            .players
            .iter()
            .filter(|p| {
                p.is_in_game() && self.are_teammates(controller, p.id) && eligible_player(p.id)
            })
            .map(|p| p.id)
            .collect();

        let your_commanders = self
            .player(controller)
            .map(|p| p.commanders.clone())
            .unwrap_or_default();

        let mut tagged_objects = std::collections::HashMap::new();
        let mut tagged_players = std::collections::HashMap::new();
        if let Some(initiative_holder) = self.initiative {
            tagged_players.insert(
                crate::tag::TagKey::from(crate::tag::INITIATIVE_HOLDER_TAG),
                vec![initiative_holder],
            );
        }
        if let Some(source_id) = source
            && let Some(source_obj) = self.object(source_id)
        {
            tagged_objects.extend(source_obj.cast_tagged_objects.clone());
            tagged_objects.insert(
                crate::tag::TagKey::from(crate::tag::SOURCE_OBJECT_TAG),
                vec![crate::snapshot::ObjectSnapshot::from_object(
                    source_obj, self,
                )],
            );
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
            source_snapshot: None,
            caster: None,
            active_player: self.active_player_id(),
            opponents,
            teammates,
            players_in_range: self.range_players_for_source(controller, source),
            defending_player: None,
            defending_players: Vec::new(),
            attacking_player: None,
            attacking_players: Vec::new(),
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
                identity = identity.union(self.commander_object_color_identity(obj));
            } else {
                // Commander might have moved zones and have a different ID.
                // Search through all objects for one with matching stable_id
                for obj in self.objects.values() {
                    if obj.stable_id == StableId::from(commander_id) {
                        identity = identity.union(self.commander_object_color_identity(obj));
                        break;
                    }
                }
            }
        }

        identity
    }

    fn commander_object_color_identity(
        &self,
        object: &crate::object::Object,
    ) -> crate::color::ColorSet {
        let mut identity = object.color_identity();
        if let Some(other_face) = self.linked_face_definition_by_name_or_id(
            object.other_face_name.as_deref(),
            object.other_face,
        ) {
            identity = identity.union(other_face.card.color_identity());
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
            crate::ConditionExpr::AttachmentCount {
                attachment, host, ..
            } => {
                Self::filter_reads_tapped_state(attachment)
                    || matches!(
                        host,
                        ironsmith_core::AttachmentConditionHost::Matching(filter)
                            if Self::filter_reads_tapped_state(filter)
                    )
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
            || filter
                .characteristic_relations
                .iter()
                .any(|relation| Self::filter_reads_tapped_state(&relation.comparison))
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
            crate::ConditionExpr::AttachmentCount {
                attachment, host, ..
            } => {
                Self::filter_reads_face_down_state(attachment)
                    || matches!(
                        host,
                        ironsmith_core::AttachmentConditionHost::Matching(filter)
                            if Self::filter_reads_face_down_state(filter)
                    )
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
            || filter
                .characteristic_relations
                .iter()
                .any(|relation| Self::filter_reads_face_down_state(&relation.comparison))
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
            crate::ConditionExpr::AttachmentCount {
                attachment, host, ..
            } => {
                Self::filter_reads_summoning_sickness_state(attachment)
                    || matches!(
                        host,
                        ironsmith_core::AttachmentConditionHost::Matching(filter)
                            if Self::filter_reads_summoning_sickness_state(filter)
                    )
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
                .characteristic_relations
                .iter()
                .any(|relation| Self::filter_reads_summoning_sickness_state(&relation.comparison))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{LinkedFaceLayout, PowerToughness};
    use crate::cards::CardDefinitionBuilder;
    use crate::color::Color;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn commander_color_identity_unions_both_linked_faces() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let front_id = CardId::from_raw(90_001);
        let back_id = CardId::from_raw(90_002);

        let front = CardDefinitionBuilder::new(front_id, "Front Commander")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::White]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .other_face(back_id)
            .other_face_name("Back Commander")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        let back = CardDefinitionBuilder::new(back_id, "Back Commander")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .other_face(front_id)
            .other_face_name("Front Commander")
            .linked_face_layout(LinkedFaceLayout::TransformLike)
            .build();
        game.register_linked_face_definition(&front);
        game.register_linked_face_definition(&back);

        let commander = game.create_object_from_definition(&front, alice, Zone::Command);
        game.player_mut(alice)
            .expect("Alice should exist")
            .add_commander(commander);

        let identity = game.get_commander_color_identity(alice);
        assert!(identity.contains(Color::White));
        assert!(identity.contains(Color::Black));
        assert_eq!(identity.count(), 2);
    }
}
