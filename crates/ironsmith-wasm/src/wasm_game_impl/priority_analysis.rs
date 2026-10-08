/// An exact owned snapshot; no checkpoint reconstruction or live-state writes
/// occur while the search is running. The worker yields between step calls.
pub(super) struct PriorityAnalysisJob {
    token: String,
    key: SnapshotCacheKey,
    game: GameState,
    player: PlayerId,
    candidates: std::collections::VecDeque<PriorityCandidate>,
    actions: Vec<LegalAction>,
    provisional_actions: Vec<LegalAction>,
    presentation_actions: Vec<LegalAction>,
}

struct PriorityCandidate {
    player: PlayerId,
    source: Option<ObjectId>,
    session: ironsmith::decision::ManaAnalysisSession,
    work_units: usize,
    presentation_checked: bool,
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
        if !candidate.presentation_checked {
            let eligible = match candidate.source {
                Some(source) => ironsmith::decision::compute_actions_assuming_mana_for_presentation(
                    &job.game, candidate.player, Some(source),
                ),
                None => Ok(Vec::new()),
            };
            restore_id_counters(id_counters);
            for action in eligible.map_err(ironsmith::game_loop::GameLoopError::from)? {
                if !job.presentation_actions.contains(&action) { job.presentation_actions.push(action); }
            }
            candidate.presentation_checked = true;
        }
        let (actions, complete) = candidate.session.run_for_game(&job.game, budget.clamp(1, 64), || {
            match candidate.source {
                Some(source) => ironsmith::decision::compute_actions_for_source(&job.game, candidate.player, Some(source)),
                None => ironsmith::decision::compute_global_actions(&job.game, candidate.player),
            }
        });
        restore_id_counters(id_counters);
        self.last_analysis_slice_nodes = candidate.session.last_slice_nodes();
        candidate.work_units = candidate.work_units.saturating_add(self.last_analysis_slice_nodes);
        if let Some(error) = candidate.session.failure().cloned() {
            // A failed candidate cannot become a completed negative entry or
            // a permanently pending retry. Leave the published menu explicitly
            // incomplete and surface the original typed error to the host.
            return Err(ironsmith::game_loop::GameLoopError::from(error));
        }
        let actions = actions.map_err(ironsmith::game_loop::GameLoopError::from)?;
        // Each positive result has a proven payment even when another method
        // for this source is still searching. Publish free/alternative routes
        // immediately rather than holding them behind the normal mana cost.
        for action in actions {
            if !job.actions.contains(&action) { job.actions.push(action); }
        }
        if complete {
            if !job.candidates.iter().any(|other| other.source == candidate.source) {
                job.provisional_actions.retain(|action| ironsmith::decision::legal_action_source(action) != candidate.source);
            }
        } else {
            // Rotate unresolved cards: one search cannot consume every slice.
            job.candidates.push_back(candidate);
        }
        let finished = job.candidates.is_empty();
        let mut displayed = job.actions.clone();
        for action in &job.provisional_actions { if !displayed.contains(action) { displayed.push(action.clone()); } }
        let mut ctx = ironsmith::decisions::context::PriorityContext::new(
            &job.game, job.player, displayed,
        ).map_err(ironsmith::effects::ExecutionError::ContinuousDiscovery)?;
        ctx.presentation_actions = job.presentation_actions.clone();
        ctx.analysis_complete = finished;
        ctx.payment_proven_actions = Some(job.actions.clone());
        self.pending_decision = Some(DecisionContext::Priority(ctx));
        self.cached_snapshot = None;
        if !finished {
            // Publishing a partial menu changes the decision hash, not the game.
            job.key = self.priority_analysis_key();
            self.priority_analysis_job = Some(job);
        }
        Ok(Some(finished))
    }

    fn current_non_mana_eligibility(&self, cached: &[LegalAction]) -> Result<Vec<LegalAction>, ironsmith::effects::ExecutionError> {
        if cached.is_empty() { return Ok(Vec::new()); }
        let counters = snapshot_id_counters();
        let result = (|| {
            let mut eligible = Vec::new();
            for actor in self.game.priority_team_players() {
                let mut sources: Vec<_> = cached.iter().filter_map(ironsmith::decision::legal_action_source).collect();
                sources.sort_unstable();
                sources.dedup();
                for source in sources {
                    eligible.extend(ironsmith::decision::compute_actions_assuming_mana_for_presentation(&self.game, actor, Some(source))?);
                }
            }
            Ok(eligible)
        })();
        restore_id_counters(counters);
        result
    }

    fn refresh_priority_affordability_display(&mut self) -> Result<(), ironsmith::effects::ExecutionError> {
        let Some(DecisionContext::Priority(ctx)) = self.pending_decision.as_ref() else { return Ok(()); };
        if self.pregame.is_some() { return Ok(()); }
        let player = ctx.player;
        let cached = self.priority_affordability_cache.get(&player).cloned().unwrap_or_default();
        if ctx.analysis_complete {
            let key = self.priority_analysis_key();
            if self.priority_affordability_completed_key.as_ref() == Some(&key) { return Ok(()); }
            let confirmed = ctx.actions.to_vec();
            self.remember_confirmed_affordability(player, confirmed)?;
            self.priority_affordability_seed_key = None;
            self.priority_affordability_completed_key = Some(key);
            return Ok(());
        }
        if self.priority_analysis_job.is_some() || ctx.actions.as_ref() != [LegalAction::PassPriority] { return Ok(()); }
        let mut key = self.priority_analysis_key();
        key.pending_decision_hash = 0;
        if self.priority_affordability_seed_key.as_ref() == Some(&key) { return Ok(()); }
        let eligible = self.current_non_mana_eligibility(&cached)?;
        let mut displayed = vec![LegalAction::PassPriority];
        for action in cached {
            if eligible.contains(&action) && !displayed.contains(&action) { displayed.push(action); }
        }
        let mut ctx = ironsmith::decisions::context::PriorityContext::new(&self.game, player, displayed)
            .map_err(ironsmith::effects::ExecutionError::ContinuousDiscovery)?;
        ctx.analysis_complete = false;
        ctx.payment_proven_actions = Some(vec![LegalAction::PassPriority]);
        self.pending_decision = Some(DecisionContext::Priority(ctx));
        self.priority_affordability_seed_key = Some(key);
        self.priority_affordability_completed_key = None;
        Ok(())
    }

    fn remember_confirmed_affordability(&mut self, player: PlayerId, confirmed: Vec<LegalAction>) -> Result<(), ironsmith::effects::ExecutionError> {
        let cached = self.priority_affordability_cache.get(&player).cloned().unwrap_or_default();
        let eligible = self.current_non_mana_eligibility(&cached)?;
        // Timing/target restrictions supply no new affordability result.
        let mut retained: Vec<_> = cached.into_iter().filter(|action| {
            let source_exists = ironsmith::decision::legal_action_source(action)
                .is_none_or(|source| self.game.object(source).is_some());
            source_exists && (!eligible.contains(action) || confirmed.contains(action))
        }).collect();
        for action in confirmed {
            if !matches!(action, LegalAction::PassPriority) && !retained.contains(&action) { retained.push(action); }
        }
        self.priority_affordability_cache.insert(player, retained);
        self.priority_affordability_seed_key = None;
        Ok(())
    }

    fn priority_analysis_key(&self) -> SnapshotCacheKey {
        self.snapshot_cache_key(None, false, None, &None)
    }
}

#[wasm_bindgen]
impl WasmGame {
    /// Local worker results update presentation affordability only. These
    /// references never become authoritative legal actions or bypass payment.
    #[wasm_bindgen(js_name = rememberPriorityAffordability)]
    pub fn remember_priority_affordability(&mut self, references: JsValue) -> Result<(), JsValue> {
        let Some(DecisionContext::Priority(ctx)) = self.pending_decision.clone() else { return Ok(()); };
        let player = ctx.player;
        let references: Vec<PriorityActionRef> = serde_wasm_bindgen::from_value(references)
            .map_err(|error| JsValue::from_str(&format!("invalid affordability references: {error}")))?;
        let mut confirmed = Vec::new();
        let counters = snapshot_id_counters();
        let result = (|| {
            for reference in &references {
                if let Some(action) = resolve_priority_action(&self.game, &ctx, None, Some(reference))?
                    && !confirmed.contains(&action) {
                    confirmed.push(action);
                }
            }
            self.remember_confirmed_affordability(player, confirmed)
        })();
        restore_id_counters(counters);
        result.map_err(|error| JsValue::from_str(&format!("affordability cache refresh failed: {error}")))
    }

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

    /// Read-only profiling metadata for unresolved candidates in this exact job.
    /// Opaque object IDs and consumed work expose neither card text nor speculative
    /// legality, and querying this does not advance or reconstruct the snapshot.
    #[wasm_bindgen(js_name = priorityAnalysisProgress)]
    pub fn priority_analysis_progress(&self) -> Result<JsValue, JsValue> {
        let rows: Vec<_> = self.priority_analysis_job.as_ref().into_iter()
            .flat_map(|job| job.candidates.iter())
            .map(|candidate| serde_json::json!({
                "player": candidate.player.0,
                "source": candidate.source.map(|source| source.0),
                "workUnits": candidate.work_units,
            })).collect();
        serde_wasm_bindgen::to_value(&rows)
            .map_err(|error| JsValue::from_str(&format!("analysis progress encode failed: {error}")))
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
                candidates.push_back(PriorityCandidate { player, source: Some(source), session: Default::default(), work_units: 0, presentation_checked: false });
            }
            candidates.push_back(PriorityCandidate { player, source: None, session: Default::default(), work_units: 0, presentation_checked: false });
        }
        self.priority_analysis_job = Some(Box::new(PriorityAnalysisJob {
            token,
            key: self.priority_analysis_key(),
            game: self.game.clone(),
            player: ctx.player,
            candidates,
            actions: vec![LegalAction::PassPriority],
            presentation_actions: Vec::new(),
            provisional_actions: ctx.actions.iter().filter(|action| !matches!(action, LegalAction::PassPriority)).cloned().collect(),
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
        let mut wasm = WasmGame::new_with_registry(CardRegistry::new());
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
    fn free_and_warp_methods_are_published_before_normal_payment_finishes() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            for free in [true, false] {
                let (mut wasm, _restore) = fixture();
                let alice = PlayerId::from_index(0);
                wasm.game.turn.active_player = alice;
                wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                wasm.game.turn.step = None;
                let card = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Early alternative")
                    .card_types(vec![ironsmith::types::CardType::Creature])
                    .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(5)]))
                    .build();
                let mut definition = ironsmith::cards::CardDefinition::new(card);
                if free {
                    let omniscience = ironsmith_registry_test::compile_to_runtime_definition(
                        "Omniscience", "Mana cost: {7}{U}{U}{U}\nType: Enchantment\nYou may cast spells from your hand without paying their mana costs.", false,
                    ).unwrap();
                    wasm.game.create_object_from_definition(&omniscience, alice, ironsmith::Zone::Battlefield);
                } else {
                    definition.alternative_casts.push(ironsmith::alternative_cast::AlternativeCastingMethod::Warp {
                        cost: ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(2), ironsmith::ManaSymbol::Red]),
                        additional_cost: ironsmith::cost::TotalCost::free(),
                    });
                }
                wasm.game.player_mut(alice).unwrap().mana_pool.red = 3;
                let spell = wasm.game.create_object_from_definition(&definition, alice, ironsmith::Zone::Hand);
                wasm.pending_decision = Some(DecisionContext::Priority(
                    ironsmith::game_loop::priority_context(&wasm.game, alice).unwrap(),
                ));
                let view = DecisionView::from_context(&wasm.game, wasm.pending_decision.as_ref().unwrap(), alice, None, None);
                let DecisionView::Priority { actions, .. } = view else { panic!() };
                assert!(!actions.iter().any(|action| action.object_id == Some(spell.0)),
                    "rendering must not enumerate undiscovered prices");
                assert!(wasm.begin_priority_analysis("early-method".into()));
                assert_eq!(wasm.advance_priority_analysis("early-method", 1).unwrap(), Some(false));
                let view = DecisionView::from_context(&wasm.game, wasm.pending_decision.as_ref().unwrap(), alice, None, None);
                let DecisionView::Priority { actions, .. } = view else { panic!() };
                let prices: Vec<_> = actions.iter().filter(|action| action.object_id == Some(spell.0)).collect();
                assert_eq!(prices.len(), 2, "both prices are offered after their source is analyzed");
                assert!(prices.iter().any(|action| action.payment_proven == Some(true)));
                assert!(prices.iter().any(|action| action.payment_proven == Some(false)));
                let Some(DecisionContext::Priority(ctx)) = wasm.pending_decision.as_ref() else { panic!() };
                let proven = ctx.payment_proven_actions.as_ref().unwrap();
                assert!(proven.iter().any(|action| matches!(action, LegalAction::CastSpell {
                    spell_id, casting_method, ..
                } if *spell_id == spell && !matches!(casting_method, ironsmith::alternative_cast::CastingMethod::Normal))),
                    "free or Warp payment is proven without waiting for normal cost");
                assert!(!proven.iter().any(|action| matches!(action, LegalAction::CastSpell {
                    spell_id, casting_method: ironsmith::alternative_cast::CastingMethod::Normal, ..
                } if *spell_id == spell)));
                assert!(!ctx.analysis_complete);
            }
        });
    }

    #[test]
    fn timing_candidates_are_clickable_without_funding_and_remain_timing_checked() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            let (mut wasm, _restore) = fixture();
            let alice = PlayerId::from_index(0);
            wasm.game.turn.active_player = alice;
            wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            wasm.game.turn.step = None;
            let card = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Unfunded sorcery")
                .card_types(vec![ironsmith::types::CardType::Sorcery])
                .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Red])).build();
            let spell = wasm.game.create_object_from_card(&card, alice, ironsmith::Zone::Hand);
            wasm.pending_decision = Some(DecisionContext::Priority(ironsmith::game_loop::priority_context(&wasm.game, alice).unwrap()));
            let deferred_view = DecisionView::from_context(&wasm.game, wasm.pending_decision.as_ref().unwrap(), alice, None, None);
            let DecisionView::Priority { actions: pending, analysis_complete, .. } = deferred_view else { panic!("expected priority"); };
            assert!(!analysis_complete);
            assert!(!pending.iter().any(|action| action.object_id == Some(spell.0)), "snapshot must not enumerate undiscovered candidates");
            assert!(wasm.begin_priority_analysis("unfunded".into()));
            let mut finished = false;
            for _ in 0..1000 {
                if wasm.advance_priority_analysis("unfunded", 8).unwrap() == Some(true) { finished = true; break; }
            }
            assert!(finished, "unfunded eligibility analysis must finish");
            let Some(DecisionContext::Priority(ctx)) = wasm.pending_decision.clone() else { panic!() };
            let view = DecisionView::from_context(&wasm.game, &DecisionContext::Priority(ctx.clone()), alice, None, None);
            let DecisionView::Priority { actions, .. } = view else { panic!("expected priority"); };
            let action = actions.iter().find(|action| action.object_id == Some(spell.0)).expect("unfunded sorcery must appear at main-phase timing");
            assert_eq!(action.payment_proven, Some(false));
            assert!(resolve_priority_action(&wasm.game, &ctx, None, Some(&action.action_ref)).unwrap().is_some());
            wasm.game.turn.active_player = PlayerId::from_index(1);
            assert!(resolve_priority_action(&wasm.game, &ctx, None, Some(&action.action_ref)).unwrap().is_none(), "off-turn sorcery must remain illegal");
        });
    }

    #[test]
    fn affordability_cache_keeps_previous_mana_result_but_checks_timing_immediately() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            let (mut wasm, _restore) = fixture();
            let alice = PlayerId::from_index(0);
            wasm.game.turn.active_player = alice;
            wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            wasm.game.turn.step = None;
            wasm.game.player_mut(alice).unwrap().mana_pool.red = 1;
            let card = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Cached sorcery")
                .card_types(vec![ironsmith::types::CardType::Sorcery])
                .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Red])).build();
            let spell = wasm.game.create_object_from_card(&card, alice, ironsmith::Zone::Hand);
            let has_spell = |wasm: &WasmGame| match &wasm.pending_decision {
                Some(DecisionContext::Priority(ctx)) => ctx.actions.iter().any(|action|
                    matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)),
                _ => false,
            };
            wasm.pending_decision = Some(DecisionContext::Priority(ironsmith::game_loop::analyze_priority_context(&wasm.game, alice).unwrap()));
            wasm.refresh_priority_affordability_display().unwrap();
            assert!(has_spell(&wasm));
            // Timing must mask the cached positive even before any mana search.
            wasm.game.turn.phase = ironsmith::game_state::Phase::Combat;
            wasm.pending_decision = Some(DecisionContext::Priority(ironsmith::game_loop::priority_context(&wasm.game, alice).unwrap()));
            wasm.refresh_priority_affordability_display().unwrap();
            assert!(!has_spell(&wasm));
            wasm.pending_decision = Some(DecisionContext::Priority(ironsmith::game_loop::analyze_priority_context(&wasm.game, alice).unwrap()));
            wasm.refresh_priority_affordability_display().unwrap();
            // Returning to main restores the old affordability while recomputing.
            wasm.game.turn.phase = ironsmith::game_state::Phase::NextMain;
            wasm.game.player_mut(alice).unwrap().mana_pool.red = 0;
            wasm.pending_decision = Some(DecisionContext::Priority(ironsmith::game_loop::priority_context(&wasm.game, alice).unwrap()));
            wasm.refresh_priority_affordability_display().unwrap();
            assert!(has_spell(&wasm));
            let Some(DecisionContext::Priority(ctx)) = &wasm.pending_decision else { panic!() };
            let provisional_index = ctx.actions.iter().position(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
            let reference = priority_action_ref(&ctx.actions[provisional_index]);
            assert!(resolve_priority_action(&wasm.game, ctx, None, Some(&reference)).unwrap().is_some());
            assert!(resolve_priority_action(&wasm.game, ctx, Some(provisional_index), None).unwrap().is_some());
            assert!(!ironsmith::game_loop::analyze_priority_context(&wasm.game, alice).unwrap().actions.iter().any(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)),
                "presentation assumption must never leak into authoritative affordability");
            assert!(wasm.begin_priority_analysis("cached".into()));
            for _ in 0..100 {
                if wasm.advance_priority_analysis("cached", 64).unwrap() == Some(true) { break; }
            }
            assert!(!wasm.priority_analysis_pending());
            assert!(!has_spell(&wasm), "completed negative must replace provisional positive");
            wasm.refresh_priority_affordability_display().unwrap();
            wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            wasm.pending_decision = Some(DecisionContext::Priority(ironsmith::game_loop::priority_context(&wasm.game, alice).unwrap()));
            wasm.refresh_priority_affordability_display().unwrap();
            assert!(!has_spell(&wasm), "cached negative stays unavailable during recheck");
        });
    }

    #[test]
    fn incomplete_priority_pass_does_not_search_unrelated_mana_sources() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            let (mut game, player, _, _) = crate::resource_payment_test_fixture();
            let spell = ironsmith::CardBuilder::new(ironsmith::CardId::new(), "Resource priority spell")
                .card_types(vec![ironsmith::types::CardType::Creature])
                .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Green])).build();
            game.create_object_from_card(&spell, player, Zone::Hand);
            game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() });
            let mut context = ironsmith::decisions::context::PriorityContext::new(&game, player, vec![LegalAction::PassPriority]).unwrap();
            context.analysis_complete = false;
            let reference = priority_action_ref(&LegalAction::PassPriority);
            assert_eq!(resolve_priority_action(&game, &context, None, Some(&reference)).unwrap(), Some(LegalAction::PassPriority));
            assert_eq!(resolve_priority_action(&game, &context, Some(0), None).unwrap(), Some(LegalAction::PassPriority));
            assert!(ironsmith::decision::compute_actions_for_source(&game, player, None).is_err(), "fixture must exercise a failing mana search");
        });
    }

    #[test]
    fn resource_failed_priority_candidate_never_completes_as_unpayable() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            let (mut wasm, _restore) = fixture();
            let (mut game, player, source, _) = crate::resource_payment_test_fixture();
            let spell = ironsmith::CardBuilder::new(ironsmith::CardId::new(), "Resource priority spell")
                .card_types(vec![ironsmith::CardType::Creature])
                .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Green])).build();
            let spell = game.create_object_from_card(&spell, player, Zone::Hand);
            game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() });
            wasm.game = game;
            let mut context = ironsmith::decisions::context::PriorityContext::new(&wasm.game, player, vec![LegalAction::PassPriority]).unwrap();
            context.analysis_complete = false;
            wasm.pending_decision = Some(DecisionContext::Priority(context));
            wasm.priority_analysis_job = Some(Box::new(PriorityAnalysisJob {
                token: "resource".into(), key: wasm.priority_analysis_key(), game: wasm.game.clone(), player,
                candidates: std::collections::VecDeque::from([PriorityCandidate { player, source: Some(spell), session: Default::default(), work_units: 0, presentation_checked: false }]),
                actions: vec![LegalAction::PassPriority],
                provisional_actions: Vec::new(),
                presentation_actions: Vec::new(),
            }));
            let mut failed = false;
            for _ in 0..256 {
                match wasm.advance_priority_analysis("resource", 1) {
                    Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ironsmith::effects::ExecutionError::ResourceLimitExceeded { .. })) => { failed = true; break; }
                    Ok(Some(false)) => {}
                    result => panic!("incomplete calculation misreported: {result:?}"),
                }
            }
            assert!(failed); assert!(wasm.priority_analysis_job.is_none());
            let Some(DecisionContext::Priority(context)) = wasm.pending_decision.as_ref() else { panic!() };
            assert!(!context.analysis_complete);
            assert_eq!(&*context.actions, &[LegalAction::PassPriority]);
            assert!(!wasm.game.is_tapped(source)); assert!(wasm.game.player(player).unwrap().hand.contains(&spell));
        });
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
    fn keyword_payment_analysis_obeys_budget_and_preserves_exact_final_menu() {
        with_fixture_stack(|| {
            let _ids = crate::test_id_counter_guard();
            for amount in [2, 3] {
                let (mut wasm, _restore) = fixture();
                let alice = PlayerId::from_index(0);
                wasm.game.turn.active_player = alice;
                wasm.game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                wasm.game.turn.step = None;
                let source = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Life-cost source")
                    .card_types(vec![ironsmith::types::CardType::Land])
                    .with_ability(ironsmith::Ability::mana_with_effects(
                        ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::life(1)),
                        vec![ironsmith::effect::Effect::add_mana_of_any_color_restricted(1,
                            vec![ironsmith::color::Color::White, ironsmith::color::Color::Blue])])).build();
                for _ in 0..2 { wasm.game.create_object_from_definition(&source, alice, ironsmith::Zone::Battlefield); }
                let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Improvise slice probe")
                    .card_types(vec![ironsmith::types::CardType::Artifact])
                    .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::ManaSymbol::Blue]; amount]))
                    .with_ability(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::improvise()))
                    .build();
                let spell = wasm.game.create_object_from_definition(&definition, alice, ironsmith::Zone::Hand);
                wasm.pending_decision = Some(DecisionContext::Priority(
                    ironsmith::game_loop::priority_context(&wasm.game, alice).unwrap()));
                let expected = ironsmith::game_loop::analyze_priority_context(&wasm.game, alice).unwrap().actions;
                assert_eq!(expected.iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)), amount == 2);
                assert!(wasm.begin_priority_analysis("keyword".into()));
                assert_eq!(wasm.advance_priority_analysis("keyword", 1).unwrap(), Some(false));
                let retained = wasm.priority_analysis_job.as_ref().unwrap().candidates.iter().any(|candidate| candidate.source == Some(spell));
                let Some(DecisionContext::Priority(first)) = &wasm.pending_decision else { panic!() };
                let first_has_spell = first.actions.iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
                assert!(retained || first_has_spell == (amount == 2),
                    "unfinished queries must retain their frontier; a completed shortcut must already be exact");
                assert!(wasm.last_analysis_slice_nodes <= 1);
                let mut completed = false;
                for _ in 0..1000 {
                    let done = wasm.advance_priority_analysis("keyword", 1).unwrap() == Some(true);
                    assert!(wasm.last_analysis_slice_nodes <= 1);
                    if done { completed = true; break; }
                }
                assert!(completed);
                let Some(DecisionContext::Priority(ctx)) = &wasm.pending_decision else { panic!() };
                assert!(ctx.analysis_complete);
                assert_eq!(ctx.actions.len(), expected.len());
                assert!(expected.iter().all(|action| ctx.actions.contains(action)));
                assert_eq!(wasm.game.player(alice).unwrap().life, 20);
            }
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
            let mut candidates = Vec::new();
            for actor in self.game.priority_team_players() {
                candidates.extend(ironsmith::decision::compute_actions_assuming_mana_for_presentation(
                    &checked, actor, Some(ObjectId::from_raw(object_id)),
                ).map_err(|error| JsValue::from_str(&format!("inspector timing analysis failed: {error}")))?);
            }
            for (index, action) in candidates.iter().enumerate() {
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
    payable: bool,
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
            payable: ctx.plan.payable,
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
        if job.payable && plan.score >= job.score {
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
                    PlannedPipPayment::Waterbend(source) => (source, ManaPaymentSourceKind::Waterbend),
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
