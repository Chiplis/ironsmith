//! Exact zone originals, replacement prefixes and deferred completion receipts.
use crate::effect::EffectOutcome;
use crate::effects::{ExecutionContext, ExecutionError, SimultaneousEffectCommit, SimultaneousEffectCompletion};
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
        self.ranges.insert(object, start..self.draws.0.len());
    }
    pub fn finish_replacement(
        self, game: &mut GameState, ctx: &mut ExecutionContext,
        original: EffectOutcome, receipts: Vec<ZoneReceipt>,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        if self.draws.0.iter().any(|draw| draw.completion.is_some()) {
            return Ok(self.finish(original, receipts));
        }
        // No inner original paused. Execute added non-draw prefixes now; the
        // first actual draw, plus later programs, belongs to its continuation.
        let mut original = EffectOutcome::aggregate_replacement_outcomes(
            original, self.draws.0.into_iter().map(|draw| draw.outcome),
        );
        game.freeze_completed_entry_events(original.events.iter_mut())?;
        let frozen = super::freeze_zone_change_receipts(game, receipts);
        let programs = super::bind_frozen_zone_programs(frozen)?;
        crate::effects::replacement::prepare_zone_draw_tail(game, ctx, original, programs, &[])
    }
    pub fn finish(self, original: EffectOutcome, receipts: Vec<ZoneReceipt>) -> SimultaneousEffectCommit {
        let receipts = receipts.into_iter().map(|receipt| {
            let range = self.ranges.get(&receipt.0).cloned().unwrap_or(0..0);
            (receipt, range)
        }).collect();
        prepare_zone_instruction_completion(original, receipts, self.draws)
    }
}

pub(crate) fn prepare_zone_instruction_completion(
    mut original: EffectOutcome,
    receipts: Vec<(ZoneReceipt, std::ops::Range<usize>)>,
    draws: ZoneDrawContinuations,
) -> SimultaneousEffectCommit {
    let events = draws.0.iter().flat_map(|draw| draw.outcome.events.iter().cloned()).collect::<Vec<_>>();
    let facts = draws.0.iter().flat_map(|draw| draw.outcome.execution_facts.iter().cloned()).collect::<Vec<_>>();
    let prefix_events = events.len();
    let prefix_facts = facts.len();
    original.events.splice(0..0, events);
    original.execution_facts.splice(0..0, facts);
    SimultaneousEffectCommit {
        outcome: original,
        completion: Some(Box::new(ZoneInstructionCompletion {
            receipts: Some(receipts), frozen: Vec::new(), delayed: Vec::new(),
            draws, prefix_events, prefix_facts,
        })),
    }
}

pub(crate) fn complete_zone_instruction(
    game: &mut GameState, ctx: &mut ExecutionContext, committed: SimultaneousEffectCommit,
) -> Result<EffectOutcome, ExecutionError> {
    if let Some(mut completion) = committed.completion {
        completion.freeze(game)?;
        completion.complete(game, ctx, committed.outcome)
    } else { Ok(committed.outcome) }
}

struct ZoneInstructionCompletion {
    receipts: Option<Vec<(ZoneReceipt, std::ops::Range<usize>)>>,
    frozen: Vec<Option<super::FrozenZoneChangeReceipts>>,
    delayed: Vec<(usize, ZoneReceipt, usize)>,
    draws: crate::events::processing::ZoneDrawContinuations,
    prefix_events: usize,
    prefix_facts: usize,
}
impl SimultaneousEffectCompletion for ZoneInstructionCompletion {
    fn prepare_draw_boundary(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        mut original: EffectOutcome) -> Result<SimultaneousEffectCommit, ExecutionError> {
        if self.draws.0.iter().any(|draw| draw.completion.is_some()) {
            return Ok(SimultaneousEffectCommit { outcome: original, completion: Some(self) });
        }
        original.events.drain(..self.prefix_events);
        original.execution_facts.drain(..self.prefix_facts);
        let original = EffectOutcome::aggregate_replacement_outcomes(
            original, self.draws.0.into_iter().map(|draw| draw.outcome),
        );
        let mut programs = Vec::new();
        for frozen in self.frozen {
            let frozen = frozen.ok_or_else(|| ExecutionError::InternalError(
                "zone replacement boundary requires a frozen original".into()))?;
            programs.extend(super::bind_frozen_zone_programs(frozen)?);
        }
        crate::effects::replacement::prepare_zone_draw_tail(game, ctx, original, programs, &[])
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

    fn complete(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        mut original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::runtime::capture_triggers_before_added_program(
            game, ctx, None, original.events.iter_mut(),
        )?;
        // Resumed subtree receipts include their prefixes exactly once. The
        // original batch exposed those prefixes for event-time matching only.
        original.events.drain(..self.prefix_events);
        original.execution_facts.drain(..self.prefix_facts);
        let mut completed = Vec::new();
        for (index, draw) in std::mem::take(&mut self.draws.0).into_iter().enumerate() {
            completed.push(if let Some(completion) = draw.completion {
                completion.complete(game, ctx, draw.outcome)?
            } else { draw.outcome });
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let mut remaining = Vec::new();
            for (slot, receipt, end) in std::mem::take(&mut self.delayed) {
                if end == index + 1 {
                    self.frozen[slot] = Some(super::freeze_zone_change_receipts(game, vec![receipt]));
                } else { remaining.push((slot, receipt, end)); }
            }
            self.delayed = remaining;
        }
        let mut original = EffectOutcome::aggregate_replacement_outcomes(original, completed);
        for frozen in self.frozen {
            let frozen = frozen.ok_or_else(|| ExecutionError::InternalError(
                "zone instruction completion has an unfinished original receipt".into()))?;
            original = super::finish_zone_change_receipts_frozen(game, ctx, original, frozen)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        }
        Ok(original)
    }
}
