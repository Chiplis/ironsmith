//! Immutable request for an ordinary zone movement original.

use super::AppliedZoneChange;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::cause::EventCause;
use crate::events::processing::{EventOutcome, PreparedEventOutcome};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::replacement::ReplacementEffect;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Resolve a movement's objects through the ordinary selection owners. Target
/// counts constrain announcement; losing a target does not cancel the other
/// legal objects at resolution. Intrinsic source/tag references retain their
/// own identity resolution rather than becoming an unrelated target choice.
pub(crate) fn resolve_zone_move_objects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
) -> Result<Vec<ObjectId>, ExecutionError> {
    let resolution_spec = if spec.is_target() {
        ChooseSpec::target(spec.base().clone())
    } else {
        spec.clone()
    };
    let mut objects = if spec.is_target() {
        crate::effects::helpers::resolve_objects_from_spec(game, &resolution_spec, ctx)?
    } else {
        crate::effects::helpers::resolve_objects_for_effect(game, ctx, &resolution_spec)?
    };
    if spec.is_target() && matches!(spec.base(), ChooseSpec::Object(_)) {
        objects.retain(|id| {
            crate::effects::helpers::validate_target(
                game,
                &crate::effects::ResolvedTarget::Object(*id),
                &resolution_spec,
                ctx,
            )
        });
    }
    if spec.is_target()
        && let Some(max) = spec.count().max
    {
        objects.truncate(max);
    }
    Ok(objects)
}

/// Observe only performed original movements before any additions. The action
/// adapter owns its value/status and authored arrival work; replacement payloads
/// retain their own subjects and are never projected as parent movements.
pub(crate) fn observe_zone_move_originals(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipts: &[(ObjectId, PreparedEventOutcome<AppliedZoneChange>)],
    snapshots: &std::collections::HashMap<ObjectId, ObjectSnapshot>,
    pending_start: usize,
) -> Result<Vec<(ObjectId, AppliedZoneChange, ObjectSnapshot)>, ExecutionError> {
    let cause = ctx.cause.clone();
    observe_zone_move_originals_with_cause(game, ctx, receipts, snapshots, pending_start, cause)
}

/// Preserve an instruction cause that differs from its enclosing resolution.
pub(crate) fn observe_zone_move_originals_with_cause(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipts: &[(ObjectId, PreparedEventOutcome<AppliedZoneChange>)],
    snapshots: &std::collections::HashMap<ObjectId, ObjectSnapshot>,
    pending_start: usize,
    cause: EventCause,
) -> Result<Vec<(ObjectId, AppliedZoneChange, ObjectSnapshot)>, ExecutionError> {
    let mut originals = Vec::new();
    let mut routes = Vec::new();
    for (id, receipt) in receipts {
        let EventOutcome::Proceed(change) = &receipt.original else {
            continue;
        };
        if change.new_object_ids.is_empty() {
            continue;
        }
        let snapshot = snapshots.get(id).cloned().ok_or_else(|| {
            ExecutionError::InternalError("movement receipt has no original snapshot".into())
        })?;
        ctx.refresh_target_snapshot(snapshot.clone());
        if snapshot.object_id == ctx.source {
            ctx.refresh_source_snapshot(snapshot.clone());
        }
        let route = (snapshot.zone, change.final_zone);
        if !routes.contains(&route) {
            routes.push(route);
        }
        originals.push((*id, change.clone(), snapshot));
    }
    for (from, to) in routes {
        group_zone_move_observations_with_cause(
            game,
            ctx,
            pending_start,
            receipts,
            snapshots,
            from,
            to,
            cause.clone(),
        );
    }
    Ok(originals)
}

/// Façades select/configure moves; this request owns source identity and
/// replacement-aware commitment. Entry and action-specific observations still
/// belong to their prepared lifecycle owner rather than arbitrary follow-ups.
#[derive(Debug, Clone)]
pub(crate) struct PreparedZoneMove {
    object: ObjectId,
    from: Zone,
    to: Zone,
    cause: EventCause,
    snapshot: Option<ObjectSnapshot>,
}

impl PreparedZoneMove {
    pub(crate) fn capture(
        game: &GameState,
        object: ObjectId,
        from: Zone,
        to: Zone,
        cause: EventCause,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        Self {
            object,
            from,
            to,
            cause,
            snapshot: snapshot.or_else(|| ObjectSnapshot::from_object_id(game, object)),
        }
    }

    /// Resolve replacements without committing this original. Selected
    /// simultaneous instructions prepare every request before moving siblings.
    fn prepare(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        additional: &[ReplacementEffect],
        mut draws: Option<&mut super::ZoneInstructionDraws>,
    ) -> Result<
        (
            ObjectId,
            PreparedEventOutcome<crate::events::processing::PreparedZoneChange>,
        ),
        ExecutionError,
    > {
        if !game
            .object(self.object)
            .is_some_and(|object| object.zone == self.from)
        {
            return Ok((
                self.object,
                PreparedEventOutcome {
                    original: EventOutcome::NotApplicable,
                    programs: Vec::new(),
                },
            ));
        }
        let scope = crate::events::processing::ReplacementEventContext::with_scope(
            game,
            crate::events::Event::zone_change(
                self.object,
                self.from,
                self.to,
                self.cause.clone(),
                self.snapshot.clone(),
            )
            .with_provenance(ctx.provenance),
            &ctx.replacement,
        );
        let draw_start = draws.as_ref().map_or(0, |draws| draws.draws.0.len());
        let prepared = crate::events::processing::prepare_zone_change_scoped_with_draws(
            game,
            self.object,
            self.from,
            self.to,
            self.cause,
            &mut *ctx.decision_maker,
            additional,
            self.snapshot,
            Some(&scope),
            Vec::new(),
            None,
            draws.as_deref_mut().map(|draws| &mut draws.draws),
        )?;
        if let Some(draws) = draws {
            draws.record(self.object, draw_start);
        }
        Ok((self.object, prepared))
    }

    pub(crate) fn commit_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        additional: &[ReplacementEffect],
    ) -> Result<crate::events::processing::CommittedZoneChange<AppliedZoneChange>, ExecutionError>
    {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || {
                crate::events::processing::CommittedZoneChange::from_receipt(PreparedEventOutcome {
                    original: EventOutcome::Prevented,
                    programs: Vec::new(),
                })
            },
            |game, ctx| {
                let (object, prepared) = self.prepare(game, ctx, additional, None)?;
                super::commit_zone_change_proposal_with_outputs(
                    game,
                    object,
                    prepared,
                    &mut *ctx.decision_maker,
                )
            },
        )
    }
}

/// Shared replacement preparation for a selected movement instruction.
/// The caller retains its compound checkpoint and action-specific completion;
/// no original movement or added program runs between sibling proposals.
pub(crate) fn prepare_zone_moves(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requests: Vec<PreparedZoneMove>,
) -> Result<
    (
        Vec<(
            ObjectId,
            PreparedEventOutcome<crate::events::processing::PreparedZoneChange>,
        )>,
        super::ZoneInstructionDraws,
    ),
    ExecutionError,
> {
    let additional = ctx.additional_replacement_effects_snapshot();
    let mut prepared = Vec::with_capacity(requests.len());
    let mut draws = super::ZoneInstructionDraws::default();
    for request in requests {
        prepared.push(request.prepare(game, ctx, &additional, Some(&mut draws))?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok((Vec::new(), draws));
        }
    }
    Ok((prepared, draws))
}

/// Commit only retained zone proposals. Selection, replacements and added
/// programs belong to the preparation and completion phases respectively.
pub(super) fn commit_prepared_zone_moves(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    proposals: Vec<(
        ObjectId,
        PreparedEventOutcome<crate::events::processing::PreparedZoneChange>,
    )>,
    draws: &mut super::ZoneInstructionDraws,
) -> Result<Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>, ExecutionError> {
    let mut receipts = Vec::with_capacity(proposals.len());
    for (object, proposal) in proposals {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        draws.commit_pending_replacement(game, object, &mut *ctx.decision_maker)?;
        let committed = super::commit_zone_change_proposal_with_outputs(
            game,
            object,
            proposal,
            &mut *ctx.decision_maker,
        )?;
        receipts.push((object, draws.retain_committed_zone_receipt(committed)));
    }
    Ok(receipts)
}

/// Commit a frozen set of ordinary moves, author arrival metadata once all
/// originals are present, then finish deferred replacement programs centrally.
/// The callback must not substitute a second move or a late entry modifier.
pub(crate) fn execute_zone_moves<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    moves: Vec<PreparedZoneMove>,
    original: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &[(ObjectId, PreparedEventOutcome<AppliedZoneChange>)],
    ) -> Result<crate::effect::EffectOutcome, ExecutionError>,
) -> Result<crate::effect::EffectOutcome, ExecutionError> {
    execute_zone_moves_with_outputs(game, ctx, moves, original)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Ordinary execution retains the packets supplied by the movement completion
/// owner without introducing a deferred stage or a second execution.
pub(crate) fn execute_zone_moves_with_outputs<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    moves: Vec<PreparedZoneMove>,
    original: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &[(ObjectId, PreparedEventOutcome<AppliedZoneChange>)],
    ) -> Result<crate::effect::EffectOutcome, ExecutionError>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    commit_zone_moves_with_completion(
        game,
        ctx,
        moves,
        || {
            crate::effects::CompletedEffectOutputs::aggregate_only(
                crate::effect::EffectOutcome::count(0),
            )
        },
        original,
        |game, ctx, outcome, receipts, draws| {
            let committed = draws.finish(outcome, receipts, ctx);
            super::continuation::complete_zone_instruction_with_outputs(game, ctx, committed)
        },
    )
}

pub(crate) fn commit_zone_moves<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    moves: Vec<PreparedZoneMove>,
    deferred: bool,
    original: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &[(ObjectId, PreparedEventOutcome<AppliedZoneChange>)],
    ) -> Result<crate::effect::EffectOutcome, ExecutionError>,
) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    commit_zone_moves_with_completion(
        game,
        ctx,
        moves,
        || {
            crate::effects::SimultaneousEffectCommit::finished(crate::effect::EffectOutcome::count(
                0,
            ))
        },
        original,
        |game, ctx, outcome, receipts, draws| {
            let committed = draws.finish(outcome, receipts, ctx);
            if deferred {
                Ok(committed)
            } else {
                super::complete_zone_instruction(game, ctx, committed)
                    .map(crate::effects::SimultaneousEffectCommit::finished)
            }
        },
    )
}

/// The movement owner determines originals once; the caller selects the
/// ordinary retained completion or the existing deferred completion contract.
pub(crate) fn commit_zone_moves_with_completion<'a, R>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    moves: Vec<PreparedZoneMove>,
    pending: impl Fn() -> R,
    original: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &[(ObjectId, PreparedEventOutcome<AppliedZoneChange>)],
    ) -> Result<crate::effect::EffectOutcome, ExecutionError>,
    complete: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        crate::effect::EffectOutcome,
        Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>,
        super::ZoneInstructionDraws,
    ) -> Result<R, ExecutionError>,
) -> Result<R, ExecutionError> {
    crate::effects::composition::execute_transaction(game, ctx, &pending, |game, ctx| {
        let opened = moves.len() > 1 && game.open_simultaneous_action();
        let pinned = moves.len() > 1
            && crate::effects::helpers::begin_simultaneous_zone_change_lookback(game);
        let result: Result<(_, _, _), ExecutionError> = (|| {
            let (proposals, mut draws) = prepare_zone_moves(game, ctx, moves)?;
            let receipts = commit_prepared_zone_moves(game, ctx, proposals, &mut draws)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok((crate::effect::EffectOutcome::count(0), Vec::new(), draws));
            }
            let outcome = original(game, ctx, &receipts)?;
            Ok((outcome, receipts, draws))
        })();
        // Added programs are separate actions and observe the completed
        // original world rather than inheriting this batch's look-back.
        crate::effects::helpers::end_simultaneous_zone_change_lookback(game, pinned);
        game.close_simultaneous_action(opened);
        let (outcome, receipts, draws) = result?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(pending());
        }
        complete(game, ctx, outcome, receipts, draws)
    })
}

/// Group actual original arrivals for a simultaneous zone instruction, using
/// the captured source world rather than reconstructing departed objects.
pub(crate) fn group_zone_move_observations(
    game: &mut GameState,
    ctx: &ExecutionContext,
    pending_start: usize,
    receipts: &[(ObjectId, PreparedEventOutcome<AppliedZoneChange>)],
    snapshots: &std::collections::HashMap<ObjectId, ObjectSnapshot>,
    from: Zone,
    to: Zone,
) {
    group_zone_move_observations_with_cause(
        game,
        ctx,
        pending_start,
        receipts,
        snapshots,
        from,
        to,
        ctx.cause.clone(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn group_zone_move_observations_with_cause(
    game: &mut GameState,
    ctx: &ExecutionContext,
    pending_start: usize,
    receipts: &[(ObjectId, PreparedEventOutcome<AppliedZoneChange>)],
    snapshots: &std::collections::HashMap<ObjectId, ObjectSnapshot>,
    from: Zone,
    to: Zone,
    cause: EventCause,
) {
    let changes = receipts
        .iter()
        .filter_map(|(id, receipt)| match &receipt.original {
            EventOutcome::Proceed(change) if change.final_zone == to => snapshots
                .get(id)
                .filter(|snapshot| snapshot.zone == from)
                .map(|snapshot| (*id, change, snapshot)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if changes.len() < 2 {
        return;
    }
    let objects = changes.iter().map(|(id, _, _)| *id).collect::<Vec<_>>();
    let removed = game.remove_pending_trigger_events_matching_from(pending_start, |event| {
        event
            .downcast::<crate::events::ZoneChangeEvent>()
            .is_some_and(|event| {
                event.from == from
                    && event.to == to
                    && event.objects.len() == 1
                    && objects.contains(&event.objects[0])
            })
    });
    if removed.is_empty() {
        return;
    }
    let mut lookback = Vec::new();
    for snapshot in removed
        .iter()
        .flat_map(|event| event.lookback_source_snapshots())
    {
        if !lookback
            .iter()
            .any(|existing: &ObjectSnapshot| existing.stable_id == snapshot.stable_id)
        {
            lookback.push(snapshot.clone());
        }
    }
    let mut event = crate::events::ZoneChangeEvent::batch_with_snapshots(
        objects,
        from,
        to,
        cause,
        changes
            .iter()
            .map(|(_, _, snapshot)| (*snapshot).clone())
            .collect(),
    );
    event.result_objects = changes
        .iter()
        .flat_map(|(_, change, _)| change.new_object_ids.iter().copied())
        .collect();
    game.queue_trigger_event(
        ctx.provenance,
        crate::triggers::TriggerEvent::new_with_provenance(event, ctx.provenance)
            .with_lookback_source_snapshots(lookback),
    );
}

/// Shared completion owner for both ordinary moves and battlefield entries.
struct MovementCompletion {
    published: Vec<crate::effects::PublishedEffectOutputs>,
    iterated_player: Option<crate::ids::PlayerId>,
    receipts: Option<Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>>,
    frozen: Option<super::FrozenZoneChangeReceipts>,
}
impl crate::effects::SimultaneousEffectCompletion for MovementCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        let receipts = self.receipts.take().ok_or_else(|| {
            ExecutionError::InternalError("movement receipts already frozen".into())
        })?;
        self.frozen = Some(super::freeze_zone_change_receipts(game, receipts));
        Ok(())
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<crate::effect::EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effect::EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let frozen = self.frozen.ok_or_else(|| {
            ExecutionError::InternalError(
                "movement completion requires a frozen original batch".into(),
            )
        })?;
        let mut outputs = ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
            super::finish_zone_change_receipts_frozen_with_outputs(game, ctx, original, frozen)
        })?;
        if !ctx.decision_maker.awaiting_choice() {
            outputs.retain_published_references(self.published);
        }
        Ok(outputs)
    }
}

/// Retain the same completion contract in ordinary and simultaneous modes.
pub(crate) fn complete_movement_batch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: crate::effect::EffectOutcome,
    receipts: Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>,
    deferred: bool,
) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    complete_movement_batch_with_outputs(
        game,
        ctx,
        crate::effects::CompletedEffectOutputs::aggregate_only(original),
        receipts,
        deferred,
        Vec::new(),
    )
    .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
}

/// The actual original stays in the prepared commit; the same completion owner
/// retains entry-published references until deferred zone programs finish.
fn complete_movement_batch_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: crate::effects::CompletedEffectOutputs,
    receipts: Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>,
    deferred: bool,
    published: Vec<crate::effects::PublishedEffectOutputs>,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    if deferred {
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: original,
            completion: Some(Box::new(MovementCompletion {
                published,
                iterated_player: ctx.iteration.iterated_player,
                receipts: Some(receipts),
                frozen: None,
            })),
        })
    } else {
        let mut outputs = super::finish_zone_change_receipts_with_outputs(
            game,
            ctx,
            original.outcome.clone(),
            receipts,
        )?;
        if !ctx.decision_maker.awaiting_choice() {
            outputs.retain_published_children([original]);
            outputs.retain_published_references(published);
        }
        Ok(crate::effects::SimultaneousEffectCommit::finished(outputs))
    }
}

/// Entry options are fixed before any original is committed. Authored arrival
/// work runs on the whole original batch, before deferred entry programs.
pub(crate) fn execute_battlefield_entries<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    requests: Vec<(ObjectId, super::BattlefieldEntryOptions)>,
    deferred: bool,
    original: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &[super::BattlefieldEntryReceipt],
    ) -> Result<crate::effect::EffectOutcome, ExecutionError>,
) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
    execute_battlefield_entries_with_outputs(
        game,
        ctx,
        requests,
        deferred,
        |game, ctx, receipts| {
            original(game, ctx, receipts)
                .map(crate::effects::CompletedEffectOutputs::aggregate_only)
        },
    )
    .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
}

pub(crate) fn execute_battlefield_entries_with_outputs<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    requests: Vec<(ObjectId, super::BattlefieldEntryOptions)>,
    deferred: bool,
    original: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &[super::BattlefieldEntryReceipt],
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError>,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    crate::effects::composition::execute_result_transaction(game, ctx, |game, ctx| {
        let expected = requests.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        let receipts = super::move_to_battlefield_batch_with_options(game, ctx, requests)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(
                    crate::effect::EffectOutcome::count(0),
                ),
            ));
        }
        if receipts.len() != expected.len() {
            return Err(ExecutionError::InternalError(
                "entry program lost a movement receipt".into(),
            ));
        }
        let outcome = original(game, ctx, &receipts)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(
                    crate::effect::EffectOutcome::count(0),
                ),
            ));
        }
        let mut published = Vec::new();
        let receipts = receipts
            .into_iter()
            .zip(expected)
            .map(|(receipt, expected)| {
                let ((id, receipt), packets) = receipt.into_zone_receipt_with_outputs();
                crate::effects::PublishedEffectOutputs::append_distinct(&mut published, packets);
                if id != expected {
                    return Err(ExecutionError::InternalError(
                        "entry program changed original identity".into(),
                    ));
                }
                Ok((id, receipt))
            })
            .collect::<Result<Vec<_>, ExecutionError>>()?;
        complete_movement_batch_with_outputs(game, ctx, outcome, receipts, deferred, published)
    })
}
