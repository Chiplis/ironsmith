// A local transaction retains live continuations, unlike a wire checkpoint.
// Registries remain shared; reusable analysis follows its owning branch.
#[derive(Default)]
struct RuntimeBranchAnalysis {
    // Revision keys only identify a state within one branch. Move this memo
    // with its game so equal revisions on diverged branches cannot alias.
    mana_inventory: Option<(u64, Vec<ManaActivationOptionView>)>,
    priority: Option<Box<PriorityAnalysisJob>>,
    payment: Option<Box<PaymentAnalysisJob>>,
    inspector: Option<Box<InspectorAnalysisJob>>,
    last_slice_nodes: usize,
    object_views: Box<SnapshotObjectViewCache>,
    #[cfg(target_arch = "wasm32")]
    js_encoding: SnapshotJsEncodingCache,
}

macro_rules! runtime_savepoint {
    ($($field:ident: $kind:ty),* $(,)?) => {
        pub(super) struct RuntimeSavepoint {
            $($field: $kind,)*
            id_counters: ironsmith::ids::IdCountersSnapshot,
            analysis: RuntimeBranchAnalysis,
        }
        impl Clone for RuntimeSavepoint {
            fn clone(&self) -> Self {
                Self { $($field: self.$field.clone(),)* id_counters: self.id_counters,
                    analysis: RuntimeBranchAnalysis {
                        mana_inventory: self.analysis.mana_inventory.clone(),
                        ..Default::default()
                    } }
            }
        }
        impl RuntimeSavepoint {
            fn capture(game: &WasmGame) -> Self {
                Self { $($field: game.$field.clone(),)* id_counters: snapshot_id_counters(),
                    analysis: RuntimeBranchAnalysis {
                        mana_inventory: game.mana_activation_inventory_cache.borrow().clone(),
                        ..Default::default()
                    } }
            }
            fn restore(self, game: &mut WasmGame) {
                $(game.$field = self.$field;)*
                // Source registration is shared session state, not part of a
                // game transaction. Never reuse a definition ID allocated while
                // preparing a command; only gameplay IDs rewind exactly.
                let mut ids = self.id_counters;
                ids.card = ids.card.max(snapshot_id_counters().card);
                restore_id_counters(ids);
                *game.mana_activation_inventory_cache.get_mut() = self.analysis.mana_inventory;
                game.priority_analysis_job = None;
                game.payment_analysis_job = None;
                game.inspector_analysis_job = None;
                game.snapshot_object_view_cache = Box::default();
                #[cfg(target_arch = "wasm32")]
                { game.snapshot_js_encoding_cache = SnapshotJsEncodingCache::default(); }
            }
            fn exchange(&mut self, game: &mut WasmGame) {
                $(std::mem::swap(&mut self.$field, &mut game.$field);)*
                std::mem::swap(&mut self.analysis.mana_inventory, game.mana_activation_inventory_cache.get_mut());
                let current = snapshot_id_counters();
                let mut incoming = self.id_counters;
                incoming.card = incoming.card.max(current.card);
                restore_id_counters(incoming);
                self.id_counters = current;
                std::mem::swap(&mut self.analysis.priority, &mut game.priority_analysis_job);
                std::mem::swap(&mut self.analysis.payment, &mut game.payment_analysis_job);
                std::mem::swap(&mut self.analysis.inspector, &mut game.inspector_analysis_job);
                std::mem::swap(&mut self.analysis.last_slice_nodes, &mut game.last_analysis_slice_nodes);
                std::mem::swap(&mut self.analysis.object_views, &mut game.snapshot_object_view_cache);
                #[cfg(target_arch = "wasm32")]
                { std::mem::swap(&mut self.analysis.js_encoding, &mut game.snapshot_js_encoding_cache); }
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
    pending_decision_game: Option<Box<GameState>>,
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
    last_snapshot_perf: Option<SnapshotPerfMetrics>,
    last_replay_execution_perf: Option<ReplayExecutionPerfMetrics>,
    last_advance_until_decision_perf: Option<AdvanceUntilDecisionPerfMetrics>,
    dispatch_advance_until_decision_perfs: Vec<AdvanceUntilDecisionPerfMetrics>,
    last_dispatch_perf: Option<DispatchPerfMetrics>,
    manabrew_game_id: String,
    manabrew_human_players: Vec<bool>,
    manabrew_next_prompt_id: u32,
    manabrew_open_prompt: Option<ManabrewOpenPrompt>,
    cached_snapshot: Option<CachedSnapshot>,
}

impl WasmGame {
    /// Execute on a candidate branch while retaining the original runtime and
    /// its analysis jobs. Session catalog registrations remain shared.
    fn with_runtime_transaction<T, E>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut previous = RuntimeSavepoint::capture(self);
        previous.exchange(self);
        match operation(self) {
            Ok(value) => Ok(value),
            Err(error) => {
                previous.exchange(self);
                Err(error)
            }
        }
    }
}

#[wasm_bindgen]
impl WasmGame {
    /// Copy a retained branch into the visible runtime, preserving the branch.
    #[wasm_bindgen(js_name = copyRuntimeSavepoint)]
    pub fn copy_runtime_savepoint(&mut self, handle: u32) -> Result<JsValue, JsValue> {
        let point = self.runtime_savepoints.get(&handle)
            .ok_or_else(|| JsValue::from_str("runtime branch has expired"))?.clone();
        point.restore(self);
        self.snapshot()
    }

    /// Switch between two lossless runtime branches without copying the game.
    /// The handle retains the previously active branch and remains reusable.
    #[wasm_bindgen(js_name = exchangeRuntimeSavepoint)]
    pub fn exchange_runtime_savepoint(&mut self, handle: u32) -> Result<(), JsValue> {
        let mut point = self.runtime_savepoints.remove(&handle)
            .ok_or_else(|| JsValue::from_str("runtime branch has expired"))?;
        point.exchange(self);
        self.runtime_savepoints.insert(handle, point);
        Ok(())
    }

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
