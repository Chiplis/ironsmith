//! Exact zone originals, replacement prefixes and deferred completion receipts.
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError, SimultaneousEffectCommit, SimultaneousEffectCompletion};
use crate::events::processing::{EventOutcome, PreparedEventOutcome, ZoneDrawContinuations};
use crate::game_state::GameState;
use crate::ids::ObjectId;

type ZoneReceipt = (ObjectId, PreparedEventOutcome<super::AppliedZoneChange>);

#[derive(Debug, Default)]
pub(crate) struct ZoneInstructionDraws {
    pub draws: ZoneDrawContinuations,
    pub ranges: std::collections::HashMap<ObjectId, std::ops::Range<usize>>,
    pub prepared: std::collections::HashMap<ObjectId, PreparedEventOutcome<crate::events::processing::PreparedZoneChange>>,
    pub snapshots: std::collections::HashMap<ObjectId, crate::snapshot::ObjectSnapshot>,
}
impl ZoneInstructionDraws {
    pub fn record(&mut self, object: ObjectId, start: usize) {
        let end = self.draws.0.len();
        self.ranges.entry(object).and_modify(|range| range.end = end).or_insert(start..end);
    }
    pub fn finish_replacement(
        self, game: &mut GameState, ctx: &mut ExecutionContext,
        original: EffectOutcome, receipts: Vec<ZoneReceipt>,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.finish_replacement_with_outputs(game, ctx, original, receipts)
            .map(SimultaneousEffectCommit::into_aggregate)
    }
    pub fn finish_replacement_with_outputs(
        self, game: &mut GameState, ctx: &mut ExecutionContext,
        original: EffectOutcome, receipts: Vec<ZoneReceipt>,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let mut committed = self.finish(original, receipts, ctx);
        if let Some(mut completion) = committed.completion.take() {
            completion.freeze(game)?;
            completion.observe_original(game, ctx, &mut committed.outcome)?;
            completion.prepare_draw_boundary_with_outputs(game, ctx, committed.outcome)
        } else { Ok(committed.into_retained()) }
    }
    pub fn finish(self, original: EffectOutcome, receipts: Vec<ZoneReceipt>, ctx: &ExecutionContext) -> SimultaneousEffectCommit {
        let receipts = receipts.into_iter().map(|receipt| {
            let range = self.ranges.get(&receipt.0).cloned().unwrap_or(0..0);
            (receipt, range)
        }).collect();
        prepare_zone_instruction_completion(original, receipts, self.draws, ctx.iteration.iterated_player)
    }
}

pub(crate) fn prepare_zone_instruction_completion(
    mut original: EffectOutcome,
    receipts: Vec<(ZoneReceipt, std::ops::Range<usize>)>,
    draws: ZoneDrawContinuations,
    iterated_player: Option<crate::ids::PlayerId>,
) -> SimultaneousEffectCommit {
    let events = draws.0.iter().flat_map(|draw| draw.outcome.outcome.events.iter().cloned()).collect::<Vec<_>>();
    let facts = draws.0.iter().flat_map(|draw| draw.outcome.outcome.execution_facts.iter().cloned()).collect::<Vec<_>>();
    let prefix_events = events.len();
    let prefix_facts = facts.len();
    original.events.splice(0..0, events);
    original.execution_facts.splice(0..0, facts);
    SimultaneousEffectCommit {
        outcome: original,
        completion: Some(Box::new(ZoneInstructionCompletion {
            receipts: Some(receipts), frozen: Vec::new(), delayed: Vec::new(),
            draws, prefix_events, prefix_facts, iterated_player,
        })),
    }
}

pub(crate) fn complete_zone_instruction(
    game: &mut GameState, ctx: &mut ExecutionContext, committed: SimultaneousEffectCommit,
) -> Result<EffectOutcome, ExecutionError> {
    complete_zone_instruction_with_outputs(game, ctx, committed).map(CompletedEffectOutputs::into_outcome)
}

pub(crate) fn complete_zone_instruction_with_outputs(
    game: &mut GameState, ctx: &mut ExecutionContext, committed: SimultaneousEffectCommit,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::complete_standalone_original_with_outputs(game, ctx, committed)
}

/// Arrival facts freeze in the completed original world, alongside main's
/// entry observations. A departed arrival keeps its retained receipt; never
/// follow a later incarnation of the same physical card.
fn freeze_original_arrival_facts(game: &GameState, original: &mut EffectOutcome)
    -> Result<(), ExecutionError> {
    for fact in &mut original.execution_facts {
        if let crate::effect::ExecutionFact::OriginalZoneMoveCards(cards) = fact {
            for snapshot in cards {
                if game.object(snapshot.object_id).is_some_and(|object|
                    object.stable_id == snapshot.stable_id && object.zone == snapshot.zone)
                {
                    *snapshot = crate::snapshot::ObjectSnapshot::try_from_object_id(game, snapshot.object_id)?
                        .ok_or_else(|| ExecutionError::IncompleteEvidence("original arrival disappeared during completed capture".into()))?;
                }
            }
        }
    }
    if let Some(result) = original.instruction_result.as_deref_mut() {
        freeze_original_arrival_facts(game, result)?;
    }
    Ok(())
}

struct ZoneInstructionCompletion {
    receipts: Option<Vec<(ZoneReceipt, std::ops::Range<usize>)>>,
    frozen: Vec<Option<super::FrozenZoneChangeReceipts>>,
    delayed: Vec<(usize, ZoneReceipt, usize)>,
    draws: crate::events::processing::ZoneDrawContinuations,
    prefix_events: usize,
    prefix_facts: usize,
    iterated_player: Option<crate::ids::PlayerId>,
}
impl SimultaneousEffectCompletion for ZoneInstructionCompletion {
    fn prepare_draw_boundary(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        original: EffectOutcome) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.prepare_draw_boundary_with_outputs(game, ctx, original).map(SimultaneousEffectCommit::into_aggregate)
    }
    fn prepare_draw_boundary_with_outputs(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        mut original: EffectOutcome) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        if self.draws.0.iter().any(|draw| draw.completion.is_some()) {
            return Ok(SimultaneousEffectCommit { outcome: CompletedEffectOutputs::aggregate_only(original), completion: Some(self) });
        }
        original.events.drain(..self.prefix_events);
        original.execution_facts.drain(..self.prefix_facts);
        let mut outputs = CompletedEffectOutputs::aggregate_only(EffectOutcome::aggregate_replacement_outcomes(
            original, self.draws.0.iter().map(|draw| draw.outcome.outcome.clone()),
        ));
        outputs.retain_batch_children(self.draws.0.into_iter().map(|draw| draw.outcome));
        let mut programs = Vec::new();
        for frozen in self.frozen {
            let frozen = frozen.ok_or_else(|| ExecutionError::InternalError(
                "zone replacement boundary requires a frozen original".into()))?;
            programs.extend(super::bind_frozen_zone_programs(frozen)?);
        }
        let mut prepared = crate::effects::replacement::prepare_zone_draw_tail_with_outputs(
            game, ctx, outputs.outcome.clone(), programs, &[])?;
        prepared.outcome.retain_batch_children([outputs]);
        Ok(prepared)
    }
    fn observe_original(&mut self, game: &mut GameState, ctx: &mut ExecutionContext,
        original: &mut EffectOutcome) -> Result<(), ExecutionError> {
        game.freeze_completed_entry_events(original.events.iter_mut())?;
        freeze_original_arrival_facts(game, original)?;
        for draw in &mut self.draws.0 {
            if let Some(completion) = &mut draw.completion {
                completion.observe_original(game, ctx, &mut draw.outcome.outcome)?;
                draw.outcome.synchronize_observations();
            }
        }
        Ok(())
    }
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        let Some(receipts) = self.receipts.take() else {
            // A containing program can freeze the same retained frame again;
            // already frozen arrival bindings must never be reconstructed.
            return Ok(());
        };
        for ((object, receipt), range) in receipts {
            let index = self.frozen.len();
            let arrivals = game.take_zone_change_results(object);
            let has_arrival = !arrivals.is_empty();
            if has_arrival { game.record_zone_change_results(object, arrivals); }
            let pending = matches!(&receipt.original, EventOutcome::Replaced)
                && !has_arrival
                && self.draws.0[range.clone()].iter().any(|draw| draw.completion.is_some());
            if pending {
                // The replacement's draw suffix may produce the original
                // object's exact arrival receipt. Freeze it at its own finish,
                // before another continuation or any added program can move it.
                self.frozen.push(None);
                self.delayed.push((index, (object, receipt), range.end));
            } else {
                self.frozen.push(Some(super::freeze_zone_change_receipts(game, vec![(object, receipt)])));
            }
        }
        for draw in &mut self.draws.0 {
            if let Some(completion) = &mut draw.completion { completion.freeze(game)?; }
        }
        Ok(())
    }

    fn complete(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        original: EffectOutcome) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original).map(CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        mut original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let iterated_player = self.iterated_player;
        ctx.with_temp_iterated_player(iterated_player, |ctx| {
        crate::effects::runtime::capture_triggers_before_added_program(
            game, ctx, None, original.events.iter_mut(),
        )?;
        // Resumed subtree receipts include their prefixes exactly once. The
        // original batch exposed those prefixes for event-time matching only.
        original.events.drain(..self.prefix_events);
        original.execution_facts.drain(..self.prefix_facts);
        let mut completed = Vec::new();
        for (index, draw) in std::mem::take(&mut self.draws.0).into_iter().enumerate() {
            completed.push(crate::effects::composition::complete_committed_original_with_outputs(game, ctx, draw)?);
            if ctx.decision_maker.awaiting_choice() { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
            let mut remaining = Vec::new();
            for (slot, receipt, end) in std::mem::take(&mut self.delayed) {
                if end == index + 1 {
                    self.frozen[slot] = Some(super::freeze_zone_change_receipts(game, vec![receipt]));
                } else { remaining.push((slot, receipt, end)); }
            }
            self.delayed = remaining;
        }
        let aggregate = EffectOutcome::aggregate_replacement_outcomes(original,
            completed.iter().map(|child| child.outcome.clone()));
        let mut outputs = CompletedEffectOutputs::aggregate_only(aggregate);
        outputs.retain_batch_children(completed);
        for frozen in self.frozen {
            let frozen = frozen.ok_or_else(|| ExecutionError::InternalError(
                "zone instruction completion has an unfinished original receipt".into()))?;
            let mut completed = super::finish_zone_change_receipts_frozen_with_outputs(
                game, ctx, outputs.outcome.clone(), frozen)?;
            completed.retain_batch_children([outputs]);
            outputs = completed;
            if ctx.decision_maker.awaiting_choice() { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
        }
        Ok(outputs)
        })
    }
}
