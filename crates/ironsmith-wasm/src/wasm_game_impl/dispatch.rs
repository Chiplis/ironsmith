#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
impl WasmGame {
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
            active_resolving_stack_object: None,
            loaded_decks: Vec::new(),
            last_snapshot_perf: None,
            last_replay_execution_perf: None,
            last_advance_until_decision_perf: None,
            last_dispatch_perf: None,
        }
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
        self.populate_demo_libraries()?;
        self.finish_match_setup(7)
    }

    /// Start a fully specified match from a synchronized lobby payload.
    #[wasm_bindgen(js_name = startMatch)]
    pub fn start_match(&mut self, config: JsValue) -> Result<JsValue, JsValue> {
        let config: MatchSetupInput = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("invalid match config: {e}")))?;

        if config.player_names.is_empty() {
            return Err(JsValue::from_str("player_names cannot be empty"));
        }

        let opening_hand_size = config.opening_hand_size.unwrap_or(7);
        self.initialize_empty_match(config.player_names, config.starting_life, config.seed);
        self.match_format = config.format;

        if let MatchFormatInput::Commander = config.format {
            let Some(decks) = config.decks.as_ref() else {
                return Err(JsValue::from_str(
                    "commander matches require explicit decklists",
                ));
            };
            let Some(commanders) = config.commanders.as_ref() else {
                return Err(JsValue::from_str(
                    "commander matches require commander lists",
                ));
            };
            self.validate_commander_setup(decks, commanders)?;
        }

        if let Some(decks) = config.decks {
            if decks.len() != self.game.players.len() {
                return Err(JsValue::from_str(
                    "deck count must match number of players in game",
                ));
            }
            self.populate_explicit_libraries(&decks)?;
        } else {
            self.populate_demo_libraries()?;
        }

        if let Some(commanders) = config.commanders {
            if commanders.len() != self.game.players.len() {
                return Err(JsValue::from_str(
                    "commander count must match number of players in game",
                ));
            }
            self.populate_explicit_commanders(&commanders)?;
        }

        self.finish_match_setup(opening_hand_size)?;
        self.snapshot()
    }

    #[wasm_bindgen(js_name = validateMatchConfig)]
    pub fn validate_match_config(&mut self, config: JsValue) -> Result<JsValue, JsValue> {
        let config: MatchSetupInput = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("invalid match config: {e}")))?;
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
        self.snapshot_serial = self.snapshot_serial.saturating_add(1);
        let snapshot_id = self.snapshot_serial;
        let transitions_started_at = PerfTimer::start();
        let battlefield_transitions =
            battlefield_transition_snapshots(self.game.take_ui_battlefield_transitions());
        let battlefield_transition_ms = transitions_started_at.elapsed_ms();
        let build_started_at = PerfTimer::start();
        let mut snap = GameSnapshot::from_game(
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
        );
        let snapshot_build_ms = build_started_at.elapsed_ms();
        let pending_insert_started_at = PerfTimer::start();
        insert_pending_stack_object_snapshots(&mut snap, self.pending_trigger_stack_objects());
        let pending_stack_insert_ms = pending_insert_started_at.elapsed_ms();
        let player_count = snap.players.len();
        let battlefield_size = snap.battlefield_size;
        let stack_size = snap.stack_size;
        let encode_started_at = PerfTimer::start();
        let encoded = serde_wasm_bindgen::to_value(&snap)
            .map_err(|e| JsValue::from_str(&format!("snapshot encode failed: {e}")))?;
        let snapshot_encode_ms = encode_started_at.elapsed_ms();
        let total_snapshot_ms = snapshot_started_at.elapsed_ms();
        self.last_snapshot_perf = Some(SnapshotPerfMetrics {
            snapshot_id,
            battlefield_transition_ms,
            snapshot_build_ms,
            pending_stack_insert_ms,
            snapshot_encode_ms,
            total_snapshot_ms,
            player_count,
            battlefield_size,
            stack_size,
        });
        Ok(encoded)
    }

    /// Return the current UI state from the selected player perspective.
    #[wasm_bindgen(js_name = uiState)]
    pub fn ui_state(&mut self) -> Result<JsValue, JsValue> {
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
        let details = build_object_details_snapshot(&self.game, ObjectId::from_raw(object_id))
            .ok_or_else(|| JsValue::from_str(&format!("unknown object id: {object_id}")))?;
        serde_wasm_bindgen::to_value(&details)
            .map_err(|e| JsValue::from_str(&format!("objectDetails encode failed: {e}")))
    }

    /// Return game snapshot as pretty JSON.
    #[wasm_bindgen(js_name = snapshotJson)]
    pub fn snapshot_json(&mut self) -> Result<String, JsValue> {
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
        let mut snap = GameSnapshot::from_game(
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
        );
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
                    && let Some(score) = Self::semantic_score_for_name(name.as_str())
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
        if skip_triggers {
            let object_id = self
                .game
                .create_object_from_catalog_definition(definition, &self.registry, player_id, zone);
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

                if self
                    .game
                    .object(entered_id)
                    .is_some_and(|obj| obj.subtypes.contains(&Subtype::Saga))
                {
                    ironsmith::game_loop::add_lore_counter_and_check_chapters(
                        &mut self.game,
                        entered_id,
                        &mut self.trigger_queue,
                    );
                }
            }

            entered_id
        } else {
            self.game
                .move_object_by_effect(temp_id, zone)
                .unwrap_or(temp_id)
        };
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

        let zone = match zone_name.trim().to_lowercase().as_str() {
            "hand" => ironsmith::zone::Zone::Hand,
            "battlefield" => ironsmith::zone::Zone::Battlefield,
            "graveyard" => ironsmith::zone::Zone::Graveyard,
            "exile" => ironsmith::zone::Zone::Exile,
            "library" => ironsmith::zone::Zone::Library,
            "command" => ironsmith::zone::Zone::Command,
            other => {
                return Err(JsValue::from_str(&format!("unknown zone: {other}")));
            }
        };

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
            let (pending_context, viewed_cards) = replay_dm.finish();
            self.active_viewed_cards = viewed_cards;

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
        self.populate_demo_libraries()?;
        self.finish_match_setup(7)
    }

    /// Load explicit decks by card name. JS format: `string[][]`.
    ///
    /// Deck list index maps to player index.
    /// Returns a JSON object with total and categorized failures:
    /// `{ loaded, failed, failedBelowThreshold, failedToParse }`.
    /// Unknown cards are skipped rather than aborting the entire load.
    #[wasm_bindgen(js_name = loadDecks)]
    pub fn load_decks(&mut self, decks_js: JsValue) -> Result<JsValue, JsValue> {
        let decks: Vec<Vec<String>> = serde_wasm_bindgen::from_value(decks_js)
            .map_err(|e| JsValue::from_str(&format!("invalid decks payload: {e}")))?;

        if decks.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "deck count must match number of players in game",
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
        self.initialize_empty_match(names, starting_life, seed);

        let mut loaded: u32 = 0;
        let mut failed: Vec<String> = Vec::new();
        let mut failed_below_threshold: Vec<String> = Vec::new();
        let mut failed_to_parse: Vec<String> = Vec::new();

        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, deck) in player_ids.iter().zip(decks.iter()) {
            self.registry
                .ensure_cards_loaded(deck.iter().map(|name| name.as_str()));

            for name in deck {
                if let Some(definition) = self.find_card_definition(name).cloned() {
                    if self.semantic_threshold > 0.0
                        && let Some(score) = Self::semantic_score_for_name(definition.name())
                        && score < self.semantic_threshold
                    {
                        failed.push(name.clone());
                        failed_below_threshold.push(name.clone());
                        continue;
                    }
                    self.game.create_object_from_catalog_definition(
                        &definition,
                        &self.registry,
                        player_id,
                        ironsmith::zone::Zone::Library,
                    );
                    loaded += 1;
                } else {
                    failed.push(name.clone());
                    failed_to_parse.push(name.clone());
                }
            }

            self.game.shuffle_player_library(player_id);
        }

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
            other => {
                return Err(JsValue::from_str(&format!("unknown zone: {other}")));
            }
        };

        let compiled = self.compile_custom_card_faces(&payload.draft)?;
        for definition in &compiled {
            self.registry.register(definition.clone());
            self.game.register_linked_face_definition(definition);
        }
        let Some(front) = compiled.first() else {
            return Err(JsValue::from_str("custom card draft produced no faces"));
        };

        let object_id = if payload.skip_triggers {
            let object_id =
                self.game
                    .create_object_from_catalog_definition(front, &self.registry, player_id, zone);
            self.recompute_ui_decision()?;
            object_id
        } else {
            self.add_definition_to_zone_with_triggers(front, player_id, zone)?
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
        Self::semantic_score_for_name(card_name).unwrap_or(-1.0)
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
        self.clear_active_resolving_stack_object();
        self.recompute_ui_decision()?;
        self.snapshot()
    }

    /// Apply a player command for the currently pending decision.
    #[wasm_bindgen]
    pub fn dispatch(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let dispatch_started_at = PerfTimer::start();
        self.last_dispatch_perf = None;
        let command_decode_started_at = PerfTimer::start();
        let command: UiCommand = serde_wasm_bindgen::from_value(command)
            .map_err(|e| JsValue::from_str(&format!("invalid command payload: {e}")))?;
        let command_decode_ms = command_decode_started_at.elapsed_ms();
        self.clear_active_resolving_stack_object();

        let pending_ctx = self
            .pending_decision
            .take()
            .ok_or_else(|| JsValue::from_str("no pending decision to dispatch"))?;
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
                let mut live_dm = WasmReplayDecisionMaker::new(&[]);
                let result = apply_priority_response_with_dm(
                    &mut self.game,
                    &mut self.trigger_queue,
                    &mut self.priority_state,
                    &response,
                    &mut live_dm,
                );
                let (pending_context, viewed_cards) = live_dm.finish();
                self.active_viewed_cards = viewed_cards;

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
