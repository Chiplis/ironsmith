/// An exact owned snapshot; no checkpoint reconstruction or live-state writes
/// occur while the search is running. The worker yields between step calls.
pub(super) struct PriorityAnalysisJob {
    token: String,
    key: SnapshotCacheKey,
    game: GameState,
    player: PlayerId,
    candidates: std::collections::VecDeque<PriorityCandidate>,
    actions: Vec<LegalAction>,
}

struct PriorityCandidate {
    player: PlayerId,
    source: Option<ObjectId>,
    session: ironsmith::decision::ManaAnalysisSession,
}

impl WasmGame {
    fn advance_priority_analysis(&mut self, token: &str, budget: usize) -> Result<Option<bool>, ironsmith::game_loop::GameLoopError> {
        let Some(mut job) = self.priority_analysis_job.take() else {
            return Ok(None);
        };
        if job.token != token || job.key != self.priority_analysis_key() {
            return Ok(None);
        }
        let Some(mut candidate) = job.candidates.pop_front() else { return Ok(Some(true)); };
        let id_counters = snapshot_id_counters();
        let (actions, complete) = candidate.session.run(budget.clamp(1, 64), || {
            match candidate.source {
                Some(source) => ironsmith::decision::compute_actions_for_source(&job.game, candidate.player, Some(source)),
                None => ironsmith::decision::compute_global_actions(&job.game, candidate.player),
            }
        });
        restore_id_counters(id_counters);
        self.last_analysis_slice_nodes = candidate.session.last_slice_nodes();
        let actions = actions.map_err(ironsmith::game_loop::GameLoopError::from)?;
        if complete {
            for action in actions {
                if !job.actions.contains(&action) { job.actions.push(action); }
            }
        } else {
            // Rotate unresolved cards: one search cannot consume every slice.
            job.candidates.push_back(candidate);
        }
        let finished = job.candidates.is_empty();
        let mut ctx = ironsmith::decisions::context::PriorityContext::new(
            &job.game, job.player, job.actions.clone(),
        ).map_err(ironsmith::effects::ExecutionError::ContinuousDiscovery)?;
        ctx.analysis_complete = finished;
        self.pending_decision = Some(DecisionContext::Priority(ctx));
        self.cached_snapshot = None;
        if !finished {
            // Publishing a partial menu changes the decision hash, not the game.
            job.key = self.priority_analysis_key();
            self.priority_analysis_job = Some(job);
        }
        Ok(Some(finished))
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

    /// Node pops consumed by the most recent analysis slice. Compared against
    /// the budget that was requested, this separates search cost from the fixed
    /// per-slice cost of rebuilding the menu.
    #[wasm_bindgen(js_name = lastAnalysisSliceNodes)]
    pub fn last_analysis_slice_nodes(&self) -> usize {
        self.last_analysis_slice_nodes
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

    #[wasm_bindgen(js_name = hasPriorityDecision)]
    pub fn has_priority_decision(&self) -> bool {
        matches!(self.pending_decision, Some(DecisionContext::Priority(_))) && self.pregame.is_none()
    }

    #[wasm_bindgen(js_name = priorityAnalysisPending)]
    pub fn priority_analysis_pending(&self) -> bool {
        matches!(self.pending_decision.as_ref(), Some(DecisionContext::Priority(ctx)) if !ctx.analysis_complete)
            && self.pregame.is_none()
    }

    #[wasm_bindgen(js_name = beginPriorityAnalysis)]
    pub fn begin_priority_analysis(&mut self, token: String) -> bool {
        let Some(DecisionContext::Priority(ctx)) = self.pending_decision.as_ref() else {
            return false;
        };
        if ctx.analysis_complete || self.pregame.is_some() {
            return false;
        }
        let mut candidates = std::collections::VecDeque::new();
        for player in self.game.priority_team_players() {
            for source in ironsmith::decision::priority_analysis_sources(&self.game, player) {
                candidates.push_back(PriorityCandidate { player, source: Some(source), session: Default::default() });
            }
            candidates.push_back(PriorityCandidate { player, source: None, session: Default::default() });
        }
        self.priority_analysis_job = Some(Box::new(PriorityAnalysisJob {
            token,
            key: self.priority_analysis_key(),
            game: self.game.clone(),
            player: ctx.player,
            candidates,
            actions: vec![LegalAction::PassPriority],
        }));
        true
    }

    /// False means cancelled/stale. Every step returns cumulative confirmed actions;
    /// analysis_complete distinguishes pending cards from proven unavailable cards.
    #[wasm_bindgen(js_name = stepPriorityAnalysis)]
    pub fn step_priority_analysis(
        &mut self,
        token: String,
        budget: usize,
    ) -> Result<JsValue, JsValue> {
        match self.advance_priority_analysis(&token, budget)
            .map_err(|error| JsValue::from_str(&format!("priority action analysis failed: {error}")))? {
            None => return Ok(JsValue::FALSE),
            Some(false) | Some(true) => {}
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
            ironsmith::game_loop::priority_context(&wasm.game, alice).expect("fixture has complete replacement state"),
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
                ironsmith::game_loop::analyze_priority_context(&wasm.game, alice).expect("fixture has complete replacement state").actions;
            assert!(wasm.begin_priority_analysis("one".into()));
            assert_eq!(wasm.advance_priority_analysis("one", 1).expect("fixture has complete replacement state"), Some(true));
            let Some(DecisionContext::Priority(ctx)) = wasm.pending_decision.as_ref() else {
                panic!("missing priority");
            };
            assert!(ctx.analysis_complete);
            assert_eq!(ctx.actions, expected);
            assert_eq!(wasm.game.turn.priority_player, Some(alice));
        });
    }
    #[test]
    fn incremental_land_precedes_warp_and_final_actions_match_full_enumeration() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            let (mut wasm, _restore) = fixture();
            let alice = PlayerId::from_index(0);
            wasm.game.turn.active_player = alice;
            wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            wasm.game.turn.step = None;
            wasm.game.player_mut(alice).unwrap().mana_pool.red = 3;
            let card = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Warp probe")
                .card_types(vec![ironsmith::types::CardType::Creature])
                .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(5)]))
                .build();
            let mut def = ironsmith::cards::CardDefinition::new(card);
            def.alternative_casts.push(ironsmith::alternative_cast::AlternativeCastingMethod::Warp {
                cost: ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(2), ironsmith::ManaSymbol::Red]),
                additional_cost: ironsmith::cost::TotalCost::free(),
            });
            let spell = wasm.game.create_object_from_definition(&def, alice, ironsmith::Zone::Hand);
            let land_card = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Land probe")
                .card_types(vec![ironsmith::types::CardType::Land]).build();
            let land = wasm.game.create_object_from_card(&land_card, alice, ironsmith::Zone::Hand);
            let bob = PlayerId::from_index(1);
            let foreign_graveyard_land = wasm.game.create_object_from_card(&land_card, bob, ironsmith::Zone::Graveyard);
            let foreign_sideboard_land = wasm.game.create_object_from_card(&land_card, bob, ironsmith::Zone::OutsideGame);
            let sources = ironsmith::decision::priority_analysis_sources(&wasm.game, alice);
            assert!(sources.contains(&foreign_graveyard_land));
            assert!(sources.contains(&foreign_sideboard_land));
            // Exercise candidate coverage outside hand, including top library.
            for zone in [ironsmith::Zone::Battlefield, ironsmith::Zone::Graveyard, ironsmith::Zone::Exile, ironsmith::Zone::Library, ironsmith::Zone::Command] {
                wasm.game.create_object_from_definition(&def, alice, zone);
            }
            wasm.pending_decision = Some(DecisionContext::Priority(
                ironsmith::game_loop::priority_context(&wasm.game, alice).unwrap()));
            let expected = ironsmith::game_loop::analyze_priority_context(&wasm.game, alice).unwrap().actions;
            assert!(expected.iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method: ironsmith::alternative_cast::CastingMethod::Alternative(0), .. } if *spell_id == spell)));
            assert!(wasm.begin_priority_analysis("progress".into()));
            assert_eq!(wasm.advance_priority_analysis("progress", 1).unwrap(), Some(false));
            let Some(DecisionContext::Priority(first)) = &wasm.pending_decision else { panic!() };
            assert!(first.actions.contains(&LegalAction::PlayLand { land_id: land }));
            assert!(!first.analysis_complete);
            assert!(!first.actions.iter().any(|a| matches!(a, LegalAction::CastSpell { .. })));
            let mut previous = first.actions.clone();
            for _ in 0..1000 {
                let done = wasm.advance_priority_analysis("progress", 8).unwrap() == Some(true);
                let Some(DecisionContext::Priority(ctx)) = &wasm.pending_decision else { panic!() };
                assert!(previous.iter().all(|action| ctx.actions.contains(action)));
                previous = ctx.actions.clone();
                if done {
                    assert!(ctx.analysis_complete);
                    assert_eq!(ctx.actions.len(), expected.len());
                    assert!(expected.iter().all(|action| ctx.actions.contains(action)));
                    return;
                }
            }
            panic!("analysis did not finish");
        });
    }
    #[test]
    fn priority_analysis_rejects_mutation_perspective_and_cancelled_jobs() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            let (mut wasm, _restore) = fixture();
            assert!(wasm.begin_priority_analysis("one".into()));
            wasm.game.player_mut(PlayerId::from_index(0)).unwrap().life -= 1;
            assert_eq!(wasm.advance_priority_analysis("one", 128).expect("fixture has complete replacement state"), None);
            assert!(wasm.begin_priority_analysis("two".into()));
            wasm.perspective = PlayerId::from_index(1);
            assert_eq!(wasm.advance_priority_analysis("two", 128).expect("fixture has complete replacement state"), None);
            assert!(wasm.begin_priority_analysis("three".into()));
            wasm.cancel_priority_analysis();
            assert_eq!(wasm.advance_priority_analysis("three", 128).expect("fixture has complete replacement state"), None);
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
        let checked = self.game.continuous_query_snapshot().map_err(|error|
            JsValue::from_str(&format!("inspector action analysis failed: {error}")))?;
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
                    &checked,
                    self.perspective,
                    self.active_viewed_cards.as_ref(),
                    index,
                    action,
                    None,
                );
                if view.object_id.is_some() {
                    view.mana_payment_available = activation_mana_payment_available(
                        &checked,
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
        let mut slice_units = 0usize;
        let result = self.inspector_actions_with(job.object_id, job.ability, &mut |request| {
            if pending {
                return None;
            }
            if index == job.searches.len() {
                job.searches
                    .push(ManaPaymentAnalysis::check(&self.game, request.clone()));
            }
            let outcome = job.searches[index].step(budget.clamp(1, 64));
            slice_units = slice_units.saturating_add(job.searches[index].last_slice_units());
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
        self.last_analysis_slice_nodes = slice_units;
        if pending {
            self.inspector_analysis_job = Some(job);
            return Ok(JsValue::NULL);
        }
        result
    }
}

// Optional payment ranking never mutates a game. The result is a constrained
// replan command, sent through the normal authoritative multiplayer path.
pub(super) struct PaymentAnalysisJob {
    token: String,
    key: SnapshotCacheKey,
    analysis: ironsmith::mana_payment::ManaPaymentAnalysis,
    request: ironsmith::mana_payment::ManaPaymentRequest,
    score: ironsmith::mana_payment::ManaPaymentScore,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(js_name = beginPaymentAnalysis)]
    pub fn begin_payment_analysis(&mut self, token: String) -> bool {
        let Some(DecisionContext::ManaPayment(ctx)) = self.pending_decision.as_ref() else {
            return false;
        };
        self.payment_analysis_job = Some(Box::new(PaymentAnalysisJob {
            token,
            key: self.priority_analysis_key(),
            analysis: ironsmith::mana_payment::ManaPaymentAnalysis::ranked(
                &self.game,
                ctx.request.clone(),
            ),
            request: ctx.request.clone(),
            score: ctx.plan.score,
        }));
        true
    }

    #[wasm_bindgen(js_name = cancelPaymentAnalysis)]
    pub fn cancel_payment_analysis(&mut self) {
        self.payment_analysis_job = None;
    }

    #[wasm_bindgen(js_name = stepPaymentAnalysis)]
    pub fn step_payment_analysis(
        &mut self,
        token: String,
        budget: usize,
    ) -> Result<JsValue, JsValue> {
        use ironsmith::mana_payment::{
            ManaPaymentSourceKind, PlannedPipPayment, RequiredAlternativePayment,
            RequiredManaActivation,
        };
        let Some(mut job) = self.payment_analysis_job.take() else {
            return Ok(JsValue::FALSE);
        };
        if job.token != token || job.key != self.priority_analysis_key() {
            return Ok(JsValue::FALSE);
        }
        let counters = snapshot_id_counters();
        let result = job.analysis.step(budget.clamp(1, 8));
        restore_id_counters(counters);
        let Some(result) = result else {
            self.payment_analysis_job = Some(job);
            return Ok(JsValue::NULL);
        };
        let Ok(plan) = result else {
            return Ok(JsValue::FALSE);
        };
        if plan.score >= job.score {
            return Ok(JsValue::FALSE);
        }
        let mut preferences = job.request.preferences;
        preferences.required_activations = plan
            .mana_ability_steps
            .iter()
            .map(|step| RequiredManaActivation {
                source: step.source,
                ability_index: step.ability_index,
                color_restriction: step.color_restriction.clone(),
            })
            .collect();
        preferences.required_alternatives = plan
            .allocations
            .iter()
            .filter_map(|allocation| {
                let (source, kind) = match allocation.payment {
                    PlannedPipPayment::Convoke(source) => (source, ManaPaymentSourceKind::Convoke),
                    PlannedPipPayment::Improvise(source) => {
                        (source, ManaPaymentSourceKind::Improvise)
                    }
                    PlannedPipPayment::Delve(source) => (source, ManaPaymentSourceKind::Delve),
                    _ => return None,
                };
                Some(RequiredAlternativePayment { source, kind })
            })
            .collect();
        let command = UiCommand::ManaPayment {
            response: manabrew_replan_command(preferences),
        };
        serde_wasm_bindgen::to_value(&command)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}
