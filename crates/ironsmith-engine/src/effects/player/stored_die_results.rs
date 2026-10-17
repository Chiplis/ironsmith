//! Die results stored on an object (Centaur of Attention):
//!
//! - "roll five six-sided dice and store those results on it"
//! - "you may reroll any number of this creature's stored results"
//!
//! Both roll through the shared die-roll transaction, so roll replacements,
//! modifiers and die-roll triggers see ordinary simultaneous rolls (CR 706).
//! The results live on the source object and are read by
//! `Value::GreatestEqualStoredDieResultsOnSource`.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

use super::die_roll_transaction::{DieRollCompletion, roll_dice_with_modifiers};

pub type RollAndStoreDieResultsEffect = ironsmith_core::RollAndStoreDieResultsEffect;
pub type RerollStoredDieResultsEffect = ironsmith_core::RerollStoredDieResultsEffect;

impl EffectExecutor for RollAndStoreDieResultsEffect {
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
        crate::effects::composition::execute_transaction_from_body(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player = resolve_player_filter(game, &self.player, ctx)?;
                if self.count == 0 || self.sides == 0 || game.object(ctx.source).is_none() {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                }
                let Some(transaction) =
                    roll_dice_with_modifiers(game, ctx, player, self.count, self.sides)?
                else {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                };
                let results: Vec<u32> = transaction.rolls.iter().map(|roll| roll.result).collect();
                let displayed = results.iter().copied().max().unwrap_or(0);
                game.set_stored_die_results(ctx.source, self.sides, results.clone());
                transaction.complete_with_outputs(
                    game,
                    ctx,
                    player,
                    self.sides,
                    displayed,
                    DieRollCompletion::Simultaneous,
                    EffectOutcome::count(results.len() as i64),
                )
            },
        )
    }
}

impl EffectExecutor for RerollStoredDieResultsEffect {
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
        crate::effects::composition::execute_transaction_from_body(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player = resolve_player_filter(game, &self.player, ctx)?;
                let Some((sides, stored)) = game
                    .stored_die_results(ctx.source)
                    .map(|(sides, results)| (sides, results.to_vec()))
                else {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                };
                if stored.is_empty() || sides == 0 {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                }
                // "any number": choose which stored results to reroll, from
                // none to all of them.
                let choice = crate::decisions::context::SelectOptionsContext::new(
                    player,
                    Some(ctx.source),
                    "Choose stored results to reroll",
                    stored
                        .iter()
                        .enumerate()
                        .map(|(index, result)| {
                            crate::decisions::context::SelectableOption::new(
                                index,
                                format!("Result {}: {result}", index + 1),
                            )
                        })
                        .collect(),
                    0,
                    stored.len(),
                );
                let mut chosen = ctx.decision_maker.decide_options(game, &choice);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                }
                chosen.sort_unstable();
                chosen.dedup();
                chosen.retain(|index| *index < stored.len());
                if chosen.is_empty() {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                }
                let count = u32::try_from(chosen.len()).unwrap_or(u32::MAX);
                let Some(transaction) = roll_dice_with_modifiers(game, ctx, player, count, sides)?
                else {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                };
                let mut results = stored;
                for (index, roll) in chosen.iter().zip(transaction.rolls.iter()) {
                    results[*index] = roll.result;
                }
                let displayed = transaction
                    .rolls
                    .iter()
                    .map(|roll| roll.result)
                    .max()
                    .unwrap_or(0);
                let rerolled = transaction.rolls.len();
                game.set_stored_die_results(ctx.source, sides, results);
                transaction.complete_with_outputs(
                    game,
                    ctx,
                    player,
                    sides,
                    displayed,
                    DieRollCompletion::Simultaneous,
                    EffectOutcome::count(rerolled as i64),
                )
            },
        )
    }
}
