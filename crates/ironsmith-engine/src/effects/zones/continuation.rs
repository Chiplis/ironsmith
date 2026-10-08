//! Exact zone originals, replacement prefixes and deferred completion receipts.
use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionError, SimultaneousEffectCommit,
    SimultaneousEffectCompletion,
};
use crate::events::processing::{EventOutcome, PreparedEventOutcome, ZoneDrawContinuations};
use crate::game_state::GameState;
use crate::ids::ObjectId;

type ZoneReceipt = (ObjectId, PreparedEventOutcome<super::AppliedZoneChange>);

#[derive(Debug, Default)]
pub(crate) struct ZoneInstructionDraws {
    pub draws: ZoneDrawContinuations,
    pub ranges: std::collections::HashMap<ObjectId, Vec<std::ops::Range<usize>>>,
    pub prepared: std::collections::HashMap<
        ObjectId,
        PreparedEventOutcome<crate::events::processing::PreparedZoneChange>,
    >,
    pub snapshots: std::collections::HashMap<ObjectId, crate::snapshot::ObjectSnapshot>,
}
impl ZoneInstructionDraws {
    /// Retain entry programme packets while projecting its legacy movement
    /// receipt. Normal and draw-boundary completion share this publisher.
    pub(crate) fn retain_entry_receipt(
        &mut self,
        receipt: super::BattlefieldEntryReceipt,
    ) -> ZoneReceipt {
        let (movement, published) = receipt.into_zone_receipt_with_outputs();
        crate::effects::PublishedEffectOutputs::append_distinct(&mut self.draws.1, published);
        movement
    }
    /// Project the committed movement while retaining the actual entry packets
    /// for this instruction's original and full-completion gateways.
    pub(crate) fn retain_committed_zone_receipt(
        &mut self,
        committed: crate::events::processing::CommittedZoneChange<super::AppliedZoneChange>,
    ) -> crate::events::processing::PreparedEventOutcome<super::AppliedZoneChange> {
        crate::effects::PublishedEffectOutputs::append_distinct(
            &mut self.draws.1,
            committed.published_outputs,
        );
        committed.receipt
    }

    pub fn record(&mut self, object: ObjectId, start: usize) {
        let end = self.draws.0.len();
        if start < end {
            self.ranges.entry(object).or_default().push(start..end);
        }
    }

    pub(crate) fn commit_pending_replacement(
        &mut self,
        game: &mut GameState,
        object: ObjectId,
        dm: &mut dyn crate::DecisionMaker,
    ) -> Result<(), ExecutionError> {
        let start = self.draws.0.len();
        self.draws.commit_pending_replacement(game, object, dm)?;
        self.record(object, start);
        Ok(())
    }
    pub fn finish_replacement(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
        receipts: Vec<ZoneReceipt>,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.finish_replacement_with_outputs(game, ctx, original, receipts)
            .map(SimultaneousEffectCommit::into_aggregate)
    }
    pub fn finish_replacement_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
        receipts: Vec<ZoneReceipt>,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let mut committed = self.finish(original, receipts, ctx);
        if let Some(mut completion) = committed.completion.take() {
            completion.freeze(game)?;
            completion.observe_original(game, ctx, &mut committed.outcome)?;
            completion.prepare_draw_boundary_with_outputs(game, ctx, committed.outcome)
        } else {
            Ok(committed.into_retained())
        }
    }
    /// Merge only committed original receipts from a nested cohort.
    /// Prefix ranges are shifted, never widened across sibling observations.
    pub(crate) fn append_committed_originals(
        &mut self,
        mut other: Self,
    ) -> Result<(), ExecutionError> {
        if !other.prepared.is_empty() {
            return Err(ExecutionError::InternalError(
                "zone cohort transfer contains an uncommitted original".into(),
            ));
        }
        other
            .draws
            .require_committed_originals("zone cohort transfer contains an uncommitted original")?;
        let offset = self.draws.0.len();
        self.draws.0.append(&mut other.draws.0);
        crate::effects::PublishedEffectOutputs::append_distinct(&mut self.draws.1, other.draws.1);
        for (object, ranges) in other.ranges {
            self.ranges.entry(object).or_default().extend(
                ranges
                    .into_iter()
                    .map(|range| range.start + offset..range.end + offset),
            );
        }
        for (object, snapshot) in other.snapshots {
            self.snapshots.entry(object).or_insert(snapshot);
        }
        Ok(())
    }

    pub fn finish(
        self,
        original: EffectOutcome,
        receipts: Vec<ZoneReceipt>,
        ctx: &ExecutionContext,
    ) -> SimultaneousEffectCommit {
        let (outcome, completion) = self.finish_original(original, receipts, ctx);
        SimultaneousEffectCommit {
            outcome,
            completion: Some(completion),
        }
    }

    pub(crate) fn finish_original(
        self,
        original: EffectOutcome,
        receipts: Vec<ZoneReceipt>,
        ctx: &ExecutionContext,
    ) -> (EffectOutcome, Box<ZoneInstructionCompletion>) {
        let receipts = receipts
            .into_iter()
            .map(|receipt| {
                let range = self.ranges.get(&receipt.0).cloned().unwrap_or_default();
                (receipt, range)
            })
            .collect();
        prepare_zone_original_completion(
            original,
            receipts,
            self.draws,
            ctx.iteration.iterated_player,
        )
    }
}

pub(crate) fn prepare_zone_instruction_completion(
    original: EffectOutcome,
    receipts: Vec<(ZoneReceipt, Vec<std::ops::Range<usize>>)>,
    draws: ZoneDrawContinuations,
    iterated_player: Option<crate::ids::PlayerId>,
) -> SimultaneousEffectCommit {
    let (outcome, completion) =
        prepare_zone_original_completion(original, receipts, draws, iterated_player);
    SimultaneousEffectCommit {
        outcome,
        completion: Some(completion),
    }
}

pub(crate) fn prepare_zone_original_completion(
    mut original: EffectOutcome,
    receipts: Vec<(ZoneReceipt, Vec<std::ops::Range<usize>>)>,
    draws: ZoneDrawContinuations,
    iterated_player: Option<crate::ids::PlayerId>,
) -> (EffectOutcome, Box<ZoneInstructionCompletion>) {
    let events = draws
        .0
        .iter()
        .flat_map(|draw| draw.outcome.outcome.events.iter().cloned())
        .collect::<Vec<_>>();
    let facts = draws
        .0
        .iter()
        .flat_map(|draw| draw.outcome.outcome.execution_facts.iter().cloned())
        .collect::<Vec<_>>();
    let prefix_events = events.len();
    let prefix_facts = facts.len();
    original.events.splice(0..0, events);
    original.execution_facts.splice(0..0, facts);
    (
        original,
        Box::new(ZoneInstructionCompletion {
            receipts: Some(receipts),
            frozen: Vec::new(),
            delayed: Vec::new(),
            draws,
            prefix_events,
            prefix_facts,
            iterated_player,
        }),
    )
}

pub(crate) fn complete_zone_instruction(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    committed: SimultaneousEffectCommit,
) -> Result<EffectOutcome, ExecutionError> {
    complete_zone_instruction_with_outputs(game, ctx, committed)
        .map(CompletedEffectOutputs::into_outcome)
}

pub(crate) fn complete_zone_instruction_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    committed: SimultaneousEffectCommit,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::complete_standalone_original_with_outputs(game, ctx, committed)
}

/// Arrival facts freeze in the completed original world, alongside main's
/// entry observations. A departed arrival keeps its retained receipt; never
/// follow a later incarnation of the same physical card.
fn freeze_original_arrival_facts(
    game: &GameState,
    original: &mut EffectOutcome,
) -> Result<(), ExecutionError> {
    for fact in &mut original.execution_facts {
        if let crate::effect::ExecutionFact::OriginalZoneMoveCards(cards) = fact {
            for snapshot in cards {
                if game.object(snapshot.object_id).is_some_and(|object| {
                    object.stable_id == snapshot.stable_id && object.zone == snapshot.zone
                }) {
                    *snapshot = crate::snapshot::ObjectSnapshot::try_from_object_id(
                        game,
                        snapshot.object_id,
                    )?
                    .ok_or_else(|| {
                        ExecutionError::IncompleteEvidence(
                            "original arrival disappeared during completed capture".into(),
                        )
                    })?;
                }
            }
        }
    }
    if let Some(result) = original.instruction_result.as_deref_mut() {
        freeze_original_arrival_facts(game, result)?;
    }
    Ok(())
}

pub(crate) struct ZoneInstructionCompletion {
    receipts: Option<Vec<(ZoneReceipt, Vec<std::ops::Range<usize>>)>>,
    frozen: Vec<Option<super::FrozenZoneChangeReceipts>>,
    delayed: Vec<(usize, ZoneReceipt, usize)>,
    draws: crate::events::processing::ZoneDrawContinuations,
    prefix_events: usize,
    prefix_facts: usize,
    iterated_player: Option<crate::ids::PlayerId>,
}
impl SimultaneousEffectCompletion for ZoneInstructionCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        if self.draws.has_pending_originals() {
            crate::effects::OriginalPhaseStatus::Combined
        } else {
            crate::effects::OriginalPhaseStatus::Retained
        }
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let completed = self.complete_original_with_outputs(game, ctx, original)?;
        Ok(SimultaneousEffectCommit {
            outcome: completed.outputs,
            completion: Some(Box::new(ZoneAddedProgramsCompletion {
                frozen: completed.frozen,
                iterated_player: completed.iterated_player,
            })),
        })
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let completed = self.complete_original_with_outputs(game, ctx, original)?;
        Ok(SimultaneousEffectCommit {
            outcome: completed.outputs,
            completion: Some(Box::new(ZoneAddedProgramsCompletion {
                frozen: completed.frozen,
                iterated_player: completed.iterated_player,
            })),
        })
    }

    fn prepare_draw_boundary(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.prepare_draw_boundary_with_outputs(game, ctx, original)
            .map(SimultaneousEffectCommit::into_aggregate)
    }
    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.prepare_draw_boundary_from_outputs(
            game,
            ctx,
            CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        mut original: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.draws.require_committed_originals(
            "zone completion received an uncommitted replacement original",
        )?;
        if self.draws.0.iter().any(|draw| draw.completion.is_some()) {
            return Ok(SimultaneousEffectCommit {
                outcome: original,
                completion: Some(self),
            });
        }
        original.outcome.events.drain(..self.prefix_events);
        original.outcome.execution_facts.drain(..self.prefix_facts);
        let outputs = retain_completed_zone_original_outputs(
            original,
            self.draws.0.into_iter().map(|draw| draw.outcome).collect(),
            self.draws.1,
        );
        let mut programs = Vec::new();
        for frozen in self.frozen {
            let frozen = frozen.ok_or_else(|| {
                ExecutionError::InternalError(
                    "zone replacement boundary requires a frozen original".into(),
                )
            })?;
            programs.extend(super::bind_frozen_zone_programs(frozen)?);
        }
        crate::effects::replacement::prepare_zone_draw_tail_with_outputs(
            game,
            ctx,
            outputs,
            programs,
            &[],
        )
    }
    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
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
        self.draws.require_committed_originals(
            "zone completion received an uncommitted replacement original",
        )?;
        let Some(receipts) = self.receipts.take() else {
            // A containing program can freeze the same retained frame again;
            // already frozen arrival bindings must never be reconstructed.
            return Ok(());
        };
        for ((object, receipt), range) in receipts {
            let index = self.frozen.len();
            let arrivals = game.take_zone_change_results(object);
            let has_arrival = !arrivals.is_empty();
            if has_arrival {
                game.record_zone_change_results(object, arrivals);
            }
            let pending = matches!(&receipt.original, EventOutcome::Replaced)
                && !has_arrival
                && range.iter().any(|part| {
                    self.draws.0[part.clone()]
                        .iter()
                        .any(|draw| draw.completion.is_some())
                });
            if pending {
                // The replacement's draw suffix may produce the original
                // object's exact arrival receipt. Freeze it at its own finish,
                // before another continuation or any added program can move it.
                self.frozen.push(None);
                self.delayed.push((
                    index,
                    (object, receipt),
                    range.iter().map(|part| part.end).max().unwrap_or(0),
                ));
            } else {
                self.frozen.push(Some(super::freeze_zone_change_receipts(
                    game,
                    vec![(object, receipt)],
                )));
            }
        }
        for draw in &mut self.draws.0 {
            if let Some(completion) = &mut draw.completion {
                completion.freeze(game)?;
            }
        }
        Ok(())
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
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let original = crate::effects::composition::complete_retained_original_phase_with_outputs(
            game,
            ctx,
            SimultaneousEffectCommit {
                outcome: CompletedEffectOutputs::aggregate_only(original),
                completion: Some(self),
            },
        )?;
        crate::effects::composition::complete_committed_original_with_outputs(game, ctx, original)
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let original = crate::effects::composition::complete_retained_original_phase_with_outputs(
            game,
            ctx,
            SimultaneousEffectCommit {
                outcome: original,
                completion: Some(self),
            },
        )?;
        crate::effects::composition::complete_committed_original_with_outputs(game, ctx, original)
    }
}

impl ZoneInstructionCompletion {
    pub(crate) fn complete_original_with_outputs<O: crate::effects::OriginalEffectOutput>(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: O,
    ) -> Result<CompletedZoneOriginals, ExecutionError> {
        self.draws.require_committed_originals(
            "zone completion received an uncommitted replacement original",
        )?;
        let mut original = original.into_outputs();
        let iterated_player = self.iterated_player;
        ctx.with_temp_iterated_player(iterated_player, |ctx| {
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                ctx,
                None,
                original.outcome.events.iter_mut(),
            )?;
            // Resumed subtree receipts include their prefixes exactly once. The
            // original batch exposed those prefixes for event-time matching only.
            original.outcome.events.drain(..self.prefix_events);
            original.outcome.execution_facts.drain(..self.prefix_facts);
            let Some(completed) =
                crate::effects::composition::complete_retained_originals_with_outputs(
                    game,
                    ctx,
                    std::mem::take(&mut self.draws.0),
                    |game, _, index, _| {
                        let mut remaining = Vec::new();
                        for (slot, receipt, end) in std::mem::take(&mut self.delayed) {
                            if end == index + 1 {
                                self.frozen[slot] =
                                    Some(super::freeze_zone_change_receipts(game, vec![receipt]));
                            } else {
                                remaining.push((slot, receipt, end));
                            }
                        }
                        self.delayed = remaining;
                        Ok(())
                    },
                )?
            else {
                return Ok(CompletedZoneOriginals {
                    outputs: CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                    frozen: Vec::new(),
                    iterated_player,
                });
            };
            let outputs = retain_completed_zone_original_outputs(
                original,
                completed,
                std::mem::take(&mut self.draws.1),
            );
            Ok(CompletedZoneOriginals {
                outputs,
                frozen: self.frozen,
                iterated_player,
            })
        })
    }
}

fn retain_completed_zone_original_outputs<O: crate::effects::OriginalEffectOutput>(
    original: O,
    completed: Vec<CompletedEffectOutputs>,
    published: Vec<crate::effects::PublishedEffectOutputs>,
) -> CompletedEffectOutputs {
    let mut outputs = original.into_outputs();
    let aggregate = EffectOutcome::aggregate_replacement_outcomes(
        outputs.outcome.clone(),
        completed.iter().map(|child| child.outcome.clone()),
    );
    outputs = outputs.project_aggregate(aggregate);
    outputs.projections_complete = false;
    outputs.retain_batch_children(completed);
    outputs.retain_published_references(published);
    outputs
}

/// Completed originals retain their frozen additions for the enclosing owner.
/// Native cohorts may finish sibling actions before consuming this phase.
pub(crate) struct CompletedZoneOriginals {
    pub(crate) outputs: CompletedEffectOutputs,
    frozen: Vec<Option<super::FrozenZoneChangeReceipts>>,
    iterated_player: Option<crate::ids::PlayerId>,
}
/// The generic phase handoff retains bindings without executing additions.
struct ZoneAddedProgramsCompletion {
    frozen: Vec<Option<super::FrozenZoneChangeReceipts>>,
    iterated_player: Option<crate::ids::PlayerId>,
}
impl SimultaneousEffectCompletion for ZoneAddedProgramsCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Complete
    }
    fn freeze(&mut self, _game: &mut GameState) -> Result<(), ExecutionError> {
        Ok(())
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
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        CompletedZoneOriginals {
            outputs: CompletedEffectOutputs::aggregate_only(original),
            frozen: self.frozen,
            iterated_player: self.iterated_player,
        }
        .complete_added_programs_with_outputs(game, ctx)
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        CompletedZoneOriginals {
            outputs: original,
            frozen: self.frozen,
            iterated_player: self.iterated_player,
        }
        .complete_added_programs_with_outputs(game, ctx)
    }
}

impl CompletedZoneOriginals {
    pub(crate) fn complete_added_programs_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let iterated_player = self.iterated_player;
        let mut outputs = self.outputs;
        outputs.projections_complete = false;
        ctx.with_temp_iterated_player(iterated_player, |ctx| {
            for frozen in self.frozen {
                let frozen = frozen.ok_or_else(|| {
                    ExecutionError::InternalError(
                        "zone instruction completion has an unfinished original receipt".into(),
                    )
                })?;
                let completed = super::finish_zone_change_receipts_frozen_with_outputs(
                    game, ctx, outputs, frozen,
                )?;
                outputs = completed;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
            }
            Ok(outputs)
        })
    }
}
