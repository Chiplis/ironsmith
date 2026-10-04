use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, helpers::resolve_player_filter};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::other::DieRolledEvent;
use crate::game_state::GameState;
use crate::target::PlayerFilter;

use super::die_roll_transaction::roll_dice_with_modifiers;

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
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let player = resolve_player_filter(game, &self.player, ctx)?;
            if self.count == 0 || self.sides == 0 {
                return Ok(EffectOutcome::count(0));
            }

            let Some(rolls) = roll_dice_with_modifiers(game, ctx, player, self.count, self.sides)?
            else {
                return Ok(EffectOutcome::count(0));
            };

            let options = rolls
                .iter()
                .enumerate()
                .map(|(idx, roll)| SelectableOption::new(idx, roll.result.to_string()))
                .collect::<Vec<_>>();
            let choice_ctx = SelectOptionsContext::new(
                player,
                Some(ctx.source),
                "Choose one result",
                options,
                1,
                1,
            );
            let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let chosen_idx = selected
                .into_iter()
                .next()
                .filter(|idx| *idx < rolls.len())
                .unwrap_or(0);
            let chosen = rolls[chosen_idx];
            let other = rolls
                .iter()
                .enumerate()
                .find_map(|(idx, roll)| (idx != chosen_idx).then_some(roll.result))
                .unwrap_or(chosen.result);

            // CR 706.1 / 706.4: every die was rolled; only the chosen result is
            // used by the effect, but both count as rolls for history and for
            // "whenever you roll ..." triggers.
            let ordinal = game.turn_store.turn_history.record_completed_die_rolls(
                player,
                &rolls.iter().map(|roll| roll.result).collect::<Vec<_>>(),
                false,
            )?;
            // The recorded rolls must invalidate any continuous effects that
            // depend on die-roll history.
            game.mark_continuous_state_dirty();
            game.record_ui_effect_event(
                "die_roll",
                Some(player),
                None,
                Vec::new(),
                Some(i64::from(chosen.result)),
                Some(format!("d{}", self.sides)),
            );
            let batch = game
                .alloc_child_event_provenance(ctx.provenance, crate::events::EventKind::DieRolled);
            // Each die is its own event (distinct provenance, so turn history
            // keeps both), sharing one simultaneous batch.
            let events = rolls
                .iter()
                .enumerate()
                .map(|(index, roll)| {
                    let provenance = game.alloc_child_event_provenance(
                        ctx.provenance,
                        crate::events::EventKind::DieRolled,
                    );
                    crate::triggers::TriggerEvent::new_with_provenance(
                        DieRolledEvent::new_with_natural_result(
                            player,
                            ctx.source,
                            roll.natural_result,
                            roll.result,
                            self.sides,
                        )
                        .with_turn_ordinal(ordinal + index as u32),
                        provenance,
                    )
                    .with_simultaneous_batch(batch)
                })
                .collect::<Vec<_>>();

            Ok(EffectOutcome::count(i64::from(chosen.result))
                .with_events(events)
                .with_execution_fact(ExecutionFact::ChosenNumber(chosen.result))
                .with_execution_fact(ExecutionFact::OtherNumber(other)))
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            game.restore_execution_checkpoint(checkpoint, pending && result.is_ok());
            context_checkpoint.restore(ctx);
        }
        if pending && result.is_ok() {
            return Ok(EffectOutcome::count(0));
        }
        result
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
