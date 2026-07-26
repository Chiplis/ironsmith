#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

fn public_identity_after_hidden_position_reveal(
    info: &ironsmith::game_state::HiddenCardInfo,
    input_position: u16,
    input_position_commitment: Option<&str>,
) -> (Option<u16>, Option<String>) {
    let previous_public_slot = info.public_slot.unwrap_or(info.slot);
    let previous_public_commitment = info
        .public_commitment
        .clone()
        .unwrap_or_else(|| info.commitment.clone());
    let incoming_public_commitment = input_position_commitment
        .map(str::to_string)
        .unwrap_or_else(|| info.commitment.clone());
    let has_existing_public_identity =
        info.public_slot.is_some() || info.public_commitment.is_some();
    let preserve_existing_public_identity = has_existing_public_identity
        && (previous_public_slot != input_position
            || previous_public_commitment != incoming_public_commitment);

    if preserve_existing_public_identity {
        (Some(previous_public_slot), Some(previous_public_commitment))
    } else {
        (Some(input_position), Some(incoming_public_commitment))
    }
}

fn hidden_position_continuation_target<'a, I>(
    hidden_cards: I,
    owner: ironsmith::ids::PlayerId,
    position: u16,
    position_commitment: Option<&str>,
) -> Option<(ironsmith::ids::ObjectId, ironsmith::zone::Zone)>
where
    I: IntoIterator<
        Item = (
            &'a ironsmith::ids::ObjectId,
            &'a ironsmith::game_state::HiddenCardInfo,
        ),
    >,
{
    let commitment = position_commitment.filter(|value| !value.is_empty());
    hidden_cards.into_iter().find_map(|(object_id, info)| {
        if info.owner != owner {
            return None;
        }
        if let Some(commitment) = commitment {
            let matches_commitment = info.commitment == commitment
                || info.public_commitment.as_deref() == Some(commitment);
            if !matches_commitment {
                return None;
            }
        }
        let matches_position = info.public_slot == Some(position)
            || (info.public_slot.is_none() && info.slot == position)
            || commitment.is_some_and(|value| info.public_commitment.as_deref() == Some(value));
        matches_position.then_some((*object_id, info.zone))
    })
}

fn hidden_position_reveal_commitment_matches(
    info: &ironsmith::game_state::HiddenCardInfo,
    position_commitment: Option<&str>,
) -> bool {
    position_commitment.is_none_or(|commitment| {
        info.commitment == commitment || info.public_commitment.as_deref() == Some(commitment)
    })
}

fn hidden_position_reveal_position_matches(
    info: &ironsmith::game_state::HiddenCardInfo,
    position: u16,
    position_commitment: Option<&str>,
) -> bool {
    if let Some(commitment) = position_commitment {
        let matches_commitment =
            info.commitment == commitment || info.public_commitment.as_deref() == Some(commitment);
        let matches_position_number = info.slot == position || info.public_slot == Some(position);
        return matches_commitment
            && (matches_position_number || info.public_commitment.as_deref() == Some(commitment));
    }
    info.slot == position || info.public_slot == Some(position)
}

fn replay_can_apply_legend_rule_choice_live(root: &ReplayRoot, ctx: &DecisionContext) -> bool {
    let ReplayRoot::Advance = root else {
        return false;
    };
    let DecisionContext::SelectObjects(objects) = ctx else {
        return false;
    };
    objects.source.is_none()
        && objects.min == 1
        && objects.max == Some(1)
        && !objects.allow_partial_completion
        && objects
            .description
            .to_ascii_lowercase()
            .contains("legend rule")
}

fn zone_from_ui_name(zone_name: &str) -> Result<Zone, String> {
    match zone_name.trim().to_lowercase().as_str() {
        "hand" => Ok(Zone::Hand),
        "battlefield" => Ok(Zone::Battlefield),
        "graveyard" => Ok(Zone::Graveyard),
        "exile" => Ok(Zone::Exile),
        "library" => Ok(Zone::Library),
        "command" => Ok(Zone::Command),
        "sideboard" | "outside_game" | "outside the game" => Ok(Zone::OutsideGame),
        other => Err(format!("unknown zone: {other}")),
    }
}

#[derive(Debug, Clone)]
struct ValidatedHiddenPositionReveal {
    input: RevealHiddenPositionInput,
    owner: PlayerId,
    object_id: ObjectId,
    updated_info: ironsmith::game_state::HiddenCardInfo,
    definition: CardDefinition,
    object_already_revealed: bool,
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;

    #[test]
    fn position_reveal_preserves_existing_public_hidden_identity() {
        let info = ironsmith::game_state::HiddenCardInfo {
            owner: ironsmith::ids::PlayerId::from_index(0),
            zone: ironsmith::zone::Zone::Hand,
            slot: 10,
            commitment: "ziffle:initial-deck:10".to_string(),
            public_slot: Some(51),
            public_commitment: Some("ziffle:shuffle-deck:51".to_string()),
        };

        let (public_slot, public_commitment) =
            public_identity_after_hidden_position_reveal(&info, 10, Some("ziffle:initial-deck:10"));

        assert_eq!(public_slot, Some(51));
        assert_eq!(public_commitment.as_deref(), Some("ziffle:shuffle-deck:51"));
    }

    #[test]
    fn position_reveal_sets_public_identity_when_none_exists() {
        let info = ironsmith::game_state::HiddenCardInfo {
            owner: ironsmith::ids::PlayerId::from_index(0),
            zone: ironsmith::zone::Zone::Hand,
            slot: 10,
            commitment: "ziffle:initial-deck:10".to_string(),
            public_slot: None,
            public_commitment: None,
        };

        let (public_slot, public_commitment) =
            public_identity_after_hidden_position_reveal(&info, 10, Some("ziffle:initial-deck:10"));

        assert_eq!(public_slot, Some(10));
        assert_eq!(public_commitment.as_deref(), Some("ziffle:initial-deck:10"));
    }

    #[test]
    fn continuation_position_reveal_ignores_original_slot_collision() {
        let owner = ironsmith::ids::PlayerId::from_index(0);
        let original_slot_object = ironsmith::ids::ObjectId::from_raw(10);
        let position_object = ironsmith::ids::ObjectId::from_raw(20);
        let hidden_cards = std::collections::HashMap::from([
            (
                original_slot_object,
                ironsmith::game_state::HiddenCardInfo {
                    owner,
                    zone: ironsmith::zone::Zone::Hand,
                    slot: 13,
                    commitment: "slot-13-private".to_string(),
                    public_slot: None,
                    public_commitment: None,
                },
            ),
            (
                position_object,
                ironsmith::game_state::HiddenCardInfo {
                    owner,
                    zone: ironsmith::zone::Zone::Library,
                    slot: 6,
                    commitment: "slot-6-private".to_string(),
                    public_slot: Some(24),
                    public_commitment: Some("ziffle:deck:24".to_string()),
                },
            ),
        ]);

        let target = hidden_position_continuation_target(
            hidden_cards.iter(),
            owner,
            24,
            Some("ziffle:deck:24"),
        );

        assert_eq!(
            target,
            Some((position_object, ironsmith::zone::Zone::Library))
        );
    }

    #[test]
    fn replay_advance_legend_rule_prompt_is_live_applicable() {
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

        let alice = ironsmith::ids::PlayerId::from_index(0);
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        wasm.game.turn.step = None;

        let legend = ironsmith::card::CardBuilder::new(
            ironsmith::ids::CardId::from_raw(90_300),
            "Scale Probe Relic",
        )
        .supertypes(vec![ironsmith::types::Supertype::Legendary])
        .card_types(vec![ironsmith::types::CardType::Artifact])
        .build();
        let keep_id =
            wasm.game
                .create_object_from_card(&legend, alice, ironsmith::zone::Zone::Battlefield);
        wasm.game
            .create_object_from_card(&legend, alice, ironsmith::zone::Zone::Battlefield);

        let checkpoint = wasm.capture_replay_checkpoint();
        let outcome = wasm
            .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
            .expect("auto-advance should reach the legend-rule prompt");
        let legend_ctx = match outcome {
            ReplayOutcome::NeedsDecision(DecisionContext::SelectObjects(ctx)) => ctx,
            other => panic!("expected legend-rule select_objects prompt, got {other:?}"),
        };
        assert!(
            legend_ctx.description.contains("legend rule"),
            "prompt should be the legend-rule decision"
        );
        assert!(
            legend_ctx
                .candidates
                .iter()
                .any(|candidate| candidate.id == keep_id),
            "the selected legend should be a legal candidate"
        );
        assert!(
            replay_can_apply_legend_rule_choice_live(
                &ReplayRoot::Advance,
                &DecisionContext::SelectObjects(legend_ctx.clone()),
            ),
            "advance-sourced legend-rule object prompts can be applied to the live paused state"
        );
        assert!(
            !replay_can_apply_legend_rule_choice_live(
                &ReplayRoot::Response(PriorityResponse::PriorityAction(LegalAction::PassPriority,)),
                &DecisionContext::SelectObjects(legend_ctx),
            ),
            "non-advance replay roots still use root reexecution"
        );
    }
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = selectGrandMeleeStack)]
    pub fn select_grand_melee_stack(
        &mut self,
        player_index: u8,
        marker: u32,
    ) -> Result<JsValue, JsValue> {
        self.select_grand_melee_stack_lane(player_index, marker)?;
        self.snapshot()
    }

    fn snapshot_state_shape_hash(&self) -> u64 {
        hash_debug_value(&(
            self.game.players.len(),
            self.game
                .players
                .iter()
                .map(|p| (&p.name, p.life))
                .collect::<Vec<_>>(),
            self.game.turn.turn_number,
            self.game.turn.active_player,
            self.game.turn.phase,
            self.game.turn.step,
            self.game.object_ids_in_deterministic_order().len(),
            self.game.stack.len(),
        ))
    }

    fn snapshot_cache_key(
        &self,
        pending_cast_stack_id: Option<ObjectId>,
        cancelable: bool,
        undo_land_stable_id: Option<u64>,
        mana_payment_view: &Option<ManaPaymentView>,
    ) -> SnapshotCacheKey {
        SnapshotCacheKey {
            mutation_revision: self.game.mutation_revision(),
            state_shape_hash: self.snapshot_state_shape_hash(),
            zone_revision: self.game.zone_revisions().all,
            perspective: self.perspective,
            pending_decision_hash: hash_debug_value(&self.pending_decision),
            mana_payment_hash: hash_debug_value(mana_payment_view),
            game_over_hash: hash_debug_value(&self.game_over),
            pending_cast_stack_id,
            active_resolving_stack_hash: hash_debug_value(&self.active_resolving_stack_object),
            active_viewed_cards_hash: hash_debug_value(&self.active_viewed_cards),
            crypto_requirements_hash: hash_debug_value(&self.last_crypto_requirements),
            cancelable,
            undo_land_stable_id,
        }
    }

    fn finish_dispatch_with_snapshot(
        &mut self,
        started_at: PerfTimer,
        mut perf: DispatchPerfMetrics,
    ) -> Result<JsValue, JsValue> {
        let snapshot = self.snapshot();
        perf.snapshot = self.last_snapshot_perf.clone();
        perf.total_dispatch_ms = started_at.elapsed_ms();
        self.last_dispatch_perf = Some(perf);
        snapshot
    }

    fn store_dispatch_perf(&mut self, started_at: PerfTimer, mut perf: DispatchPerfMetrics) {
        perf.total_dispatch_ms = started_at.elapsed_ms();
        self.last_dispatch_perf = Some(perf);
    }

    fn current_mana_payment_view(&self) -> Option<ManaPaymentView> {
        if let Some(pending) = self.priority_state.pending_cast.as_ref()
            && let Some(view) = mana_payment_view_from_pending_cast(&self.game, pending)
        {
            return Some(view);
        }

        if let Some(pending) = self.priority_state.pending_activation.as_ref()
            && let Some(view) = mana_payment_view_from_pending_activation(pending)
        {
            return Some(view);
        }

        None
    }

    fn pending_priority_decision_is_stale(&self) -> bool {
        if self.pregame.is_some() {
            return false;
        }
        if let Some(DecisionContext::Priority(priority)) = self.pending_decision.as_ref()
            && self.game.turn.priority_player != Some(priority.player)
        {
            return true;
        }
        false
    }

    fn recompute_stale_priority_decision(&mut self) -> Result<(), JsValue> {
        if self.pending_priority_decision_is_stale() {
            self.rebuild_stale_priority_decision();
        }
        Ok(())
    }

    fn rebuild_stale_priority_decision(&mut self) -> bool {
        if !self.pending_priority_decision_is_stale() {
            return false;
        }
        let Some(priority_player) = self.game.turn.priority_player else {
            self.pending_decision = None;
            return true;
        };
        self.pending_decision = Some(DecisionContext::Priority(
            ironsmith::decisions::context::PriorityContext::new(
                priority_player,
                ironsmith::decision::compute_legal_actions(&self.game, priority_player),
            ),
        ));
        self.runner_pending_decision = false;
        true
    }

    fn should_preserve_decision_after_hidden_reveal(&self) -> bool {
        if self.pending_priority_decision_is_stale() {
            return false;
        }
        self.pending_live_continuation.is_some()
            || self.pending_replay_action.is_some()
            || self.runner_pending_decision
            || self.active_viewed_cards.is_some()
            || self.pending_decision.is_some()
    }

    fn reveal_hidden_card_in_live_continuation_checkpoint(
        &mut self,
        owner: PlayerId,
        slots: &[u16],
        commitments: &[String],
        definition: &CardDefinition,
    ) {
        let Some(continuation) = self.pending_live_continuation.as_mut() else {
            return;
        };
        // Card definitions may be loaded lazily while a live replay is suspended.
        // Keep their allocator progress outside the rollback boundary so the next
        // lazy definition cannot reuse a CardId already cached by the game.
        continuation.checkpoint.id_counters.card = snapshot_id_counters().card;
        continuation.speculative_progress = None;
        let target =
            continuation
                .checkpoint
                .game
                .hidden_card_entries()
                .find_map(|(object_id, info)| {
                    if info.owner != owner {
                        return None;
                    }
                    let slot_matches = slots.iter().any(|slot| {
                        info.slot == *slot
                            || info
                                .public_slot
                                .is_some_and(|public_slot| public_slot == *slot)
                    });
                    if !slot_matches {
                        return None;
                    }
                    let commitment_matches =
                        commitments.iter().all(|commitment| commitment.is_empty())
                            || commitments
                                .iter()
                                .filter(|commitment| !commitment.is_empty())
                                .any(|commitment| {
                                    info.commitment == *commitment
                                        || info.public_commitment.as_deref()
                                            == Some(commitment.as_str())
                                });
                    commitment_matches.then_some(*object_id)
                });
        if let Some(object_id) = target {
            continuation
                .checkpoint
                .game
                .register_linked_face_family_from_catalog(definition, &self.registry);
            let _ = continuation
                .checkpoint
                .game
                .reveal_hidden_card_with_definition(object_id, definition);
        }
    }

    fn reveal_hidden_position_in_live_continuation_checkpoint(
        &mut self,
        owner: PlayerId,
        position: u16,
        position_commitment: Option<&str>,
        updated_info: ironsmith::game_state::HiddenCardInfo,
        definition: &CardDefinition,
    ) {
        let Some(continuation) = self.pending_live_continuation.as_mut() else {
            return;
        };
        continuation.checkpoint.id_counters.card = snapshot_id_counters().card;
        continuation.speculative_progress = None;
        let target = hidden_position_continuation_target(
            continuation.checkpoint.game.hidden_card_entries(),
            owner,
            position,
            position_commitment,
        );
        if let Some((object_id, zone)) = target {
            let mut continuation_info = updated_info;
            continuation_info.zone = zone;
            continuation
                .checkpoint
                .game
                .set_hidden_card_info(object_id, continuation_info);
            continuation
                .checkpoint
                .game
                .register_linked_face_family_from_catalog(definition, &self.registry);
            let _ = continuation
                .checkpoint
                .game
                .reveal_hidden_card_with_definition(object_id, definition);
        }
    }

    fn finish_hidden_card_reveal(&mut self, recompute_decision: bool) -> Result<JsValue, JsValue> {
        self.last_crypto_requirements.clear();
        self.pending_crypto_audit_before = None;
        // A staged cast/activation owns the pending decision: recomputing here
        // would discard the chain prompt and advance to a fresh priority
        // decision while priority_state still holds the staged action, so the
        // chain's remaining decision commands (synced from the actor) would no
        // longer match the pending decision and the peer would flag a cheat.
        let mid_action_chain = self.priority_state.pending_activation.is_some()
            || self.priority_state.pending_cast.is_some()
            || self.pending_live_continuation.is_some();
        let recompute_decision = recompute_decision && !mid_action_chain;
        if !recompute_decision && self.rebuild_stale_priority_decision() {
            return self.snapshot();
        }
        let preserve_decision =
            !recompute_decision && self.should_preserve_decision_after_hidden_reveal();
        if preserve_decision {
            if self.pending_live_continuation.is_some() {
                return self.refresh_live_continuation_after_hidden_reveal();
            }
            return self.snapshot();
        }
        self.recompute_ui_decision()?;
        self.snapshot()
    }

    fn pending_trigger_stack_objects(&self) -> Vec<StackObjectSnapshot> {
        if self.priority_state.pending_cast.is_some()
            || self.priority_state.pending_activation.is_some()
        {
            return Vec::new();
        }

        let Some(ironsmith::decisions::context::DecisionContext::Targets(ctx)) =
            self.pending_decision.as_ref()
        else {
            return Vec::new();
        };

        let has_matching_target_prompt = self.trigger_queue.entries.iter().any(|trigger| {
            trigger.source == ctx.source
                && trigger.controller == ctx.player
                && !trigger.ability.choices.is_empty()
                && trigger.ability.choices.len() == ctx.requirements.len()
        });
        if !has_matching_target_prompt {
            return Vec::new();
        }

        self.trigger_queue
            .entries
            .iter()
            .enumerate()
            .map(|(index, trigger)| {
                let mut entry = StackEntry::ability(
                    trigger.source,
                    trigger.controller,
                    trigger.ability.effects.clone(),
                )
                .with_source_info(trigger.source_stable_id, trigger.source_name.clone())
                .with_triggering_event(trigger.triggering_event.clone())
                .with_tagged_objects(trigger.tagged_objects.clone())
                .with_provenance(trigger.triggering_event.provenance());
                if let Some(snapshot) = trigger.source_snapshot.clone() {
                    entry = entry.with_source_snapshot(snapshot);
                }
                if let Some(x_value) = trigger.x_value {
                    entry.x_value = Some(x_value);
                }
                if let Some(intervening_if) = trigger.ability.intervening_if.clone() {
                    entry = entry.with_intervening_if(intervening_if);
                }

                let mut snapshot = build_stack_object_snapshot(
                    &self.game,
                    self.perspective,
                    self.active_viewed_cards.as_ref(),
                    &entry,
                );
                snapshot.id = pending_stack_preview_id(index);
                snapshot
            })
            .collect()
    }

    /// Construct a demo game with two players.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut priority_state = PriorityLoopState::new(2);
        priority_state.set_auto_choose_single_pip_payment(false);
        Self {
            game: GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20),
            registry: CardRegistry::new(),
            trigger_queue: TriggerQueue::new(),
            priority_state,
            pregame: None,
            match_format: MatchFormatInput::Normal,
            pending_decision: None,
            pending_replay_action: None,
            pending_action_checkpoint: None,
            pending_live_action_root: None,
            pending_live_continuation: None,
            game_over: None,
            perspective: PlayerId::from_index(0),
            runner: None,
            grand_melee_host_lanes: HashMap::new(),
            suspended_subgame_hosts: Vec::new(),
            runner_awaiting_priority: false,
            runner_pending_decision: false,
            auto_cleanup_discard: true,
            priority_epoch_checkpoint: None,
            priority_epoch_has_undoable_action: false,
            priority_epoch_undo_locked_by_mana: false,
            priority_epoch_undo_land_stable_id: None,
            semantic_threshold: 0.0,
            snapshot_serial: 0,
            active_viewed_cards: None,
            active_audit_viewed_cards: Vec::new(),
            last_crypto_requirements: Vec::new(),
            pending_crypto_audit_before: None,
            active_resolving_stack_object: None,
            loaded_decks: Vec::new(),
            external_parse_sources: HashMap::new(),
            external_compile_errors: HashMap::new(),
            external_semantic_scores: HashMap::new(),
            last_snapshot_perf: None,
            last_replay_execution_perf: None,
            last_advance_until_decision_perf: None,
            last_dispatch_perf: None,
            snapshot_object_view_cache: SnapshotObjectViewCache::default(),
            #[cfg(target_arch = "wasm32")]
            snapshot_js_encoding_cache: SnapshotJsEncodingCache::default(),
            manabrew_game_id: "ironsmith-uninitialized".to_string(),
            manabrew_human_players: vec![true, true],
            manabrew_next_prompt_id: 1,
            manabrew_open_prompt: None,
            cached_snapshot: None,
        }
    }

    #[wasm_bindgen(js_name = setAutoChooseSingleObjectDecisions)]
    pub fn set_auto_choose_single_object_decisions(&mut self, enabled: bool) {
        self.game.set_auto_choose_single_object_decisions(enabled);
    }

    /// Enable the CR 801 multiplayer option in current player-seat order.
    #[wasm_bindgen(js_name = setLimitedRangeOfInfluence)]
    pub fn set_limited_range_of_influence(&mut self, ranges: JsValue) -> Result<(), JsValue> {
        let ranges: Vec<u8> = serde_wasm_bindgen::from_value(ranges)
            .map_err(|error| JsValue::from_str(&format!("invalid ranges: {error}")))?;
        let seats = self.game.players.iter().map(|player| player.id).collect();
        self.game
            .enable_limited_range_of_influence(seats, ranges)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Enable or disable the CR 803 attack-left/attack-right option.
    #[wasm_bindgen(js_name = setAttackDirection)]
    pub fn set_attack_direction(&mut self, direction: Option<String>) -> Result<(), JsValue> {
        let direction = match direction.as_deref() {
            None | Some("") => None,
            Some("left") => Some(ironsmith::game_state::AttackDirection::Left),
            Some("right") => Some(ironsmith::game_state::AttackDirection::Right),
            Some(other) => {
                return Err(JsValue::from_str(&format!(
                    "invalid attack direction '{other}'; expected 'left', 'right', or null"
                )));
            }
        };
        self.game.set_attack_direction(direction);
        Ok(())
    }

    /// Configure explicit multiplayer teams as arrays of player indices.
    #[wasm_bindgen(js_name = setTeams)]
    pub fn set_teams(&mut self, teams: JsValue) -> Result<(), JsValue> {
        let teams: Vec<Vec<u8>> = serde_wasm_bindgen::from_value(teams)
            .map_err(|error| JsValue::from_str(&format!("invalid teams: {error}")))?;
        self.game
            .set_teams(
                teams
                    .into_iter()
                    .map(|team| team.into_iter().map(PlayerId::from_index).collect())
                    .collect(),
            )
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Enable or disable the CR 804 deploy-creatures option.
    #[wasm_bindgen(js_name = setDeployCreatures)]
    pub fn set_deploy_creatures(&mut self, enabled: bool) {
        self.game.set_deploy_creatures(enabled);
    }

    /// Enable or disable the CR 805 shared-team-turns option.
    #[wasm_bindgen(js_name = setSharedTeamTurns)]
    pub fn set_shared_team_turns(&mut self, enabled: bool) -> Result<(), JsValue> {
        if enabled {
            self.game
                .enable_shared_team_turns()
                .map_err(|error| JsValue::from_str(&error))
        } else {
            self.game.disable_shared_team_turns();
            Ok(())
        }
    }

    /// Record one team's chosen within-team order for CR 805 simultaneous
    /// choices, actions, and trigger placement.
    #[wasm_bindgen(js_name = setSharedTeamMemberOrder)]
    pub fn set_shared_team_member_order(
        &mut self,
        team: usize,
        order: JsValue,
    ) -> Result<(), JsValue> {
        let order: Vec<u8> = serde_wasm_bindgen::from_value(order)
            .map_err(|error| JsValue::from_str(&format!("invalid team member order: {error}")))?;
        self.game
            .set_shared_team_member_order(
                team,
                order.into_iter().map(PlayerId::from_index).collect(),
            )
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Reset game with custom player names and starting life.
    #[wasm_bindgen(js_name = reset)]
    pub fn reset_from_js(
        &mut self,
        player_names: JsValue,
        starting_life: i32,
    ) -> Result<(), JsValue> {
        let names: Vec<String> = serde_wasm_bindgen::from_value(player_names)
            .map_err(|e| JsValue::from_str(&format!("invalid player_names: {e}")))?;

        if names.is_empty() {
            return Err(JsValue::from_str("player_names cannot be empty"));
        }

        let seed = deterministic_match_seed(
            &names,
            starting_life,
            MatchFormatInput::Normal,
            None,
            None,
            7,
        );
        self.initialize_empty_match(names, starting_life, seed);
        self.populate_demo_libraries()
            .map_err(|error| JsValue::from_str(&error))?;
        self.finish_match_setup(7)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Prepare an empty match for puzzle/board-position zone imports.
    #[wasm_bindgen(js_name = resetEmpty)]
    pub fn reset_empty_from_js(
        &mut self,
        player_names: JsValue,
        starting_life: i32,
    ) -> Result<(), JsValue> {
        let names: Vec<String> = serde_wasm_bindgen::from_value(player_names)
            .map_err(|e| JsValue::from_str(&format!("invalid player_names: {e}")))?;

        if names.is_empty() {
            return Err(JsValue::from_str("player_names cannot be empty"));
        }

        let seed = deterministic_match_seed(
            &names,
            starting_life,
            MatchFormatInput::Normal,
            None,
            None,
            0,
        );
        self.initialize_empty_match(names, starting_life, seed);
        self.reset_runtime_state();
        Ok(())
    }

    /// Finish a puzzle import after all requested zones have been populated.
    #[wasm_bindgen(js_name = finishPuzzleSetup)]
    pub fn finish_puzzle_setup(&mut self) -> Result<(), JsValue> {
        self.finish_match_setup(0)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Start a fully specified match from a synchronized lobby payload.
    #[wasm_bindgen(js_name = startMatch)]
    pub fn start_match(&mut self, config: JsValue) -> Result<JsValue, JsValue> {
        let companion_selections: CompanionSelectionsInput =
            serde_wasm_bindgen::from_value(config.clone()).map_err(|e| {
                JsValue::from_str(&format!("invalid companion match config: {e}"))
            })?;
        let config: MatchSetupInput = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("invalid match config: {e}")))?;
        self.apply_match_setup_with_companions(config, companion_selections.companions)?;
        self.snapshot()
    }

    #[cfg(test)]
    fn apply_match_setup(&mut self, config: MatchSetupInput) -> Result<(), JsValue> {
        self.apply_match_setup_with_companions(config, None)
    }

    fn apply_match_setup_with_companions(
        &mut self,
        config: MatchSetupInput,
        companions: Option<Vec<Option<String>>>,
    ) -> Result<(), JsValue> {
        self.apply_match_setup_with_companions_native(config, companions)
            .map_err(|error| JsValue::from_str(&error))
    }

    fn apply_match_setup_with_companions_native(
        &mut self,
        config: MatchSetupInput,
        companions: Option<Vec<Option<String>>>,
    ) -> Result<(), String> {
        if config.player_names.is_empty() {
            return Err("player_names cannot be empty".to_string());
        }

        let player_count = config.player_names.len();
        let opening_hand_size = config
            .format
            .effective_opening_hand_size(config.opening_hand_size);
        let starting_life = config
            .format
            .effective_starting_life(player_count, config.starting_life);
        let hidden_manifests = config.hidden_deck_manifests.as_deref().unwrap_or(&[]);
        config
            .validate_multiplayer_profile()
            ?;
        let prepared_companions = self
            .validate_companion_setup(&config, companions.as_deref())
            ?;

        if matches!(
            config.format,
            MatchFormatInput::Normal
                | MatchFormatInput::FreeForAll
                | MatchFormatInput::GrandMelee
                | MatchFormatInput::TeamVsTeam
                | MatchFormatInput::Emperor
                | MatchFormatInput::TwoHeadedGiant
                | MatchFormatInput::AlternatingTeams
                | MatchFormatInput::Ante
                | MatchFormatInput::Planechase
                | MatchFormatInput::Vanguard
                | MatchFormatInput::Archenemy
                | MatchFormatInput::SupervillainRumble
        ) {
            if config
                .commanders
                .as_ref()
                .is_some_and(|commanders| commanders.iter().any(|list| !list.is_empty()))
            {
                return Err("normal constructed matches cannot designate commanders".to_string());
            }
            Self::validate_ante_manifest_visibility(config.format, hidden_manifests)
                ?;
            self.validate_normal_constructed_setup(
                player_count,
                config.decks.as_deref(),
                config.sideboards.as_deref(),
                hidden_manifests,
            )
            ?;
            self.validate_ante_card_legality_for_setup(
                config.decks.as_deref(),
                config.sideboards.as_deref(),
                config.format == MatchFormatInput::Ante,
            )
            ?;
        }

        if config.format == MatchFormatInput::ConspiracyDraft {
            if config
                .commanders
                .as_ref()
                .is_some_and(|commanders| commanders.iter().any(|list| !list.is_empty()))
            {
                return Err("Conspiracy Draft games cannot designate commanders".to_string());
            }
            self.validate_conspiracy_limited_setup(
                player_count,
                config.decks.as_deref(),
                config.sideboards.as_deref(),
                hidden_manifests,
            )
            ?;
            self.validate_ante_card_legality_for_setup(
                config.decks.as_deref(),
                config.sideboards.as_deref(),
                false,
            )
            ?;
        }

        let prepared_planar_decks = match config.format {
            MatchFormatInput::Planechase => Some(
                self.load_planar_decks_for_setup(
                    config.planar_decks.as_deref().ok_or_else(|| {
                        "Planechase matches require planar decks".to_string()
                    })?,
                    player_count,
                )
                ?,
            ),
            MatchFormatInput::GrandMelee
                if config
                    .planar_decks
                    .as_ref()
                    .is_some_and(|decks| !decks.is_empty()) =>
            {
                Some(
                    self.load_planar_decks_for_setup(
                        config.planar_decks.as_deref().expect("checked nonempty"),
                        player_count,
                    )
                    ?,
                )
            }
            _ => {
                if config
                    .planar_decks
                    .as_ref()
                    .is_some_and(|decks| !decks.is_empty())
                {
                    return Err("planar decks may be supplied only for a Planechase match".to_string());
                }
                None
            }
        };

        let prepared_vanguards = match config.format {
            MatchFormatInput::Vanguard => Some(
                self.load_vanguards_for_setup(
                    config.vanguards.as_deref().ok_or_else(|| {
                        "Vanguard matches require vanguard cards".to_string()
                    })?,
                    player_count,
                )
                ?,
            ),
            _ => {
                if config
                    .vanguards
                    .as_ref()
                    .is_some_and(|cards| !cards.is_empty())
                {
                    return Err("vanguard cards may be supplied only for a Vanguard match".to_string());
                }
                None
            }
        };

        let prepared_scheme_decks = match config.format {
            MatchFormatInput::Archenemy => Some(
                self.load_scheme_decks_for_setup(
                    config.scheme_decks.as_deref().ok_or_else(|| {
                        "Archenemy matches require scheme decks".to_string()
                    })?,
                    player_count,
                    ironsmith::game_state::ArchenemyVariant::Default,
                )
                ?,
            ),
            MatchFormatInput::SupervillainRumble => Some(
                self.load_scheme_decks_for_setup(
                    config.scheme_decks.as_deref().ok_or_else(|| {
                        "Supervillain Rumble matches require scheme decks".to_string()
                    })?,
                    player_count,
                    ironsmith::game_state::ArchenemyVariant::SupervillainRumble,
                )
                ?,
            ),
            MatchFormatInput::ArchenemyCommander => Some(
                self.load_scheme_decks_for_setup(
                    config.scheme_decks.as_deref().ok_or_else(|| {
                        "Archenemy Commander matches require scheme decks".to_string()
                    })?,
                    player_count,
                    ironsmith::game_state::ArchenemyVariant::Commander,
                )
                ?,
            ),
            _ => {
                if config
                    .scheme_decks
                    .as_ref()
                    .is_some_and(|decks| decks.iter().any(|deck| !deck.is_empty()))
                {
                    return Err("scheme decks may be supplied only for an Archenemy match".to_string());
                }
                None
            }
        };

        let prepared_conspiracies = match config.format {
            MatchFormatInput::ConspiracyDraft => Some(
                self.load_conspiracies_for_setup(
                    config.conspiracies.as_deref().ok_or_else(|| {
                        "Conspiracy Draft games require conspiracy selections".to_string()
                    })?,
                    config.sideboards.as_deref().ok_or_else(|| {
                        "Conspiracy Draft games require drafted sideboards".to_string()
                    })?,
                    player_count,
                )
                ?,
            ),
            _ => {
                if config
                    .conspiracies
                    .as_ref()
                    .is_some_and(|lists| lists.iter().any(|list| !list.is_empty()))
                {
                    return Err("conspiracies may be supplied only for a Conspiracy Draft game".to_string());
                }
                None
            }
        };

        if config.format.uses_commander_setup() {
            let Some(decks) = config.decks.as_ref() else {
                return Err("commander-variant matches require explicit decklists".to_string());
            };
            let Some(commanders) = config.commanders.as_ref() else {
                return Err("commander-variant matches require commander lists".to_string());
            };
            match config.format {
                MatchFormatInput::Commander | MatchFormatInput::ArchenemyCommander => self
                    .validate_commander_setup(
                        player_count,
                        decks,
                        commanders,
                        config.sideboards.as_deref(),
                        hidden_manifests,
                    )
                    ?,
                MatchFormatInput::CommanderDraft => self
                    .validate_commander_draft_setup(
                        player_count,
                        decks,
                        commanders,
                        config.sideboards.as_deref(),
                        hidden_manifests,
                        config
                            .commander_draft
                            .as_ref()
                            .expect("validated Commander Draft metadata"),
                    )
                    ?,
                MatchFormatInput::Brawl if hidden_manifests.is_empty() => self
                    .validate_brawl_setup(decks, commanders)
                    ?,
                MatchFormatInput::Brawl => {
                    for (player_index, commander_list) in commanders.iter().enumerate() {
                        if commander_list.len() != 1 {
                            return Err("Brawl matches require exactly one commander per player".to_string());
                        }
                        let Some(manifest) = hidden_manifests
                            .iter()
                            .find(|manifest| usize::from(manifest.owner) == player_index)
                        else {
                            return Err("Brawl committed setup requires one hidden manifest per player".to_string());
                        };
                        if manifest.deck_count != 59 || manifest.commander_count != 1 {
                            return Err("Brawl committed setup requires 59 main-deck cards and one commander per player".to_string());
                        }
                    }
                }
                MatchFormatInput::Normal
                | MatchFormatInput::FreeForAll
                | MatchFormatInput::GrandMelee
                | MatchFormatInput::TeamVsTeam
                | MatchFormatInput::Emperor
                | MatchFormatInput::TwoHeadedGiant
                | MatchFormatInput::AlternatingTeams
                | MatchFormatInput::Ante
                | MatchFormatInput::Planechase
                | MatchFormatInput::Vanguard
                | MatchFormatInput::Archenemy
                | MatchFormatInput::SupervillainRumble
                | MatchFormatInput::ConspiracyDraft => unreachable!(),
            }
        }

        // Setup validation is deliberately completed before replacing the live
        // match so an invalid Commander payload cannot partially mutate state.
        self.initialize_empty_match(config.player_names, starting_life, config.seed);
        self.match_format = config.format;
        self.game
            .set_commander_damage_loss_enabled(config.format.commander_damage_loss_enabled());
        let free_for_all_profile = match config.format {
            MatchFormatInput::FreeForAll => Some(config.free_for_all.unwrap_or_default()),
            MatchFormatInput::Planechase if player_count > 2 => {
                Some(FreeForAllOptionsInput::default())
            }
            MatchFormatInput::SupervillainRumble
            | MatchFormatInput::ConspiracyDraft
            | MatchFormatInput::CommanderDraft => Some(FreeForAllOptionsInput::default()),
            _ => None,
        };
        if config.format == MatchFormatInput::GrandMelee {
            self.game
                .enable_grand_melee()
                ?;
        }
        if config.format == MatchFormatInput::TeamVsTeam {
            let teams = config
                .teams
                .as_ref()
                .expect("validated Team vs. Team blocks")
                .iter()
                .map(|team| {
                    team.iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect::<Vec<_>>()
                })
                .collect();
            self.game
                .enable_team_vs_team(teams)
                ?;
        }
        if config.format == MatchFormatInput::Emperor {
            let teams = config
                .teams
                .as_ref()
                .expect("validated Emperor team blocks")
                .iter()
                .map(|team| {
                    team.iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect::<Vec<_>>()
                })
                .collect();
            self.game
                .enable_emperor(teams)
                ?;
        }
        if config.format == MatchFormatInput::TwoHeadedGiant {
            let teams = config
                .teams
                .as_ref()
                .expect("validated Two-Headed Giant team blocks")
                .iter()
                .map(|team| {
                    team.iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect::<Vec<_>>()
                })
                .collect();
            self.game
                .enable_two_headed_giant(teams)
                ?;
        }
        if config.format == MatchFormatInput::AlternatingTeams {
            let teams = config
                .teams
                .as_ref()
                .expect("validated Alternating Teams team blocks")
                .iter()
                .map(|team| {
                    team.iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect::<Vec<_>>()
                })
                .collect();
            let options = config.free_for_all.unwrap_or(FreeForAllOptionsInput {
                attack: FreeForAllAttackInput::MultiplePlayers,
                range_of_influence: Some(2),
                deploy_creatures: false,
            });
            let attack = match options.attack {
                FreeForAllAttackInput::Left => ironsmith::FreeForAllAttackOption::Left,
                FreeForAllAttackInput::Right => ironsmith::FreeForAllAttackOption::Right,
                FreeForAllAttackInput::MultiplePlayers => {
                    ironsmith::FreeForAllAttackOption::MultiplePlayers
                }
            };
            self.game
                .enable_alternating_teams(
                    teams,
                    attack,
                    options.range_of_influence,
                    options.deploy_creatures,
                )
                ?;
        }
        if let Some(options) = free_for_all_profile {
            let attack = match options.attack {
                FreeForAllAttackInput::Left => ironsmith::FreeForAllAttackOption::Left,
                FreeForAllAttackInput::Right => ironsmith::FreeForAllAttackOption::Right,
                FreeForAllAttackInput::MultiplePlayers => {
                    ironsmith::FreeForAllAttackOption::MultiplePlayers
                }
            };
            self.game
                .enable_free_for_all(attack, options.range_of_influence)
                ?;
        }
        let hidden_manifests = config.hidden_deck_manifests.unwrap_or_default();

        if let Some(decks) = config.decks {
            if decks.len() != self.game.players.len() {
                return Err("deck count must match number of players in game".to_string());
            }
            if hidden_manifests.is_empty() {
                self.populate_explicit_libraries(&decks)?;
            } else {
                self.populate_libraries_with_hidden_manifests(&decks, &hidden_manifests)?;
            }
        } else {
            self.populate_demo_libraries()?;
        }

        let mut sideboards = config
            .sideboards
            .unwrap_or_else(|| vec![Vec::new(); self.game.players.len()]);
        if let Some(selections) = prepared_conspiracies.as_ref() {
            for (owner, cards) in selections {
                let sideboard = sideboards
                    .get_mut(usize::from(owner.0))
                    .ok_or_else(|| "invalid conspiracy owner".to_string())?;
                for setup in cards {
                    let Some(position) = sideboard
                        .iter()
                        .position(|name| name.trim().eq_ignore_ascii_case(setup.definition.name()))
                    else {
                        return Err("selected conspiracy disappeared from its drafted sideboard".to_string());
                    };
                    sideboard.remove(position);
                }
            }
        }
        if !sideboards.is_empty() {
            if sideboards.len() != self.game.players.len() {
                return Err("sideboard count must match number of players in game".to_string());
            }
            self.populate_explicit_sideboards(&sideboards)?;
        }
        if self.match_format == MatchFormatInput::Normal {
            self.populate_hidden_manifest_sideboards(&sideboards, &hidden_manifests);
        }

        if let Some(commanders) = config.commanders {
            if commanders.len() != self.game.players.len() {
                return Err("commander count must match number of players in game".to_string());
            }
            self.populate_explicit_commanders(&commanders)?;
        }

        self.populate_companion_designations(&prepared_companions)?;

        if self.match_format == MatchFormatInput::Ante {
            let players = self
                .game
                .players
                .iter()
                .map(|player| player.id)
                .collect::<Vec<_>>();
            for player in players {
                self.game
                    .ante_random_library_card(player)
                    ?;
            }
        }

        if let Some(mut planar_decks) = prepared_planar_decks {
            if planar_decks.len() == 1 {
                self.game
                    .enable_planechase_communal(planar_decks.pop().expect("one planar deck"))
                    ?;
            } else {
                let players = self
                    .game
                    .players
                    .iter()
                    .map(|player| player.id)
                    .collect::<Vec<_>>();
                self.game
                    .enable_planechase(players.into_iter().zip(planar_decks).collect())
                    ?;
            }
        }

        if let Some(vanguards) = prepared_vanguards {
            let players = self
                .game
                .players
                .iter()
                .map(|player| player.id)
                .collect::<Vec<_>>();
            self.game
                .enable_vanguard(players.into_iter().zip(vanguards).collect())
                ?;
        }

        if let Some(scheme_decks) = prepared_scheme_decks {
            let variant = match self.match_format {
                MatchFormatInput::Archenemy => ironsmith::game_state::ArchenemyVariant::Default,
                MatchFormatInput::SupervillainRumble => {
                    ironsmith::game_state::ArchenemyVariant::SupervillainRumble
                }
                MatchFormatInput::ArchenemyCommander => {
                    ironsmith::game_state::ArchenemyVariant::Commander
                }
                _ => unreachable!(),
            };
            self.game
                .enable_archenemy(variant, scheme_decks)
                ?;
        }

        if let Some(conspiracies) = prepared_conspiracies {
            self.game
                .enable_conspiracy(conspiracies)
                ?;
        }

        self.finish_match_setup(opening_hand_size)
    }

    #[wasm_bindgen(js_name = revealHiddenObject)]
    pub fn reveal_hidden_object(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: RevealHiddenObjectInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid reveal input: {e}")))?;
        let object_id = ObjectId::from_raw(input.object_id);
        let Some(info) = self.game.hidden_card_info(object_id).cloned() else {
            return Err(JsValue::from_str("object is not a hidden card placeholder"));
        };
        if let Some(slot) = input.slot
            && slot != info.slot
        {
            return Err(JsValue::from_str("hidden card slot does not match reveal"));
        }
        if let Some(commitment) = input.commitment.as_deref()
            && commitment != info.commitment
        {
            return Err(JsValue::from_str(
                "hidden card commitment does not match reveal",
            ));
        }
        self.registry
            .ensure_cards_loaded([input.card_name.as_str()]);
        let definition = self
            .find_card_definition(&input.card_name)
            .cloned()
            .ok_or_else(|| JsValue::from_str(&format!("unknown card name: {}", input.card_name)))?;
        self.validate_hidden_commander_reveal(info.owner, object_id, &definition)
            .map_err(|error| JsValue::from_str(&error))?;
        self.validate_hidden_normal_reveal(info.owner, object_id, &definition)
            .map_err(|error| JsValue::from_str(&error))?;
        self.game
            .register_linked_face_family_from_catalog(&definition, &self.registry);
        self.game
            .reveal_hidden_card_with_definition(object_id, &definition)
            .ok_or_else(|| JsValue::from_str("failed to reveal hidden card"))?;
        self.reveal_hidden_card_in_live_continuation_checkpoint(
            info.owner,
            &[info.slot],
            &[info.commitment],
            &definition,
        );
        self.finish_hidden_card_reveal(input.recompute_decision)
    }

    #[wasm_bindgen(js_name = revealHiddenSlot)]
    pub fn reveal_hidden_slot(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: RevealHiddenSlotInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid reveal input: {e}")))?;
        self.reveal_hidden_slot_input(input)
    }

    fn reveal_hidden_slot_input(
        &mut self,
        input: RevealHiddenSlotInput,
    ) -> Result<JsValue, JsValue> {
        let owner = PlayerId::from_index(input.owner);
        let Some((&object_id, info)) = self
            .game
            .hidden_card_entries()
            .find(|(_, info)| info.owner == owner && info.slot == input.slot)
        else {
            return Err(JsValue::from_str(
                "hidden slot is not present in this engine",
            ));
        };
        if let Some(commitment) = input.commitment.as_deref()
            && commitment != info.commitment
        {
            return Err(JsValue::from_str(
                "hidden card commitment does not match reveal",
            ));
        }
        self.registry
            .ensure_cards_loaded([input.card_name.as_str()]);
        let definition = self
            .find_card_definition(&input.card_name)
            .cloned()
            .ok_or_else(|| JsValue::from_str(&format!("unknown card name: {}", input.card_name)))?;
        self.validate_hidden_commander_reveal(owner, object_id, &definition)
            .map_err(|error| JsValue::from_str(&error))?;
        self.validate_hidden_normal_reveal(owner, object_id, &definition)
            .map_err(|error| JsValue::from_str(&error))?;
        self.game
            .register_linked_face_family_from_catalog(&definition, &self.registry);
        self.game
            .reveal_hidden_card_with_definition(object_id, &definition)
            .ok_or_else(|| JsValue::from_str("failed to reveal hidden card"))?;
        self.reveal_hidden_card_in_live_continuation_checkpoint(
            owner,
            &[input.slot],
            &[input.commitment.unwrap_or_default()],
            &definition,
        );
        self.finish_hidden_card_reveal(input.recompute_decision)
    }

    #[wasm_bindgen(js_name = revealHiddenPosition)]
    pub fn reveal_hidden_position(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: RevealHiddenPositionInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid reveal input: {e}")))?;
        self.registry
            .ensure_cards_loaded([input.card_name.as_str()]);
        let reveal = self.validate_hidden_position_reveal(&input)?;
        self.apply_validated_hidden_position_reveal(&reveal)?;
        self.finish_hidden_card_reveal(input.recompute_decision)
    }

    #[wasm_bindgen(js_name = revealHiddenPositions)]
    pub fn reveal_hidden_positions(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let input: RevealHiddenPositionsInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid batch reveal input: {e}")))?;
        if input.reveals.is_empty() {
            return self.snapshot();
        }
        self.registry
            .ensure_cards_loaded(input.reveals.iter().map(|reveal| reveal.card_name.as_str()));
        let mut seen_objects = HashSet::new();
        let mut reveals = Vec::with_capacity(input.reveals.len());
        for reveal_input in &input.reveals {
            let reveal = self.validate_hidden_position_reveal(reveal_input)?;
            if !seen_objects.insert(reveal.object_id) {
                return Err(JsValue::from_str(
                    "batch hidden position reveal targets the same object more than once",
                ));
            }
            reveals.push(reveal);
        }
        self.validate_hidden_commander_position_reveals(&reveals)
            .map_err(|error| JsValue::from_str(&error))?;
        self.validate_hidden_normal_position_reveals(&reveals)
            .map_err(|error| JsValue::from_str(&error))?;
        for reveal in &reveals {
            self.apply_validated_hidden_position_reveal(reveal)?;
        }
        let recompute_decision = input.recompute_decision
            || input.reveals.iter().any(|reveal| reveal.recompute_decision);
        self.finish_hidden_card_reveal(recompute_decision)
    }

    fn validate_hidden_position_reveal(
        &self,
        input: &RevealHiddenPositionInput,
    ) -> Result<ValidatedHiddenPositionReveal, JsValue> {
        let owner = PlayerId::from_index(input.owner);
        let position_commitment = input.position_commitment.as_deref();
        let explicit_target = if let Some(raw) = input.object_id {
            let object_id = ObjectId::from_raw(raw);
            let Some(info) = self.game.hidden_card_info(object_id).cloned() else {
                return Err(JsValue::from_str(
                    "explicit hidden ziffle object is not present in this engine",
                ));
            };
            if info.owner != owner
                || !hidden_position_reveal_position_matches(
                    &info,
                    input.position,
                    position_commitment,
                )
            {
                return Err(JsValue::from_str(
                    "explicit hidden ziffle object does not match reveal position",
                ));
            }
            Some((object_id, info))
        } else {
            None
        };
        let target = explicit_target.or_else(|| {
            self.game
                .hidden_card_entries()
                .find(|(object_id, info)| {
                    info.owner == owner
                        && self.game.is_hidden_card_placeholder(**object_id)
                        && hidden_position_reveal_position_matches(
                            info,
                            input.position,
                            position_commitment,
                        )
                })
                .map(|(object_id, info)| (*object_id, info.clone()))
        });
        let Some((object_id, info)) = target else {
            return Err(JsValue::from_str(
                "hidden ziffle position is not present in this engine",
            ));
        };
        if !hidden_position_reveal_commitment_matches(&info, position_commitment) {
            return Err(JsValue::from_str(
                "hidden ziffle position commitment does not match reveal",
            ));
        }
        let zone = info.zone;
        let (public_slot, public_commitment) = public_identity_after_hidden_position_reveal(
            &info,
            input.position,
            input.position_commitment.as_deref(),
        );
        let updated_info = ironsmith::game_state::HiddenCardInfo {
            owner,
            zone,
            slot: input.original_slot,
            commitment: input.commitment.clone().unwrap_or_default(),
            public_slot,
            public_commitment,
        };
        let definition = self
            .find_card_definition(&input.card_name)
            .cloned()
            .ok_or_else(|| JsValue::from_str(&format!("unknown card name: {}", input.card_name)))?;
        let Some(existing_name) = self
            .game
            .object(object_id)
            .map(|object| object.name.clone())
        else {
            return Err(JsValue::from_str(
                "hidden ziffle object is not present in this engine",
            ));
        };
        let object_already_revealed = if existing_name != "Hidden Card" {
            if existing_name != input.card_name {
                return Err(JsValue::from_str(
                    "opened object identity does not match reveal",
                ));
            }
            true
        } else {
            false
        };
        Ok(ValidatedHiddenPositionReveal {
            input: input.clone(),
            owner,
            object_id,
            updated_info,
            definition,
            object_already_revealed,
        })
    }

    fn apply_validated_hidden_position_reveal(
        &mut self,
        reveal: &ValidatedHiddenPositionReveal,
    ) -> Result<(), JsValue> {
        self.validate_hidden_commander_reveal(reveal.owner, reveal.object_id, &reveal.definition)
            .map_err(|error| JsValue::from_str(&error))?;
        self.validate_hidden_normal_reveal(reveal.owner, reveal.object_id, &reveal.definition)
            .map_err(|error| JsValue::from_str(&error))?;
        self.game
            .set_hidden_card_info(reveal.object_id, reveal.updated_info.clone());
        if let Some(existing_name) = self
            .game
            .object(reveal.object_id)
            .map(|object| object.name.clone())
            && existing_name != "Hidden Card"
        {
            if existing_name != reveal.input.card_name {
                return Err(JsValue::from_str(
                    "opened object identity does not match reveal",
                ));
            }
        } else if !reveal.object_already_revealed {
            self.game
                .register_linked_face_family_from_catalog(&reveal.definition, &self.registry);
            self.game
                .reveal_hidden_card_with_definition(reveal.object_id, &reveal.definition)
                .ok_or_else(|| JsValue::from_str("failed to reveal hidden card"))?;
        }
        self.reveal_hidden_position_in_live_continuation_checkpoint(
            reveal.owner,
            reveal.input.position,
            reveal.input.position_commitment.as_deref(),
            reveal.updated_info.clone(),
            &reveal.definition,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name = exportHiddenCardOpening)]
    pub fn export_hidden_card_opening(&self, object_id: u64) -> Result<JsValue, JsValue> {
        let opening = self.hidden_card_opening_export(ObjectId::from_raw(object_id))?;
        serde_wasm_bindgen::to_value(&opening)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize hidden opening: {e}")))
    }

    #[wasm_bindgen(js_name = previewCryptoRequirements)]
    pub fn preview_crypto_requirements(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let checkpoint = self.capture_replay_checkpoint();
        let pregame = self.pregame.clone();
        let pending_decision = self.pending_decision.clone();
        let pending_replay_action = self.pending_replay_action.clone();
        let pending_action_checkpoint = self.pending_action_checkpoint.clone();
        let pending_live_action_root = self.pending_live_action_root.clone();
        let pending_live_continuation = self.pending_live_continuation.clone();
        let runner = self.runner.clone();
        let runner_awaiting_priority = self.runner_awaiting_priority;
        let runner_pending_decision = self.runner_pending_decision;
        let priority_epoch_checkpoint = self.priority_epoch_checkpoint.clone();
        let priority_epoch_has_undoable_action = self.priority_epoch_has_undoable_action;
        let priority_epoch_undo_locked_by_mana = self.priority_epoch_undo_locked_by_mana;
        let priority_epoch_undo_land_stable_id = self.priority_epoch_undo_land_stable_id;
        let active_viewed_cards = self.active_viewed_cards.clone();
        let active_resolving_stack_object = self.active_resolving_stack_object.clone();
        let snapshot_serial = self.snapshot_serial;
        let last_snapshot_perf = self.last_snapshot_perf.clone();
        let last_replay_execution_perf = self.last_replay_execution_perf.clone();
        let last_advance_until_decision_perf = self.last_advance_until_decision_perf.clone();
        let last_dispatch_perf = self.last_dispatch_perf.clone();

        let preview_result = self.dispatch(command);
        let requirements = match preview_result {
            Ok(_) => Ok(self.last_crypto_requirements.clone()),
            Err(err) => Err(err),
        };

        self.restore_replay_checkpoint(&checkpoint);
        self.pregame = pregame;
        self.pending_decision = pending_decision;
        self.pending_replay_action = pending_replay_action;
        self.pending_action_checkpoint = pending_action_checkpoint;
        self.pending_live_action_root = pending_live_action_root;
        self.pending_live_continuation = pending_live_continuation;
        self.runner = runner;
        self.runner_awaiting_priority = runner_awaiting_priority;
        self.runner_pending_decision = runner_pending_decision;
        self.priority_epoch_checkpoint = priority_epoch_checkpoint;
        self.priority_epoch_has_undoable_action = priority_epoch_has_undoable_action;
        self.priority_epoch_undo_locked_by_mana = priority_epoch_undo_locked_by_mana;
        self.priority_epoch_undo_land_stable_id = priority_epoch_undo_land_stable_id;
        self.active_viewed_cards = active_viewed_cards;
        self.active_resolving_stack_object = active_resolving_stack_object;
        self.snapshot_serial = snapshot_serial;
        self.last_snapshot_perf = last_snapshot_perf;
        self.last_replay_execution_perf = last_replay_execution_perf;
        self.last_advance_until_decision_perf = last_advance_until_decision_perf;
        self.last_dispatch_perf = last_dispatch_perf;

        let requirements = requirements?;
        serde_wasm_bindgen::to_value(&requirements).map_err(|e| {
            JsValue::from_str(&format!("failed to serialize crypto requirements: {e}"))
        })
    }

    #[wasm_bindgen(js_name = injectTranscriptRandomSeeds)]
    pub fn inject_transcript_random_seeds(&mut self, input: JsValue) -> Result<(), JsValue> {
        let input: TranscriptRandomSeedsInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid random seed input: {e}")))?;
        let mut seeds = Vec::with_capacity(input.seeds.len());
        for seed in input.seeds {
            let normalized = seed.trim().trim_start_matches("0x");
            let trimmed = if normalized.len() > 16 {
                &normalized[normalized.len() - 16..]
            } else {
                normalized
            };
            let value = u64::from_str_radix(trimmed, 16)
                .map_err(|e| JsValue::from_str(&format!("invalid random seed hex: {e}")))?;
            seeds.push(value);
        }
        self.game.queue_transcript_random_seeds(seeds);
        for shuffle in input.library_shuffles {
            if shuffle.before_order.is_empty()
                || shuffle.before_order.len() != shuffle.after_order.len()
            {
                continue;
            }
            self.game.queue_transcript_library_shuffle_order(
                PlayerId::from_index(shuffle.owner),
                shuffle
                    .before_order
                    .into_iter()
                    .map(ObjectId::from_raw)
                    .collect(),
                shuffle
                    .after_order
                    .into_iter()
                    .map(ObjectId::from_raw)
                    .collect(),
            );
        }
        Ok(())
    }

    fn reseal_verified_hidden_library_shuffle(
        &mut self,
        input: ApplyHiddenLibraryShuffleInput,
    ) -> Result<(), JsValue> {
        let owner = PlayerId::from_index(input.owner);
        let library = self
            .game
            .player(owner)
            .ok_or_else(|| JsValue::from_str("hidden shuffle owner is not present"))?
            .library
            .clone();
        let order = if input.after_order.is_empty() {
            library
        } else {
            input
                .after_order
                .iter()
                .copied()
                .map(ObjectId::from_raw)
                .map(|object_id| {
                    self.game
                        .current_object_id_after_zone_change(object_id)
                        .unwrap_or(object_id)
                })
                .collect()
        };
        if !input.after_order.is_empty() {
            let current_library = self
                .game
                .player(owner)
                .ok_or_else(|| JsValue::from_str("hidden shuffle owner is not present"))?
                .library
                .clone();
            let current_library_set = current_library.iter().copied().collect::<HashSet<_>>();
            let reordered_library = order
                .iter()
                .copied()
                .filter(|object_id| current_library_set.contains(object_id))
                .collect::<Vec<_>>();
            let reordered_set = reordered_library.iter().copied().collect::<HashSet<_>>();
            if reordered_library.len() != current_library.len()
                || reordered_set.len() != current_library_set.len()
                || !current_library_set
                    .iter()
                    .all(|id| reordered_set.contains(id))
            {
                return Err(JsValue::from_str(
                    "verified hidden shuffle order does not cover the current library",
                ));
            }
            if let Some(player) = self.game.player_mut(owner) {
                player.library = reordered_library;
            }
        }
        let mut seen = HashSet::new();
        for (position, object_id) in order.iter().copied().enumerate() {
            if position > u16::MAX as usize {
                return Err(JsValue::from_str("hidden shuffle library is too large"));
            }
            if !seen.insert(object_id) {
                return Err(JsValue::from_str(
                    "hidden shuffle order contains duplicate cards",
                ));
            }
            let Some(zone) = self.game.object(object_id).map(|object| object.zone) else {
                return Err(JsValue::from_str("hidden shuffle card is not present"));
            };
            let Some(info) = self.game.hidden_card_info(object_id).cloned() else {
                if zone.is_hidden()
                    && self
                        .game
                        .object(object_id)
                        .is_some_and(|object| object.card.is_some() && object.owner == owner)
                {
                    if let Some(object) = self.game.object_mut(object_id) {
                        object.redact_to_hidden_card();
                    }
                    self.game.set_hidden_card_info(
                        object_id,
                        ironsmith::game_state::HiddenCardInfo {
                            owner,
                            zone,
                            slot: position as u16,
                            commitment: format!("ziffle:{}:{}", input.deck_hash, position),
                            public_slot: Some(position as u16),
                            public_commitment: Some(format!(
                                "ziffle:{}:{}",
                                input.deck_hash, position
                            )),
                        },
                    );
                    continue;
                }
                return Err(JsValue::from_str(&format!(
                    "cannot reseal library shuffle with non-hidden card {} at position {}",
                    object_id.0, position
                )));
            };
            if info.owner != owner {
                return Err(JsValue::from_str("hidden shuffle library owner mismatch"));
            }
            if zone.is_hidden()
                && self
                    .game
                    .object(object_id)
                    .is_some_and(|object| object.card.is_some())
            {
                if let Some(object) = self.game.object_mut(object_id) {
                    object.redact_to_hidden_card();
                }
            }
            self.game.set_hidden_card_info(
                object_id,
                ironsmith::game_state::HiddenCardInfo {
                    owner,
                    zone,
                    public_slot: Some(position as u16),
                    public_commitment: Some(format!("ziffle:{}:{}", input.deck_hash, position)),
                    ..info
                },
            );
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = applyVerifiedHiddenLibraryShuffle)]
    pub fn apply_verified_hidden_library_shuffle(
        &mut self,
        input: JsValue,
    ) -> Result<JsValue, JsValue> {
        let input: ApplyHiddenLibraryShuffleInput = serde_wasm_bindgen::from_value(input)
            .map_err(|e| JsValue::from_str(&format!("invalid hidden shuffle input: {e}")))?;
        self.reseal_verified_hidden_library_shuffle(input)?;
        self.last_crypto_requirements.clear();
        self.pending_crypto_audit_before = None;
        self.recompute_ui_decision()?;
        self.snapshot()
    }

    fn hidden_card_opening_export(
        &self,
        object_id: ObjectId,
    ) -> Result<HiddenCardOpeningExport, JsValue> {
        let info = self
            .game
            .hidden_card_info(object_id)
            .ok_or_else(|| JsValue::from_str("object is not tracked as a hidden card"))?;
        let object = self
            .game
            .object(object_id)
            .ok_or_else(|| JsValue::from_str("hidden card object is not present"))?;
        let Some(_card) = object.card.as_ref() else {
            return Err(JsValue::from_str("hidden card is not open in this engine"));
        };
        Ok(HiddenCardOpeningExport {
            object_id: object_id.0,
            owner: info.owner.index() as u8,
            slot: info.slot,
            card: object.name.to_string(),
            commitment: info.commitment.clone(),
            public_slot: info.public_slot,
            public_commitment: info.public_commitment.clone(),
        })
    }

    #[wasm_bindgen(js_name = validateMatchConfig)]
    pub fn validate_match_config(&mut self, config: JsValue) -> Result<JsValue, JsValue> {
        let companion_selections: CompanionSelectionsInput =
            serde_wasm_bindgen::from_value(config.clone()).map_err(|e| {
                JsValue::from_str(&format!("invalid companion match config: {e}"))
            })?;
        let config: MatchSetupInput = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("invalid match config: {e}")))?;
        self.validate_companion_setup(&config, companion_selections.companions.as_deref())
            .map_err(|error| JsValue::from_str(&error))?;
        let validation = self.validate_match_setup_input(&config)?;
        serde_wasm_bindgen::to_value(&validation)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize match validation: {e}")))
    }

    /// Return a JS object snapshot of public game state.
    #[wasm_bindgen]
    pub fn snapshot(&mut self) -> Result<JsValue, JsValue> {
        let snapshot_started_at = PerfTimer::start();
        let pending_cast_stack_id = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = self.is_cancelable();
        let undo_land_stable_id = self.visible_undo_land_stable_id(cancelable);
        let mana_payment_view = self.current_mana_payment_view();
        let cache_key = self.snapshot_cache_key(
            pending_cast_stack_id,
            cancelable,
            undo_land_stable_id,
            &mana_payment_view,
        );
        #[cfg(target_arch = "wasm32")]
        if !self.game.has_ui_battlefield_transitions()
            && self.pending_crypto_audit_before.is_none()
            && let Some(cached) = self.cached_snapshot.as_ref()
            && cached.key == cache_key
        {
            self.last_snapshot_perf = Some(cached.perf.clone());
            return Ok(cached.value.clone());
        }
        self.snapshot_serial = self.snapshot_serial.saturating_add(1);
        let snapshot_id = self.snapshot_serial;
        let transitions_started_at = PerfTimer::start();
        let battlefield_transitions =
            battlefield_transition_snapshots(self.game.take_ui_battlefield_transitions());
        let had_battlefield_transitions = !battlefield_transitions.is_empty();
        #[cfg(not(target_arch = "wasm32"))]
        let _ = had_battlefield_transitions;
        let battlefield_transition_ms = transitions_started_at.elapsed_ms();
        self.game.refresh_continuous_state();
        let build_started_at = PerfTimer::start();
        if let Some(before) = self.pending_crypto_audit_before.take() {
            self.update_crypto_requirements_from(before);
        }
        let mut snap = GameSnapshot::from_game_with_object_view_cache(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            mana_payment_view,
            self.game_over.as_ref(),
            pending_cast_stack_id,
            self.active_resolving_stack_object.clone(),
            battlefield_transitions,
            self.active_viewed_cards.as_ref(),
            cancelable,
            undo_land_stable_id,
            snapshot_id,
            &self.snapshot_object_view_cache,
        );
        snap.crypto_requirements = self.last_crypto_requirements.clone();
        let snapshot_build_ms = build_started_at.elapsed_ms();
        let pending_insert_started_at = PerfTimer::start();
        insert_pending_stack_object_snapshots(&mut snap, self.pending_trigger_stack_objects());
        let pending_stack_insert_ms = pending_insert_started_at.elapsed_ms();
        let player_count = snap.players.len();
        let battlefield_size = snap.battlefield_size;
        let stack_size = snap.stack_size;
        let encode_started_at = PerfTimer::start();
        #[cfg(target_arch = "wasm32")]
        let encoded = self
            .snapshot_js_encoding_cache
            .encode_snapshot(&snap)
            .map_err(|e| JsValue::from_str(&format!("snapshot encode failed: {e:?}")))?;
        #[cfg(not(target_arch = "wasm32"))]
        let encoded = JsValue::NULL;
        let snapshot_encode_ms = encode_started_at.elapsed_ms();
        let total_snapshot_ms = snapshot_started_at.elapsed_ms();
        let perf = SnapshotPerfMetrics {
            snapshot_id,
            battlefield_transition_ms,
            snapshot_build_ms,
            pending_stack_insert_ms,
            snapshot_encode_ms,
            total_snapshot_ms,
            player_count,
            battlefield_size,
            stack_size,
        };
        self.last_snapshot_perf = Some(perf.clone());
        #[cfg(target_arch = "wasm32")]
        if !had_battlefield_transitions && self.pending_crypto_audit_before.is_none() {
            self.cached_snapshot = Some(CachedSnapshot {
                key: cache_key,
                value: encoded.clone(),
                perf,
            });
        } else {
            self.cached_snapshot = None;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = cache_key;
            self.cached_snapshot = None;
        }
        Ok(encoded)
    }

    /// Return the current UI state from the selected player perspective.
    #[wasm_bindgen(js_name = uiState)]
    pub fn ui_state(&mut self) -> Result<JsValue, JsValue> {
        self.recompute_stale_priority_decision()?;
        self.snapshot()
    }

    #[wasm_bindgen(js_name = lastSnapshotPerf)]
    pub fn last_snapshot_perf_js(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.last_snapshot_perf)
            .map_err(|e| JsValue::from_str(&format!("lastSnapshotPerf encode failed: {e}")))
    }

    #[wasm_bindgen(js_name = lastDispatchPerf)]
    pub fn last_dispatch_perf_js(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.last_dispatch_perf)
            .map_err(|e| JsValue::from_str(&format!("lastDispatchPerf encode failed: {e}")))
    }

    #[wasm_bindgen(js_name = lastWorkCounters)]
    pub fn last_work_counters_js(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.game.work_counters())
            .map_err(|e| JsValue::from_str(&format!("lastWorkCounters encode failed: {e}")))
    }

    #[wasm_bindgen(js_name = lastReplayExecutionPerf)]
    pub fn last_replay_execution_perf_js(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.last_replay_execution_perf)
            .map_err(|e| JsValue::from_str(&format!("lastReplayExecutionPerf encode failed: {e}")))
    }

    #[wasm_bindgen(js_name = lastAdvanceUntilDecisionPerf)]
    pub fn last_advance_until_decision_perf_js(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.last_advance_until_decision_perf).map_err(|e| {
            JsValue::from_str(&format!("lastAdvanceUntilDecisionPerf encode failed: {e}"))
        })
    }

    /// Number of cards currently available in the registry.
    #[wasm_bindgen(js_name = registrySize)]
    pub fn registry_size(&self) -> usize {
        self.registry.len()
    }

    /// Incremental generated-registry preload status.
    #[wasm_bindgen(js_name = preloadRegistryStatus)]
    pub fn preload_registry_status(&self) -> Result<JsValue, JsValue> {
        // Fidelity coverage is precomputed during the build pipeline, so this is
        // effectively complete immediately.
        let total = CardRegistry::generated_parser_semantic_scored_count();
        let status = RegistryPreloadStatus {
            loaded: total,
            cursor: total,
            total,
            done: true,
        };
        serde_wasm_bindgen::to_value(&status)
            .map_err(|e| JsValue::from_str(&format!("preloadRegistryStatus encode failed: {e}")))
    }

    /// Parse/register the next batch of generated cards for startup warmup.
    #[wasm_bindgen(js_name = preloadRegistryChunk)]
    pub fn preload_registry_chunk(&mut self, _chunk_size: usize) -> Result<JsValue, JsValue> {
        self.preload_registry_status()
    }

    /// Return a detailed, human-readable object snapshot for inspector UI.
    #[wasm_bindgen(js_name = objectDetails)]
    pub fn object_details(&self, object_id: u64) -> Result<JsValue, JsValue> {
        let object_id = ObjectId::from_raw(object_id);
        if self.game.is_face_down_conspiracy(object_id)
            && self
                .game
                .object(object_id)
                .is_some_and(|object| object.owner != self.perspective)
        {
            return Err(JsValue::from_str(
                "a face-down conspiracy may be inspected only by its controller",
            ));
        }
        let details = build_object_details_snapshot(&self.game, object_id)
            .ok_or_else(|| JsValue::from_str(&format!("unknown object id: {}", object_id.0)))?;
        serde_wasm_bindgen::to_value(&details)
            .map_err(|e| JsValue::from_str(&format!("objectDetails encode failed: {e}")))
    }

    /// Return game snapshot as pretty JSON.
    #[wasm_bindgen(js_name = snapshotJson)]
    pub fn snapshot_json(&mut self) -> Result<String, JsValue> {
        self.cached_snapshot = None;
        let pending_cast_stack_id = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = self.is_cancelable();
        let undo_land_stable_id = self.visible_undo_land_stable_id(cancelable);
        self.snapshot_serial = self.snapshot_serial.saturating_add(1);
        let snapshot_id = self.snapshot_serial;
        let battlefield_transitions =
            battlefield_transition_snapshots(self.game.take_ui_battlefield_transitions());
        self.game.refresh_continuous_state();
        let mut snap = GameSnapshot::from_game_with_object_view_cache(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.current_mana_payment_view(),
            self.game_over.as_ref(),
            pending_cast_stack_id,
            self.active_resolving_stack_object.clone(),
            battlefield_transitions,
            self.active_viewed_cards.as_ref(),
            cancelable,
            undo_land_stable_id,
            snapshot_id,
            &self.snapshot_object_view_cache,
        );
        snap.crypto_requirements = self.last_crypto_requirements.clone();
        insert_pending_stack_object_snapshots(&mut snap, self.pending_trigger_stack_objects());
        serde_json::to_string_pretty(&snap)
            .map_err(|e| JsValue::from_str(&format!("json encode failed: {e}")))
    }

    /// Return locally-known card name suggestions from the generated registry.
    #[wasm_bindgen(js_name = autocompleteCardNames)]
    pub fn autocomplete_card_names(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return serde_wasm_bindgen::to_value(&Vec::<String>::new()).map_err(|e| {
                JsValue::from_str(&format!("autocompleteCardNames encode failed: {e}"))
            });
        }

        let query_lower = trimmed.to_lowercase();
        let capped_limit = limit.unwrap_or(5).clamp(1, 25);
        let threshold = self.semantic_threshold;
        let mut matches = Self::autocomplete_name_corpus()
            .iter()
            .filter_map(|(name, lower)| {
                let rank = if lower == &query_lower {
                    0u8
                } else if lower.starts_with(&query_lower) {
                    1
                } else if lower
                    .split_whitespace()
                    .any(|word| word.starts_with(&query_lower))
                {
                    2
                } else if lower.contains(&query_lower) {
                    3
                } else {
                    return None;
                };

                if threshold > 0.0
                    && let Some(score) = self.semantic_score_for_name(name.as_str())
                    && score < threshold
                {
                    return None;
                }

                Some((rank, name.len(), name))
            })
            .collect::<Vec<_>>();
        matches.sort_unstable_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(right.2))
        });
        let suggestions: Vec<String> = matches
            .into_iter()
            .take(capped_limit)
            .map(|(_, _, name)| name.clone())
            .collect();

        serde_wasm_bindgen::to_value(&suggestions)
            .map_err(|e| JsValue::from_str(&format!("autocompleteCardNames encode failed: {e}")))
    }

    /// Return whether the query resolves to a locally known card name.
    #[wasm_bindgen(js_name = isKnownCardName)]
    pub fn is_known_card_name(&mut self, query: String) -> bool {
        self.is_known_card_name_query(query.trim())
    }

    /// Set a player's life total.
    #[wasm_bindgen(js_name = setLife)]
    pub fn set_life(&mut self, player_index: u8, life: i32) -> Result<(), JsValue> {
        let player_id = PlayerId::from_index(player_index);
        let Some(player) = self.game.player_mut(player_id) else {
            return Err(JsValue::from_str("invalid player index"));
        };
        player.life = life;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Add a signed life delta (negative = damage, positive = gain).
    #[wasm_bindgen(js_name = addLifeDelta)]
    pub fn add_life_delta(&mut self, player_index: u8, delta: i32) -> Result<(), JsValue> {
        let player_id = PlayerId::from_index(player_index);
        let Some(player) = self.game.player_mut(player_id) else {
            return Err(JsValue::from_str("invalid player index"));
        };
        player.life += delta;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Mark a player as having forfeited the match.
    #[wasm_bindgen(js_name = forfeitPlayer)]
    pub fn forfeit_player(&mut self, player_index: u8) -> Result<JsValue, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        // Integrity guard (defense-in-depth): the engine must never mutate a match
        // that is already decided, and must never re-forfeit a player who is no
        // longer in the game. Authorization that a forfeit is legitimate lives in
        // the protocol layer (live receive-gate + transcript verifier); these checks
        // only ensure a forfeit cannot rewrite a finished or already-eliminated
        // result. No legitimate flow forfeits post-game or double-forfeits, so a
        // caller that hits either case is rejected.
        if self.game_over.is_some() {
            return Err(JsValue::from_str(
                "cannot forfeit a player after the game is already decided",
            ));
        }
        let Some(player) = self.game.player(player_id) else {
            return Err(JsValue::from_str("invalid player index"));
        };
        if !player.is_in_game() {
            return Err(JsValue::from_str(
                "cannot forfeit a player who is no longer in the game",
            ));
        }

        self.game.mark_player_lost(player_id);
        self.priority_state.player_left_game(&self.game);

        let remaining: Vec<_> = self
            .game
            .players
            .iter()
            .filter(|candidate| candidate.is_in_game())
            .map(|candidate| candidate.id)
            .collect();
        let result = if remaining.is_empty() {
            Some(GameResult::Draw)
        } else if remaining.len() == 1 {
            Some(GameResult::Winner(remaining[0]))
        } else {
            None
        };
        if let Some(result) = result {
            self.record_game_result(result);
        }

        self.recompute_ui_decision()?;
        self.snapshot()
    }

    /// Queue a forced die result for deterministic test harness scenarios.
    #[wasm_bindgen(js_name = forceNextDieRoll)]
    pub fn force_next_die_roll(&mut self, result: u32) {
        self.game.force_next_die_roll(result);
        if let Some(checkpoint) = self.priority_epoch_checkpoint.as_mut() {
            checkpoint.game.force_next_die_roll(result);
        }
        if let Some(action) = self.pending_replay_action.as_mut() {
            action.checkpoint.game.force_next_die_roll(result);
        }
        if let Some(checkpoint) = self.pending_action_checkpoint.as_mut() {
            checkpoint.game.force_next_die_roll(result);
        }
        if let Some(continuation) = self.pending_live_continuation.as_mut() {
            continuation.checkpoint.game.force_next_die_roll(result);
        }
    }

    #[wasm_bindgen(js_name = setDaytime)]
    pub fn set_daytime(&mut self, daytime: bool) -> Result<JsValue, JsValue> {
        self.game.set_daytime(daytime);
        if let Some(checkpoint) = self.priority_epoch_checkpoint.as_mut() {
            checkpoint.game.set_daytime(daytime);
        }
        if let Some(action) = self.pending_replay_action.as_mut() {
            action.checkpoint.game.set_daytime(daytime);
        }
        if let Some(checkpoint) = self.pending_action_checkpoint.as_mut() {
            checkpoint.game.set_daytime(daytime);
        }
        if let Some(continuation) = self.pending_live_continuation.as_mut() {
            continuation.checkpoint.game.set_daytime(daytime);
        }
        self.recompute_ui_decision()?;
        self.snapshot()
    }

    #[wasm_bindgen(js_name = isDaytime)]
    pub fn is_daytime(&self) -> bool {
        self.game.is_daytime()
    }

    #[wasm_bindgen(js_name = hasDayNight)]
    pub fn has_day_night(&self) -> bool {
        self.game.has_day_night()
    }

    /// Draw one card for a player.
    #[wasm_bindgen(js_name = drawCard)]
    pub fn draw_card(&mut self, player_index: u8) -> Result<usize, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }
        let drawn = self.game.draw_cards(player_id, 1);
        self.recompute_ui_decision()?;
        Ok(drawn.len())
    }

    /// Move a hand card onto the battlefield with the shared morph-style
    /// face-down overlay. This is used by ported test harnesses that set up a
    /// cast result directly when the UI has no payable cast action exposed.
    #[wasm_bindgen(js_name = moveHandCardToBattlefieldFaceDown)]
    pub fn move_hand_card_to_battlefield_face_down(
        &mut self,
        player_index: u8,
        object_id: u64,
        ward_generic_cost: u8,
    ) -> Result<u64, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        let id = ObjectId(object_id);
        let object = self
            .game
            .object_mut(id)
            .ok_or_else(|| JsValue::from_str("object not found"))?;
        if object.owner != player_id || object.zone != Zone::Hand {
            return Err(JsValue::from_str("object is not in that player's hand"));
        }
        object.apply_face_down_cast_overlay();
        let new_id = self
            .game
            .move_object(
                id,
                Zone::Battlefield,
                ironsmith::events::cause::EventCause::from_game_rule(),
            )
            .ok_or_else(|| JsValue::from_str("failed to move object to battlefield"))?;
        self.game.set_face_down(new_id);
        self.game.set_summoning_sick(new_id);
        if ward_generic_cost > 0
            && let Some(object) = self.game.object_mut(new_id)
        {
            let mana_cost = ironsmith::mana::ManaCost::from_symbols(vec![
                ironsmith::mana::ManaSymbol::Generic(ward_generic_cost),
            ]);
            object
                .abilities_mut()
                .push(ironsmith::ability::Ability::static_ability(
                    ironsmith::static_abilities::StaticAbility::ward(
                        ironsmith::cost::TotalCost::mana(mana_cost),
                    ),
                ));
        }
        self.recompute_ui_decision()?;
        Ok(new_id.0)
    }

    /// Turn a face-down permanent face up without going through priority action
    /// enumeration. Ported tests use this when the UI has not exposed the
    /// special action because mana was supplied out of band.
    #[wasm_bindgen(js_name = forceTurnFaceUp)]
    pub fn force_turn_face_up(&mut self, player_index: u8, object_id: u64) -> Result<(), JsValue> {
        let player_id = PlayerId::from_index(player_index);
        let id = ObjectId(object_id);
        let controller = self
            .game
            .object(id)
            .map(|object| (self.game.controller_of(object), object.zone))
            .ok_or_else(|| JsValue::from_str("object not found"))?;
        if controller.0 != player_id || controller.1 != Zone::Battlefield {
            return Err(JsValue::from_str(
                "object is not a battlefield permanent controlled by that player",
            ));
        }
        let object = self
            .game
            .object_mut(id)
            .ok_or_else(|| JsValue::from_str("object not found"))?;
        object.end_face_down_cast_overlay();
        self.game.set_face_up(id);

        let root = self.game.provenance_graph_mut().alloc_root(
            ironsmith::provenance::ProvenanceNodeKind::EffectExecution {
                source: id,
                controller: player_id,
            },
        );
        let event_provenance = self
            .game
            .alloc_child_event_provenance(root, ironsmith::events::EventKind::TurnedFaceUp);
        self.game.queue_trigger_event(
            root,
            ironsmith::TriggerEvent::new_with_provenance(
                ironsmith::events::TurnedFaceUpEvent::new(id, player_id),
                event_provenance,
            ),
        );
        ironsmith::game_loop::drain_pending_trigger_events(&mut self.game, &mut self.trigger_queue);
        ironsmith::put_triggers_on_stack(&mut self.game, &mut self.trigger_queue).map_err(
            |err| JsValue::from_str(&format!("failed to put triggers on stack: {err:?}")),
        )?;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Add a specific card by name to a player's hand.
    #[wasm_bindgen(js_name = addCardToHand)]
    pub fn add_card_to_hand(
        &mut self,
        player_index: u8,
        card_name: String,
    ) -> Result<u64, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }

        let query = card_name.trim();
        if query.is_empty() {
            return Err(JsValue::from_str("card name cannot be empty"));
        }

        self.registry.ensure_cards_loaded([query]);
        let definition = self.load_compilable_card_definition(query)?;

        let object_id = self.game.create_object_from_catalog_definition(
            &definition,
            &self.registry,
            player_id,
            ironsmith::zone::Zone::Hand,
        );
        self.recompute_ui_decision()?;
        Ok(object_id.0)
    }

    fn add_card_to_zone_with_dm(
        &mut self,
        player_id: PlayerId,
        definition: &CardDefinition,
        zone: Zone,
        skip_triggers: bool,
        dm: &mut impl DecisionMaker,
    ) -> Result<u64, String> {
        fn align_manual_add_stable_id(game: &mut GameState, object_id: ObjectId) {
            if let Some(object) = game.object_mut(object_id) {
                object.stable_id = StableId::from(object_id);
            }
        }

        if skip_triggers {
            if zone == Zone::Battlefield {
                let event_record_len = self.game.turn_store.turn_history.event_records.len();
                let staged_event_record_len =
                    self.game.turn_store.turn_history.staged_event_records.len();
                self.game
                    .register_linked_face_family_from_catalog(definition, &self.registry);
                let temp_id =
                    self.game
                        .create_object_from_definition(definition, player_id, Zone::Command);
                let Some(result) = self.game.move_object_with_etb_processing_with_dm(
                    temp_id,
                    Zone::Battlefield,
                    dm,
                ) else {
                    self.game.remove_object(temp_id);
                    return Err("battlefield entry was prevented by replacement effect".to_string());
                };
                align_manual_add_stable_id(&mut self.game, result.new_id);
                self.game.take_pending_trigger_events();
                self.game
                    .turn_store
                    .turn_history
                    .event_records
                    .truncate(event_record_len);
                self.game
                    .turn_store
                    .turn_history
                    .staged_event_records
                    .truncate(staged_event_record_len);
                return Ok(result.new_id.0);
            }
            let object_id = self.game.create_object_from_catalog_definition(
                definition,
                &self.registry,
                player_id,
                zone,
            );
            if zone == Zone::Command {
                self.game.set_as_commander(object_id, player_id);
            }
            return Ok(object_id.0);
        }

        // Create in Command zone first, then move to target zone so that
        // zone-change triggers (ETB, etc.) fire naturally.
        self.game
            .register_linked_face_family_from_catalog(definition, &self.registry);
        let temp_id = self
            .game
            .create_object_from_definition(definition, player_id, Zone::Command);
        let object_id = if zone == Zone::Battlefield {
            let Some(result) =
                self.game
                    .move_object_with_etb_processing_with_dm(temp_id, Zone::Battlefield, dm)
            else {
                self.game.remove_object(temp_id);
                return Err("battlefield entry was prevented by replacement effect".to_string());
            };

            let entered_id = result.new_id;
            align_manual_add_stable_id(&mut self.game, entered_id);
            let entered_tapped = result.enters_tapped;
            let entered_battlefield = self
                .game
                .object(entered_id)
                .is_some_and(|obj| obj.zone == Zone::Battlefield);
            if entered_battlefield {
                let etb_event_provenance = self
                    .game
                    .provenance_graph_mut()
                    .alloc_root_event(ironsmith::events::EventKind::EnterBattlefield);
                let event = if entered_tapped {
                    ironsmith::triggers::TriggerEvent::new_with_provenance(
                        ironsmith::events::EnterBattlefieldEvent::tapped(entered_id, Zone::Command),
                        etb_event_provenance,
                    )
                } else {
                    ironsmith::triggers::TriggerEvent::new_with_provenance(
                        ironsmith::events::EnterBattlefieldEvent::new(entered_id, Zone::Command),
                        etb_event_provenance,
                    )
                };
                self.game.queue_trigger_event(etb_event_provenance, event);

                ironsmith::game_loop::drain_pending_trigger_events(
                    &mut self.game,
                    &mut self.trigger_queue,
                );

                ironsmith::game_loop::handle_saga_enters_battlefield(
                    &mut self.game,
                    entered_id,
                    &mut self.trigger_queue,
                    dm,
                );
            }

            entered_id
        } else {
            self.game
                .move_object_by_effect(temp_id, zone)
                .unwrap_or(temp_id)
        };
        align_manual_add_stable_id(&mut self.game, object_id);
        if zone == Zone::Command {
            self.game.set_as_commander(object_id, player_id);
        }
        ironsmith::game_loop::drain_pending_trigger_events(&mut self.game, &mut self.trigger_queue);
        Ok(object_id.0)
    }

    /// Add a specific card by name to a player's zone.
    ///
    /// When `skip_triggers` is true the card is placed directly without
    /// processing ETB or other zone-change triggers.
    #[wasm_bindgen(js_name = addCardToZone)]
    pub fn add_card_to_zone(
        &mut self,
        player_index: u8,
        card_name: String,
        zone_name: String,
        skip_triggers: bool,
    ) -> Result<u64, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }

        let query = card_name.trim();
        if query.is_empty() {
            return Err(JsValue::from_str("card name cannot be empty"));
        }

        let zone = zone_from_ui_name(&zone_name).map_err(|err| JsValue::from_str(&err))?;
        self.validate_commander_manual_zone_addition(zone)
            .map_err(|error| JsValue::from_str(&error))?;

        self.registry.ensure_cards_loaded([query]);
        let definition = self.load_compilable_card_definition(query)?;

        if zone == Zone::Battlefield && !skip_triggers {
            let checkpoint = self.capture_replay_checkpoint();
            let root = ReplayRoot::AddCardToZone {
                player: player_id,
                card_name: definition.name().to_string(),
                zone,
                skip_triggers,
            };
            let mut replay_dm = WasmReplayDecisionMaker::new(&[]);
            let add_result = self.add_card_to_zone_with_dm(
                player_id,
                &definition,
                zone,
                skip_triggers,
                &mut replay_dm,
            );
            let (pending_context, viewed_cards, audit_viewed_cards) = replay_dm.finish();
            self.active_viewed_cards = viewed_cards;
            self.active_audit_viewed_cards = audit_viewed_cards;

            if let Some(ctx) = pending_context {
                self.restore_replay_checkpoint(&checkpoint);
                self.pending_decision = Some(ctx);
                self.runner_pending_decision = false;
                self.pending_replay_action = Some(PendingReplayAction {
                    checkpoint,
                    root,
                    nested_answers: Vec::new(),
                });
                self.clear_active_resolving_stack_object();
                return Ok(0);
            }

            let object_id = add_result.map_err(|err| JsValue::from_str(&err))?;
            self.recompute_ui_decision()?;
            Ok(object_id)
        } else {
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let object_id = self
                .add_card_to_zone_with_dm(player_id, &definition, zone, skip_triggers, &mut dm)
                .map_err(|err| JsValue::from_str(&err))?;
            self.recompute_ui_decision()?;
            Ok(object_id)
        }
    }

    /// Add many cards to player zones and recompute UI state once.
    #[wasm_bindgen(js_name = addCardsToZones)]
    pub fn add_cards_to_zones(&mut self, cards_js: JsValue) -> Result<JsValue, JsValue> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AddCardToZoneInput {
            player_index: u8,
            card_name: String,
            zone_name: String,
            #[serde(default = "default_skip_triggers")]
            skip_triggers: bool,
        }

        struct ValidatedAddCardToZone {
            player_id: PlayerId,
            definition_index: usize,
            zone: Zone,
            skip_triggers: bool,
        }

        fn default_skip_triggers() -> bool {
            true
        }

        let cards: Vec<AddCardToZoneInput> = serde_wasm_bindgen::from_value(cards_js)
            .map_err(|e| JsValue::from_str(&format!("invalid addCardsToZones payload: {e}")))?;
        if cards.is_empty() {
            return serde_wasm_bindgen::to_value(&Vec::<u64>::new()).map_err(|e| {
                JsValue::from_str(&format!("failed to serialize addCardsToZones result: {e}"))
            });
        }

        let mut definition_queries: Vec<String> = Vec::new();
        for card in &cards {
            let query = card.card_name.trim();
            if query.is_empty() {
                return Err(JsValue::from_str("card name cannot be empty"));
            }
            if !definition_queries
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(query))
            {
                definition_queries.push(query.to_string());
            }
        }

        self.registry
            .ensure_cards_loaded(definition_queries.iter().map(String::as_str));
        let mut definitions: Vec<(String, CardDefinition)> = Vec::new();
        for query in &definition_queries {
            let definition = self.load_compilable_card_definition(query)?;
            definitions.push((query.clone(), definition));
        }

        let mut validated = Vec::with_capacity(cards.len());
        for card in &cards {
            let player_id = PlayerId::from_index(card.player_index);
            if self.game.player(player_id).is_none() {
                return Err(JsValue::from_str("invalid player index"));
            }
            let zone = zone_from_ui_name(&card.zone_name).map_err(|err| JsValue::from_str(&err))?;
            self.validate_commander_manual_zone_addition(zone)
                .map_err(|error| JsValue::from_str(&error))?;
            if zone == Zone::Battlefield && !card.skip_triggers {
                return Err(JsValue::from_str(
                    "addCardsToZones requires skipTriggers for battlefield cards",
                ));
            }
            let query = card.card_name.trim();
            let definition_index = definitions
                .iter()
                .position(|(candidate, _)| candidate.eq_ignore_ascii_case(query))
                .ok_or_else(|| JsValue::from_str(&format!("unknown card name: {query}")))?;
            validated.push(ValidatedAddCardToZone {
                player_id,
                definition_index,
                zone,
                skip_triggers: card.skip_triggers,
            });
        }

        let mut object_ids = Vec::with_capacity(validated.len());
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        for entry in validated {
            let definition = &definitions[entry.definition_index].1;
            if entry.skip_triggers {
                let object_id = self.game.create_object_from_catalog_definition(
                    definition,
                    &self.registry,
                    entry.player_id,
                    entry.zone,
                );
                if let Some(object) = self.game.object_mut(object_id) {
                    object.stable_id = StableId::from(object_id);
                }
                if entry.zone == Zone::Command {
                    self.game.set_as_commander(object_id, entry.player_id);
                }
                object_ids.push(object_id.0);
            } else {
                let object_id = self
                    .add_card_to_zone_with_dm(
                        entry.player_id,
                        definition,
                        entry.zone,
                        entry.skip_triggers,
                        &mut dm,
                    )
                    .map_err(|err| JsValue::from_str(&err))?;
                object_ids.push(object_id);
            }
        }
        self.recompute_ui_decision()?;
        serde_wasm_bindgen::to_value(&object_ids).map_err(|e| {
            JsValue::from_str(&format!("failed to serialize addCardsToZones result: {e}"))
        })
    }

    /// Set an explicit combat damage assignment for the next combat damage step.
    #[wasm_bindgen(js_name = setCombatDamageAssignment)]
    pub fn set_combat_damage_assignment(
        &mut self,
        attacker_id: u64,
        recipient_id: u64,
        amount: u32,
    ) {
        self.game.set_combat_damage_assignment(
            ObjectId::from_raw(attacker_id),
            ObjectId::from_raw(recipient_id),
            amount,
        );
    }

    /// Return the rules-defined chooser for a creature's combat-damage division.
    #[wasm_bindgen(js_name = combatDamageAssignmentPlayer)]
    pub fn combat_damage_assignment_player(&self, source_id: u64) -> Option<u8> {
        self.game
            .combat_damage_assignment_player(ObjectId::from_raw(source_id))
            .map(|player| player.0)
    }

    /// Set a combat-damage assignment on behalf of the rules-defined chooser.
    #[wasm_bindgen(js_name = setCombatDamageAssignmentForPlayer)]
    pub fn set_combat_damage_assignment_for_player(
        &mut self,
        assigning_player: u8,
        source_id: u64,
        recipient_id: u64,
        amount: u32,
    ) -> Result<(), JsValue> {
        self.game
            .set_combat_damage_assignment_for_player(
                PlayerId(assigning_player),
                ObjectId::from_raw(source_id),
                ObjectId::from_raw(recipient_id),
                amount,
            )
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Record an attacking band for the current combat.
    #[wasm_bindgen(js_name = setAttackingBand)]
    pub fn set_attacking_band(&mut self, member_ids: js_sys::Array) -> Result<(), JsValue> {
        let mut members = Vec::with_capacity(member_ids.length() as usize);
        for value in member_ids.iter() {
            let Some(id) = value.as_f64() else {
                return Err(JsValue::from_str(
                    "attacking band member id must be numeric",
                ));
            };
            members.push(ObjectId::from_raw(id as u64));
        }

        if let Some(runner) = self.runner.as_mut() {
            let result = ironsmith::combat_state::set_attacking_band(
                &self.game,
                runner.combat_mut(),
                members,
            );
            self.game.combat = Some(runner.combat().clone());
            return result.map_err(|err| JsValue::from_str(&err.to_string()));
        }

        let mut combat = self
            .game
            .combat
            .take()
            .ok_or_else(|| JsValue::from_str("no active combat to record attacking band"))?;
        let result = ironsmith::combat_state::set_attacking_band(&self.game, &mut combat, members);
        self.game.combat = Some(combat);
        result.map_err(|err| JsValue::from_str(&err.to_string()))
    }

    /// Draw opening hands for all players.
    #[wasm_bindgen(js_name = drawOpeningHands)]
    pub fn draw_opening_hands(&mut self, cards_per_player: usize) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for player_id in player_ids {
            let _ = self.game.draw_cards(player_id, cards_per_player);
        }
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Replace game state with demo decks and no battlefield/stack state.
    #[wasm_bindgen(js_name = loadDemoDecks)]
    pub fn load_demo_decks(&mut self) -> Result<(), JsValue> {
        let names: Vec<String> = self.game.players.iter().map(|p| p.name.clone()).collect();
        let starting_life = self.game.players.first().map_or(20, |p| p.life);
        let seed = deterministic_match_seed(
            &names,
            starting_life,
            MatchFormatInput::Normal,
            None,
            None,
            7,
        );
        self.initialize_empty_match(names, starting_life, seed);
        self.populate_demo_libraries()
            .map_err(|error| JsValue::from_str(&error))?;
        self.finish_match_setup(7)
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Load explicit decks by card name. JS format: `string[][]` or
    /// `{ decks: string[][], sideboards?: string[][] }`.
    ///
    /// Deck list index maps to player index.
    /// Returns a JSON object with total and categorized failures:
    /// `{ loaded, failed, failedBelowThreshold, failedToParse }`.
    /// Unknown cards are skipped rather than aborting the entire load.
    #[wasm_bindgen(js_name = loadDecks)]
    pub fn load_decks(&mut self, decks_js: JsValue) -> Result<JsValue, JsValue> {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum DeckLoadPayload {
            Decks(Vec<Vec<String>>),
            Structured {
                decks: Vec<Vec<String>>,
                #[serde(default)]
                sideboards: Vec<Vec<String>>,
            },
        }

        let payload: DeckLoadPayload = serde_wasm_bindgen::from_value(decks_js)
            .map_err(|e| JsValue::from_str(&format!("invalid decks payload: {e}")))?;
        let (decks, sideboards) = match payload {
            DeckLoadPayload::Decks(decks) => (decks, Vec::new()),
            DeckLoadPayload::Structured { decks, sideboards } => (decks, sideboards),
        };

        if decks.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "deck count must match number of players in game",
            ));
        }
        if !sideboards.is_empty() && sideboards.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "sideboard count must match number of players in game",
            ));
        }

        let names: Vec<String> = self.game.players.iter().map(|p| p.name.clone()).collect();
        let starting_life = self.game.players.first().map_or(20, |p| p.life);
        let seed = deterministic_match_seed(
            &names,
            starting_life,
            MatchFormatInput::Normal,
            Some(&decks),
            None,
            7,
        );

        let mut loaded: u32 = 0;
        let mut failed: Vec<String> = Vec::new();
        let mut failed_below_threshold: Vec<String> = Vec::new();
        let mut failed_to_parse: Vec<String> = Vec::new();
        let mut accepted_decks = vec![Vec::new(); decks.len()];
        let mut accepted_sideboards = vec![Vec::new(); decks.len()];

        for (player_index, deck) in decks.iter().enumerate() {
            self.registry
                .ensure_cards_loaded(deck.iter().map(|name| name.as_str()));
            if let Some(sideboard) = sideboards.get(player_index) {
                self.registry
                    .ensure_cards_loaded(sideboard.iter().map(|name| name.as_str()));
            }

            for name in deck {
                if let Some(definition) = self.find_card_definition(name).cloned() {
                    if self.semantic_threshold > 0.0
                        && let Some(score) = self.semantic_score_for_name(definition.name())
                        && score < self.semantic_threshold
                    {
                        failed.push(name.clone());
                        failed_below_threshold.push(name.clone());
                        continue;
                    }
                    accepted_decks[player_index].push(definition.name().to_string());
                    loaded += 1;
                } else {
                    failed.push(name.clone());
                    failed_to_parse.push(name.clone());
                }
            }

            if let Some(sideboard) = sideboards.get(player_index) {
                for name in sideboard {
                    if let Some(definition) = self.find_card_definition(name).cloned() {
                        if self.semantic_threshold > 0.0
                            && let Some(score) = self.semantic_score_for_name(definition.name())
                            && score < self.semantic_threshold
                        {
                            failed.push(name.clone());
                            failed_below_threshold.push(name.clone());
                            continue;
                        }
                        accepted_sideboards[player_index].push(definition.name().to_string());
                        loaded += 1;
                    } else {
                        failed.push(name.clone());
                        failed_to_parse.push(name.clone());
                    }
                }
            }
        }

        self.validate_normal_constructed_setup(
            names.len(),
            Some(&accepted_decks),
            Some(&accepted_sideboards),
            &[],
        )
        .map_err(|error| JsValue::from_str(&error))?;

        // Loading is a setup transaction: the live match is not replaced until
        // every accepted main deck and sideboard is legal.
        self.initialize_empty_match(names, starting_life, seed);
        self.populate_explicit_libraries(&accepted_decks)?;
        self.populate_explicit_sideboards(&accepted_sideboards)?;

        self.finish_match_setup(7)?;

        serde_wasm_bindgen::to_value(&DeckLoadResult {
            loaded,
            failed,
            failed_below_threshold,
            failed_to_parse,
        })
        .map_err(|e| JsValue::from_str(&format!("failed to serialize deck load result: {e}")))
    }

    #[wasm_bindgen(js_name = cardLoadDiagnostics)]
    pub fn card_load_diagnostics(
        &mut self,
        card_name: String,
        error_message: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let diagnostics = self.build_card_load_diagnostics(&card_name, error_message.as_deref());
        serde_wasm_bindgen::to_value(&diagnostics)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize card diagnostics: {e}")))
    }

    #[wasm_bindgen(js_name = sampleLoadedDeckSeed)]
    pub fn sample_loaded_deck_seed(&mut self, player_index: u8) -> Result<JsValue, JsValue> {
        let seed = self.build_loaded_deck_seed(player_index)?;
        serde_wasm_bindgen::to_value(&seed)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize custom card seed: {e}")))
    }

    #[wasm_bindgen(js_name = previewCustomCard)]
    pub fn preview_custom_card(&self, draft_js: JsValue) -> Result<JsValue, JsValue> {
        let draft: CustomCardInput = serde_wasm_bindgen::from_value(draft_js)
            .map_err(|e| JsValue::from_str(&format!("invalid custom card draft: {e}")))?;
        let preview = self.build_custom_card_preview(&draft)?;
        serde_wasm_bindgen::to_value(&preview).map_err(|e| {
            JsValue::from_str(&format!("failed to serialize custom card preview: {e}"))
        })
    }

    #[wasm_bindgen(js_name = createCustomCard)]
    pub fn create_custom_card(&mut self, payload_js: JsValue) -> Result<u64, JsValue> {
        let payload: CreateCustomCardInput = serde_wasm_bindgen::from_value(payload_js)
            .map_err(|e| JsValue::from_str(&format!("invalid custom card payload: {e}")))?;
        let player_id = PlayerId::from_index(payload.player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }

        let zone = match payload.zone_name.trim().to_lowercase().as_str() {
            "hand" => ironsmith::zone::Zone::Hand,
            "battlefield" => ironsmith::zone::Zone::Battlefield,
            "graveyard" => ironsmith::zone::Zone::Graveyard,
            "exile" => ironsmith::zone::Zone::Exile,
            "library" => ironsmith::zone::Zone::Library,
            "command" => ironsmith::zone::Zone::Command,
            "sideboard" | "outside_game" | "outside the game" => ironsmith::zone::Zone::OutsideGame,
            other => {
                return Err(JsValue::from_str(&format!("unknown zone: {other}")));
            }
        };
        self.validate_commander_manual_zone_addition(zone)
            .map_err(|error| JsValue::from_str(&error))?;

        let compiled = self.compile_custom_card_faces(&payload.draft)?;
        for definition in &compiled {
            self.registry.register(definition.clone());
            self.game.register_linked_face_definition(definition);
        }
        let Some(front) = compiled.first() else {
            return Err(JsValue::from_str("custom card draft produced no faces"));
        };

        let object_id = if payload.skip_triggers {
            let object_id = self.game.create_object_from_catalog_definition(
                front,
                &self.registry,
                player_id,
                zone,
            );
            if zone == Zone::Command {
                self.game.set_as_commander(object_id, player_id);
            }
            self.recompute_ui_decision()?;
            object_id
        } else {
            let object_id = self.add_definition_to_zone_with_triggers(front, player_id, zone)?;
            if zone == Zone::Command {
                self.game.set_as_commander(object_id, player_id);
            }
            object_id
        };

        Ok(object_id.0)
    }

    /// Advance to next phase (or next turn if ending phase).
    /// Resets the TurnRunner so it picks up from the new game state.
    #[wasm_bindgen(js_name = advancePhase)]
    pub fn advance_phase(&mut self) -> Result<(), JsValue> {
        ironsmith::turn::advance_step(&mut self.game)
            .map_err(|e| JsValue::from_str(&format!("advance_step failed: {e:?}")))?;
        self.runner = None;
        self.runner_awaiting_priority = false;
        self.runner_pending_decision = false;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Move directly into an inserted combat phase without rebuilding from a sync checkpoint.
    #[wasm_bindgen(js_name = enterAdditionalCombatPhase)]
    pub fn enter_additional_combat_phase(&mut self) -> Result<(), JsValue> {
        self.game.turn.phase = ironsmith::game_state::Phase::Combat;
        self.game.turn.step = Some(ironsmith::game_state::Step::BeginCombat);
        self.game.turn.priority_player = Some(self.game.turn.active_player);
        self.runner = None;
        self.runner_awaiting_priority = false;
        self.runner_pending_decision = false;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Toggle automatic cleanup discard (random cards).
    #[wasm_bindgen(js_name = setAutoCleanupDiscard)]
    pub fn set_auto_cleanup_discard(&mut self, enabled: bool) {
        self.auto_cleanup_discard = enabled;
    }

    /// Set the semantic similarity threshold for card addition (0..100%, 0 = off).
    #[wasm_bindgen(js_name = setSemanticThreshold)]
    pub fn set_semantic_threshold(&mut self, threshold: f32) {
        self.semantic_threshold = (threshold / 100.0).clamp(0.0, 1.0);
    }

    /// Get the current semantic threshold as percentage points.
    #[wasm_bindgen(js_name = getSemanticThreshold)]
    pub fn get_semantic_threshold(&self) -> f32 {
        self.semantic_threshold * 100.0
    }

    /// Get the semantic score for a specific card. Returns -1.0 if score is unavailable.
    #[wasm_bindgen(js_name = getCardSemanticScore)]
    pub fn get_card_semantic_score(&self, card_name: &str) -> f32 {
        self.semantic_score_for_name(card_name).unwrap_or(-1.0)
    }

    /// Get the count of scored cards meeting the current threshold.
    #[wasm_bindgen(js_name = cardsMeetingThreshold)]
    pub fn cards_meeting_threshold(&self) -> usize {
        if self.semantic_threshold <= 0.0 {
            return CardRegistry::generated_parser_semantic_scored_count();
        }
        let threshold_counts = CardRegistry::generated_parser_semantic_threshold_counts();
        let threshold_index = ((self.semantic_threshold * 100.0).ceil() as usize)
            .clamp(1, threshold_counts.len())
            - 1;
        threshold_counts[threshold_index]
    }

    /// Switch local perspective to the next player.
    #[wasm_bindgen(js_name = switchPerspective)]
    pub fn switch_perspective(&mut self) -> Result<u8, JsValue> {
        let current_index = self
            .game
            .players
            .iter()
            .position(|p| p.id == self.perspective)
            .unwrap_or(0);
        let next_index = (current_index + 1) % self.game.players.len().max(1);
        self.perspective = self.game.players[next_index].id;
        Ok(self.perspective.0)
    }

    /// Set local perspective explicitly.
    #[wasm_bindgen(js_name = setPerspective)]
    pub fn set_perspective(&mut self, player_index: u8) -> Result<(), JsValue> {
        let pid = PlayerId::from_index(player_index);
        if self.game.player(pid).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }
        self.perspective = pid;
        Ok(())
    }

    /// Cancel the current pending decision chain.
    ///
    /// Rollback preference:
    /// 1. The active user-action checkpoint (start of this spell/ability chain).
    /// 2. The active replay-action checkpoint (for speculative nested prompts).
    /// 3. The priority-epoch checkpoint (start of this priority round).
    ///
    /// This mirrors "take back this action chain" behavior first, while still
    /// preserving the broader epoch rollback as a fallback.
    #[wasm_bindgen(js_name = cancelDecision)]
    pub fn cancel_decision(&mut self) -> Result<JsValue, JsValue> {
        if !self.is_cancelable() {
            return Err(JsValue::from_str("current decision cannot be cancelled"));
        }
        if let Some(checkpoint) = self.pending_action_checkpoint.as_ref().cloned() {
            self.restore_replay_checkpoint(&checkpoint);
        } else if let Some(checkpoint) = self
            .pending_replay_action
            .as_ref()
            .map(|replay| replay.checkpoint.clone())
        {
            self.restore_replay_checkpoint(&checkpoint);
        } else if let Some(epoch) = self.priority_epoch_checkpoint.as_ref().cloned() {
            self.restore_replay_checkpoint(&epoch);
        }
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.pending_live_continuation = None;
        self.priority_epoch_has_undoable_action = false;
        self.priority_epoch_undo_locked_by_mana = false;
        self.priority_epoch_undo_land_stable_id = None;
        self.active_viewed_cards = None;
        self.active_audit_viewed_cards.clear();
        self.clear_active_resolving_stack_object();
        self.recompute_ui_decision()?;
        self.snapshot()
    }

    /// Apply a player command for the currently pending decision.
    #[wasm_bindgen]
    pub fn dispatch(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let dispatch_started_at = PerfTimer::start();
        self.last_dispatch_perf = None;
        if self.pending_priority_decision_is_stale() {
            self.recompute_stale_priority_decision()?;
            return Err(JsValue::from_str(
                "pending priority decision no longer matches the game priority holder",
            ));
        }
        let command_decode_started_at = PerfTimer::start();
        let command: UiCommand = serde_wasm_bindgen::from_value(command)
            .map_err(|e| JsValue::from_str(&format!("invalid command payload: {e}")))?;
        let command_decode_ms = command_decode_started_at.elapsed_ms();
        self.clear_active_resolving_stack_object();
        self.last_crypto_requirements.clear();
        self.pending_crypto_audit_before = Some(self.capture_crypto_audit_state());

        let pending_ctx = self
            .pending_decision
            .take()
            .ok_or_else(|| JsValue::from_str("no pending decision to dispatch"))?;
        if matches!(pending_ctx, DecisionContext::Priority(_)) {
            self.active_viewed_cards = None;
            self.active_audit_viewed_cards.clear();
        }
        let mut dispatch_perf = DispatchPerfMetrics {
            command_kind: ui_command_kind(&command).to_string(),
            pending_decision_kind: decision_context_kind(&pending_ctx).to_string(),
            command_decode_ms,
            ..DispatchPerfMetrics::default()
        };

        if self.pregame.is_some() {
            return self.dispatch_pregame_decision(pending_ctx, command);
        }

        // If this decision came from the TurnRunner, route through runner.respond_*()
        if self.runner_pending_decision {
            self.runner_pending_decision = false;
            return self.dispatch_runner_decision(pending_ctx, command);
        }

        if self.pending_live_continuation.is_some() {
            return self.dispatch_live_priority_continuation(pending_ctx, command);
        }

        if self.pending_replay_action.is_none()
            && self.decision_uses_live_priority_response(&pending_ctx)
        {
            return self.dispatch_live_priority_response(pending_ctx, command);
        }

        if let Some(mut replay) = self.pending_replay_action.take() {
            if replay_can_apply_legend_rule_choice_live(&replay.root, &pending_ctx) {
                let DecisionContext::SelectObjects(ref objects) = pending_ctx else {
                    unreachable!("legend-rule replay prompt should be select_objects");
                };
                let UiCommand::SelectObjects {
                    object_ids,
                    object_stable_ids,
                    object_hidden_refs,
                } = command
                else {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = Some(replay);
                    return Err(JsValue::from_str(
                        "unexpected command for legend rule decision",
                    ));
                };
                let object_ids = match normalize_select_object_choice_ids(
                    &self.game,
                    objects,
                    &object_ids,
                    &object_stable_ids,
                    &object_hidden_refs,
                ) {
                    Ok(ids) => ids,
                    Err(err) => {
                        self.pending_decision = Some(pending_ctx);
                        self.pending_replay_action = Some(replay);
                        return Err(err);
                    }
                };
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|obj| obj.legal)
                    .map(|obj| obj.id.0)
                    .collect();
                if let Err(err) = validate_object_selection(
                    objects.min,
                    objects.max,
                    objects.allow_partial_completion,
                    &object_ids,
                    &legal_ids,
                ) {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = Some(replay);
                    return Err(err);
                }
                let Some(keep_id) = object_ids.first().copied().map(ObjectId::from_raw) else {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = Some(replay);
                    return Err(JsValue::from_str("legend rule requires one chosen object"));
                };
                let legend_group = legal_ids
                    .iter()
                    .copied()
                    .map(ObjectId::from_raw)
                    .collect::<Vec<_>>();
                ironsmith::rules::state_based::apply_legend_rule_choice_from_group(
                    &mut self.game,
                    keep_id,
                    &legend_group,
                );
                drain_pending_trigger_events(&mut self.game, &mut self.trigger_queue);
                self.pending_action_checkpoint = None;
                self.pending_replay_action = None;
                self.pending_decision = None;
                self.clear_active_resolving_stack_object();
                if let Err(err) = self.advance_until_decision() {
                    self.restore_replay_checkpoint(&replay.checkpoint);
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = Some(replay);
                    return Err(err);
                }
                return self.snapshot();
            }
            let answer = match self.command_to_replay_answer(&pending_ctx, command.clone()) {
                Ok(answer) => answer,
                Err(err) => {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = Some(replay);
                    return Err(err);
                }
            };
            replay.nested_answers.push(answer);
            let should_track_action_checkpoint = self.pending_action_checkpoint.is_none()
                && Self::replay_answers_start_cancelable_action_chain(
                    &replay.root,
                    &replay.nested_answers,
                );
            let live_checkpoint = self.capture_replay_checkpoint();
            let progress = if self.decision_requires_root_reexecution(&pending_ctx) {
                match self.execute_with_replay(
                    &replay.checkpoint,
                    &replay.root,
                    &replay.nested_answers,
                ) {
                    Ok(ReplayOutcome::NeedsDecision(next_ctx)) => {
                        if should_track_action_checkpoint {
                            self.pending_action_checkpoint = Some(replay.checkpoint.clone());
                        }
                        self.pending_decision = Some(next_ctx);
                        self.pending_replay_action = Some(replay);
                        return self.snapshot();
                    }
                    Ok(ReplayOutcome::Complete(progress)) => progress,
                    Err(err) => {
                        self.restore_replay_checkpoint(&live_checkpoint);
                        self.pending_decision = Some(pending_ctx);
                        self.pending_replay_action = Some(replay);
                        return Err(err);
                    }
                }
            } else {
                let response = match self.command_to_response(&pending_ctx, command) {
                    Ok(response) => response,
                    Err(err) => {
                        self.pending_decision = Some(pending_ctx);
                        self.pending_replay_action = Some(replay);
                        return Err(err);
                    }
                };
                let carry_viewed_cards = self.active_viewed_cards.clone();
                let mut live_dm = WasmReplayDecisionMaker::new(&[]);
                let result = apply_priority_response_with_dm(
                    &mut self.game,
                    &mut self.trigger_queue,
                    &mut self.priority_state,
                    &response,
                    &mut live_dm,
                );
                let (pending_context, viewed_cards, audit_viewed_cards) = live_dm.finish();
                self.active_viewed_cards =
                    merge_carried_active_viewed_cards(carry_viewed_cards, viewed_cards);
                self.active_audit_viewed_cards = audit_viewed_cards;

                if let Some(next_ctx) = pending_context {
                    self.sync_active_resolving_stack_object_for_prompt(Some(&live_checkpoint));
                    if should_track_action_checkpoint {
                        self.pending_action_checkpoint = Some(replay.checkpoint.clone());
                    }
                    if self.decision_requires_root_reexecution(&next_ctx) {
                        self.priority_state.pending_continuation = None;
                        self.pending_live_continuation = Some(LivePriorityContinuation {
                            checkpoint: live_checkpoint,
                            root: PendingPriorityContinuation::ApplyResponse(response),
                            answers: Vec::new(),
                            speculative_progress: match (&next_ctx, &result) {
                                (DecisionContext::Boolean(_), Ok(progress)) => {
                                    Some(progress.clone())
                                }
                                _ => None,
                            },
                        });
                        self.pending_decision = Some(next_ctx);
                        self.pending_replay_action = None;
                        return self.snapshot();
                    }
                    self.pending_decision = Some(next_ctx);
                    self.pending_replay_action = Some(replay);
                    return self.snapshot();
                }

                match result {
                    Ok(progress) => progress,
                    Err(err) => {
                        self.restore_replay_checkpoint(&live_checkpoint);
                        self.pending_decision = Some(pending_ctx);
                        self.pending_replay_action = Some(replay);
                        return Err(JsValue::from_str(&format!("dispatch failed: {err}")));
                    }
                }
            };

            match progress {
                GameProgress::NeedsDecisionCtx(next_ctx) => {
                    self.sync_active_resolving_stack_object_for_prompt(Some(&live_checkpoint));
                    if self.priority_action_chain_still_pending() {
                        if should_track_action_checkpoint {
                            self.pending_action_checkpoint = Some(replay.checkpoint.clone());
                        }
                        self.pending_decision = Some(next_ctx);
                        self.pending_replay_action = Some(replay);
                        return self.snapshot();
                    }

                    // The spell/ability is now committed. Follow-up prompts
                    // produced during resolution must not preserve Undo for
                    // the action that just finished paying its costs.
                    self.pending_action_checkpoint = None;
                    self.pending_decision = Some(next_ctx);
                    self.pending_replay_action = Some(replay);
                    self.snapshot()
                }
                progress => {
                    self.clear_active_resolving_stack_object();
                    if Self::replay_root_starts_undoable_action(&replay.root) {
                        self.priority_epoch_has_undoable_action = true;
                    }
                    if self.replay_chain_has_irreversible_mana_activation(&replay)
                        || self.replay_root_mana_activation_added_to_stack(
                            &replay.checkpoint,
                            &replay.root,
                        )
                    {
                        self.priority_epoch_undo_locked_by_mana = true;
                    }
                    self.priority_epoch_undo_land_stable_id =
                        self.committed_undo_land_stable_id(&replay.checkpoint, &replay.root);
                    self.pending_action_checkpoint = None;
                    self.pending_replay_action = None;
                    self.apply_progress(progress)?;
                    self.snapshot()
                }
            }
        } else {
            dispatch_perf.route_kind = "fresh_response".to_string();
            let command_to_response_started_at = PerfTimer::start();
            let response = match self.command_to_response(&pending_ctx, command) {
                Ok(response) => response,
                Err(err) => {
                    self.pending_decision = Some(pending_ctx);
                    dispatch_perf.outcome_kind = "command_to_response_error".to_string();
                    self.store_dispatch_perf(dispatch_started_at, dispatch_perf);
                    return Err(err);
                }
            };
            dispatch_perf.command_to_response_ms = command_to_response_started_at.elapsed_ms();

            let checkpoint_started_at = PerfTimer::start();
            let checkpoint = self.capture_replay_checkpoint();
            dispatch_perf.checkpoint_capture_ms = checkpoint_started_at.elapsed_ms();
            let should_track_action_checkpoint = self.pending_action_checkpoint.is_none()
                && Self::response_starts_cancelable_action_chain(&response);
            let root = ReplayRoot::Response(response);
            let execute_started_at = PerfTimer::start();
            let outcome = match self.execute_with_replay(&checkpoint, &root, &[]) {
                Ok(outcome) => outcome,
                Err(err) => {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = None;
                    dispatch_perf.execute_with_replay_ms = execute_started_at.elapsed_ms();
                    dispatch_perf.replay_execution = self.last_replay_execution_perf.clone();
                    dispatch_perf.outcome_kind = "execute_with_replay_error".to_string();
                    self.store_dispatch_perf(dispatch_started_at, dispatch_perf);
                    return Err(err);
                }
            };
            dispatch_perf.execute_with_replay_ms = execute_started_at.elapsed_ms();
            dispatch_perf.replay_execution = self.last_replay_execution_perf.clone();
            match outcome {
                ReplayOutcome::NeedsDecision(next_ctx) => {
                    self.sync_active_resolving_stack_object_for_prompt(Some(&checkpoint));
                    if should_track_action_checkpoint {
                        self.pending_action_checkpoint = Some(checkpoint.clone());
                    }
                    self.pending_decision = Some(next_ctx);
                    self.pending_replay_action = Some(PendingReplayAction {
                        checkpoint,
                        root,
                        nested_answers: Vec::new(),
                    });
                    dispatch_perf.outcome_kind = "needs_decision".to_string();
                    self.finish_dispatch_with_snapshot(dispatch_started_at, dispatch_perf)
                }
                ReplayOutcome::Complete(progress) => {
                    match progress {
                        GameProgress::NeedsDecisionCtx(next_ctx) => {
                            self.sync_active_resolving_stack_object_for_prompt(Some(&checkpoint));
                            if self.priority_action_chain_still_pending() {
                                if should_track_action_checkpoint {
                                    self.pending_action_checkpoint = Some(checkpoint.clone());
                                }
                                self.pending_decision = Some(next_ctx);
                                self.pending_replay_action = Some(PendingReplayAction {
                                    checkpoint,
                                    root,
                                    nested_answers: Vec::new(),
                                });
                                dispatch_perf.outcome_kind =
                                    "complete_pending_needs_decision".to_string();
                                return self.finish_dispatch_with_snapshot(
                                    dispatch_started_at,
                                    dispatch_perf,
                                );
                            }

                            // The spell/ability is now committed. Follow-up prompts
                            // produced during resolution must not preserve Undo for
                            // the action that just finished paying its costs.
                            self.pending_action_checkpoint = None;
                            self.pending_decision = Some(next_ctx);
                            self.pending_replay_action = Some(PendingReplayAction {
                                checkpoint,
                                root,
                                nested_answers: Vec::new(),
                            });
                            dispatch_perf.outcome_kind = "complete_resolution_prompt".to_string();
                            self.finish_dispatch_with_snapshot(dispatch_started_at, dispatch_perf)
                        }
                        progress => {
                            self.clear_active_resolving_stack_object();
                            if Self::replay_root_starts_undoable_action(&root) {
                                self.priority_epoch_has_undoable_action = true;
                            }
                            if Self::replay_root_has_irreversible_mana_activation(
                                &checkpoint.game,
                                &root,
                            ) || self
                                .replay_root_mana_activation_added_to_stack(&checkpoint, &root)
                            {
                                self.priority_epoch_undo_locked_by_mana = true;
                            }
                            self.priority_epoch_undo_land_stable_id =
                                self.committed_undo_land_stable_id(&checkpoint, &root);
                            self.pending_action_checkpoint = None;
                            self.pending_replay_action = None;
                            let apply_progress_started_at = PerfTimer::start();
                            self.apply_progress(progress)?;
                            dispatch_perf.apply_progress_ms =
                                apply_progress_started_at.elapsed_ms();
                            dispatch_perf.advance_until_decision =
                                self.last_advance_until_decision_perf.clone();
                            dispatch_perf.outcome_kind = "complete_progress".to_string();
                            self.finish_dispatch_with_snapshot(dispatch_started_at, dispatch_perf)
                        }
                    }
                }
            }
        }
    }
}
