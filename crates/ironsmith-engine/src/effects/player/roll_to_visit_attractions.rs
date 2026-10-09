//! Roll to visit your Attractions (CR 701.52): one owner for the turn-based
//! action (CR 505.5b) and the instruction ("When this creature enters, roll
//! to visit your Attractions.").

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::PlayerId;

use super::die_roll_transaction::{DieRollCompletion, roll_dice_with_modifiers};

pub type RollToVisitAttractionsEffect = ironsmith_core::RollToVisitAttractionsEffect;

/// The completed visit roll: the six-sided die result, the die-roll
/// completion outputs (die history and "whenever you roll to visit" events),
/// and one visit per Attraction of the player lit for that result.
pub(crate) struct AttractionVisitRoll {
    pub(crate) result: u32,
    pub(crate) roll_outputs: CompletedEffectOutputs,
    pub(crate) visits: Vec<KeywordActionEvent>,
}

/// CR 701.52a: roll a six-sided die, then name every Attraction `player`
/// controls whose lit-up numbers include the result. The visits are returned
/// as one simultaneous batch for the caller to publish. `None` means the roll
/// did not complete (a pending choice or a replaced roll).
pub(crate) fn roll_to_visit_attractions_for_player(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
) -> Result<Option<AttractionVisitRoll>, ExecutionError> {
    game.turn_store
        .turn_history
        .check_completed_die_roll_capacity(player, 1)?;
    let Some(transaction) = roll_dice_with_modifiers(game, ctx, player, 1, 6)? else {
        return Ok(None);
    };
    let roll = transaction.rolls[0];
    let roll_outputs = transaction.complete_with_outputs(
        game,
        ctx,
        player,
        6,
        roll.result,
        DieRollCompletion::AttractionVisit,
        EffectOutcome::resolved(),
    )?;
    let visits = game
        .attraction_visit_profiles(player, roll.result)
        .iter()
        .map(|visit| {
            KeywordActionEvent::new(KeywordActionKind::VisitAttraction, player, visit.object, 1)
        })
        .collect();
    Ok(Some(AttractionVisitRoll {
        result: roll.result,
        roll_outputs,
        visits,
    }))
}

impl EffectExecutor for RollToVisitAttractionsEffect {
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
                let Some(roll) = roll_to_visit_attractions_for_player(game, ctx, player)? else {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                }
                // CR 701.52a: every lit Attraction is visited; each visit is a
                // keyword action its own "Visit" triggered ability observes.
                let mut children = vec![roll.roll_outputs];
                for visit in roll.visits {
                    children.push(crate::effects::composition::complete_keyword_action_with_outputs(
                        game,
                        ctx,
                        CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved()),
                        visit,
                    )?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(
                            0,
                        )));
                    }
                }
                let primary = EffectOutcome::count(i64::from(roll.result));
                Ok(CompletedEffectOutputs::from_children(children, |outcomes| {
                    EffectOutcome::aggregate_with_primary_result(primary, outcomes)
                }))
            },
        )
    }
}
