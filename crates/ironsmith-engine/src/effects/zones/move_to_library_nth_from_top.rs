//! Move an object to the Nth position from the top of its owner's library.

use crate::effects::CompletedEffectOutputs;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub type MoveToLibraryNthFromTopEffect = ironsmith_core::MoveToLibraryNthFromTopEffect;

impl EffectExecutor for MoveToLibraryNthFromTopEffect {
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
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if object_ids.is_empty() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }

                let raw_position = resolve_value(game, &self.position, ctx)?;
                let position = raw_position.max(1) as usize;

                let snapshots = object_ids
                    .iter()
                    .filter_map(|id| {
                        ObjectSnapshot::from_object_id(game, *id).map(|snapshot| (*id, snapshot))
                    })
                    .collect::<std::collections::HashMap<_, _>>();
                let moves = object_ids
                    .into_iter()
                    .filter_map(|id| {
                        snapshots.get(&id).map(|snapshot| {
                            super::PreparedZoneMove::capture(
                                game,
                                id,
                                snapshot.zone,
                                Zone::Library,
                                ctx.cause.clone(),
                                Some(snapshot.clone()),
                            )
                        })
                    })
                    .collect();
                super::execute_zone_moves_with_outputs(game, ctx, moves, |game, ctx, receipts| {
                    let placements = snapshots
                        .iter()
                        .map(|(id, snapshot)| {
                            (
                                *id,
                                (
                                    crate::effects::cards::LibraryCardPosition::NthFromTop(
                                        position,
                                    ),
                                    snapshot.clone(),
                                ),
                            )
                        })
                        .collect();
                    super::positioned_move::complete_positioned_moves(
                        game,
                        ctx,
                        receipts,
                        &placements,
                        super::positioned_move::SourceCompletion::SnapshotOnly,
                    )
                })
            },
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to move into library at a fixed top position"
    }
}
