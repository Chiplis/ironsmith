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
            game.can_target_player_from_source(*id, ctx.source)
                && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::ObjectOrPlayer(filter, _)) => game
            .object(*id)
            .is_some_and(|object| filter.matches(object, &filter_ctx, game)),
        (ResolvedTarget::Player(id), ChooseSpec::ObjectOrPlayer(_, filter)) => {
            game.can_target_player_from_source(*id, ctx.source)
                && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Player(id), ChooseSpec::PlayerOrPlaneswalker(filter)) => {
            game.can_target_player_from_source(*id, ctx.source)
                && filter.matches_player(*id, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::PlayerOrPlaneswalker(_)) => game
            .object(*id)
            .is_some_and(|obj| obj.has_card_type(crate::types::CardType::Planeswalker)),
        (ResolvedTarget::Object(id), ChooseSpec::AnyTarget) => game.object(*id).is_some(),
        (ResolvedTarget::Player(id), ChooseSpec::AnyTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
                && game.can_target_player_from_source(*id, ctx.source)
        }
        (ResolvedTarget::Object(id), ChooseSpec::AnyOtherTarget) => {
            game.object(*id).is_some_and(|obj| obj.id != ctx.source)
        }
        (ResolvedTarget::Player(id), ChooseSpec::AnyOtherTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
                && game.can_target_player_from_source(*id, ctx.source)
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
) -> bool {
    if !game.effect_store.per_event_trigger_matching
        || game.effect_store.trigger_matching_holds > 0
        || ctx.decision_maker.awaiting_choice()
        || next.is_some_and(effect_chooses_new_targets_for_copy)
    {
        return false;
    }
    let fresh = reported
        .into_iter()
        .filter(|event| !outcome_event_already_matched(game, event))
        .cloned()
        .collect::<Vec<_>>();
    if fresh.is_empty() && game.effect_store.pending_trigger_events.is_empty() {
        return true;
    }
    let mut matched = crate::triggers::TriggerQueue::new();
    for event in &fresh {
        game.effect_store
            .matched_outcome_events
            .insert(event.occurrence_key(), event.clone());
    }
    crate::game_loop::queue_triggers_from_reported_events(game, &mut matched, fresh, false);
    crate::game_loop::drain_pending_trigger_events(game, &mut matched);
    game.defer_trigger_entries(matched.take_all());
    true
}

/// Whether a boundary inside the current resolution already matched `event`.
fn outcome_event_already_matched(game: &GameState, event: &crate::triggers::TriggerEvent) -> bool {
    game.effect_store
        .matched_outcome_events
        .contains_key(&event.occurrence_key())
}

/// Drop the events a boundary inside the current resolution already matched,
/// so whoever consumes a resolution's reported events matches only the rest.
pub(crate) fn retain_unmatched_outcome_events(
    game: &GameState,
    events: &mut Vec<crate::triggers::TriggerEvent>,
) {
    if game.effect_store.matched_outcome_events.is_empty() {
        return;
    }
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

pub fn execute_effect(
    game: &mut GameState,
    effect: &Effect,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    // CR 724.1b/724.2b stop the resolving spell or ability immediately. Composite
    // executors route child effects through this function, so this guard also
    // suppresses later instructions inside a sequence, modal branch, loop, or
    // other nested effect after EndTurnEffect requests the scheduler jump.
    if game.turn_store.end_turn_procedure_pending
        || game.turn_store.end_combat_phase_procedure_pending
    {
        return Ok(EffectOutcome::resolved());
    }
    let previous_effect = ctx.executing_effect;
    let effect_identity =
        effect.0.as_ref() as *const dyn crate::effects::EffectExecutor as *const () as usize;
    ctx.executing_effect = Some(effect_identity);
    let execution = effect.0.execute(game, ctx);
    ctx.executing_effect = previous_effect;
    let mut outcome = match execution {
        Ok(outcome) => outcome,
        // CR 801.10: only the out-of-range portion does nothing. Treat that
        // instruction as resolved so later instructions still happen.
        Err(ExecutionError::OutOfRange) => EffectOutcome::resolved(),
        Err(error) => return Err(error),
    };

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
        for event in &outcome.events {
            game.stage_turn_history_event(event);
        }
    }

    Ok(outcome)
}
