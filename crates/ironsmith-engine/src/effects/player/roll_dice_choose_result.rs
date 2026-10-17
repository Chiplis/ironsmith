use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::composition::scoped_choice::{SingleChoicePolicy, select_typed_option};
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
    /// The result is the total of every retained roll (no choice).
    pub total: bool,
    /// A dynamic number of dice ("roll X d6"); replaces `count` when present.
    pub count_value: Option<crate::effect::Value>,
    /// The first roll is the result and the second the other result.
    pub in_order: bool,
}

impl RollDiceChooseResultEffect {
    pub fn new(player: PlayerFilter, count: u32, sides: u32) -> Self {
        Self {
            player,
            count,
            sides,
            die_text: None,
            ignore_lower: false,
            total: false,
            count_value: None,
            in_order: false,
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
            total: false,
            count_value: None,
            in_order: false,
        }
    }

    /// Keep the highest roll instead of choosing one.
    pub fn with_ignore_lower(mut self, ignore_lower: bool) -> Self {
        self.ignore_lower = ignore_lower;
        self
    }

    /// Sum every roll instead of choosing one ("the total of those results").
    pub fn with_total(mut self, count_value: Option<crate::effect::Value>) -> Self {
        self.total = true;
        self.count_value = count_value;
        self
    }

    /// The first roll is the result and the second the other result.
    pub fn with_in_order(mut self, in_order: bool) -> Self {
        self.in_order = in_order;
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
                let count = match &self.count_value {
                    Some(value) => {
                        crate::effects::helpers::resolve_nonnegative_u32(game, value, ctx)?
                    }
                    None => self.count,
                };
                if count == 0 || self.sides == 0 {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let Some(transaction) =
                    roll_dice_with_modifiers(game, ctx, player, count, self.sides)?
                else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };

                if self.total {
                    // "the total of those results" (CR 706.2): every retained
                    // roll contributes; nothing is chosen.
                    let total = transaction
                        .rolls
                        .iter()
                        .map(|roll| u64::from(roll.result))
                        .sum::<u64>();
                    let total = u32::try_from(total).map_err(|_| {
                        ExecutionError::ResourceLimitExceeded {
                            resource: "die-roll total",
                            requested: u128::from(total),
                            maximum: u128::from(u32::MAX),
                        }
                    })?;
                    return transaction.complete_with_outputs(
                        game,
                        ctx,
                        player,
                        self.sides,
                        total,
                        super::die_roll_transaction::DieRollCompletion::Simultaneous,
                        EffectOutcome::count(i64::from(total))
                            .with_execution_fact(ExecutionFact::ChosenNumber(total)),
                    );
                }

                let rolls = &transaction.rolls;
                let options = rolls
                    .iter()
                    .enumerate()
                    .map(|(idx, roll)| (idx, roll.result.to_string()))
                    .collect::<Vec<_>>();
                // "Roll two d20 and ignore the lower roll": the higher roll is
                // the result; nothing is chosen (CR 706.1).
                let highest_idx = rolls
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, roll)| roll.result)
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);
                let chosen_idx = if self.in_order {
                    0
                } else if self.ignore_lower {
                    highest_idx
                } else {
                    let Some(index) = select_typed_option(
                        game,
                        ctx,
                        player,
                        "Choose one result",
                        &options,
                        SingleChoicePolicy::FirstOrDefault,
                    )?
                    else {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    };
                    index
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
