//! One simultaneous return instruction, with a cause for each ended duration.
use super::{BattlefieldEntryOptions, PreparedZoneMove};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::events::cause::EventCause;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::zone::Zone;

pub(crate) fn execute_duration_returns(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    requests: Vec<(ObjectId, Zone, EventCause)>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(crate::effect::EffectOutcome::count(0)),
        |game, ctx| {
            let order = requests.iter().map(|(id, _, _)| *id).collect::<Vec<_>>();
            let opened = requests.len() > 1 && game.open_simultaneous_action();
            let result = (|| {
                let mut battlefield = Vec::new();
                let mut other = Vec::new();
                for (object, zone, cause) in requests {
                    if zone == Zone::Battlefield {
                        let mut options = BattlefieldEntryOptions::owner(false);
                        options.cause_override = Some(cause);
                        battlefield.push((object, options));
                    } else {
                        other.push(PreparedZoneMove::capture(
                            game,
                            object,
                            Zone::Exile,
                            zone,
                            cause,
                            None,
                        ));
                    }
                }
                let (proposals, mut draws) = super::prepare_zone_moves(game, ctx, other)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                super::battlefield_entry::move_to_battlefield_batch_with_companions(
                    game,
                    ctx,
                    battlefield,
                    Default::default(),
                    |game, ctx| {
                        let receipts = super::prepared_move::commit_prepared_zone_moves(
                            game, ctx, proposals, &mut draws,
                        )?;
                        Ok((receipts, draws))
                    },
                )
            })();
            game.close_simultaneous_action(opened);
            let Some((entries, (mut receipts, mut draws))) = result? else {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    crate::effect::EffectOutcome::count(0),
                ));
            };
            for entry in entries {
                receipts.push(draws.retain_entry_receipt(entry));
            }
            receipts.sort_by_key(|(id, _)| order.iter().position(|original| original == id));
            let committed = draws.finish(crate::effect::EffectOutcome::count(0), receipts, ctx);
            super::continuation::complete_zone_instruction_with_outputs(game, ctx, committed)
        },
    )
}
