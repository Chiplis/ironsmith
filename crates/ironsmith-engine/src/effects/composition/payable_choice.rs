//! Choice among actions that can currently be paid. Payment execution remains
//! with the selected effect; this is not a replacement for an atomic cost owner.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

pub(crate) fn payable_action_indices(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    actions: &[Effect],
) -> Vec<usize> {
    actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            action
                .0
                .can_execute_as_cost(game, source, controller)
                .is_ok()
                .then_some(index)
        })
        .collect()
}

/// Rebuild the eligible menu on every execution/repetition. A suspended choice
/// never falls back to a payment. Preserve the existing first-eligible fallback
/// for a completed choice with no valid answer.
pub(crate) fn execute_payable_choice(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    actions: &[Effect],
    label: impl Fn(usize) -> String,
    unavailable: &str,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    super::execute_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let options: Vec<_> = payable_action_indices(game, ctx.source, ctx.controller, actions)
                .into_iter()
                .map(|index| (label(index), index))
                .collect();
            if options.is_empty() {
                return Err(ExecutionError::Impossible(unavailable.into()));
            }
            let choice = crate::decisions::ask_choose_one(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                ctx.source,
                &options,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            crate::effects::execute_effect_with_outputs(
                game,
                &actions[choice.unwrap_or(options[0].1)],
                ctx,
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Choice {
        pending: bool,
        pause: bool,
        labels: Vec<String>,
    }
    impl crate::decision::DecisionMaker for Choice {
        fn decide_options(
            &mut self,
            _: &GameState,
            menu: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.labels = menu
                .options
                .iter()
                .map(|option| option.description.clone())
                .collect();
            self.pending = self.pause;
            vec![0]
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    #[test]
    fn payable_menu_filters_and_rechecks_resources_after_each_payment() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId(0);
        let source = game.new_object_id();
        let actions = [Effect::pay_life(30), Effect::pay_life(12)];
        let mut ctx = ExecutionContext::new_default(source, player);
        assert_eq!(
            payable_action_indices(&game, source, player, &actions),
            vec![1]
        );
        execute_payable_choice(
            &mut game,
            &mut ctx,
            &actions,
            |i| format!("Payment {i}"),
            "unavailable",
        )
        .unwrap();
        assert_eq!(game.player(player).unwrap().life, 8);
        assert!(matches!(
            execute_payable_choice(
                &mut game,
                &mut ctx,
                &actions,
                |i| format!("Payment {i}"),
                "unavailable"
            ),
            Err(ExecutionError::Impossible(_))
        ));
        assert_eq!(game.player(player).unwrap().life, 8);
    }

    #[test]
    fn suspended_payable_choice_does_not_fall_back_and_replay_pays_once() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId(0);
        let source = game.new_object_id();
        let actions = [Effect::pay_life(30), Effect::pay_life(3)];
        let mut dm = Choice {
            pending: false,
            pause: true,
            labels: vec![],
        };
        {
            let mut ctx = ExecutionContext::new(source, player, &mut dm);
            let result = execute_payable_choice(
                &mut game,
                &mut ctx,
                &actions,
                |i| format!("Payment {i}"),
                "unavailable",
            )
            .unwrap();
            assert_eq!(result.outcome.value, crate::effect::OutcomeValue::Count(0));
        }
        assert_eq!(game.player(player).unwrap().life, 20);
        assert_eq!(dm.labels, vec!["Payment 1"]);
        dm.pause = false;
        dm.pending = false;
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        execute_payable_choice(
            &mut game,
            &mut ctx,
            &actions,
            |i| format!("Payment {i}"),
            "unavailable",
        )
        .unwrap();
        assert_eq!(game.player(player).unwrap().life, 17);
    }
}
