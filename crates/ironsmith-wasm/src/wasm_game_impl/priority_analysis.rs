/// An exact owned snapshot; no checkpoint reconstruction or live-state writes
/// occur while the search is running. The worker yields between step calls.
pub(super) struct PriorityAnalysisJob {
    token: String,
    key: SnapshotCacheKey,
    game: GameState,
    player: PlayerId,
    session: ironsmith::decision::ManaAnalysisSession,
}

impl WasmGame {
    fn advance_priority_analysis(&mut self, token: &str, budget: usize) -> Option<bool> {
        let Some(mut job) = self.priority_analysis_job.take() else {
            return None;
        };
        if job.token != token || job.key != self.priority_analysis_key() {
            return None;
        }
        let id_counters = snapshot_id_counters();
        let (ctx, complete) = job.session.run(budget.clamp(1, 4096), || {
            ironsmith::game_loop::analyze_priority_context(&job.game, job.player)
        });
        restore_id_counters(id_counters);
        if !complete {
            self.priority_analysis_job = Some(job);
            return Some(false);
        }
        self.pending_decision = Some(DecisionContext::Priority(ctx));
        self.cached_snapshot = None;
        Some(true)
    }

    fn priority_analysis_key(&self) -> SnapshotCacheKey {
        self.snapshot_cache_key(None, false, None, &None)
    }
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = setDeferredPriorityAnalysis)]
    pub fn set_deferred_priority_analysis(&mut self, enabled: bool) {
        ironsmith::game_loop::set_priority_analysis_deferred(enabled);
        self.priority_analysis_job = None;
        self.inspector_analysis_job = None;
    }

    #[wasm_bindgen(js_name = priorityAnalysisIdentity)]
    pub fn priority_analysis_identity(&self) -> String {
        format!("{:?}", self.priority_analysis_key())
    }

    #[wasm_bindgen(js_name = cancelPriorityAnalysis)]
    pub fn cancel_priority_analysis(&mut self) {
        self.priority_analysis_job = None;
        self.inspector_analysis_job = None;
    }

    #[wasm_bindgen(js_name = beginPriorityAnalysis)]
    pub fn begin_priority_analysis(&mut self, token: String) -> bool {
        let Some(DecisionContext::Priority(ctx)) = self.pending_decision.as_ref() else {
            return false;
        };
        if ctx.analysis_complete || self.pregame.is_some() {
            return false;
        }
        self.priority_analysis_job = Some(Box::new(PriorityAnalysisJob {
            token,
            key: self.priority_analysis_key(),
            game: self.game.clone(),
            player: ctx.player,
            session: Default::default(),
        }));
        true
    }

    /// Null means pending; false means cancelled/stale. Only a complete exact menu is published.
    #[wasm_bindgen(js_name = stepPriorityAnalysis)]
    pub fn step_priority_analysis(
        &mut self,
        token: String,
        budget: usize,
    ) -> Result<JsValue, JsValue> {
        match self.advance_priority_analysis(&token, budget) {
            None => return Ok(JsValue::FALSE),
            Some(false) => return Ok(JsValue::NULL),
            Some(true) => {}
        }
        let decision = DecisionView::from_context(
            &self.game,
            self.pending_decision.as_ref().unwrap(),
            self.perspective,
            self.active_viewed_cards.as_ref(),
            self.visible_undo_land_stable_id(self.is_cancelable()),
        );
        serde_wasm_bindgen::to_value(&decision).map_err(|error| {
            JsValue::from_str(&format!("priority analysis encode failed: {error}"))
        })
    }
}

#[cfg(test)]
mod priority_analysis_tests {
    use super::*;
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            ironsmith::game_loop::set_priority_analysis_deferred(self.0);
        }
    }
    fn fixture() -> (WasmGame, Restore) {
        let restore = Restore(ironsmith::game_loop::priority_analysis_deferred());
        let mut wasm = WasmGame::new();
        wasm.set_deferred_priority_analysis(true);
        let alice = PlayerId::from_index(0);
        wasm.game.turn.priority_player = Some(alice);
        wasm.pending_decision = Some(DecisionContext::Priority(
            ironsmith::game_loop::priority_context(&wasm.game, alice),
        ));
        (wasm, restore)
    }
    // Fixture registration compiles cards and needs more stack than a small
    // libtest worker in debug builds. Browser WASM uses optimized code.
    fn with_fixture_stack(run: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(run)
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn priority_analysis_publishes_exact_menu_without_advancing_priority() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            let (mut wasm, _restore) = fixture();
            let alice = PlayerId::from_index(0);
            let expected =
                ironsmith::game_loop::analyze_priority_context(&wasm.game, alice).actions;
            assert!(wasm.begin_priority_analysis("one".into()));
            assert_eq!(wasm.advance_priority_analysis("one", 1), Some(true));
            let Some(DecisionContext::Priority(ctx)) = wasm.pending_decision.as_ref() else {
                panic!("missing priority");
            };
            assert!(ctx.analysis_complete);
            assert_eq!(ctx.actions, expected);
            assert_eq!(wasm.game.turn.priority_player, Some(alice));
        });
    }
    #[test]
    fn priority_analysis_rejects_mutation_perspective_and_cancelled_jobs() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            let (mut wasm, _restore) = fixture();
            assert!(wasm.begin_priority_analysis("one".into()));
            wasm.game.player_mut(PlayerId::from_index(0)).unwrap().life -= 1;
            assert_eq!(wasm.advance_priority_analysis("one", 128), None);
            assert!(wasm.begin_priority_analysis("two".into()));
            wasm.perspective = PlayerId::from_index(1);
            assert_eq!(wasm.advance_priority_analysis("two", 128), None);
            assert!(wasm.begin_priority_analysis("three".into()));
            wasm.cancel_priority_analysis();
            assert_eq!(wasm.advance_priority_analysis("three", 128), None);
        });
    }
}

impl WasmGame {
    fn inspector_actions_with(
        &self,
        object_id: u64,
        requested_ability: Option<usize>,
        planner: &mut impl FnMut(&ironsmith::mana_payment::ManaPaymentRequest) -> Option<bool>,
    ) -> Result<JsValue, JsValue> {
        let mut actions = Vec::new();
        if let Some(DecisionContext::Priority(priority)) = self.pending_decision.as_ref() {
            for (index, action) in priority.actions.iter().enumerate() {
                let (LegalAction::ActivateAbility {
                    source,
                    ability_index,
                }
                | LegalAction::ActivateManaAbility {
                    source,
                    ability_index,
                }) = action
                else {
                    continue;
                };
                if source.0 != object_id
                    || requested_ability.is_some_and(|index| index != *ability_index)
                {
                    continue;
                }
                let mut view = build_action_view(
                    &self.game,
                    self.perspective,
                    self.active_viewed_cards.as_ref(),
                    index,
                    action,
                );
                if view.object_id.is_some() {
                    view.mana_payment_available = activation_mana_payment_available(
                        &self.game,
                        priority.player,
                        action,
                        planner,
                    );
                    actions.push(view);
                }
            }
        }
        serde_wasm_bindgen::to_value(&actions)
            .map_err(|e| JsValue::from_str(&format!("inspectorActions encode failed: {e}")))
    }
}

pub(super) struct InspectorAnalysisJob {
    token: String,
    key: SnapshotCacheKey,
    object_id: u64,
    ability: Option<usize>,
    searches: Vec<ironsmith::mana_payment::ManaPaymentAnalysis>,
    counters: ironsmith::ids::IdCountersSnapshot,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = beginInspectorAnalysis)]
    pub fn begin_inspector_analysis(
        &mut self,
        token: String,
        object_id: u64,
        ability: Option<usize>,
    ) {
        self.inspector_analysis_job = Some(Box::new(InspectorAnalysisJob {
            token,
            key: self.priority_analysis_key(),
            object_id,
            ability,
            searches: Vec::new(),
            counters: snapshot_id_counters(),
        }));
    }
    #[wasm_bindgen(js_name = stepInspectorAnalysis)]
    pub fn step_inspector_analysis(
        &mut self,
        token: String,
        budget: usize,
    ) -> Result<JsValue, JsValue> {
        use ironsmith::mana_payment::{ManaPaymentAnalysis, ManaPaymentFailure};
        let Some(mut job) = self.inspector_analysis_job.take() else {
            return Ok(JsValue::FALSE);
        };
        if job.token != token || job.key != self.priority_analysis_key() {
            return Ok(JsValue::FALSE);
        }
        let counters = snapshot_id_counters();
        restore_id_counters(job.counters);
        let mut index = 0;
        let mut pending = false;
        let result = self.inspector_actions_with(job.object_id, job.ability, &mut |request| {
            if pending {
                return None;
            }
            if index == job.searches.len() {
                job.searches
                    .push(ManaPaymentAnalysis::new(&self.game, request.clone()));
            }
            let outcome = job.searches[index].step(budget.clamp(1, 8));
            index += 1;
            match outcome {
                None => {
                    pending = true;
                    None
                }
                Some(Ok(_)) => Some(true),
                Some(Err(ManaPaymentFailure::NoLegalPlan)) => Some(false),
                Some(Err(_)) => None,
            }
        });
        job.counters = snapshot_id_counters();
        restore_id_counters(counters);
        if pending {
            self.inspector_analysis_job = Some(job);
            return Ok(JsValue::NULL);
        }
        result
    }
}
