use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase};

#[derive(Debug, Clone, PartialEq)]
pub struct EndTurnEffect {
    pub player: crate::target::PlayerFilter,
}

impl EndTurnEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for EndTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        execute_ending_procedure_with_outputs(game, ctx, EndingProcedure::Turn(&self.player))
    }
}

pub(super) enum EndingProcedure<'a> {
    Turn(&'a crate::target::PlayerFilter),
    CombatPhase,
}

/// Both ending instructions share prior-trigger discard, stack exile and the
/// no-priority scheduler handoff. Admission and the selected runner procedure
/// remain distinct. Triggers created by exile stay staged for the runner.
pub(super) fn execute_ending_procedure_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    procedure: EndingProcedure<'_>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_result_checkpoint_transaction(game, ctx, |game, ctx| {
        match &procedure {
            EndingProcedure::Turn(filter) => {
                let player = resolve_player_filter(game, filter, ctx)?;
                if !game.is_active_player(player) {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }
            }
            EndingProcedure::CombatPhase if game.turn.phase != Phase::Combat => {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::resolved(),
                ));
            }
            EndingProcedure::CombatPhase => {}
        }

        let _ = game.take_pending_trigger_events();
        let _ = game.take_pending_trigger_entries();
        let stack_exile = exile_stack_for_ending_procedure_with_outputs(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }

        // The runner owns the following SBA/cleanup or following-phase steps.
        // Do not mark the ending instruction finished before its exile programs.
        match procedure {
            EndingProcedure::Turn(_) => game.turn_store.end_turn_procedure_pending = true,
            EndingProcedure::CombatPhase => {
                game.turn_store.end_combat_phase_procedure_pending = true
            }
        }
        game.turn.priority_player = None;
        Ok(CompletedEffectOutputs::from_children(
            [stack_exile],
            |children| {
                EffectOutcome::aggregate_with_primary_result(EffectOutcome::resolved(), children)
            },
        ))
    })
}

/// Exile every concrete stack object, including the resolving object. Ordinary
/// ability sources elsewhere have no zone object to move. The shared zone
/// cohort owns preparation, originals, draw continuations and added programs.
fn exile_stack_for_ending_procedure_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_result_checkpoint_transaction(game, ctx, |game, ctx| {
        use crate::zone::Zone;
        let mut stack_objects = Vec::with_capacity(game.stack.len() + 1);
        if let Some(resolving_object) = ctx.cause.source {
            stack_objects.push(resolving_object);
        }
        stack_objects.extend(game.stack.iter().rev().map(|entry| entry.object_id));
        game.stack.clear();
        let mut seen = std::collections::HashSet::new();
        stack_objects.retain(|object_id| seen.insert(*object_id));

        let snapshots = stack_objects
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .filter(|object| object.zone == Zone::Stack)?;
                crate::snapshot::ObjectSnapshot::from_object_id(game, *id)
                    .map(|snapshot| (*id, snapshot))
            })
            .collect::<std::collections::HashMap<_, _>>();
        let cause = crate::events::cause::EventCause::from_game_rule();
        let requests = stack_objects
            .into_iter()
            .filter_map(|id| {
                snapshots.get(&id).cloned().map(|snapshot| {
                    crate::effects::zones::PreparedZoneMove::capture(
                        game,
                        id,
                        Zone::Stack,
                        Zone::Exile,
                        cause.clone(),
                        Some(snapshot),
                    )
                })
            })
            .collect();
        let pending_start = game.effect_store.pending_trigger_events.len();
        crate::effects::zones::execute_zone_moves_with_outputs(
            game,
            ctx,
            requests,
            |game, ctx, receipts| {
                let originals = crate::effects::zones::observe_zone_move_originals_with_cause(
                    game,
                    ctx,
                    receipts,
                    &snapshots,
                    pending_start,
                    cause,
                )?;
                let arrivals = originals
                    .iter()
                    .flat_map(|(_, change, _)| change.new_object_ids.iter().copied())
                    .collect();
                let memories = originals
                    .into_iter()
                    .map(|(_, _, snapshot)| snapshot)
                    .collect();
                Ok(EffectOutcome::resolved()
                    .with_affected_objects(arrivals)
                    .with_affected_object_memory(memories))
            },
        )
    })
}
