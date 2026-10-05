//! Immutable request for an ordinary zone movement original.

use super::AppliedZoneChange;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::cause::EventCause;
use crate::events::processing::{EventOutcome, PreparedEventOutcome};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::replacement::ReplacementEffect;
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

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

    pub(crate) fn commit(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        additional: &[ReplacementEffect],
    ) -> Result<PreparedEventOutcome<AppliedZoneChange>, ExecutionError> {
        if !game
            .object(self.object)
            .is_some_and(|object| object.zone == self.from)
        {
            return Ok(PreparedEventOutcome {
                original: EventOutcome::NotApplicable,
                programs: Vec::new(),
            });
        }
        super::commit_zone_move_request(
            game,
            self.object,
            self.from,
            self.to,
            self.cause,
            ctx,
            additional,
            self.snapshot,
        )
    }
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
    commit_zone_moves(game, ctx, moves, false, original).map(|commit| commit.outcome)
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
    let checkpoint = game.clone();
    let context = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        let additional = ctx.additional_replacement_effects_snapshot();
        let mut receipts = Vec::with_capacity(moves.len());
        for request in moves {
            let object = request.object;
            receipts.push((object, request.commit(game, ctx, &additional)?));
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::SimultaneousEffectCommit::finished(
                    crate::effect::EffectOutcome::count(0),
                ));
            }
        }
        let original = original(game, ctx, &receipts)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effect::EffectOutcome::count(0),
            ));
        }
        complete_movement_batch(game, ctx, original, receipts, deferred)
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            checkpoint,
            result.is_ok() && ctx.decision_maker.awaiting_choice(),
        );
        context.restore(ctx);
    }
    result
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
    let changes = receipts
        .iter()
        .filter_map(|(id, receipt)| match &receipt.original {
            EventOutcome::Proceed(change) if change.final_zone == to => {
                snapshots.get(id).map(|snapshot| (*id, change, snapshot))
            }
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
        ctx.cause.clone(),
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
    receipts: Option<Vec<(ObjectId, PreparedEventOutcome<AppliedZoneChange>)>>,
    frozen: Option<super::FrozenZoneChangeReceipts>,
}
impl crate::effects::SimultaneousEffectCompletion for MovementCompletion {
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
        let frozen = self.frozen.ok_or_else(|| {
            ExecutionError::InternalError(
                "movement completion requires a frozen original batch".into(),
            )
        })?;
        super::finish_zone_change_receipts_frozen(game, ctx, original, frozen)
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
    if deferred {
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: original,
            completion: Some(Box::new(MovementCompletion {
                receipts: Some(receipts),
                frozen: None,
            })),
        })
    } else {
        super::finish_zone_change_receipts(game, ctx, original, receipts)
            .map(crate::effects::SimultaneousEffectCommit::finished)
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
    let checkpoint = game.clone();
    let context = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        let expected = requests.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        let receipts = super::move_to_battlefield_batch_with_options(game, ctx, requests)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effect::EffectOutcome::count(0),
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
                crate::effect::EffectOutcome::count(0),
            ));
        }
        let receipts = receipts
            .into_iter()
            .zip(expected)
            .map(|(receipt, expected)| {
                let (id, receipt) = receipt.into_zone_receipt();
                if id != expected {
                    return Err(ExecutionError::InternalError(
                        "entry program changed original identity".into(),
                    ));
                }
                Ok((id, receipt))
            })
            .collect::<Result<Vec<_>, ExecutionError>>()?;
        complete_movement_batch(game, ctx, outcome, receipts, deferred)
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            checkpoint,
            result.is_ok() && ctx.decision_maker.awaiting_choice(),
        );
        context.restore(ctx);
    }
    result
}
