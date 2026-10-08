//! Retained prevention programs stop at their first actual draw continuation.
use super::*;
use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, EffectOutcomeScope, ExecutionContext, ExecutionContextCheckpoint,
    ExecutionError, SharedEffectOutcome, SharedOutcomeOwnership, SimultaneousEffectCommit,
    SimultaneousEffectCompletion,
};
use crate::prevention::PendingPreventionFollowUp;
use crate::triggers::TriggerEvent;

type OwnedOutput = (Vec<EffectOutcomeScope>, CompletedEffectOutputs);

/// The same binding owner serves immediate and retained prevention completion.
/// Future queue entries acquire current/LKI source evidence only when reached.
pub(super) fn context<'a>(
    game: &GameState,
    dm: &'a mut dyn DecisionMaker,
    pending: &PendingPreventionFollowUp,
) -> Result<ExecutionContext<'a>, DamageProcessingError> {
    let follow_up = &pending.follow_up;
    let mut event = crate::events::RawEvent::new(pending.damage.clone(), pending.provenance);
    if let Some(snapshot) = &pending.damage_source_snapshot {
        event = event.with_source_snapshot(snapshot.clone());
    }
    let mut ctx = ExecutionContext::new(follow_up.source, follow_up.controller, dm)
        .with_triggering_event(event)
        .with_cause(crate::events::cause::EventCause::from_effect(
            follow_up.source,
            follow_up.controller,
        ))
        .with_provenance(pending.provenance);
    ctx.replacement = pending.replacement_scope.clone();
    ctx.source_snapshot = game
        .object(follow_up.source)
        .filter(|_| !game.is_phased_out(follow_up.source))
        .map(|object| {
            crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .transpose()
        .map_err(|error| DamageProcessingError {
            source: follow_up.source,
            error,
        })?
        .or_else(|| game.source_last_known_snapshot(follow_up.source).cloned())
        .or_else(|| pending.source_snapshot.clone());
    if follow_up.targets.is_empty() {
        ctx.targets.push(match pending.damage.target {
            DamageTarget::Player(player) => crate::effects::ResolvedTarget::Player(player),
            DamageTarget::Object(object) => crate::effects::ResolvedTarget::Object(object),
        });
    } else {
        ctx.targets = follow_up.targets.clone();
        ctx.target_assignments = follow_up.target_assignments.clone();
    }
    Ok(ctx)
}

fn aggregate(outputs: &[OwnedOutput]) -> EffectOutcome {
    EffectOutcome::aggregate(outputs.iter().map(|(_, output)| output.outcome.clone()))
}

fn retain_owned_outputs(outputs: Vec<OwnedOutput>) -> CompletedEffectOutputs {
    let mut retained = CompletedEffectOutputs::aggregate_only(aggregate(&outputs));
    retained.shared = outputs
        .into_iter()
        .map(|(scopes, outputs)| SharedEffectOutcome {
            ownership: if scopes.is_empty() {
                SharedOutcomeOwnership::Batch
            } else {
                SharedOutcomeOwnership::Participants(scopes)
            },
            outputs: outputs.into(),
        })
        .collect();
    retained.synchronize_observations();
    retained
}

fn publish_new_events(
    game: &mut GameState,
    output: &CompletedEffectOutputs,
    published: &[TriggerEvent],
) {
    let mut events = output
        .outcome
        .events
        .iter()
        .filter(|event| !published.iter().any(|previous| previous.ptr_eq(event)))
        .cloned()
        .collect();
    crate::effects::retain_unmatched_outcome_events(game, &mut events);
    for event in events {
        game.queue_trigger_event(event.provenance(), event);
    }
}

impl CapturedPreventionFollowUps {
    pub(crate) fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Execute prefixes in order, retaining the first actual draw and every
    /// unstarted queue entry. Each packet keeps its original assignment scopes.
    pub(crate) fn prepare_draw_boundary_with_outputs(
        self,
        game: &mut GameState,
        dm: &mut dyn DecisionMaker,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, DamageProcessingError> {
        if dm.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let source = self
            .pending
            .first()
            .map(|pending| pending.follow_up.source)
            .unwrap_or(ObjectId::from_raw(0));
        let (root, meter) = game.begin_token_resource_scope();
        let checkpoint = game.clone();
        let mut result = prepare(game, dm, self.pending);
        if result.is_ok()
            && let Some(error) = game.token_resource_failure()
        {
            result = Err(DamageProcessingError { source, error });
        }
        if result.is_err() || dm.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && dm.awaiting_choice());
        }
        game.end_token_resource_scope(root, &meter);
        if dm.awaiting_choice() {
            return result.map(|_| {
                SimultaneousEffectCommit::finished(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ))
            });
        }
        result
    }
}

fn prepare(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
    pending: Vec<PendingPreventionFollowUp>,
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, DamageProcessingError> {
    let mut before: Vec<OwnedOutput> = Vec::new();
    let mut pending = pending.into_iter();
    while let Some(next) = pending.next() {
        let source = next.follow_up.source;
        let mut ctx = context(game, &mut *dm, &next)?;
        for (index, effect) in next.follow_up.effects.iter().enumerate() {
            crate::effects::capture_triggers_before_added_program(
                game,
                &ctx,
                Some(effect),
                before
                    .iter_mut()
                    .flat_map(|(_, output)| output.outcome.events.iter_mut()),
            )
            .map_err(|error| DamageProcessingError { source, error })?;
            let first = crate::effects::with_per_event_trigger_matching(game, true, |game| {
                crate::effects::replacement::prepare_replacement_child(game, &mut ctx, effect)
            })
            .map_err(|error| DamageProcessingError { source, error })?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            publish_new_events(game, &first.prefix, &[]);
            if let Some(resume) = first.resume {
                let prefix = EffectOutcome::aggregate(
                    before
                        .iter()
                        .map(|(_, output)| output.outcome.clone())
                        .chain(std::iter::once(first.prefix.outcome.clone())),
                );
                let scope = ExecutionContextCheckpoint::capture(&ctx);
                return Ok(SimultaneousEffectCommit {
                    outcome: CompletedEffectOutputs::aggregate_only(prefix),
                    completion: Some(Box::new(PreventionDrawCompletion {
                        before,
                        first: first.prefix,
                        resume,
                        scopes: next.participant_scopes,
                        effects: next.follow_up.effects[index + 1..].to_vec(),
                        remaining: pending.collect(),
                        scope,
                    })),
                });
            }
            before.push((next.participant_scopes.clone(), first.prefix));
        }
        crate::effects::capture_triggers_before_added_program(
            game,
            &ctx,
            None,
            before
                .iter_mut()
                .flat_map(|(_, output)| output.outcome.events.iter_mut()),
        )
        .map_err(|error| DamageProcessingError { source, error })?;
    }
    Ok(SimultaneousEffectCommit::finished(retain_owned_outputs(
        before,
    )))
}

struct PreventionDrawCompletion {
    before: Vec<OwnedOutput>,
    first: CompletedEffectOutputs,
    resume: Box<dyn crate::effects::replacement::ReplacementResume>,
    effects: Vec<crate::effect::Effect>,
    scopes: Vec<EffectOutcomeScope>,
    remaining: Vec<PendingPreventionFollowUp>,
    scope: ExecutionContextCheckpoint,
}

impl SimultaneousEffectCompletion for PreventionDrawCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        // This continuation is constructed only from added programs/follow-ups.
        // Their internal pending draws are additions to the enclosing action.
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        Ok(SimultaneousEffectCommit {
            outcome: CompletedEffectOutputs::aggregate_only(original),
            completion: Some(self),
        })
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        Ok(SimultaneousEffectCommit {
            outcome: original,
            completion: Some(self),
        })
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        game.freeze_completed_entry_events(
            self.before
                .iter_mut()
                .flat_map(|(_, output)| output.outcome.events.iter_mut())
                .chain(self.first.outcome.events.iter_mut()),
        )?;
        self.resume.freeze(game)?;
        Ok(())
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        let parent = ExecutionContextCheckpoint::capture(ctx);
        self.scope.restore_ref(ctx);
        let result = (|| {
            for (_, output) in &mut self.before {
                crate::effects::composition::inherit_original_observations(
                    &mut output.outcome,
                    &original.events,
                );
            }
            crate::effects::composition::inherit_original_observations(
                &mut self.first.outcome,
                &original.events,
            );
            crate::effects::capture_triggers_before_added_program(
                game,
                ctx,
                None,
                self.before
                    .iter_mut()
                    .flat_map(|(_, output)| output.outcome.events.iter_mut())
                    .chain(self.first.outcome.events.iter_mut()),
            )?;
            self.resume.observe_prefix(&self.first.outcome.events);
            for (_, output) in &mut self.before {
                output.synchronize_observations();
            }
            self.first.synchronize_observations();
            *original = EffectOutcome::aggregate(
                self.before
                    .iter()
                    .map(|(_, output)| output.outcome.clone())
                    .chain(std::iter::once(self.first.outcome.clone())),
            );
            Ok(())
        })();
        parent.restore(ctx);
        result
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn complete_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let parent = ExecutionContextCheckpoint::capture(ctx);
        self.scope.restore_ref(ctx);
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                for (_, output) in &mut self.before {
                    crate::effects::composition::inherit_original_observations(
                        &mut output.outcome,
                        &original.events,
                    );
                }
                crate::effects::composition::inherit_original_observations(
                    &mut self.first.outcome,
                    &original.events,
                );
                let published = self.first.outcome.events.clone();
                self.resume.observe_prefix(&self.first.outcome.events);
                let mut first = crate::effects::replacement::resume_replacement_child_with_outputs(
                    game,
                    ctx,
                    self.resume,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                publish_new_events(game, &first, &published);
                first.retain_owned_child(self.first);
                self.before.push((self.scopes.clone(), first));
                for effect in self.effects {
                    crate::effects::capture_triggers_before_added_program(
                        game,
                        ctx,
                        Some(&effect),
                        self.before
                            .iter_mut()
                            .flat_map(|(_, output)| output.outcome.events.iter_mut()),
                    )?;
                    let output = crate::effects::execute_effect_with_outputs(game, &effect, ctx)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    publish_new_events(game, &output, &[]);
                    self.before.push((self.scopes.clone(), output));
                }
                crate::effects::capture_triggers_before_added_program(
                    game,
                    ctx,
                    self.remaining
                        .first()
                        .and_then(|pending| pending.follow_up.effects.first()),
                    self.before
                        .iter_mut()
                        .flat_map(|(_, output)| output.outcome.events.iter_mut()),
                )?;
                let remaining = self.remaining;
                let suffix = crate::effects::with_per_event_trigger_matching(game, true, |game| {
                    execute_prevention_follow_ups_with_owned_outputs(
                        game,
                        &mut *ctx.decision_maker,
                        remaining,
                    )
                })
                .map_err(|error| error.error)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                self.before.extend(suffix);
                Ok(retain_owned_outputs(self.before))
            },
        );
        parent.restore(ctx);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;

    fn setup() -> (GameState, ObjectId, PlayerId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Prevention source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, player, crate::zone::Zone::Battlefield);
        game.create_object_from_card(&card, player, crate::zone::Zone::Library);
        (game, source, player)
    }

    fn pending(game: &mut GameState, source: ObjectId, player: PlayerId,
        effects: Vec<Effect>, scopes: Vec<EffectOutcomeScope>) -> PendingPreventionFollowUp {
        PendingPreventionFollowUp {
            participant_scopes: scopes,
            replacement_scope: Default::default(),
            source_snapshot: None,
            damage_source_snapshot: None,
            follow_up: crate::prevention::PreventionFollowUp {
                source, controller: player, prevented: 1, effects,
                targets: Vec::new(), target_assignments: Vec::new(),
            },
            damage: crate::events::DamageEvent::with_cause(source, DamageTarget::Player(player),
                1, false, crate::events::cause::EventCause::from_effect(source, player)),
            provenance: game.provenance_graph_mut().alloc_root_event(crate::events::EventKind::Damage),
        }
    }

    // Authored only: prefix executes once and later follow-up programs cannot overtake the draw.
    #[test]
    fn prevention_prefix_and_assignment_scope_survive_first_draw_boundary() {
        let (mut game, source, player) = setup();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        let scope = EffectOutcomeScope(std::sync::Arc::new(ExecutionContextCheckpoint::capture(&ctx)));
        let queue = vec![
            pending(&mut game, source, player, vec![Effect::gain_life(2), Effect::draw(1), Effect::gain_life(4)], vec![scope.clone()]),
            pending(&mut game, source, player, vec![Effect::gain_life(8)], Vec::new()),
        ];
        let prepared = CapturedPreventionFollowUps { pending: queue }
            .prepare_draw_boundary_with_outputs(&mut game, &mut *ctx.decision_maker).unwrap();
        assert!(prepared.completion.is_some());
        assert_eq!(game.player(player).unwrap().life, 22);
        assert!(game.player(player).unwrap().hand.is_empty());
        let outputs = crate::effects::composition::complete_standalone_original_with_outputs(
            &mut game, &mut ctx, prepared,
        ).unwrap();
        assert_eq!(game.player(player).unwrap().life, 34, "prefix must not execute twice");
        assert_eq!(game.player(player).unwrap().hand.len(), 1);
        assert!(outputs.shared.iter().any(|shared| matches!(&shared.ownership,
            SharedOutcomeOwnership::Participants(scopes)
                if scopes.iter().any(|retained| retained.same_instruction(&scope)))));
        assert_eq!(ctx.source, source);
    }

    #[derive(Debug, Clone)]
    struct TypedFailure;
    impl crate::effects::EffectExecutor for TypedFailure {
        fn execute(&self, _: &mut GameState, _: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
            Err(ExecutionError::IncompleteEvidence("prevention suffix fixture".into()))
        }
    }

    #[test]
    fn prevention_resume_failure_rolls_back_draw_and_later_effects_at_native_root() {
        let (mut game, source, player) = setup();
        let queue = vec![pending(&mut game, source, player,
            vec![Effect::gain_life(2), Effect::draw(1), Effect::new(TypedFailure)], Vec::new())];
        let before_library = game.player(player).unwrap().library.clone();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        let result = crate::effects::composition::execute_transaction(&mut game, &mut ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)), |game, ctx| {
                let prepared = CapturedPreventionFollowUps { pending: queue }
                    .prepare_draw_boundary_with_outputs(game, &mut *ctx.decision_maker)
                    .map_err(|error| error.error)?;
                crate::effects::composition::complete_standalone_original_with_outputs(game, ctx, prepared)
            });
        assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(detail)) if detail == "prevention suffix fixture"));
        assert_eq!(game.player(player).unwrap().life, 20);
        assert!(game.player(player).unwrap().hand.is_empty());
        assert_eq!(game.player(player).unwrap().library, before_library);
    }

    #[derive(Default)]
    struct PauseBeforeDraw { pending: bool }
    impl DecisionMaker for PauseBeforeDraw {
        fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.pending = true;
            false
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }

    #[test]
    fn prevention_pending_choice_discards_prepared_prefix_and_unstarted_tail() {
        let (mut game, source, player) = setup();
        let queue = vec![
            pending(&mut game, source, player,
                vec![Effect::gain_life(2), Effect::may(vec![Effect::draw(1)])], Vec::new()),
            pending(&mut game, source, player, vec![Effect::gain_life(8)], Vec::new()),
        ];
        let mut dm = PauseBeforeDraw::default();
        let prepared = CapturedPreventionFollowUps { pending: queue }
            .prepare_draw_boundary_with_outputs(&mut game, &mut dm).unwrap();
        assert!(dm.awaiting_choice());
        assert!(prepared.completion.is_none());
        assert_eq!(game.player(player).unwrap().life, 20);
        assert!(game.player(player).unwrap().hand.is_empty());
        assert_eq!(prepared.outcome.outcome.as_count(), Some(0));
    }
}
