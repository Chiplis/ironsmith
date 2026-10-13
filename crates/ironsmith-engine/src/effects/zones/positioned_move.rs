//! Completion of positioned library movements, before replacement additions.
use super::maybe_prompt_for_split_result_order;
use crate::events::processing::{EventOutcome, PreparedEventOutcome};
use crate::{
    effect::EffectOutcome,
    effects::{ExecutionContext, ExecutionError},
    game_state::GameState,
    ids::ObjectId,
    snapshot::ObjectSnapshot,
    zone::Zone,
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceCompletion {
    Unchanged,
    SnapshotOnly,
    Rebind,
}
pub(super) fn complete_positioned_moves(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipts: &[(ObjectId, PreparedEventOutcome<super::AppliedZoneChange>)],
    placements: &std::collections::HashMap<
        ObjectId,
        (crate::effects::cards::LibraryCardPosition, ObjectSnapshot),
    >,
    source_policy: SourceCompletion,
) -> Result<EffectOutcome, ExecutionError> {
    let mut moved_ids = Vec::new();
    let mut affected_ids = Vec::new();
    let mut any_replaced = false;
    let mut any_prevented = false;
    let mut moved_source_lki = None;
    for (object_id, receipt) in receipts {
        let object_id = *object_id;
        let (position, pre_snapshot) = &placements[&object_id];
        let source_lki_before_move = (source_policy != SourceCompletion::Unchanged
            && pre_snapshot.object_id == ctx.source)
            .then(|| pre_snapshot.clone());
        let from_zone = pre_snapshot.zone;
        let original = receipt.original.clone();
        match original {
            EventOutcome::Prevented => {
                any_prevented = true;
            }
            EventOutcome::Proceed(mut result) => {
                if result.new_object_ids.is_empty() {
                    continue;
                }
                ctx.refresh_target_snapshot(pre_snapshot.clone());
                if let Some(snapshot) = source_lki_before_move.clone() {
                    if source_policy == SourceCompletion::Rebind {
                        moved_source_lki = Some(snapshot);
                    } else {
                        ctx.refresh_source_snapshot(snapshot);
                    }
                }

                if result.final_zone == Zone::Exile {
                    for &new_id in &result.new_object_ids {
                        game.add_exiled_with_source_link(ctx.source, new_id);
                    }
                } else if result.final_zone == Zone::Library {
                    for &id in &result.new_object_ids {
                        if let Some(owner) = game.object(id).map(|object| object.owner) {
                            crate::effects::cards::position_library_card(
                                game,
                                owner,
                                id,
                                *position,
                                "card positioned in library",
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
                        game.record_zone_change_results(object_id, result.new_object_ids.clone());
                    }
                }

                affected_ids.extend(result.new_object_ids.iter().copied());
                moved_ids.extend(result.new_object_ids.iter().copied());
            }
            EventOutcome::Replaced => {
                any_replaced = true;
                if let Some(result) = super::take_recorded_zone_change(game, object_id) {
                    affected_ids.extend(result.new_object_ids);
                }
            }
            EventOutcome::NotApplicable => {}
        }
    }

    if source_policy == SourceCompletion::Rebind
        && let Some(new_source_id) = moved_ids.first().copied()
    {
        ctx.source = new_source_id;
    }
    if let Some(snapshot) = moved_source_lki {
        ctx.refresh_source_snapshot(snapshot);
    }

    if !moved_ids.is_empty() {
        return Ok(EffectOutcome::with_objects(moved_ids).with_affected_objects(affected_ids));
    }
    if any_replaced {
        return Ok(EffectOutcome::replaced().with_affected_objects(affected_ids));
    }
    if any_prevented {
        return Ok(EffectOutcome::prevented());
    }
    Ok(EffectOutcome::target_invalid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectExecutor;
    use crate::{
        card::CardBuilder,
        ids::{CardId, PlayerId},
        target::ChooseSpec,
    };
    #[test]
    fn both_positioned_moves_keep_their_source_rebinding_contract() {
        for mode in 0..3 {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let player = PlayerId(0);
            let card = CardBuilder::new(CardId::new(), "Moving source").build();
            let source = game.create_object_from_card(&card, player, Zone::Hand);
            let mut ctx = ExecutionContext::new_default(source, player);
            let output = if mode == 0 {
                super::super::MoveToLibraryNthFromTopEffect::new(ChooseSpec::Source, 1.into())
                    .execute_with_outputs(&mut game, &mut ctx)
                    .unwrap()
            } else {
                super::super::MoveToLibraryTopOrBottomChoiceEffect::new(if mode == 1 {
                    ChooseSpec::Source
                } else {
                    ChooseSpec::SpecificObject(source)
                })
                .execute_with_outputs(&mut game, &mut ctx)
                .unwrap()
            };
            let moved = output.outcome.objects().unwrap()[0];
            assert_eq!(game.object(moved).unwrap().zone, Zone::Library);
            assert_eq!(ctx.source, if mode == 1 { moved } else { source });
            assert_eq!(
                ctx.source_snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.object_id),
                if mode == 2 { None } else { Some(source) }
            );
        }
    }
}
