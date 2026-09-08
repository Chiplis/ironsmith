// A local transaction retains live continuations, unlike a wire checkpoint.
// Registries and pure memo/analysis caches are not transactional game state.
macro_rules! runtime_savepoint {
    ($($field:ident: $kind:ty),* $(,)?) => {
        pub(super) struct RuntimeSavepoint {
            $($field: $kind,)*
            id_counters: ironsmith::ids::IdCountersSnapshot,
        }
        impl RuntimeSavepoint {
            fn capture(game: &WasmGame) -> Self {
                Self { $($field: game.$field.clone(),)* id_counters: snapshot_id_counters() }
            }
            fn restore(self, game: &mut WasmGame) {
                $(game.$field = self.$field;)*
                // Source registration is shared session state, not part of a
                // game transaction. Never reuse a definition ID allocated while
                // preparing a command; only gameplay IDs rewind exactly.
                let mut ids = self.id_counters;
                ids.card = ids.card.max(snapshot_id_counters().card);
                restore_id_counters(ids);
                game.priority_analysis_job = None;
                game.inspector_analysis_job = None;
                game.snapshot_object_view_cache = Box::default();
                #[cfg(target_arch = "wasm32")]
                { game.snapshot_js_encoding_cache = SnapshotJsEncodingCache::default(); }
            }
        }
    };
}
runtime_savepoint! {
    game: GameState,
    trigger_queue: TriggerQueue,
    priority_state: PriorityLoopState,
    pregame: Option<PregameState>,
    match_format: MatchFormatInput,
    pending_decision: Option<DecisionContext>,
    pending_replay_action: Option<PendingReplayAction>,
    pending_action_checkpoint: Option<ReplayCheckpoint>,
    pending_live_action_root: Option<PriorityResponse>,
    pending_live_continuation: Option<LivePriorityContinuation>,
    game_over: Option<GameResult>,
    perspective: PlayerId,
    runner: Option<ironsmith::turn_runner::TurnRunner>,
    grand_melee_host_lanes: HashMap<u32, GrandMeleeHostLane>,
    suspended_subgame_hosts: Vec<(Option<ironsmith::turn_runner::TurnRunner>, bool, TriggerQueue, PriorityLoopState, HashMap<u32, GrandMeleeHostLane>)>,
    runner_awaiting_priority: bool,
    runner_pending_decision: bool,
    auto_cleanup_discard: bool,
    priority_epoch_checkpoint: Option<ReplayCheckpoint>,
    priority_epoch_has_undoable_action: bool,
    priority_epoch_undo_locked_by_mana: bool,
    priority_epoch_undo_land_stable_id: Option<u64>,
    semantic_threshold: f32,
    snapshot_serial: u64,
    active_viewed_cards: Option<ActiveViewedCards>,
    active_audit_viewed_cards: Vec<ActiveViewedCards>,
    last_crypto_requirements: Vec<CryptoRequirementView>,
    pending_crypto_audit_before: Option<CryptoAuditState>,
    active_resolving_stack_object: Option<StackObjectSnapshot>,
    loaded_decks: Vec<Vec<String>>,
    manabrew_game_id: String,
    manabrew_human_players: Vec<bool>,
    manabrew_next_prompt_id: u32,
    manabrew_open_prompt: Option<ManabrewOpenPrompt>,
    cached_snapshot: Option<CachedSnapshot>,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = createRuntimeSavepoint)]
    pub fn create_runtime_savepoint(&mut self) -> Result<u32, JsValue> {
        if self.runtime_savepoints.len() >= 16 {
            return Err(JsValue::from_str("too many live runtime savepoints"));
        }
        let handle = self.next_runtime_savepoint.checked_add(1)
            .ok_or_else(|| JsValue::from_str("runtime savepoint handles exhausted"))?;
        self.next_runtime_savepoint = handle;
        self.runtime_savepoints.insert(handle, Box::new(RuntimeSavepoint::capture(self)));
        Ok(handle)
    }

    #[wasm_bindgen(js_name = restoreRuntimeSavepoint)]
    pub fn restore_runtime_savepoint(&mut self, handle: u32) -> Result<JsValue, JsValue> {
        let checkpoint = self.runtime_savepoints.remove(&handle)
            .ok_or_else(|| JsValue::from_str("runtime savepoint has expired"))?;
        checkpoint.restore(self);
        self.snapshot()
    }

    #[wasm_bindgen(js_name = releaseRuntimeSavepoint)]
    pub fn release_runtime_savepoint(&mut self, handle: u32) -> bool {
        self.runtime_savepoints.remove(&handle).is_some()
    }
}
