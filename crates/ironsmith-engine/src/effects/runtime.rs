use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::game_state::GameState;
use crate::provenance::ProvenanceNodeKind;
use crate::target::ChooseSpec;

/// Resolve a Value to a concrete i32.
pub fn resolve_value(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<i32, ExecutionError> {
    crate::effects::helpers::resolve_value(game, value, ctx)
}

/// Validate that a resolved target matches a target spec.
pub fn validate_target(
    game: &GameState,
    target: &ResolvedTarget,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> bool {
    let filter_ctx = ctx.filter_context(game);
    let range_exempt =
        game.source_snapshot_is_exempt_from_range(Some(ctx.source), ctx.source_snapshot.as_ref());
    let within_range = match target {
        ResolvedTarget::Object(id) => game.object(*id).map_or_else(
            || {
                ctx.target_snapshots.get(id).is_some_and(|snapshot| {
                    range_exempt
                        || game.snapshot_is_within_range(ctx.controller, snapshot, Some(ctx.source))
                })
            },
            |_| range_exempt || game.object_is_within_range(ctx.controller, *id, Some(ctx.source)),
        ),
        ResolvedTarget::Player(id) => {
            range_exempt || game.player_is_within_range(ctx.controller, *id)
        }
    };
    if !within_range {
        return false;
    }

    match (target, spec) {
        // Selection wrappers do not change target legality.
        (
            _,
            ChooseSpec::Target(inner)
            | ChooseSpec::WithCount(inner, _)
            | ChooseSpec::WithCountValue(inner, _, _),
        ) => validate_target(game, target, inner, ctx),
        (ResolvedTarget::Object(id), ChooseSpec::Object(filter)) => {
            if let Some(obj) = game.object(*id) {
                filter.matches(obj, &filter_ctx, game)
            } else {
                false
            }
        }
        (ResolvedTarget::Player(id), ChooseSpec::Player(filter)) => {
            game.can_target_player_from_source_or_snapshot(
                *id,
                Some(ctx.source),
                ctx.source_snapshot.as_ref(),
                ctx.controller,
            ) && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::ObjectOrPlayer(filter, _)) => game
            .object(*id)
            .is_some_and(|object| filter.matches(object, &filter_ctx, game)),
        (ResolvedTarget::Player(id), ChooseSpec::ObjectOrPlayer(_, filter)) => {
            game.can_target_player_from_source_or_snapshot(
                *id,
                Some(ctx.source),
                ctx.source_snapshot.as_ref(),
                ctx.controller,
            ) && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Player(id), ChooseSpec::PlayerOrPlaneswalker(filter)) => {
            game.can_target_player_from_source_or_snapshot(
                *id,
                Some(ctx.source),
                ctx.source_snapshot.as_ref(),
                ctx.controller,
            ) && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::PlayerOrPlaneswalker(_)) => {
            game.object(*id).is_some()
                && game.current_has_card_type(*id, crate::types::CardType::Planeswalker)
        }
        (ResolvedTarget::Object(id), ChooseSpec::AnyTarget) => game.object(*id).is_some(),
        (ResolvedTarget::Player(id), ChooseSpec::AnyTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
                && game.can_target_player_from_source_or_snapshot(
                    *id,
                    Some(ctx.source),
                    ctx.source_snapshot.as_ref(),
                    ctx.controller,
                )
        }
        (ResolvedTarget::Object(id), ChooseSpec::AnyOtherTarget) => {
            game.object(*id).is_some_and(|obj| obj.id != ctx.source)
        }
        (ResolvedTarget::Player(id), ChooseSpec::AnyOtherTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
                && game.can_target_player_from_source_or_snapshot(
                    *id,
                    Some(ctx.source),
                    ctx.source_snapshot.as_ref(),
                    ctx.controller,
                )
        }
        (ResolvedTarget::Object(id), ChooseSpec::SpecificObject(expected)) => id == expected,
        (ResolvedTarget::Player(id), ChooseSpec::SpecificPlayer(expected)) => id == expected,
        _ => false,
    }
}

/// Execute an effect and return the outcome (result + events).
/// Whether `effect` chooses new targets for a copy just made. Choosing new
/// targets is part of creating the copy (CR 707.10c), so the copy's events
/// are matched once its targets are final.
fn effect_chooses_new_targets_for_copy(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
        .is_some()
        // "You may choose new targets for the copy" also lowers to an
        // optional retarget of the copy just made (a non-targeting,
        // back-referenced stack object), not only to `ChooseNewTargets`.
        || effect
            .downcast_ref::<crate::effects::RetargetStackObjectEffect>()
            .is_some_and(|retarget| !retarget.target.is_target())
        || effect
            .downcast_ref::<crate::effects::MayEffect>()
            .and_then(|may| may.effects.first())
            .is_some_and(effect_chooses_new_targets_for_copy)
        || effect
            .transparent_child_effect()
            .is_some_and(effect_chooses_new_targets_for_copy)
}

/// A boundary between two instructions of a resolving spell or ability.
///
/// Abilities trigger the moment their event happens, against the game state
/// right after it (CR 603.2, 603.6a): match the events of the instructions
/// performed so far now, so a later instruction can't change what an
/// "enters" filter sees, a permanent arriving later doesn't trigger on an
/// earlier event, and a watcher leaving later still sees it. Matched
/// abilities wait to be put on the stack the next time a player would
/// receive priority (CR 603.3). `next` is the instruction about to run.
///
/// Both kinds of events are matched here: the ones the finished instructions
/// queued, and the ones they reported in their results (`reported`). A
/// reported event keeps travelling up in the enclosing instructions' results;
/// it is remembered as matched so no later boundary matches it again.
/// Returns whether this was a matching point (so `reported` is now matched).
pub(crate) fn match_triggers_at_instruction_boundary<'a>(
    game: &mut GameState,
    ctx: &ExecutionContext,
    next: Option<&Effect>,
    reported: impl IntoIterator<Item = &'a crate::triggers::TriggerEvent>,
) -> Result<bool, ExecutionError> {
    if game.action_observations_suppressed()
        || !game.effect_store.per_event_trigger_matching
        || game.has_open_simultaneous_action()
        || game.effect_store.trigger_matching_holds > 0
        || ctx.decision_maker.awaiting_choice()
        || next.is_some_and(effect_chooses_new_targets_for_copy)
    {
        return game.token_resource_failure().map_or(Ok(false), Err);
    }
    let (root, meter) = game.begin_token_resource_scope();
    let result = crate::effects::composition::execute_world_result_transaction(game, |game| {
        let mut result = Ok(match_triggers_at_instruction_boundary_inner(
            game, ctx, next, reported,
        ));
        if let Some(error) = game.token_resource_failure() {
            result = Err(error);
        }
        result
    });
    game.end_token_resource_scope(root, &meter);
    result
}

fn match_triggers_at_instruction_boundary_inner<'a>(
    game: &mut GameState,
    ctx: &ExecutionContext,
    next: Option<&Effect>,
    reported: impl IntoIterator<Item = &'a crate::triggers::TriggerEvent>,
) -> bool {
    let fresh = reported
        .into_iter()
        .filter(|event| !outcome_event_already_matched(game, event))
        .filter(|event| !game.event_is_pending_for_trigger_matching(event))
        .cloned()
        .collect::<Vec<_>>();
    if fresh.is_empty() && game.effect_store.pending_trigger_events.is_empty() {
        return true;
    }
    let mut matched = crate::triggers::TriggerQueue::new();
    crate::game_loop::queue_triggers_from_reported_events(game, &mut matched, fresh, true);
    crate::game_loop::drain_pending_trigger_events(game, &mut matched);
    game.defer_trigger_entries(matched.take_all());
    true
}

/// A completed original operation is a real instruction boundary even when
/// its producer is a cost or turn-based combat action. Preserve outcome event
/// evidence, and attach a private receipt proof only after actual matching.
/// Simultaneous owners call this once for every original before any addition.
pub(crate) fn capture_triggers_before_added_program<'a>(
    game: &mut GameState,
    ctx: &ExecutionContext,
    next: Option<&Effect>,
    reported: impl IntoIterator<Item = &'a mut crate::triggers::TriggerEvent>,
) -> Result<bool, ExecutionError> {
    let mut reported: Vec<_> = reported.into_iter().collect();
    let event_checkpoint: Vec<_> = reported.iter().map(|event| (**event).clone()).collect();
    let (root, meter) = game.begin_token_resource_scope();
    let result = crate::effects::composition::execute_world_result_transaction(game, |game| {
        let mut result = capture_triggers_before_added_program_inner(
            game,
            ctx,
            next,
            reported.iter_mut().map(|event| &mut **event),
        );
        if let Err(error) = &result {
            game.record_token_resource_failure(error);
        }
        if let Some(error) = game.token_resource_failure() {
            result = Err(error);
        }
        result
    });
    if result.is_err() {
        for (event, previous) in reported.iter_mut().zip(event_checkpoint) {
            **event = previous;
        }
    }
    game.end_token_resource_scope(root, &meter);
    result
}

fn capture_triggers_before_added_program_inner<'a>(
    game: &mut GameState,
    ctx: &ExecutionContext,
    next: Option<&Effect>,
    reported: impl IntoIterator<Item = &'a mut crate::triggers::TriggerEvent>,
) -> Result<bool, ExecutionError> {
    if game.action_observations_suppressed()
        || game.has_open_simultaneous_action()
        || game.effect_store.trigger_matching_holds > 0
        || ctx.decision_maker.awaiting_choice()
        || next.is_some_and(effect_chooses_new_targets_for_copy)
    {
        return Ok(false);
    }
    let mut reported: Vec<_> = reported.into_iter().collect();
    game.freeze_completed_entry_events(reported.iter_mut().map(|event| &mut **event))?;
    let mut seen = std::collections::HashSet::new();
    let fresh = reported
        .iter()
        .filter(|event| !outcome_event_already_matched(game, event))
        .filter(|event| !game.event_is_pending_for_trigger_matching(event))
        .filter(|event| seen.insert(event.occurrence_key()))
        .map(|event| (**event).clone())
        .collect::<Vec<_>>();
    crate::events::damage::validate_damage_history_amounts(game, fresh.iter())?;
    let mut matched = crate::triggers::TriggerQueue::new();
    crate::game_loop::queue_triggers_from_reported_events(game, &mut matched, fresh, true);
    crate::game_loop::drain_pending_trigger_events(game, &mut matched);
    if let Some(error) = game.token_resource_failure() {
        return Err(error);
    }
    game.defer_trigger_entries(matched.take_all());
    for event in &mut reported {
        event.mark_triggers_captured();
        // Completed receipts in history retain the matching proof too, so an
        // older alias cannot lose it after the local alias map is cleared.
        game.stage_turn_history_event(event);
    }
    Ok(true)
}

/// Whether a boundary inside the current resolution already matched `event`.
fn outcome_event_already_matched(game: &GameState, event: &crate::triggers::TriggerEvent) -> bool {
    game.event_with_retained_trigger_capture(event)
        .triggers_captured()
}

/// Drop the events a boundary inside the current resolution already matched,
/// so whoever consumes a resolution's reported events matches only the rest.
pub(crate) fn retain_unmatched_outcome_events(
    game: &GameState,
    events: &mut Vec<crate::triggers::TriggerEvent>,
) {
    // Receipt proof remains valid after the enclosing matching scope clears
    // its alias map; fresh appended instructions still pass through.
    events.retain(|event| !outcome_event_already_matched(game, event));
}

/// Run `run` as the instructions of a resolving spell or ability, matching
/// triggered abilities at each instruction boundary when `enabled`
/// (CR 603.2). Restores the enclosing setting afterwards; the outermost
/// resolution forgets which reported events were matched once it is over.
pub(crate) fn with_per_event_trigger_matching<R>(
    game: &mut GameState,
    enabled: bool,
    run: impl FnOnce(&mut GameState) -> R,
) -> R {
    let previous = game.effect_store.per_event_trigger_matching;
    game.effect_store.per_event_trigger_matching = enabled;
    let result = run(game);
    game.effect_store.per_event_trigger_matching = previous;
    if !previous {
        game.effect_store.matched_outcome_events.clear();
    }
    result
}

/// An instruction acting on "all <quality> cards" in a hand reads identities
/// only the owner knows in peer matches: the owners reveal the matching cards
/// first (see `GameState::settle_hidden_hand_all_matching`). Returns `false`
/// while an owner's answer is awaited.
fn settle_hidden_hand_all_matching_specs(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> bool {
    if !game.tracks_hidden_cards() || effect.0.transparent_child_effect().is_some() {
        return true;
    }
    // A selected program acquires a child's inputs only when it reaches that
    // child. Preview traversal cannot reveal an untaken optional branch.
    let mut filters: Vec<crate::filter::ObjectFilter> = Vec::new();
    for spec in effect.0.own_preflight_object_specs() {
        if let ChooseSpec::All(filter) = spec.base() {
            if !filters.contains(filter) {
                filters.push(filter.clone());
            }
        }
    }
    if let Some(tag_matching) = effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
        && tag_matching.source_tags.is_empty()
    {
        for spec in effect.0.decision_related_object_specs() {
            if let crate::target::ChooseSpec::All(filter) = spec {
                filters.push(filter);
            }
        }
    }
    for filter in &filters {
        if filter.zone != Some(crate::zone::Zone::Hand) {
            continue;
        }
        let filter_ctx = ctx.filter_context(game);
        if !game.settle_hidden_hand_all_matching(
            &mut *ctx.decision_maker,
            ctx.source,
            filter,
            &filter_ctx,
        ) {
            return false;
        }
        if ctx.decision_maker.awaiting_choice() {
            return false;
        }
    }
    true
}

/// Acquire only the reached instruction's own inputs before it freezes a
/// native program. Ordinary dispatch and prepared selection share this gate.
pub(crate) fn prepare_reached_effect_inputs(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<bool, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() || ctx.resolution_stopped() {
        return Ok(false);
    }
    game.establish_control_transition_boundary()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    if effect
        .0
        .directly_mentions_player_filter(&crate::target::PlayerFilter::Defending)
        && !ctx.bind_defending_player(game)?
    {
        return Ok(false);
    }
    Ok(settle_hidden_hand_all_matching_specs(game, effect, ctx))
}

pub(crate) fn select_reached_action_program(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
    if !prepare_reached_effect_inputs(game, effect, ctx)? {
        return Ok(None);
    }
    with_instruction_bindings(
        game,
        effect,
        ctx,
        effect,
        || None,
        |effect, game, ctx| effect.0.select_prepared_action_program(game, ctx),
    )
}

/// Aggregate adapter for the same instruction owner used by retained callers.
pub fn execute_effect(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    execute_effect_with_outputs(game, effect, ctx)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Execute once through the ordinary resource, identity and recording boundary.
pub fn execute_effect_with_outputs(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    execute_effect_with_outputs_for_purpose(
        game,
        effect,
        ctx,
        EffectExecutionPurpose::Action,
        effect,
    )
}

/// Payment uses the ordinary identity/resource/recording owner. Only dispatch
/// selects the cost owner's captured action contract; fallback actions keep
/// their existing executor until that owner advertises prepared payment.
pub(crate) fn execute_effect_payment_with_outputs(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let origin = effect;
    // Factories own compound normalization. Nested dispatch normalizes leaf
    // payment aliases without cloning optional/limited program identities.
    let mut has_children = false;
    effect.0.visit_child_effects(&mut |_| has_children = true);
    let canonical = if has_children {
        None
    } else {
        effect
            .0
            .as_cost_executable()
            .and_then(|cost| cost.canonical_cost_effect())
    };
    let effect = canonical.as_ref().unwrap_or(effect);
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            execute_effect_with_outputs_for_purpose(
                game,
                effect,
                ctx,
                EffectExecutionPurpose::Payment,
                origin,
            )
        },
    )
}

#[derive(Clone, Copy)]
pub(crate) enum EffectExecutionPurpose {
    Action,
    Payment,
}

impl EffectExecutionPurpose {
    pub(crate) fn execute(
        self,
        game: &mut GameState,
        effect: &Effect,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        match self {
            Self::Action => execute_effect_with_outputs(game, effect, ctx),
            Self::Payment => execute_effect_payment_with_outputs(game, effect, ctx),
        }
    }
}

/// Completed payments export their owner-validated bindings exactly once.
/// A compound explicitly delegates these exports to executed payment children;
/// captured decorators instead acknowledge their original proposal here.
fn finish_effect_payment(
    game: &GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
    outputs: &crate::effects::CompletedEffectOutputs,
) -> Result<(), ExecutionError> {
    let Some(cost) = effect.0.as_cost_executable() else {
        return Ok(());
    };
    cost.validate_payment_outcome(&outputs.outcome)
        .map_err(|error| match error {
            crate::effects::CostValidationError::ExecutionFailed(error) => error,
            other => ExecutionError::Impossible(format!(
                "effect payment was not acknowledged: {other:?}"
            )),
        })?;
    if !cost.supports_prepared_payment() && cost.payment_bindings_are_owned_by_children() {
        return Ok(());
    }
    if ctx.x_value.is_none() {
        ctx.x_value = cost
            .payment_x_from_outcome(&outputs.outcome, ctx)
            .map_err(|error| match error {
                crate::effects::CostValidationError::ExecutionFailed(error) => error,
                other => ExecutionError::Impossible(format!(
                    "effect payment X was not acknowledged: {other:?}"
                )),
            })?;
    }
    let payment_x = ctx.x_value;
    cost.finalize_payment_bindings(game, &outputs.outcome, ctx, payment_x)
        .map_err(|error| match error {
            crate::cost::CostPaymentError::ExecutionFailed(error) => error,
            other => ExecutionError::Impossible(format!(
                "effect payment bindings were not acknowledged: {other}"
            )),
        })
}

fn execute_owned_instruction(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
    purpose: EffectExecutionPurpose,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let cost = match purpose {
        EffectExecutionPurpose::Payment => effect.0.as_cost_executable(),
        EffectExecutionPurpose::Action => None,
    };
    let Some(cost) = cost else {
        return effect.0.execute_with_outputs(game, ctx);
    };
    if !cost.supports_prepared_payment() {
        return cost.execute_payment_with_outputs(game, ctx);
    }
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    game.clear_pending_decision_controllers();
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            // The enclosing runtime records this instruction once. Decorator
            // constructors still record their actual nested child proposals.
            let proposal = cost.prepare_simultaneous_payment(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            if !cost.accepts_prepared_payment(proposal.as_ref()) {
                return Err(ExecutionError::Impossible(
                    "effect payment was not accepted by its owner".into(),
                ));
            }
            let simultaneous = proposal.has_simultaneous_originals();
            crate::effects::composition::complete_prepared_original_with_outputs(
                proposal,
                game,
                ctx,
                simultaneous,
            )
        },
    )
}

fn execute_effect_with_outputs_for_purpose(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
    purpose: EffectExecutionPurpose,
    origin: &Effect,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    execute_effect_with_outputs_using(game, effect, ctx, purpose, origin, |effect, game, ctx| {
        execute_owned_instruction(game, effect, ctx, purpose)
    })
}

/// Captured native programs use the ordinary identity/resource/recording owner
/// while dispatching their actual selected cursor instead of selecting again.
pub(crate) fn execute_effect_with_outputs_using<'a>(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext<'a>,
    purpose: EffectExecutionPurpose,
    origin: &Effect,
    mut execute: impl FnMut(
        &Effect,
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let (root, meter) = game.begin_token_resource_scope();
    let result = crate::effects::composition::execute_error_transaction_if(
        game,
        ctx,
        root,
        ExecutionError::is_incomplete_execution,
        |game, ctx| {
            let (mut result, recorded) =
                crate::effects::outcome_recording::capture_instruction_records(game, |game| {
                    match game.token_resource_failure() {
                        Some(error) => Err(error),
                        None => execute_effect_with_resource_scope_using(
                            game,
                            effect,
                            ctx,
                            origin,
                            &mut execute,
                        ),
                    }
                });
            if let Ok(outputs) = &mut result {
                let outcome = &mut outputs.outcome;
                if !ctx.decision_maker.awaiting_choice() {
                    crate::effects::outcome_recording::complete_outcome(
                        game,
                        effect.0.result_action(),
                        Some(ctx.controller),
                        outcome,
                        recorded,
                    );
                    outputs.synchronize_observations();
                }
            }
            if matches!(purpose, EffectExecutionPurpose::Payment)
                && !ctx.decision_maker.awaiting_choice()
            {
                if let Ok(outputs) = &result {
                    if let Err(error) = finish_effect_payment(game, effect, ctx, outputs) {
                        result = Err(error);
                    }
                }
            }
            if let Err(error) = &result {
                game.record_token_resource_failure(error);
            }
            if let Some(error) = game.token_resource_failure() {
                result = Err(error);
            }
            result
        },
    );
    game.end_token_resource_scope(root, &meter);
    result
}

/// Native deferred originals share ordinary preflight, chooser replay, identity,
/// checked resource rollback and immutable result publication.
pub(crate) fn prepare_effect_original_with_outputs<'a>(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext<'a>,
    mut prepare: impl FnMut(
        &Effect,
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    >,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    crate::effects::tokens::execute_resource_transaction_with_pending_value(
        game,
        ctx,
        || {
            crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            )
        },
        |game, ctx| {
            let mut completion = None;
            let outcome = execute_effect_with_resource_scope_using(
                game,
                effect,
                ctx,
                effect,
                |effect, game, ctx| {
                    let committed = prepare(effect, game, ctx)?;
                    completion = committed.completion;
                    Ok(committed.outcome)
                },
            )?;
            Ok(crate::effects::SimultaneousEffectCommit {
                outcome,
                completion,
            })
        },
    )
}

pub(crate) fn prepare_effect_original_with<'a>(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext<'a>,
    mut prepare: impl FnMut(
        &Effect,
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError>,
) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    prepare_effect_original_with_outputs(game, effect, ctx, |effect, game, ctx| {
        prepare(effect, game, ctx).map(crate::effects::SimultaneousEffectCommit::into_retained)
    })
    .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
}

pub(crate) fn prepare_effect_draw_continuation_with_outputs(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    prepare_effect_original_with_outputs(game, effect, ctx, |effect, game, ctx| {
        effect
            .0
            .prepare_replacement_draw_continuation_with_outputs(game, ctx)
    })
}

/// Aggregate compatibility boundary for native callers that do not retain routing.
pub(crate) fn prepare_effect_draw_continuation(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    prepare_effect_draw_continuation_with_outputs(game, effect, ctx)
        .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
}

/// Ordinary execution and prepared selection acquire missing player bindings
/// through the same instruction identity. The callback owns its actual result;
/// this scope supplies no action, receipt, cursor or replacement schedule.
pub(crate) fn with_instruction_bindings<'a, T>(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext<'a>,
    origin: &Effect,
    mut pending_value: impl FnMut() -> T,
    mut execute: impl FnMut(
        &Effect,
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    let previous_effect = ctx.executing_effect;
    let effect_identity =
        origin.0.as_ref() as *const dyn crate::effects::EffectExecutor as *const () as usize;
    ctx.executing_effect = Some(effect_identity);
    let mut execution = execute(effect, game, ctx);
    // A singular untargeted "an opponent" with several opponents is chosen
    // by the controller when the instruction is performed. The innermost
    // instruction that needed the player reports it before acting; ask once,
    // bind the answer for the rest of this resolution, and perform it again.
    if matches!(
        &execution,
        Err(ExecutionError::UnresolvableValue(message))
            if message == crate::effects::helpers::AN_OPPONENT_CHOICE_REQUIRED
    ) {
        let candidates = crate::effects::helpers::an_opponent_choice_candidates(game, ctx);
        let options = candidates
            .iter()
            .filter_map(|player_id| {
                game.player(*player_id)
                    .map(|player| (player.name.to_string(), *player_id))
            })
            .collect::<Vec<_>>();
        if !options.is_empty() {
            let chosen = crate::decisions::ask_choose_one(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                ctx.source,
                &options,
            );
            if ctx.decision_maker.awaiting_choice() {
                ctx.executing_effect = previous_effect;
                return Ok(pending_value());
            }
            if let Some(chosen) = chosen {
                ctx.set_tagged_players(
                    crate::tag::TagKey::from(crate::effects::helpers::AN_OPPONENT_CHOICE_TAG),
                    vec![chosen],
                );
                execution = execute(effect, game, ctx);
            }
        }
        if let Err(ExecutionError::UnresolvableValue(message)) = &mut execution
            && message == crate::effects::helpers::AN_OPPONENT_CHOICE_REQUIRED
        {
            *message = "Opponent filter requires a targeted player".to_string();
        }
    }
    // "A player of your choice adds {C}" (Victory Chimes): a ChosenPlayer
    // reference with no earlier choice is chosen by the controller now.
    if matches!(
        &execution,
        Err(ExecutionError::UnresolvableValue(message))
            if message == "ChosenPlayer requires a previously chosen player"
    ) && ctx.combat.chosen_player.is_none()
    {
        let options = game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| (player.name.to_string(), player.id))
            .collect::<Vec<_>>();
        if !options.is_empty() {
            let chosen = crate::decisions::ask_choose_one(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                ctx.source,
                &options,
            );
            if ctx.decision_maker.awaiting_choice() {
                ctx.executing_effect = previous_effect;
                return Ok(pending_value());
            }
            if let Some(chosen) = chosen {
                ctx.combat.chosen_player = Some(chosen);
                execution = execute(effect, game, ctx);
            }
        }
    }
    ctx.executing_effect = previous_effect;
    execution
}

fn execute_effect_with_resource_scope_using<'a>(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext<'a>,
    origin: &Effect,
    execute: impl FnMut(
        &Effect,
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    // CR 724.1b/724.2b stop the resolving spell or ability immediately. Composite
    // executors route child effects through this function, so this guard also
    // suppresses later instructions inside a sequence, modal branch, loop, or
    // other nested effect after EndTurnEffect requests the scheduler jump.
    if ctx.resolution_stopped()
        || game.turn_store.end_turn_procedure_pending
        || game.turn_store.end_combat_phase_procedure_pending
    {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ));
    }
    if !prepare_reached_effect_inputs(game, effect, ctx)? {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let execution = with_instruction_bindings(
        game,
        effect,
        ctx,
        origin,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        execute,
    );
    let mut outputs = match execution {
        Ok(outcome) => outcome,
        // A "that player" reference whose choice was made with no player
        // available (explicitly bound empty) refers to nothing, so the
        // instruction does nothing (CR 608.2c/609.3).
        Err(ExecutionError::UnresolvableValue(message))
            if message
                .strip_prefix("TaggedPlayer requires a tagged player for '")
                .and_then(|rest| rest.strip_suffix('\''))
                .is_some_and(|tag| ctx.get_tagged_players(tag).is_some_and(Vec::is_empty)) =>
        {
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))
        }
        // CR 801.10: only the out-of-range portion does nothing. Treat that
        // instruction as resolved so later instructions still happen.
        Err(ExecutionError::OutOfRange) => {
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved())
        }
        // An instruction that refers to an object an earlier instruction
        // would have produced ("its controller", "that card", "the exiled
        // card") when that earlier instruction affected nothing — zero
        // optional targets chosen, a mass destroy that found nothing, nothing
        // exiled with the source — refers to a nonexistent object and does
        // nothing (CR 608.2c, 609.3); later instructions still happen.
        Err(ExecutionError::TagNotFound(_)) => {
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::target_invalid())
        }
        Err(error) => return Err(error),
    };

    let outcome = &mut outputs.outcome;
    if !outcome.events.is_empty() {
        let execution_node = game.provenance_graph_mut().alloc_child(
            ctx.provenance,
            ProvenanceNodeKind::EffectExecution {
                source: ctx.source,
                controller: ctx.controller,
            },
        );
        for event in &mut outcome.events {
            let provenance = event.provenance();
            if provenance == crate::provenance::ProvNodeId::default()
                || game.provenance_graph().node(provenance).is_none()
            {
                let node = game.alloc_child_event_provenance(execution_node, event.kind());
                event.set_provenance(node);
            }
        }
        game.freeze_completed_entry_events(outcome.events.iter_mut())?;
        crate::events::damage::validate_damage_history_amounts(game, outcome.events.iter())?;
        for event in &outcome.events {
            game.stage_turn_history_event(event);
        }
    }

    outputs.synchronize_observations();
    Ok(outputs)
}
