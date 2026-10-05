//! Move an object to the Nth position from the top of its owner's library.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::{
    apply_zone_change_with_context_and_additional_effects, maybe_prompt_for_split_result_order,
    take_recorded_zone_change,
};
pub type MoveToLibraryNthFromTopEffect = ironsmith_core::MoveToLibraryNthFromTopEffect;

impl EffectExecutor for MoveToLibraryNthFromTopEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
            let object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if object_ids.is_empty() {
                return Ok(EffectOutcome::target_invalid());
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
            super::execute_zone_moves(game, ctx, moves, |game, ctx, receipts| {
                let mut moved_ids = Vec::new();
                let mut affected_ids = Vec::new();
                let mut any_replaced = false;
                let mut any_prevented = false;
                for (object_id, receipt) in receipts {
                    let object_id = *object_id;
                    let pre_snapshot = snapshots[&object_id].clone();
                    let from_zone = pre_snapshot.zone;
                    let original = receipt.original.clone();
                    match original {
                        EventOutcome::Prevented => {
                            any_prevented = true;
                        }
                        EventOutcome::Proceed(mut result) => {
                            if !result.new_object_ids.is_empty() {
                                ctx.refresh_target_snapshot(pre_snapshot.clone());
                                if pre_snapshot.object_id == ctx.source {
                                    ctx.refresh_source_snapshot(pre_snapshot.clone());
                                }
                                if result.final_zone == Zone::Exile {
                                    for &new_id in &result.new_object_ids {
                                        game.add_exiled_with_source_link(ctx.source, new_id);
                                    }
                                } else if result.final_zone == Zone::Library {
                                    for &new_id in &result.new_object_ids {
                                        if let Some(owner) = game.object(new_id).map(|o| o.owner) {
                                            game.move_library_card_to_nth_from_top(
                                                owner,
                                                new_id,
                                                position,
                                                "card put into library at fixed top position",
                                            );
                                        }
                                    }
                                    if from_zone == Zone::Battlefield {
                                        maybe_prompt_for_split_result_order(
                                            game,
                                            &mut ctx.decision_maker,
                                            result.final_zone,
                                            &ctx.cause,
                                            &mut result,
                                        );
                                        if ctx.decision_maker.awaiting_choice() {
                                            return Ok(EffectOutcome::count(0));
                                        }
                                        game.record_zone_change_results(
                                            object_id,
                                            result.new_object_ids.clone(),
                                        );
                                    }
                                }
                                affected_ids.extend(result.new_object_ids.iter().copied());
                                moved_ids.extend(result.new_object_ids.iter().copied());
                            }
                        }
                        EventOutcome::Replaced => {
                            any_replaced = true;
                            if let Some(result) = take_recorded_zone_change(game, object_id) {
                                affected_ids.extend(result.new_object_ids);
                            }
                        }
                        EventOutcome::NotApplicable => {}
                    }
                }

                if !moved_ids.is_empty() {
                    return Ok(
                        EffectOutcome::with_objects(moved_ids).with_affected_objects(affected_ids)
                    );
                }
                if any_replaced {
                    return Ok(EffectOutcome::replaced().with_affected_objects(affected_ids));
                }
                if any_prevented {
                    return Ok(EffectOutcome::prevented());
                }
                Ok(EffectOutcome::target_invalid())
            })
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
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
