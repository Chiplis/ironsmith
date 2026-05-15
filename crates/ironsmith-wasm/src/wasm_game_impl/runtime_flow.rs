impl WasmGame {
    fn recompute_ui_decision(&mut self) -> Result<(), JsValue> {
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.pending_live_continuation = None;
        self.priority_state.pending_continuation = None;
        self.priority_epoch_checkpoint = None;
        self.priority_epoch_has_undoable_action = false;
        self.priority_epoch_undo_locked_by_mana = false;
        self.priority_epoch_undo_land_stable_id = None;
        self.active_viewed_cards = None;
        self.active_audit_viewed_cards.clear();
        self.clear_active_resolving_stack_object();
        if self.game_over.is_some() {
            return Ok(());
        }
        self.advance_until_decision()
    }

    fn live_action_error_checkpoint(
        &self,
        local_action_checkpoint: Option<&ReplayCheckpoint>,
    ) -> Option<ReplayCheckpoint> {
        self.pending_action_checkpoint
            .as_ref()
            .or(local_action_checkpoint)
            .cloned()
    }

    fn restore_live_action_chain_to_checkpoint(
        &mut self,
        checkpoint: ReplayCheckpoint,
    ) -> Result<(), JsValue> {
        self.restore_replay_checkpoint(&checkpoint);
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.pending_live_continuation = None;
        self.priority_state.pending_continuation = None;
        self.active_viewed_cards = None;
        self.active_audit_viewed_cards.clear();
        self.clear_active_resolving_stack_object();
        self.advance_until_decision()?;
        Ok(())
    }

    fn rollback_live_action_chain_to_checkpoint(
        &mut self,
        checkpoint: ReplayCheckpoint,
    ) -> Result<JsValue, JsValue> {
        self.restore_live_action_chain_to_checkpoint(checkpoint)?;
        self.snapshot()
    }

    pub(super) fn should_auto_resolve_cleanup_discard(&self, ctx: &DecisionContext) -> bool {
        if !self.auto_cleanup_discard {
            return false;
        }
        let DecisionContext::SelectObjects(obj) = ctx else {
            return false;
        };
        self.game.turn.step == Some(ironsmith::game_state::Step::Cleanup)
            && obj.min > 0
            && obj.player != self.perspective
    }

    pub(super) fn advance_until_decision(&mut self) -> Result<(), JsValue> {
        use ironsmith::turn_runner::TurnAction;

        let total_started_at = PerfTimer::start();
        let mut perf = AdvanceUntilDecisionPerfMetrics::default();
        self.last_advance_until_decision_perf = None;

        if self.pregame.is_some() {
            for _ in 0..64 {
                perf.iterations += 1;
                let normalize_started_at = PerfTimer::start();
                self.normalize_pregame_state()?;
                perf.pregame_normalize_ms += normalize_started_at.elapsed_ms();
                let build_started_at = PerfTimer::start();
                if let Some(ctx) = self.build_pregame_decision()? {
                    perf.pregame_decision_build_ms += build_started_at.elapsed_ms();
                    self.pending_decision = Some(ctx);
                    self.runner_pending_decision = false;
                    perf.total_ms = total_started_at.elapsed_ms();
                    perf.final_outcome = "pregame_decision".to_string();
                    self.last_advance_until_decision_perf = Some(perf);
                    return Ok(());
                }
                perf.pregame_decision_build_ms += build_started_at.elapsed_ms();
                if self.pregame.is_none() {
                    break;
                }
            }
        }

        // Lazily create the TurnRunner on first call.
        if self.runner.is_none() {
            self.runner = Some(ironsmith::turn_runner::TurnRunner::new());
            self.runner_awaiting_priority = false;
        }

        for _ in 0..192 {
            perf.iterations += 1;
            // If we're NOT currently inside a priority loop, advance the TurnRunner
            if !self.runner_awaiting_priority {
                let runner_advance_started_at = PerfTimer::start();
                let action = {
                    let runner = self.runner.as_mut().unwrap();
                    runner
                        .advance(&mut self.game, &mut self.trigger_queue)
                        .map_err(|e| JsValue::from_str(&format!("{e}")))?
                };
                perf.runner_advance_ms += runner_advance_started_at.elapsed_ms();

                match action {
                    TurnAction::Continue => continue,

                    TurnAction::Decision(ctx) => {
                        self.clear_active_resolving_stack_object();
                        // Auto-resolve cleanup discards when the flag is set.
                        if self.should_auto_resolve_cleanup_discard(&ctx)
                            && let DecisionContext::SelectObjects(ref obj) = ctx
                        {
                            let auto_cleanup_started_at = PerfTimer::start();
                            let mut ids: Vec<_> = obj
                                .candidates
                                .iter()
                                .filter(|c| c.legal)
                                .map(|c| c.id)
                                .collect();
                            self.game.shuffle_slice(&mut ids);
                            ids.truncate(obj.min);
                            self.runner.as_mut().unwrap().respond_discard(ids);
                            perf.auto_cleanup_discard_ms += auto_cleanup_started_at.elapsed_ms();
                            continue;
                        }
                        self.pending_decision = Some(ctx);
                        self.runner_pending_decision = true;
                        perf.total_ms = total_started_at.elapsed_ms();
                        perf.final_outcome = "runner_decision".to_string();
                        self.last_advance_until_decision_perf = Some(perf);
                        return Ok(());
                    }

                    TurnAction::RunPriority => {
                        self.priority_state
                            .reset_for_new_priority_window(&mut self.game);
                        self.runner_awaiting_priority = true;
                        // Fall through to the priority loop below
                    }

                    TurnAction::TurnComplete => {
                        // Check for game over before starting next turn
                        let remaining: Vec<_> = self
                            .game
                            .players
                            .iter()
                            .filter(|p| p.is_in_game())
                            .collect();
                        if remaining.len() <= 1 {
                            let result = if let Some(winner) = remaining.first() {
                                GameResult::Winner(winner.id)
                            } else {
                                GameResult::Draw
                            };
                            self.game_over = Some(result);
                            return Ok(());
                        }

                        // Advance to next turn
                        self.game.next_turn();
                        self.runner = Some(ironsmith::turn_runner::TurnRunner::new());
                        self.runner_awaiting_priority = false;
                        continue;
                    }

                    TurnAction::GameOver(result) => {
                        self.game_over = Some(result);
                        perf.total_ms = total_started_at.elapsed_ms();
                        perf.final_outcome = "runner_game_over".to_string();
                        self.last_advance_until_decision_perf = Some(perf);
                        return Ok(());
                    }
                }
            }

            // We're inside a priority loop - use existing priority mechanism
            if self.priority_epoch_checkpoint.is_none() {
                self.priority_epoch_checkpoint = Some(self.capture_replay_checkpoint());
                self.priority_epoch_has_undoable_action = false;
                self.priority_epoch_undo_locked_by_mana = false;
                self.priority_epoch_undo_land_stable_id = None;
            }
            let checkpoint = self.capture_replay_checkpoint();
            let replay_started_at = PerfTimer::start();
            let outcome = self.execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])?;
            perf.replay_advance_ms += replay_started_at.elapsed_ms();
            perf.replay_execution = self.last_replay_execution_perf.clone();

            match outcome {
                ReplayOutcome::NeedsDecision(ctx) => {
                    self.pending_decision = Some(ctx);
                    self.runner_pending_decision = false;
                    self.pending_replay_action = Some(PendingReplayAction {
                        checkpoint,
                        root: ReplayRoot::Advance,
                        nested_answers: Vec::new(),
                    });
                    perf.total_ms = total_started_at.elapsed_ms();
                    perf.final_outcome = "replay_needs_decision".to_string();
                    self.last_advance_until_decision_perf = Some(perf);
                    return Ok(());
                }
                ReplayOutcome::Complete(progress) => match progress {
                    GameProgress::NeedsDecisionCtx(ctx) => {
                        self.clear_active_resolving_stack_object();
                        self.pending_decision = Some(ctx);
                        self.runner_pending_decision = false;
                        perf.total_ms = total_started_at.elapsed_ms();
                        perf.final_outcome = "progress_needs_decision".to_string();
                        self.last_advance_until_decision_perf = Some(perf);
                        return Ok(());
                    }
                    GameProgress::Continue => {
                        // Priority loop ended - notify runner
                        self.runner.as_mut().unwrap().priority_done();
                        self.runner_awaiting_priority = false;
                        self.pending_action_checkpoint = None;
                        self.priority_epoch_checkpoint = None;
                        self.priority_epoch_has_undoable_action = false;
                        self.priority_epoch_undo_locked_by_mana = false;
                        self.priority_epoch_undo_land_stable_id = None;
                        self.pending_decision = None;
                        self.clear_active_resolving_stack_object();
                        continue;
                    }
                    GameProgress::StackResolved => {
                        // New priority round after resolution — fresh epoch.
                        self.pending_action_checkpoint = None;
                        self.priority_epoch_checkpoint = None;
                        self.priority_epoch_has_undoable_action = false;
                        self.priority_epoch_undo_locked_by_mana = false;
                        self.priority_epoch_undo_land_stable_id = None;
                        self.clear_active_resolving_stack_object();
                        continue;
                    }
                    GameProgress::GameOver(result) => {
                        self.pending_action_checkpoint = None;
                        self.pending_decision = None;
                        self.clear_active_resolving_stack_object();
                        self.game_over = Some(result);
                        perf.total_ms = total_started_at.elapsed_ms();
                        perf.final_outcome = "progress_game_over".to_string();
                        self.last_advance_until_decision_perf = Some(perf);
                        return Ok(());
                    }
                },
            }
        }

        perf.total_ms = total_started_at.elapsed_ms();
        perf.final_outcome = "iteration_budget_exceeded".to_string();
        self.last_advance_until_decision_perf = Some(perf);
        Err(JsValue::from_str(
            "advance loop exceeded iteration budget (possible infinite loop)",
        ))
    }

    fn apply_progress(&mut self, progress: GameProgress) -> Result<(), JsValue> {
        match progress {
            GameProgress::NeedsDecisionCtx(ctx) => {
                self.clear_active_resolving_stack_object();
                self.pending_decision = Some(ctx);
                Ok(())
            }
            GameProgress::Continue => {
                // Priority loop ended - notify runner and continue
                if self.runner.is_some() {
                    self.runner.as_mut().unwrap().priority_done();
                    self.runner_awaiting_priority = false;
                }
                self.pending_action_checkpoint = None;
                self.priority_epoch_checkpoint = None;
                self.priority_epoch_has_undoable_action = false;
                self.priority_epoch_undo_locked_by_mana = false;
                self.priority_epoch_undo_land_stable_id = None;
                self.pending_decision = None;
                self.clear_active_resolving_stack_object();
                self.advance_until_decision()
            }
            GameProgress::GameOver(result) => {
                self.pending_action_checkpoint = None;
                self.pending_decision = None;
                self.clear_active_resolving_stack_object();
                self.game_over = Some(result);
                Ok(())
            }
            GameProgress::StackResolved => {
                self.pending_action_checkpoint = None;
                self.priority_epoch_checkpoint = None;
                self.priority_epoch_has_undoable_action = false;
                self.priority_epoch_undo_locked_by_mana = false;
                self.priority_epoch_undo_land_stable_id = None;
                self.pending_decision = None;
                self.clear_active_resolving_stack_object();
                self.advance_until_decision()
            }
        }
    }

    /// Handle a response to a TurnRunner-sourced decision (attackers/blockers/discard).
    fn dispatch_runner_decision(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let _runner = self.runner.as_mut().ok_or_else(|| {
            // Restore decision on structural error so UI can retry.
            self.pending_decision = Some(pending_ctx.clone());
            self.runner_pending_decision = true;
            JsValue::from_str("runner_pending_decision set but no runner present")
        })?;

        let restore_on_err = |this: &mut Self, ctx: DecisionContext, err: JsValue| -> JsValue {
            this.pending_decision = Some(ctx);
            this.runner_pending_decision = true;
            err
        };

        match (&pending_ctx, command) {
            (DecisionContext::Attackers(actx), UiCommand::DeclareAttackers { declarations }) => {
                let converted = validate_attacker_declarations(actx, &declarations)
                    .map_err(|e| restore_on_err(self, pending_ctx.clone(), e))?;
                self.runner.as_mut().unwrap().respond_attackers(converted);
            }
            (DecisionContext::Blockers(bctx), UiCommand::DeclareBlockers { declarations }) => {
                let player = bctx.player;
                let converted = validate_blocker_declarations(bctx, &declarations)
                    .map_err(|e| restore_on_err(self, pending_ctx.clone(), e))?;
                self.runner
                    .as_mut()
                    .unwrap()
                    .respond_blockers(converted, player);
            }
            (DecisionContext::SelectObjects(obj_ctx), UiCommand::SelectObjects { object_ids }) => {
                let object_ids = normalize_select_object_choice_ids(obj_ctx, &object_ids);
                // Validate discard selection against the decision context.
                let legal_ids: Vec<u64> = obj_ctx
                    .candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id.0)
                    .collect();
                validate_object_selection(
                    obj_ctx.min,
                    obj_ctx.max,
                    obj_ctx.allow_partial_completion,
                    &object_ids,
                    &legal_ids,
                )
                .map_err(|e| restore_on_err(self, pending_ctx.clone(), e))?;

                let cards: Vec<ObjectId> = object_ids
                    .iter()
                    .map(|&id| ObjectId::from_raw(id))
                    .collect();
                self.runner.as_mut().unwrap().respond_discard(cards);
            }
            (DecisionContext::Boolean(_), UiCommand::SelectOptions { option_indices }) => {
                validate_option_selection(1, Some(1), &option_indices, &[0usize, 1usize])?;
                let answer = option_indices.first().copied() == Some(1);
                self.runner.as_mut().unwrap().respond_boolean(answer);
            }
            _ => {
                self.pending_decision = Some(pending_ctx);
                self.runner_pending_decision = true;
                return Err(JsValue::from_str("unexpected command for runner decision"));
            }
        }

        // The runner is now in a state where advance() will apply the response.
        // We're no longer awaiting priority (runner will handle the next steps).
        self.runner_awaiting_priority = false;
        self.advance_until_decision()?;
        self.snapshot()
    }

    pub(super) fn finish_live_priority_dispatch(
        &mut self,
        progress: GameProgress,
        action_checkpoint: Option<ReplayCheckpoint>,
        resolving_checkpoint: Option<ReplayCheckpoint>,
    ) -> Result<JsValue, JsValue> {
        match progress {
            GameProgress::NeedsDecisionCtx(next_ctx) => {
                let action_still_pending = self.priority_action_chain_still_pending();
                if action_still_pending {
                    self.clear_active_resolving_stack_object();
                } else {
                    self.sync_active_resolving_stack_object(resolving_checkpoint.as_ref());
                }
                if action_still_pending {
                    if let Some(checkpoint) = action_checkpoint {
                        self.pending_action_checkpoint.get_or_insert(checkpoint);
                    }
                } else {
                    self.pending_action_checkpoint = None;
                }

                if !action_still_pending {
                    self.priority_state.pending_continuation = None;
                    self.pending_live_action_root = None;
                    self.pending_replay_action = None;
                    self.pending_live_continuation = Some(LivePriorityContinuation {
                        checkpoint: self.capture_replay_checkpoint_tagged("finish_live_dispatch"),
                        root: PendingPriorityContinuation::ApplyDecisionContext(next_ctx.clone()),
                        answers: Vec::new(),
                        speculative_progress: None,
                    });
                } else if self.decision_uses_live_priority_response(&next_ctx) {
                    self.priority_state.pending_continuation = None;
                    self.pending_live_continuation = None;
                    self.pending_replay_action = None;
                } else {
                    self.priority_state.pending_continuation = None;
                    self.pending_live_continuation = Some(LivePriorityContinuation {
                        checkpoint: self.capture_replay_checkpoint_tagged("finish_live_dispatch"),
                        root: PendingPriorityContinuation::ApplyDecisionContext(next_ctx.clone()),
                        answers: Vec::new(),
                        speculative_progress: None,
                    });
                    self.pending_replay_action = None;
                }
                self.pending_decision = Some(next_ctx);
                self.snapshot()
            }
            progress => {
                self.clear_active_resolving_stack_object();
                self.priority_state.pending_continuation = None;
                if let Some(root_response) = self.pending_live_action_root.take() {
                    self.priority_epoch_has_undoable_action |=
                        Self::response_starts_cancelable_action_chain(&root_response);

                    if let Some(checkpoint) = self
                        .pending_action_checkpoint
                        .as_ref()
                        .or(action_checkpoint.as_ref())
                    {
                        let root = ReplayRoot::Response(root_response);
                        if Self::replay_root_has_irreversible_mana_activation(
                            &checkpoint.game,
                            &root,
                        ) || self.replay_root_mana_activation_added_to_stack(checkpoint, &root)
                        {
                            self.priority_epoch_undo_locked_by_mana = true;
                        }
                        self.priority_epoch_undo_land_stable_id =
                            self.committed_undo_land_stable_id(checkpoint, &root);
                    }
                }

                self.pending_action_checkpoint = None;
                self.pending_live_continuation = None;
                self.pending_replay_action = None;
                self.apply_progress(progress)?;
                self.snapshot()
            }
        }
    }

    fn dispatch_live_priority_response(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let dispatch_started_at = PerfTimer::start();
        let mut dispatch_perf = DispatchPerfMetrics {
            command_kind: ui_command_kind(&command).to_string(),
            pending_decision_kind: decision_context_kind(&pending_ctx).to_string(),
            route_kind: "live_priority_response".to_string(),
            ..DispatchPerfMetrics::default()
        };

        let command_to_response_started_at = PerfTimer::start();
        let response = match self.command_to_response(&pending_ctx, command) {
            Ok(response) => response,
            Err(err) => {
                self.pending_decision = Some(pending_ctx);
                dispatch_perf.command_to_response_ms = command_to_response_started_at.elapsed_ms();
                dispatch_perf.outcome_kind = "command_to_response_error".to_string();
                self.store_dispatch_perf(dispatch_started_at, dispatch_perf);
                return Err(err);
            }
        };
        dispatch_perf.command_to_response_ms = command_to_response_started_at.elapsed_ms();

        let should_track_action_checkpoint = self.pending_action_checkpoint.is_none()
            && self.pending_live_action_root.is_none()
            && Self::response_starts_cancelable_action_chain(&response);
        let action_checkpoint_started_at = PerfTimer::start();
        let action_checkpoint =
            should_track_action_checkpoint.then(|| self.capture_replay_checkpoint());
        dispatch_perf.checkpoint_capture_ms += action_checkpoint_started_at.elapsed_ms();
        if should_track_action_checkpoint {
            self.pending_live_action_root = Some(response.clone());
        }

        let step_checkpoint_started_at = PerfTimer::start();
        let step_checkpoint = self.capture_replay_checkpoint_tagged("live_response_dm_capture");
        dispatch_perf.checkpoint_capture_ms += step_checkpoint_started_at.elapsed_ms();
        let carry_viewed_cards = self.active_viewed_cards.clone();
        let mut live_dm = WasmReplayDecisionMaker::new(&[]);
        let execute_started_at = PerfTimer::start();
        let result = apply_priority_response_with_dm(
            &mut self.game,
            &mut self.trigger_queue,
            &mut self.priority_state,
            &response,
            &mut live_dm,
        );
        dispatch_perf.execute_with_replay_ms = execute_started_at.elapsed_ms();
        dispatch_perf.replay_execution = Some(ReplayExecutionPerfMetrics {
            root_kind: "live_priority_response".to_string(),
            root_execution_ms: dispatch_perf.execute_with_replay_ms,
            total_ms: dispatch_perf.execute_with_replay_ms,
            outcome_kind: match &result {
                Ok(GameProgress::NeedsDecisionCtx(_)) => "needs_decision_progress".to_string(),
                Ok(GameProgress::Continue) => "continue_progress".to_string(),
                Ok(GameProgress::StackResolved) => "stack_resolved_progress".to_string(),
                Ok(GameProgress::GameOver(_)) => "game_over_progress".to_string(),
                Err(_) => "apply_priority_response_error".to_string(),
            },
            progress_kind: result
                .as_ref()
                .ok()
                .map(game_progress_kind)
                .map(str::to_string),
            priority_action: last_priority_action_perf(),
            priority_advance: last_priority_advance_perf(),
            ..ReplayExecutionPerfMetrics::default()
        });
        dispatch_perf.outcome_kind = match &result {
            Ok(GameProgress::NeedsDecisionCtx(_)) => "needs_decision_progress".to_string(),
            Ok(GameProgress::Continue) => "continue_progress".to_string(),
            Ok(GameProgress::StackResolved) => "stack_resolved_progress".to_string(),
            Ok(GameProgress::GameOver(_)) => "game_over_progress".to_string(),
            Err(_) => "apply_priority_response_error".to_string(),
        };
        let (pending_context, viewed_cards, audit_viewed_cards) = live_dm.finish();
        self.active_viewed_cards =
            merge_carried_active_viewed_cards(carry_viewed_cards, viewed_cards);
        self.active_audit_viewed_cards = audit_viewed_cards;

        if let Some(next_ctx) = pending_context {
            self.sync_active_resolving_stack_object_for_prompt(Some(&step_checkpoint));
            if self.priority_action_chain_still_pending() {
                if let Some(checkpoint) = action_checkpoint {
                    self.pending_action_checkpoint.get_or_insert(checkpoint);
                }
            } else {
                self.pending_action_checkpoint = None;
            }
            self.priority_state.pending_continuation = None;
            if self.decision_uses_live_priority_response(&next_ctx) {
                self.pending_live_continuation = None;
            } else {
                self.pending_live_continuation = Some(LivePriorityContinuation {
                    checkpoint: step_checkpoint,
                    root: PendingPriorityContinuation::ApplyResponse(response),
                    answers: Vec::new(),
                    speculative_progress: match (&next_ctx, &result) {
                        (DecisionContext::Boolean(_), Ok(progress)) => Some(progress.clone()),
                        _ => None,
                    },
                });
            }
            self.pending_decision = Some(next_ctx);
            dispatch_perf.outcome_kind = "pending_context".to_string();
            return self.finish_dispatch_with_snapshot(dispatch_started_at, dispatch_perf);
        }

        match result {
            Ok(progress) => {
                self.store_dispatch_perf(dispatch_started_at, dispatch_perf);
                self.finish_live_priority_dispatch(
                    progress,
                    action_checkpoint,
                    Some(step_checkpoint),
                )
            }
            Err(err) => {
                if let Some(checkpoint) =
                    self.live_action_error_checkpoint(action_checkpoint.as_ref())
                {
                    if should_track_action_checkpoint {
                        self.pending_live_action_root = None;
                    }
                    dispatch_perf.outcome_kind = "rolled_back_action_error".to_string();
                    self.store_dispatch_perf(dispatch_started_at, dispatch_perf);
                    return self.rollback_live_action_chain_to_checkpoint(checkpoint);
                }
                self.restore_replay_checkpoint(&step_checkpoint);
                if should_track_action_checkpoint {
                    self.pending_live_action_root = None;
                }
                self.pending_decision = Some(pending_ctx);
                self.store_dispatch_perf(dispatch_started_at, dispatch_perf);
                Err(JsValue::from_str(&format!("dispatch failed: {err}")))
            }
        }
    }

    fn dispatch_live_priority_continuation(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let mut continuation = self
            .pending_live_continuation
            .take()
            .ok_or_else(|| JsValue::from_str("no live continuation checkpoint to resume"))?;
        let answer = match self.command_to_replay_answer(&pending_ctx, command) {
            Ok(answer) => answer,
            Err(err) => {
                self.pending_decision = Some(pending_ctx);
                self.pending_live_continuation = Some(continuation);
                return Err(err);
            }
        };
        if matches!(
            (&continuation.root, &pending_ctx, &answer),
            (
                PendingPriorityContinuation::ApplyDecisionContext(DecisionContext::Boolean(_)),
                DecisionContext::Boolean(_),
                ReplayDecisionAnswer::Boolean(false),
            )
        ) && continuation
            .speculative_progress
            .as_ref()
            .is_some_and(|progress| !matches!(progress, GameProgress::NeedsDecisionCtx(_)))
        {
            return self.finish_live_priority_dispatch(
                continuation
                    .speculative_progress
                    .take()
                    .expect("checked speculative progress above"),
                None,
                Some(continuation.checkpoint.clone()),
            );
        }
        continuation.answers.push(answer);

        // Diagnostic: record whether checkpoint has pending_activation before restore
        let checkpoint_diag_tag = continuation.checkpoint.diag_tag;
        let checkpoint_has_pa = continuation
            .checkpoint
            .priority_state
            .pending_activation
            .is_some();
        let checkpoint_pa_debug = continuation
            .checkpoint
            .priority_state
            .pending_activation
            .as_ref()
            .map(|p| {
                format!(
                    "stage={}, staged_remove={}, remaining_costs={}",
                    p.stage,
                    p.pending_remove_counters_among.is_some(),
                    p.remaining_cost_steps.len()
                )
            });
        let live_pa_before = self.priority_state.pending_activation.is_some();

        let pending_crypto_audit_before = self.pending_crypto_audit_before.take();
        let carry_viewed_cards = self.active_viewed_cards.clone();
        self.restore_replay_checkpoint(&continuation.checkpoint);
        self.pending_crypto_audit_before = pending_crypto_audit_before;
        self.priority_state.pending_continuation = None;

        let live_pa_after = self.priority_state.pending_activation.is_some();
        let mut live_dm = WasmReplayDecisionMaker::new(&continuation.answers);
        let result = match &continuation.root {
            PendingPriorityContinuation::ApplyResponse(response) => {
                apply_priority_response_with_dm(
                    &mut self.game,
                    &mut self.trigger_queue,
                    &mut self.priority_state,
                    response,
                    &mut live_dm,
                )
            }
            PendingPriorityContinuation::ApplyDecisionContext(ctx) => {
                apply_decision_context_with_dm(
                    &mut self.game,
                    &mut self.trigger_queue,
                    &mut self.priority_state,
                    ctx,
                    &mut live_dm,
                )
            }
        };
        let (pending_context, viewed_cards, audit_viewed_cards) = live_dm.finish();
        self.active_viewed_cards =
            merge_carried_active_viewed_cards(carry_viewed_cards, viewed_cards);
        self.active_audit_viewed_cards = audit_viewed_cards;

        if let Some(next_ctx) = pending_context {
            self.sync_active_resolving_stack_object_for_prompt(Some(&continuation.checkpoint));
            self.priority_state.pending_continuation = None;
            continuation.checkpoint.diag_tag = "continuation_dm_capture";
            continuation.speculative_progress = match (&next_ctx, &result) {
                (DecisionContext::Boolean(_), Ok(progress)) => Some(progress.clone()),
                _ => None,
            };
            self.pending_live_continuation = Some(continuation);
            self.pending_decision = Some(next_ctx);
            return self.snapshot();
        }

        match result {
            Ok(progress) => self.finish_live_priority_dispatch(
                progress,
                None,
                Some(continuation.checkpoint.clone()),
            ),
            Err(err) => {
                if let Some(checkpoint) = self.live_action_error_checkpoint(None) {
                    return self.rollback_live_action_chain_to_checkpoint(checkpoint);
                }
                self.restore_replay_checkpoint(&continuation.checkpoint);
                self.priority_state.pending_continuation = None;
                self.pending_live_continuation = Some(continuation);
                self.pending_decision = Some(pending_ctx);
                Err(JsValue::from_str(&format!(
                    "dispatch failed: {err} [diag: tag={checkpoint_diag_tag}, checkpoint_has_pa={checkpoint_has_pa}, \
                     checkpoint_pa={checkpoint_pa_debug:?}, \
                     live_pa_before={live_pa_before}, live_pa_after={live_pa_after}]"
                )))
            }
        }
    }

    fn refresh_live_continuation_after_hidden_reveal(&mut self) -> Result<JsValue, JsValue> {
        let mut continuation = self
            .pending_live_continuation
            .take()
            .ok_or_else(|| JsValue::from_str("no live continuation checkpoint to refresh"))?;
        continuation.speculative_progress = None;
        let pending_ctx = self.pending_decision.clone();
        let pending_crypto_audit_before = self.pending_crypto_audit_before.take();
        let carry_viewed_cards = self.active_viewed_cards.clone();
        self.restore_replay_checkpoint(&continuation.checkpoint);
        self.pending_crypto_audit_before = pending_crypto_audit_before;
        self.priority_state.pending_continuation = None;

        let mut live_dm = WasmReplayDecisionMaker::new(&continuation.answers);
        let result = match &continuation.root {
            PendingPriorityContinuation::ApplyResponse(response) => apply_priority_response_with_dm(
                &mut self.game,
                &mut self.trigger_queue,
                &mut self.priority_state,
                response,
                &mut live_dm,
            ),
            PendingPriorityContinuation::ApplyDecisionContext(ctx) => {
                apply_decision_context_with_dm(
                    &mut self.game,
                    &mut self.trigger_queue,
                    &mut self.priority_state,
                    ctx,
                    &mut live_dm,
                )
            }
        };
        let (pending_context, viewed_cards, audit_viewed_cards) = live_dm.finish();
        self.active_viewed_cards =
            merge_carried_active_viewed_cards(carry_viewed_cards, viewed_cards);
        self.active_audit_viewed_cards = audit_viewed_cards;

        if let Some(next_ctx) = pending_context {
            self.sync_active_resolving_stack_object_for_prompt(Some(&continuation.checkpoint));
            self.priority_state.pending_continuation = None;
            continuation.checkpoint.diag_tag = "continuation_hidden_reveal_refresh";
            continuation.speculative_progress = match (&next_ctx, &result) {
                (DecisionContext::Boolean(_), Ok(progress)) => Some(progress.clone()),
                _ => None,
            };
            self.pending_live_continuation = Some(continuation);
            self.pending_decision = Some(next_ctx);
            return self.snapshot();
        }

        match result {
            Ok(progress) => self.finish_live_priority_dispatch(
                progress,
                None,
                Some(continuation.checkpoint.clone()),
            ),
            Err(err) => {
                self.restore_replay_checkpoint(&continuation.checkpoint);
                self.priority_state.pending_continuation = None;
                self.pending_live_continuation = Some(continuation);
                self.pending_decision = pending_ctx;
                Err(JsValue::from_str(&format!(
                    "hidden reveal continuation refresh failed: {err}"
                )))
            }
        }
    }

    fn capture_replay_checkpoint_tagged(&self, tag: &'static str) -> ReplayCheckpoint {
        ReplayCheckpoint {
            game: self.game.clone(),
            trigger_queue: self.trigger_queue.clone(),
            priority_state: self.priority_state.clone(),
            game_over: self.game_over.clone(),
            id_counters: snapshot_id_counters(),
            diag_tag: tag,
        }
    }

    pub(super) fn capture_replay_checkpoint(&self) -> ReplayCheckpoint {
        self.capture_replay_checkpoint_tagged("untagged")
    }

    fn restore_replay_checkpoint(&mut self, checkpoint: &ReplayCheckpoint) {
        restore_id_counters(checkpoint.id_counters);
        self.game = checkpoint.game.clone();
        self.trigger_queue = checkpoint.trigger_queue.clone();
        self.priority_state = checkpoint.priority_state.clone();
        self.game_over = checkpoint.game_over.clone();
        self.last_crypto_requirements.clear();
        self.pending_crypto_audit_before = None;
    }

    pub(super) fn clear_active_resolving_stack_object(&mut self) {
        self.active_resolving_stack_object = None;
    }

    fn sync_active_resolving_stack_object(&mut self, checkpoint: Option<&ReplayCheckpoint>) {
        if let Some(checkpoint) = checkpoint {
            self.update_active_resolving_stack_object_from_checkpoint(checkpoint);
        } else {
            self.clear_active_resolving_stack_object();
        }
    }

    fn sync_active_resolving_stack_object_for_prompt(
        &mut self,
        checkpoint: Option<&ReplayCheckpoint>,
    ) {
        if self.priority_action_chain_still_pending() {
            self.clear_active_resolving_stack_object();
        } else {
            self.sync_active_resolving_stack_object(checkpoint);
        }
    }

    fn resolving_stack_object_from_checkpoint(
        &self,
        checkpoint: &ReplayCheckpoint,
    ) -> Option<StackObjectSnapshot> {
        let entry = checkpoint.game.stack.last()?;
        if checkpoint.game.stack.len() != self.game.stack.len() + 1 {
            return None;
        }
        if self
            .game
            .stack
            .iter()
            .any(|current| current.object_id == entry.object_id)
        {
            return None;
        }
        Some(build_stack_object_snapshot(
            &self.game,
            self.perspective,
            self.active_viewed_cards.as_ref(),
            entry,
        ))
    }

    fn update_active_resolving_stack_object_from_checkpoint(
        &mut self,
        checkpoint: &ReplayCheckpoint,
    ) {
        self.active_resolving_stack_object =
            self.resolving_stack_object_from_checkpoint(checkpoint);
    }

    pub(super) fn execute_with_replay(
        &mut self,
        checkpoint: &ReplayCheckpoint,
        root: &ReplayRoot,
        nested_answers: &[ReplayDecisionAnswer],
    ) -> Result<ReplayOutcome, JsValue> {
        let total_started_at = PerfTimer::start();
        let mut perf = ReplayExecutionPerfMetrics {
            root_kind: replay_root_kind(root).to_string(),
            ..ReplayExecutionPerfMetrics::default()
        };
        self.last_replay_execution_perf = None;

        let restore_started_at = PerfTimer::start();
        let carry_viewed_cards = self.active_viewed_cards.clone();
        let carry_audit_viewed_cards = self.active_audit_viewed_cards.clone();
        let pending_crypto_audit_before = self.pending_crypto_audit_before.take();
        self.restore_replay_checkpoint(checkpoint);
        self.pending_crypto_audit_before = pending_crypto_audit_before;
        perf.restore_checkpoint_ms = restore_started_at.elapsed_ms();
        self.active_viewed_cards = None;
        self.active_audit_viewed_cards.clear();
        self.clear_active_resolving_stack_object();

        let mut replay_dm = WasmReplayDecisionMaker::new(nested_answers);

        let root_execution_started_at = PerfTimer::start();
        let result = match root {
            ReplayRoot::Response(response) => apply_priority_response_with_dm(
                &mut self.game,
                &mut self.trigger_queue,
                &mut self.priority_state,
                response,
                &mut replay_dm,
            )
            .map_err(|e| format!("{e}")),
            ReplayRoot::Advance => {
                // Resume only until the next externally visible priority/decision boundary.
                // Using run_priority_loop_with here would auto-pass any fresh pass-only
                // windows after a nested answer (for example, after trigger ordering),
                // which skips the per-trigger priority opportunities players must get.
                advance_priority_with_dm(&mut self.game, &mut self.trigger_queue, &mut replay_dm)
                    .map_err(|e| format!("{e}"))
            }
            ReplayRoot::AddCardToZone {
                player,
                card_name,
                zone,
                skip_triggers,
            } => {
                self.registry.ensure_cards_loaded([card_name.as_str()]);
                match self.load_compilable_card_definition(card_name) {
                    Ok(definition) => self
                        .add_card_to_zone_with_dm(
                            *player,
                            &definition,
                            *zone,
                            *skip_triggers,
                            &mut replay_dm,
                        )
                        .map(|_| GameProgress::Continue),
                    Err(err) => Err(err
                        .as_string()
                        .unwrap_or_else(|| "failed to load card for replay".to_string())),
                }
            }
        };
        perf.root_execution_ms = root_execution_started_at.elapsed_ms();
        perf.priority_action = last_priority_action_perf();
        perf.priority_advance = last_priority_advance_perf();

        let finish_started_at = PerfTimer::start();
        let (pending_context, viewed_cards, audit_viewed_cards) = replay_dm.finish();
        perf.decision_maker_finish_ms = finish_started_at.elapsed_ms();
        self.active_viewed_cards =
            merge_carried_active_viewed_cards(carry_viewed_cards, viewed_cards);
        self.active_audit_viewed_cards = if audit_viewed_cards.is_empty() {
            carry_audit_viewed_cards
        } else {
            audit_viewed_cards
        };

        if let Some(next_ctx) = pending_context {
            self.sync_active_resolving_stack_object_for_prompt(Some(checkpoint));
            let outcome = ReplayOutcome::NeedsDecision(next_ctx);
            perf.outcome_kind = replay_outcome_kind(&outcome).to_string();
            perf.total_ms = total_started_at.elapsed_ms();
            self.last_replay_execution_perf = Some(perf);
            return Ok(outcome);
        }

        match result {
            Ok(progress) => {
                if matches!(progress, GameProgress::NeedsDecisionCtx(_)) {
                    self.sync_active_resolving_stack_object_for_prompt(Some(checkpoint));
                } else {
                    self.clear_active_resolving_stack_object();
                }
                let outcome = ReplayOutcome::Complete(progress);
                perf.outcome_kind = replay_outcome_kind(&outcome).to_string();
                if let ReplayOutcome::Complete(progress) = &outcome {
                    perf.progress_kind = Some(game_progress_kind(progress).to_string());
                }
                perf.total_ms = total_started_at.elapsed_ms();
                self.last_replay_execution_perf = Some(perf);
                Ok(outcome)
            }
            Err(e) => {
                self.active_viewed_cards = None;
                self.active_audit_viewed_cards.clear();
                self.clear_active_resolving_stack_object();
                self.restore_replay_checkpoint(checkpoint);
                perf.outcome_kind = "error".to_string();
                perf.total_ms = total_started_at.elapsed_ms();
                self.last_replay_execution_perf = Some(perf);
                Err(JsValue::from_str(&format!("dispatch failed: {e}")))
            }
        }
    }

    fn command_to_replay_answer(
        &mut self,
        ctx: &DecisionContext,
        command: UiCommand,
    ) -> Result<ReplayDecisionAnswer, JsValue> {
        match (ctx, command) {
            (DecisionContext::Boolean(_), UiCommand::SelectOptions { option_indices }) => {
                validate_option_selection(1, Some(1), &option_indices, &[0usize, 1usize])?;
                let choice = option_indices
                    .first()
                    .copied()
                    .ok_or_else(|| JsValue::from_str("boolean choice requires one option"))?;
                Ok(ReplayDecisionAnswer::Boolean(choice == 1))
            }
            (DecisionContext::Number(number), UiCommand::NumberChoice { value }) => {
                if value < number.min || value > number.max {
                    return Err(JsValue::from_str(&format!(
                        "number out of range: expected {}..={}, got {}",
                        number.min, number.max, value
                    )));
                }
                Ok(ReplayDecisionAnswer::Number(value))
            }
            (DecisionContext::TextInput(text), UiCommand::TextChoice { value }) => {
                let value = value.trim();
                if value.is_empty() {
                    return Err(JsValue::from_str("text choice cannot be empty"));
                }
                if text.require_known_value && !self.is_known_card_name_query(value) {
                    return Err(JsValue::from_str(&format!("unknown card name: {value}")));
                }
                Ok(ReplayDecisionAnswer::Text(value.to_string()))
            }
            (
                DecisionContext::SelectOptions(options),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal_indices: Vec<usize> = options
                    .options
                    .iter()
                    .filter(|o| o.legal)
                    .map(|o| o.index)
                    .collect();
                validate_option_selection(
                    options.min,
                    Some(options.max),
                    &option_indices,
                    &legal_indices,
                )?;
                Ok(ReplayDecisionAnswer::Options(option_indices))
            }
            (
                DecisionContext::Priority(priority),
                UiCommand::PriorityAction {
                    action_index,
                    action_ref,
                },
            ) => {
                let action = resolve_priority_action(priority, action_index, action_ref.as_ref())
                    .ok_or_else(|| {
                    if let Some(action_ref) = action_ref.as_ref() {
                        JsValue::from_str(&format!("invalid priority action ref: {action_ref:?}"))
                    } else if let Some(action_index) = action_index {
                        JsValue::from_str(&format!("invalid priority action index: {action_index}"))
                    } else {
                        JsValue::from_str("missing priority action selector")
                    }
                })?;
                Ok(ReplayDecisionAnswer::Priority(action))
            }
            (DecisionContext::SelectObjects(objects), UiCommand::SelectObjects { object_ids }) => {
                let object_ids = normalize_select_object_choice_ids(objects, &object_ids);
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|obj| obj.legal)
                    .map(|obj| obj.id.0)
                    .collect();
                validate_object_selection(
                    objects.min,
                    objects.max,
                    objects.allow_partial_completion,
                    &object_ids,
                    &legal_ids,
                )?;
                Ok(ReplayDecisionAnswer::Objects(
                    object_ids
                        .into_iter()
                        .map(ObjectId::from_raw)
                        .collect::<Vec<_>>(),
                ))
            }
            (DecisionContext::Order(order), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = (0..order.items.len()).collect();
                validate_option_selection(
                    order.items.len(),
                    Some(order.items.len()),
                    &option_indices,
                    &legal,
                )?;
                if unique_indices(&option_indices).len() != order.items.len() {
                    return Err(JsValue::from_str(
                        "ordering requires each option index exactly once",
                    ));
                }
                Ok(ReplayDecisionAnswer::Order(
                    option_indices
                        .into_iter()
                        .filter_map(|index| order.items.get(index).map(|(id, _)| *id))
                        .collect(),
                ))
            }
            (
                DecisionContext::Distribute(distribute),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal: Vec<usize> = (0..distribute.targets.len()).collect();
                validate_option_selection(
                    0,
                    Some(distribute.total as usize),
                    &option_indices,
                    &legal,
                )?;

                if distribute.targets.is_empty() || distribute.total == 0 {
                    return Ok(ReplayDecisionAnswer::Distribute(Vec::new()));
                }

                let mut counts: HashMap<usize, u32> = HashMap::new();
                for index in option_indices {
                    *counts.entry(index).or_insert(0) += 1;
                }

                let total_assigned: u32 = counts.values().sum();
                if total_assigned != distribute.total {
                    return Err(JsValue::from_str(&format!(
                        "distribution must assign exactly {} total (got {})",
                        distribute.total, total_assigned
                    )));
                }

                if distribute.min_per_target > 0
                    && counts
                        .values()
                        .any(|amount| *amount > 0 && *amount < distribute.min_per_target)
                {
                    return Err(JsValue::from_str(&format!(
                        "each selected target must receive at least {}",
                        distribute.min_per_target
                    )));
                }

                let mut allocations: Vec<(Target, u32)> = Vec::new();
                for index in 0..distribute.targets.len() {
                    let Some(amount) = counts.get(&index).copied() else {
                        continue;
                    };
                    if amount == 0 {
                        continue;
                    }
                    allocations.push((distribute.targets[index].target, amount));
                }
                Ok(ReplayDecisionAnswer::Distribute(allocations))
            }
            (DecisionContext::Colors(colors), UiCommand::SelectOptions { option_indices }) => {
                if colors.count == 0 {
                    validate_option_selection(0, Some(0), &option_indices, &[])?;
                    return Ok(ReplayDecisionAnswer::Colors(Vec::new()));
                }

                let choices = colors_for_context(colors);
                if choices.is_empty() {
                    return Err(JsValue::from_str("no legal colors in colors decision"));
                }
                let legal: Vec<usize> = (0..choices.len()).collect();
                let max = if colors.same_color {
                    1
                } else {
                    colors.count as usize
                };
                validate_option_selection(1, Some(max), &option_indices, &legal)?;

                if colors.same_color {
                    let choice = option_indices.first().copied().ok_or_else(|| {
                        JsValue::from_str("color choice requires selecting one option")
                    })?;
                    let color = choices.get(choice).copied().ok_or_else(|| {
                        JsValue::from_str("selected color option is out of range")
                    })?;
                    return Ok(ReplayDecisionAnswer::Colors(vec![
                        color;
                        colors.count as usize
                    ]));
                }

                let mut selected: Vec<ironsmith::color::Color> = option_indices
                    .iter()
                    .copied()
                    .into_iter()
                    .filter_map(|index| choices.get(index).copied())
                    .collect();
                if selected.is_empty() {
                    return Err(JsValue::from_str("choose at least one color"));
                }
                let desired = colors.count as usize;
                if selected.len() > desired {
                    selected.truncate(desired);
                }
                if selected.len() < desired {
                    let pad = selected[0];
                    selected.resize(desired, pad);
                }
                Ok(ReplayDecisionAnswer::Colors(selected))
            }
            (DecisionContext::Counters(counters), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = counters
                    .available_counters
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, available))| *available > 0)
                    .map(|(index, _)| index)
                    .collect();
                validate_option_selection(
                    0,
                    Some(counters.max_total as usize),
                    &option_indices,
                    &legal,
                )?;

                let mut counts: HashMap<usize, u32> = HashMap::new();
                for index in option_indices {
                    *counts.entry(index).or_insert(0) += 1;
                }

                let mut selected: Vec<(ironsmith::object::CounterType, u32)> = Vec::new();
                for index in 0..counters.available_counters.len() {
                    let Some(chosen) = counts.get(&index).copied() else {
                        continue;
                    };
                    let Some((counter_type, available)) =
                        counters.available_counters.get(index).copied()
                    else {
                        continue;
                    };
                    if chosen > available {
                        return Err(JsValue::from_str(&format!(
                            "cannot remove {} of counter {} (only {} available)",
                            chosen,
                            counter_type.description(),
                            available
                        )));
                    }
                    if chosen > 0 {
                        selected.push((counter_type, chosen));
                    }
                }

                Ok(ReplayDecisionAnswer::Counters(selected))
            }
            (DecisionContext::Partition(partition), UiCommand::SelectObjects { object_ids }) => {
                let legal_ids: Vec<u64> = partition.cards.iter().map(|(id, _)| id.0).collect();
                validate_object_selection(
                    0,
                    Some(legal_ids.len()),
                    false,
                    &object_ids,
                    &legal_ids,
                )?;
                Ok(ReplayDecisionAnswer::Partition(
                    unique_object_ids(&object_ids)
                        .into_iter()
                        .map(ObjectId::from_raw)
                        .collect(),
                ))
            }
            (
                DecisionContext::Proliferate(proliferate),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let permanent_count = proliferate.eligible_permanents.len();
                let total_options = permanent_count + proliferate.eligible_players.len();
                let legal: Vec<usize> = (0..total_options).collect();
                validate_option_selection(0, Some(total_options), &option_indices, &legal)?;

                let mut response = ironsmith::decisions::specs::ProliferateResponse::default();
                for index in unique_indices(&option_indices) {
                    if index < permanent_count {
                        if let Some((permanent, _)) = proliferate.eligible_permanents.get(index) {
                            response.permanents.push(*permanent);
                        }
                        continue;
                    }
                    let player_index = index - permanent_count;
                    if let Some((player, _)) = proliferate.eligible_players.get(player_index) {
                        response.players.push(*player);
                    }
                }
                Ok(ReplayDecisionAnswer::Proliferate(response))
            }
            (DecisionContext::Targets(targets_ctx), UiCommand::SelectTargets { targets }) => {
                let converted = convert_and_validate_targets(targets_ctx, targets)
                    .map_err(|err| JsValue::from_str(&err))?;
                Ok(ReplayDecisionAnswer::Targets(converted))
            }
            (
                DecisionContext::Attackers(attackers),
                UiCommand::DeclareAttackers { declarations },
            ) => {
                let converted = validate_attacker_declarations(attackers, &declarations)?
                    .into_iter()
                    .map(
                        |declaration| ironsmith::decisions::spec::AttackerDeclaration {
                            creature: declaration.creature,
                            target: declaration.target,
                        },
                    )
                    .collect();
                Ok(ReplayDecisionAnswer::Attackers(converted))
            }
            (DecisionContext::Blockers(blockers), UiCommand::DeclareBlockers { declarations }) => {
                let converted = validate_blocker_declarations(blockers, &declarations)?
                    .into_iter()
                    .map(
                        |declaration| ironsmith::decisions::spec::BlockerDeclaration {
                            blocker: declaration.blocker,
                            blocking: declaration.blocking,
                        },
                    )
                    .collect();
                Ok(ReplayDecisionAnswer::Blockers(converted))
            }
            (DecisionContext::Modes(modes), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = modes
                    .spec
                    .modes
                    .iter()
                    .filter(|mode| mode.legal)
                    .map(|mode| mode.index)
                    .collect();
                validate_option_selection(
                    modes.spec.min_modes,
                    Some(modes.spec.max_modes),
                    &option_indices,
                    &legal,
                )?;
                Ok(ReplayDecisionAnswer::Options(option_indices))
            }
            (
                DecisionContext::HybridChoice(hybrid),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal: Vec<usize> = hybrid.options.iter().map(|opt| opt.index).collect();
                validate_option_selection(1, Some(1), &option_indices, &legal)?;
                Ok(ReplayDecisionAnswer::Options(option_indices))
            }
            (ctx, _) => Err(JsValue::from_str(&format!(
                "command type does not match pending replay decision: {}",
                decision_context_kind(ctx)
            ))),
        }
    }

    fn command_to_response(
        &self,
        ctx: &DecisionContext,
        command: UiCommand,
    ) -> Result<PriorityResponse, JsValue> {
        match (ctx, command) {
            (
                DecisionContext::Priority(priority),
                UiCommand::PriorityAction {
                    action_index,
                    action_ref,
                },
            ) => {
                let action = resolve_priority_action(priority, action_index, action_ref.as_ref())
                    .ok_or_else(|| {
                    if let Some(action_ref) = action_ref.as_ref() {
                        JsValue::from_str(&format!("invalid priority action ref: {action_ref:?}"))
                    } else if let Some(action_index) = action_index {
                        JsValue::from_str(&format!("invalid priority action index: {action_index}"))
                    } else {
                        JsValue::from_str("missing priority action selector")
                    }
                })?;
                Ok(PriorityResponse::PriorityAction(action))
            }
            (DecisionContext::Number(number), UiCommand::NumberChoice { value }) => {
                if value < number.min || value > number.max {
                    return Err(JsValue::from_str(&format!(
                        "number out of range: expected {}..={}, got {}",
                        number.min, number.max, value
                    )));
                }
                if number.is_x_value {
                    Ok(PriorityResponse::XValue(value))
                } else {
                    Ok(PriorityResponse::NumberChoice(value))
                }
            }
            (DecisionContext::TextInput(_), UiCommand::TextChoice { .. }) => {
                Err(JsValue::from_str(
                    "text input decisions should be replayed through their originating effect",
                ))
            }
            (
                DecisionContext::SelectOptions(options),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal_indices: Vec<usize> = options
                    .options
                    .iter()
                    .filter(|o| o.legal)
                    .map(|o| o.index)
                    .collect();
                validate_option_selection(
                    options.min,
                    Some(options.max),
                    &option_indices,
                    &legal_indices,
                )?;
                self.map_select_options_response(option_indices)
            }
            (DecisionContext::Modes(modes), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = modes
                    .spec
                    .modes
                    .iter()
                    .filter(|mode| mode.legal)
                    .map(|mode| mode.index)
                    .collect();
                validate_option_selection(
                    modes.spec.min_modes,
                    Some(modes.spec.max_modes),
                    &option_indices,
                    &legal,
                )?;
                Ok(PriorityResponse::Modes(option_indices))
            }
            (
                DecisionContext::HybridChoice(hybrid),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal: Vec<usize> = hybrid.options.iter().map(|opt| opt.index).collect();
                validate_option_selection(1, Some(1), &option_indices, &legal)?;
                let choice = option_indices.first().copied().ok_or_else(|| {
                    JsValue::from_str("hybrid choice requires selecting one option")
                })?;
                Ok(PriorityResponse::HybridChoice(choice))
            }
            (DecisionContext::SelectObjects(objects), UiCommand::SelectObjects { object_ids }) => {
                let object_ids = normalize_select_object_choice_ids(objects, &object_ids);
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|obj| obj.legal)
                    .map(|obj| obj.id.0)
                    .collect();
                validate_object_selection(
                    objects.min,
                    objects.max,
                    objects.allow_partial_completion,
                    &object_ids,
                    &legal_ids,
                )?;

                let chosen = object_ids.first().copied().ok_or_else(|| {
                    JsValue::from_str("select_objects requires one chosen object")
                })?;
                if let Some(pending) = self.priority_state.pending_activation.as_ref() {
                    match pending.stage {
                        ActivationStage::ChoosingSacrifice => Ok(
                            PriorityResponse::SacrificeTarget(ObjectId::from_raw(chosen)),
                        ),
                        ActivationStage::ChoosingCardCost => {
                            Ok(PriorityResponse::CardCostChoice(ObjectId::from_raw(chosen)))
                        }
                        _ => Err(JsValue::from_str(
                            "SelectObjects received while activation is not in an object-cost stage",
                        )),
                    }
                } else if self
                    .priority_state
                    .pending_cast
                    .as_ref()
                    .is_some_and(|pending| {
                        matches!(
                            pending.stage,
                            CastStage::ChoosingSacrifice | CastStage::ChoosingCardCost
                        )
                    })
                {
                    Ok(PriorityResponse::CardCostChoice(ObjectId::from_raw(chosen)))
                } else {
                    let cast_stage = self
                        .priority_state
                        .pending_cast
                        .as_ref()
                        .map(|p| p.stage.to_string());
                    let act_stage = self
                        .priority_state
                        .pending_activation
                        .as_ref()
                        .map(|p| p.stage.to_string());
                    Err(JsValue::from_str(&format!(
                        "unsupported SelectObjects context in priority flow \
                         (pending_cast={}, pending_activation={})",
                        cast_stage.as_deref().unwrap_or("none"),
                        act_stage.as_deref().unwrap_or("none"),
                    )))
                }
            }
            (DecisionContext::Targets(targets_ctx), UiCommand::SelectTargets { targets }) => {
                let converted = convert_and_validate_targets(targets_ctx, targets)
                    .map_err(|err| JsValue::from_str(&err))?;
                Ok(PriorityResponse::Targets(converted))
            }
            (
                DecisionContext::Attackers(attackers),
                UiCommand::DeclareAttackers { declarations },
            ) => {
                let converted = validate_attacker_declarations(attackers, &declarations)?;
                Ok(PriorityResponse::Attackers(converted))
            }
            (DecisionContext::Blockers(blockers), UiCommand::DeclareBlockers { declarations }) => {
                let converted = validate_blocker_declarations(blockers, &declarations)?;
                Ok(PriorityResponse::Blockers {
                    defending_player: blockers.player,
                    declarations: converted,
                })
            }
            (DecisionContext::Modes(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Modes(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Modes(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Modes(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Modes(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Targets(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Targets(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Targets(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Targets(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Targets(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Targets(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Number(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Number(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Number(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Number(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Number(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Number(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Priority(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Priority(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Priority(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Attackers(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Attackers(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Attackers(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Blockers(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Blockers(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Blockers(_), UiCommand::DeclareAttackers { .. }) => Err(
                JsValue::from_str("command type does not match pending decision"),
            ),
            (_, _) => Err(JsValue::from_str(&format!(
                "pending decision type is not yet supported in WASM dispatch: {}",
                decision_context_kind(ctx)
            ))),
        }
    }

    fn map_select_options_response(
        &self,
        option_indices: Vec<usize>,
    ) -> Result<PriorityResponse, JsValue> {
        if self.game.effect_store.pending_replacement_choice.is_some() {
            let choice = option_indices.first().copied().ok_or_else(|| {
                JsValue::from_str("replacement effect choice requires one selected option")
            })?;
            return Ok(PriorityResponse::ReplacementChoice(choice));
        }
        if self.priority_state.pending_method_selection.is_some() {
            let choice = option_indices.first().copied().ok_or_else(|| {
                JsValue::from_str("casting method choice requires one selected option")
            })?;
            return Ok(PriorityResponse::CastingMethodChoice(choice));
        }
        if self
            .priority_state
            .pending_cast
            .as_ref()
            .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingOptionalCosts))
        {
            let mut counts: HashMap<usize, u32> = HashMap::new();
            let mut order: Vec<usize> = Vec::new();
            for index in option_indices {
                if !counts.contains_key(&index) {
                    order.push(index);
                }
                *counts.entry(index).or_insert(0) += 1;
            }
            let choices: Vec<(usize, u32)> = order
                .into_iter()
                .filter_map(|index| counts.get(&index).copied().map(|count| (index, count)))
                .collect();
            return Ok(PriorityResponse::OptionalCosts(choices));
        }
        if self.priority_state.pending_mana_ability.is_some() {
            let choice = option_indices
                .first()
                .copied()
                .ok_or_else(|| JsValue::from_str("mana payment choice requires one option"))?;
            return Ok(PriorityResponse::ManaPayment(choice));
        }
        if self
            .priority_state
            .pending_activation
            .as_ref()
            .is_some_and(|pending| matches!(pending.stage, ActivationStage::ChoosingNextCost))
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingNextCost))
        {
            let choice = option_indices
                .first()
                .copied()
                .ok_or_else(|| JsValue::from_str("next-cost choice requires one option"))?;
            return Ok(PriorityResponse::NextCostChoice(choice));
        }
        if self
            .priority_state
            .pending_activation
            .as_ref()
            .is_some_and(|pending| matches!(pending.stage, ActivationStage::PayingMana))
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::PayingMana))
        {
            let choice = option_indices
                .first()
                .copied()
                .ok_or_else(|| JsValue::from_str("mana pip payment requires one option"))?;
            return Ok(PriorityResponse::ManaPipPayment(choice));
        }

        let cast_stage = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stage.to_string());
        let act_stage = self
            .priority_state
            .pending_activation
            .as_ref()
            .map(|p| p.stage.to_string());
        Err(JsValue::from_str(&format!(
            "unsupported SelectOptions context in priority flow \
             (pending_cast={}, pending_activation={}, \
             pending_mana_ability={}, pending_method={}, replacement={})",
            cast_stage.as_deref().unwrap_or("none"),
            act_stage.as_deref().unwrap_or("none"),
            self.priority_state.pending_mana_ability.is_some(),
            self.priority_state.pending_method_selection.is_some(),
            self.game.effect_store.pending_replacement_choice.is_some(),
        )))
    }
}

#[cfg(test)]
mod live_action_rollback_tests {
    use super::*;
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::OptionalCostsPaid;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::{DecisionContext, PriorityContext};
    use ironsmith::decisions::context::SelectableOption;
    use ironsmith::events::cause::EventCause;
    use ironsmith::game_loop::{CastStage, PendingCast};
    use ironsmith::game_state::Phase;
    use ironsmith::ids::{CardId, ObjectId, PlayerId};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::provenance::ProvNodeId;
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;

    fn dispatch_priority_action_matching<F>(wasm: &mut WasmGame, mut predicate: F)
    where
        F: FnMut(&LegalAction) -> bool,
    {
        let pending_ctx = wasm
            .pending_decision
            .take()
            .expect("expected pending priority decision");
        let DecisionContext::Priority(priority) = &pending_ctx else {
            panic!("expected priority decision, got {pending_ctx:?}");
        };
        let index = priority
            .actions
            .iter()
            .position(&mut predicate)
            .unwrap_or_else(|| panic!("expected matching priority action in {:?}", priority.actions));
        wasm.dispatch_live_priority_response(
            pending_ctx,
            UiCommand::PriorityAction {
                action_index: Some(index),
                action_ref: None,
            },
        )
        .expect("priority action should dispatch");
    }

    fn dispatch_pass_priority(wasm: &mut WasmGame) {
        dispatch_priority_action_matching(wasm, |action| matches!(action, LegalAction::PassPriority));
    }

    fn dispatch_select_option(wasm: &mut WasmGame, option_index: usize) {
        let pending_ctx = wasm
            .pending_decision
            .take()
            .expect("expected pending select-options decision");
        let DecisionContext::SelectOptions(_) = &pending_ctx else {
            panic!("expected select-options decision, got {pending_ctx:?}");
        };
        wasm.dispatch_live_priority_response(
            pending_ctx,
            UiCommand::SelectOptions {
                option_indices: vec![option_index],
            },
        )
        .expect("select option should dispatch");
    }

    fn dispatch_select_options_until_priority(wasm: &mut WasmGame) {
        for _ in 0..8 {
            if matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))) {
                return;
            }
            if matches!(wasm.pending_decision, Some(DecisionContext::SelectOptions(_))) {
                dispatch_select_option(wasm, 0);
                continue;
            }
            break;
        }
    }

    fn dispatch_decision_select_option(wasm: &mut WasmGame, option_index: usize) {
        let pending_ctx = wasm
            .pending_decision
            .take()
            .expect("expected pending decision");
        let command = UiCommand::SelectOptions {
            option_indices: vec![option_index],
        };
        if wasm.pending_live_continuation.is_some() {
            wasm.dispatch_live_priority_continuation(pending_ctx, command)
                .expect("live continuation decision should dispatch");
        } else if wasm.decision_uses_live_priority_response(&pending_ctx) {
            wasm.dispatch_live_priority_response(pending_ctx, command)
                .expect("live priority decision should dispatch");
        } else {
            panic!("pending decision is not dispatchable in live flow: {pending_ctx:?}");
        }
    }

    fn object_names(game: &GameState, ids: &[ObjectId]) -> Vec<String> {
        ids.iter()
            .filter_map(|&id| game.object(id).map(|object| object.name.clone()))
            .collect()
    }

    #[test]
    fn live_action_error_restore_returns_to_pre_cast_priority_state() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

        let alice = PlayerId::from_index(0);
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        wasm.runner_awaiting_priority = true;

        let colorless_rock = CardDefinitionBuilder::new(CardId::new(), "Colorless Rock")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Artifact])
            .parse_text("{T}: Add {C}{C}.")
            .expect("colorless mana rock should parse");
        let rock_id =
            wasm.game
                .create_object_from_definition(&colorless_rock, alice, Zone::Battlefield);

        let white_spell = CardDefinitionBuilder::new(CardId::new(), "White Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let hand_spell_id =
            wasm.game
                .create_object_from_definition(&white_spell, alice, Zone::Hand);

        let pre_cast_checkpoint = wasm.capture_replay_checkpoint();
        let stack_spell_id = wasm
            .game
            .move_object(hand_spell_id, Zone::Stack, EventCause::effect())
            .expect("spell should move to stack for staged cast");

        let mut pending = PendingCast::new(
            stack_spell_id,
            Zone::Hand,
            alice,
            ProvNodeId::default(),
            CastStage::PayingMana,
            None,
            Vec::new(),
            CastingMethod::Normal,
            OptionalCostsPaid::new(0),
            None,
            stack_spell_id,
        );
        pending.display_mana_pips = vec![vec![ManaSymbol::White]];
        pending.remaining_mana_pips = vec![vec![ManaSymbol::White]];
        wasm.priority_state.pending_cast = Some(pending);
        wasm.pending_action_checkpoint = Some(pre_cast_checkpoint.clone());
        wasm.pending_decision = Some(DecisionContext::SelectOptions(
            ironsmith::decisions::context::SelectOptionsContext::mana_pip_payment(
                alice,
                stack_spell_id,
                "White Probe",
                "W",
                1,
                vec![SelectableOption::new(0, "Tap Colorless Rock: Add {C}{C}")],
            ),
        ));

        wasm.restore_live_action_chain_to_checkpoint(pre_cast_checkpoint)
            .expect("rollback should return to a decision");

        assert!(
            wasm.priority_state.pending_cast.is_none(),
            "failed payment should clear the staged cast"
        );
        assert!(
            wasm.pending_action_checkpoint.is_none(),
            "failed payment rollback should clear the action checkpoint"
        );
        assert!(
            wasm.game.object(stack_spell_id).is_none(),
            "rolled-back stack object should not remain in the live game"
        );
        assert_eq!(
            wasm.game
                .object(hand_spell_id)
                .expect("original hand spell should be restored")
                .zone,
            Zone::Hand
        );
        assert!(
            !wasm.game.is_tapped(rock_id),
            "mana source activation should be undone by the rollback"
        );
        assert!(
            matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
            "rollback should return to a normal priority decision"
        );
    }

    #[test]
    fn tainted_pact_live_resolution_prompt_can_put_first_card_into_hand() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

        let alice = PlayerId::from_index(0);
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        wasm.runner_awaiting_priority = true;

        let tainted_pact = CardDefinitionBuilder::new(CardId::new(), "Tainted Pact")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Instant])
            .parse_text(
                "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.",
            )
            .expect("Tainted Pact should parse");
        let spell = wasm
            .game
            .create_object_from_definition(&tainted_pact, alice, Zone::Hand);
        if let Some(player) = wasm.game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::Colorless, 1);
            player.mana_pool.add(ManaSymbol::Black, 1);
        }
        let second = CardDefinitionBuilder::new(CardId::new(), "Second Card")
            .card_types(vec![CardType::Artifact])
            .build();
        let first = CardDefinitionBuilder::new(CardId::new(), "First Card")
            .card_types(vec![CardType::Artifact])
            .build();
        wasm.game
            .create_object_from_definition(&second, alice, Zone::Library);
        wasm.game
            .create_object_from_definition(&first, alice, Zone::Library);

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        dispatch_priority_action_matching(&mut wasm, |action| {
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)
        });
        dispatch_select_options_until_priority(&mut wasm);
        dispatch_pass_priority(&mut wasm);
        dispatch_pass_priority(&mut wasm);

        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Boolean(ctx)) => {
                assert!(
                    ctx.description.to_ascii_lowercase().contains("first card"),
                    "expected first Tainted Pact prompt, got {:?}",
                    ctx.description
                );
            }
            other => panic!("expected first Tainted Pact boolean prompt, got {other:?}"),
        }

        wasm.finish_hidden_card_reveal(false)
            .expect("post-resolution hidden opening should preserve the Tainted Pact prompt");
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Boolean(ctx)) => {
                assert!(
                    ctx.description.to_ascii_lowercase().contains("first card"),
                    "hidden opening should not clear the first Tainted Pact prompt, got {:?}",
                    ctx.description
                );
            }
            other => panic!("expected Tainted Pact prompt after hidden reveal, got {other:?}"),
        }
        assert!(
            wasm.pending_live_continuation.is_some(),
            "hidden opening must preserve the live resolution continuation"
        );

        dispatch_decision_select_option(&mut wasm, 1);

        let player = wasm.game.player(alice).expect("Alice should exist");
        let hand_names = object_names(&wasm.game, &player.hand);
        let graveyard_names = object_names(&wasm.game, &player.graveyard);
        let exile_names = object_names(&wasm.game, &wasm.game.exile);

        assert!(
            hand_names.iter().any(|name| name == "First Card"),
            "accepting the first Tainted Pact card should put it into hand; hand={hand_names:?}"
        );
        assert!(
            graveyard_names.iter().any(|name| name == "Tainted Pact"),
            "Tainted Pact should finish resolving into graveyard; graveyard={graveyard_names:?}"
        );
        assert!(
            !exile_names.iter().any(|name| name == "Tainted Pact"),
            "Tainted Pact itself should not be exiled; exile={exile_names:?}"
        );
        assert!(
            !exile_names.iter().any(|name| name == "First Card"),
            "accepted Tainted Pact card should leave exile; exile={exile_names:?}"
        );
    }

    #[test]
    fn tainted_pact_declining_revealed_unique_card_continues_to_next_prompt() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

        let alice = PlayerId::from_index(0);
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        wasm.runner_awaiting_priority = true;

        let tainted_pact = CardDefinitionBuilder::new(CardId::new(), "Tainted Pact")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Instant])
            .parse_text(
                "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.",
            )
            .expect("Tainted Pact should parse");
        let spell = wasm
            .game
            .create_object_from_definition(&tainted_pact, alice, Zone::Hand);
        if let Some(player) = wasm.game.player_mut(alice) {
            player.mana_pool.add(ManaSymbol::Colorless, 1);
            player.mana_pool.add(ManaSymbol::Black, 1);
        }
        wasm.game
            .create_hidden_card_placeholder(alice, Zone::Library, 0, "alice-slot-0".to_string());
        wasm.game
            .create_hidden_card_placeholder(alice, Zone::Library, 1, "alice-slot-1".to_string());

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        dispatch_priority_action_matching(&mut wasm, |action| {
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)
        });
        dispatch_select_options_until_priority(&mut wasm);
        dispatch_pass_priority(&mut wasm);
        dispatch_pass_priority(&mut wasm);

        wasm.reveal_hidden_slot_input(RevealHiddenSlotInput {
            owner: 0,
            slot: 1,
            card_name: "Tainted Pact".to_string(),
            commitment: Some("alice-slot-1".to_string()),
            recompute_decision: false,
        })
        .expect("first exiled card should reveal as Tainted Pact");

        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Boolean(ctx)) => {
                assert!(
                    ctx.description.to_ascii_lowercase().contains("hidden card")
                        || ctx.description.to_ascii_lowercase().contains("tainted pact"),
                    "expected first Tainted Pact prompt, got {:?}",
                    ctx.description
                );
            }
            other => panic!("expected first Tainted Pact boolean prompt, got {other:?}"),
        }

        dispatch_decision_select_option(&mut wasm, 0);

        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Boolean(ctx)) => {
                assert!(
                    ctx.description.to_ascii_lowercase().contains("hidden card"),
                    "declining a unique first card should continue to a second prompt, got {:?}",
                    ctx.description
                );
            }
            other => panic!("expected second Tainted Pact prompt after declining, got {other:?}"),
        }

        wasm.reveal_hidden_slot_input(RevealHiddenSlotInput {
            owner: 0,
            slot: 0,
            card_name: "Swamp".to_string(),
            commitment: Some("alice-slot-0".to_string()),
            recompute_decision: false,
        })
        .expect("second exiled card should reveal as Swamp");

        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Boolean(ctx)) => {
                assert!(
                    ctx.description.to_ascii_lowercase().contains("swamp"),
                    "revealing the second unique card should preserve the choice prompt, got {:?}",
                    ctx.description
                );
            }
            other => panic!("expected second Tainted Pact prompt after revealing, got {other:?}"),
        }
    }

    #[test]
    fn generated_tapped_lands_do_not_make_two_mana_creature_castable() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

        let alice = PlayerId::from_index(0);
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        wasm.runner_awaiting_priority = true;

        let lush_portico = ObjectId(
            wasm.add_card_to_zone(
                0,
                "Lush Portico".to_string(),
                "Battlefield".to_string(),
                true,
            )
            .expect("Lush Portico should load"),
        );
        let plains = ObjectId(
            wasm.add_card_to_zone(0, "Plains".to_string(), "Battlefield".to_string(), true)
                .expect("Plains should load"),
        );
        let spell = ObjectId(
            wasm.add_card_to_hand(0, "Charismatic Conqueror".to_string())
                .expect("Charismatic Conqueror should load"),
        );

        wasm.game.tap(lush_portico);
        wasm.game.tap(plains);
        wasm.game.empty_mana_pools();
        wasm.recompute_ui_decision()
            .expect("priority decision should rebuild");

        let Some(DecisionContext::Priority(priority)) = wasm.pending_decision.as_ref() else {
            panic!("expected priority decision");
        };
        let advertised_cast = priority.actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal
                } if *spell_id == spell
            )
        });
        assert!(
            !advertised_cast,
            "Charismatic Conqueror should not be castable from two tapped lands and no floating mana"
        );
    }

    #[test]
    fn mystical_tutor_resolution_prompts_for_hidden_library_choice() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);

        let alice = PlayerId::from_index(0);
        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        wasm.runner_awaiting_priority = true;

        let mystical_tutor = ironsmith_registry::compile_to_runtime_definition(
            "Mystical Tutor",
            "Mana Cost: {U}\nType: Instant\nSearch your library for an instant or sorcery card, reveal it, then shuffle and put that card on top.",
            false,
        )
        .expect("Mystical Tutor should compile");
        let spell = wasm
            .game
            .create_object_from_definition(&mystical_tutor, alice, Zone::Hand);
        wasm.game
            .player_mut(alice)
            .expect("Alice should exist")
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        let hidden_library_ids: Vec<ObjectId> = (0..3)
            .map(|slot| {
                wasm.game.create_hidden_card_placeholder(
                    alice,
                    Zone::Library,
                    slot,
                    format!("alice-hidden-library-{slot}"),
                )
            })
            .collect();

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        dispatch_priority_action_matching(&mut wasm, |action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == spell
            )
        });
        dispatch_select_option(&mut wasm, 0);
        dispatch_pass_priority(&mut wasm);
        dispatch_pass_priority(&mut wasm);

        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectObjects(ctx)) => {
                assert_eq!(ctx.player, alice);
                assert_eq!(
                    ctx.candidates
                        .iter()
                        .filter(|candidate| candidate.legal)
                        .map(|candidate| candidate.id)
                        .collect::<Vec<_>>(),
                    hidden_library_ids,
                    "Mystical Tutor should prompt with hidden library candidates"
                );
            }
            other => panic!("expected Mystical Tutor search prompt, got {other:?}"),
        }
    }
}
