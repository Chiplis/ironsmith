use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, helpers::resolve_player_filter};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

use super::die_roll_transaction::roll_dice_with_modifiers;

#[derive(Debug, Clone, PartialEq)]
pub struct RollDiceChooseResultEffect {
    pub player: PlayerFilter,
    pub count: u32,
    pub sides: u32,
    pub die_text: Option<String>,
    /// "... and ignore the lower roll": the result is the highest roll,
    /// with no choice (Berserker's Frenzy).
    pub ignore_lower: bool,
}

impl RollDiceChooseResultEffect {
    pub fn new(player: PlayerFilter, count: u32, sides: u32) -> Self {
        Self {
            player,
            count,
            sides,
            die_text: None,
            ignore_lower: false,
        }
    }

    pub fn new_with_die_text(
        player: PlayerFilter,
        count: u32,
        sides: u32,
        die_text: Option<String>,
    ) -> Self {
        Self {
            player,
            count,
            sides,
            die_text,
            ignore_lower: false,
        }
    }

    /// Keep the highest roll instead of choosing one.
    pub fn with_ignore_lower(mut self, ignore_lower: bool) -> Self {
        self.ignore_lower = ignore_lower;
        self
    }
}

impl EffectExecutor for RollDiceChooseResultEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction_from_body(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player = resolve_player_filter(game, &self.player, ctx)?;
                if self.count == 0 || self.sides == 0 {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let Some(transaction) =
                    roll_dice_with_modifiers(game, ctx, player, self.count, self.sides)?
                else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };

                let rolls = &transaction.rolls;
                let options = rolls
                    .iter()
                    .enumerate()
                    .map(|(idx, roll)| SelectableOption::new(idx, roll.result.to_string()))
                    .collect::<Vec<_>>();
                // "Roll two d20 and ignore the lower roll": the higher roll is
                // the result; nothing is chosen (CR 706.1).
                let highest_idx = rolls
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, roll)| roll.result)
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);
                let choice_ctx = SelectOptionsContext::new(
                    player,
                    Some(ctx.source),
                    "Choose one result",
                    options,
                    1,
                    1,
                );
                let chosen_idx = if self.ignore_lower {
                    highest_idx
                } else {
                    let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    selected
                        .into_iter()
                        .next()
                        .filter(|idx| *idx < rolls.len())
                        .unwrap_or(0)
                };
                let chosen = rolls[chosen_idx];
                let other = rolls
                    .iter()
                    .enumerate()
                    .find_map(|(idx, roll)| (idx != chosen_idx).then_some(roll.result))
                    .unwrap_or(chosen.result);

                // Every retained die completes even though only one result is used.
                transaction.complete_with_outputs(
                    game,
                    ctx,
                    player,
                    self.sides,
                    chosen.result,
                    super::die_roll_transaction::DieRollCompletion::Simultaneous,
                    EffectOutcome::count(i64::from(chosen.result))
                        .with_execution_fact(ExecutionFact::ChosenNumber(chosen.result))
                        .with_execution_fact(ExecutionFact::OtherNumber(other)),
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::execute_effect;
    use crate::ids::PlayerId;

    struct ChooseSecondResult;

    impl crate::decision::DecisionMaker for ChooseSecondResult {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            vec![1]
        }
    }

    #[test]
    fn roll_dice_choose_result_records_chosen_and_other_numbers() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.force_next_die_roll(5);
        game.force_next_die_roll(2);

        let mut decisions = ChooseSecondResult;
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        let outcome = execute_effect(
            &mut game,
            &crate::effect::Effect::roll_dice_choose_result_with_die_text(
                2,
                6,
                PlayerFilter::You,
                Some("d6".to_string()),
            ),
            &mut ctx,
        )
        .expect("roll-and-choose effect should resolve");

        assert_eq!(outcome.as_count(), Some(2));
        assert!(
            outcome
                .execution_facts
                .contains(&ExecutionFact::ChosenNumber(2))
        );
        assert!(
            outcome
                .execution_facts
                .contains(&ExecutionFact::OtherNumber(5))
        );
    }
}
