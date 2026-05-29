use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, helpers::resolve_player_filter};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

#[derive(Debug, Clone, PartialEq)]
pub struct RollDiceChooseResultEffect {
    pub player: PlayerFilter,
    pub count: u32,
    pub sides: u32,
    pub die_text: Option<String>,
}

impl RollDiceChooseResultEffect {
    pub fn new(player: PlayerFilter, count: u32, sides: u32) -> Self {
        Self {
            player,
            count,
            sides,
            die_text: None,
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
        }
    }
}

impl EffectExecutor for RollDiceChooseResultEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if self.count == 0 || self.sides == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let mut results = Vec::with_capacity(self.count as usize);
        for _ in 0..self.count {
            let result = if let Some(forced) = game.take_forced_die_roll() {
                forced.clamp(1, self.sides)
            } else {
                let mut faces: Vec<u32> = (1..=self.sides).collect();
                game.shuffle_slice(&mut faces);
                faces[0]
            };
            game.turn_store.turn_history.record_die_roll(player, result);
            results.push(result);
        }

        let options = results
            .iter()
            .enumerate()
            .map(|(idx, result)| SelectableOption::new(idx, result.to_string()))
            .collect::<Vec<_>>();
        let choice_ctx =
            SelectOptionsContext::new(player, Some(ctx.source), "Choose one result", options, 1, 1);
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let chosen_idx = selected
            .into_iter()
            .next()
            .filter(|idx| *idx < results.len())
            .unwrap_or(0);
        let chosen = results[chosen_idx];
        let other = results
            .iter()
            .enumerate()
            .find_map(|(idx, result)| (idx != chosen_idx).then_some(*result))
            .unwrap_or(chosen);

        Ok(EffectOutcome::count(chosen as i32)
            .with_execution_fact(ExecutionFact::ChosenNumber(chosen))
            .with_execution_fact(ExecutionFact::OtherNumber(other)))
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
