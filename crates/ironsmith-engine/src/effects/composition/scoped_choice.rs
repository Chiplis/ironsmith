//! Transient choices passed directly to their continuation, never stored on a card.
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;

/// Invalid answers either reject the instruction or produce no selection.
/// Neither policy falls through to a later index in a malformed answer.
#[derive(Clone, Copy)]
pub(crate) enum SingleChoicePolicy {
    Strict,
    First,
}

pub(crate) fn select_typed_option<T: Clone>(
    game: &GameState,
    ctx: &mut ExecutionContext,
    chooser: PlayerId,
    prompt: &str,
    options: &[(T, String)],
    policy: SingleChoicePolicy,
) -> Result<Option<T>, ExecutionError> {
    let menu = SelectOptionsContext::new(
        chooser,
        Some(ctx.source),
        prompt,
        options
            .iter()
            .enumerate()
            .map(|(i, (_, label))| SelectableOption::new(i, label.clone()))
            .collect(),
        1,
        1,
    );
    let selected = ctx.decision_maker.decide_options(game, &menu);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    if matches!(policy, SingleChoicePolicy::Strict)
        && (selected.len() != 1 || selected[0] >= options.len())
    {
        return Err(ExecutionError::InternalError(
            "invalid single-option selection".into(),
        ));
    }
    Ok(selected
        .first()
        .and_then(|index| options.get(*index))
        .map(|(value, _)| value.clone()))
}

/// A local typed choice. The continuation constructs ordinary effects from the
/// selected values. Replaying a suspended child replays the whole choice; no
/// persistent chosen-color/type state or global result slot is installed.
pub(crate) fn execute_choice_program<T: Clone>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    chooser: PlayerId,
    prompt: &str,
    options: &[(T, String)],
    maximum: usize,
    strict_single: bool,
    body: impl FnOnce(Vec<T>) -> Result<Option<Effect>, ExecutionError>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    super::execute_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            if strict_single && maximum == 1 {
                let Some(value) = select_typed_option(game, ctx, chooser, prompt, options, SingleChoicePolicy::Strict)? else {
                    return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                };
                return match body(vec![value])? {
                    Some(effect) => effect.0.execute_child_with_outputs(game, ctx),
                    None => Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))),
                };
            }
            let choice = SelectOptionsContext::new(
                chooser,
                Some(ctx.source),
                prompt,
                options
                    .iter()
                    .enumerate()
                    .map(|(i, (_, label))| SelectableOption::new(i, label.clone()))
                    .collect(),
                1,
                maximum,
            );
            let selected = ctx.decision_maker.decide_options(game, &choice);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            if strict_single && (selected.len() != 1 || selected[0] >= options.len()) {
                return Err(ExecutionError::InternalError(
                    "invalid single-option selection".into(),
                ));
            }
            let values = selected
                .into_iter()
                .take(maximum)
                .filter_map(|i| options.get(i).map(|(value, _)| value.clone()))
                .collect();
            match body(values)? {
                Some(effect) => effect.0.execute_child_with_outputs(game, ctx),
                None => Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                )),
            }
        },
    )
}

/// Select an affordable amount. None means unavailable or suspended, never a
/// successful payment of zero. Resource owners decide how to pay the result.
pub(crate) fn choose_bounded_payment_amount(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    minimum: u32,
    available: u32,
    prompt: &str,
) -> Option<u32> {
    if available < minimum || ctx.decision_maker.awaiting_choice() {
        return None;
    }
    let spec = if minimum == 0 {
        crate::decisions::NumberSpec::up_to(ctx.source, available, prompt)
    } else {
        crate::decisions::NumberSpec::range(ctx.source, minimum, available, prompt)
    };
    let chosen = crate::decisions::make_decision_with_fallback(
        game,
        &mut ctx.decision_maker,
        player,
        Some(ctx.source),
        spec,
        crate::decision::FallbackStrategy::Maximum,
    );
    (!ctx.decision_maker.awaiting_choice()).then(|| chosen.clamp(minimum, available))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Answers {
        values: Vec<usize>,
        pending: bool,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> {
            self.values.clone()
        }
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.pending = true;
            false
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    #[test]
    fn choice_continuation_suspension_rolls_back_its_completed_prefix() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId(0);
        let source = game.new_object_id();
        let mut dm = Answers {
            values: vec![0],
            pending: false,
        };
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        execute_choice_program(
            &mut game,
            &mut ctx,
            player,
            "Choose amount",
            &[(3, "Three".into())],
            1,
            true,
            |values| {
                Ok(Some(Effect::new(crate::effects::SequenceEffect::new(
                    vec![
                        Effect::gain_life(values[0]),
                        Effect::may(vec![Effect::gain_life(1)]),
                    ],
                ))))
            },
        )
        .unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(game.player(player).unwrap().life, 20);
    }
    #[test]
    fn invalid_strict_choice_never_runs_the_continuation() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId(0);
        let source = game.new_object_id();
        let mut dm = Answers {
            values: vec![3],
            pending: false,
        };
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        assert!(
            execute_choice_program(
                &mut game,
                &mut ctx,
                player,
                "Choose",
                &[(1, "One".into())],
                1,
                true,
                |_| panic!("invalid choice reached continuation")
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod payment_tests {
    use super::*;
    struct Number(u32);
    impl crate::decision::DecisionMaker for Number {
        fn decide_number(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::NumberContext,
        ) -> u32 {
            self.0
        }
    }
    #[test]
    fn amount_choice_preserves_unavailable_zero_and_bounded_payment_distinctions() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId(0);
        for (answer, min, available, expected) in [
            (0, 0, 5, Some(0)),
            (99, 2, 5, Some(5)),
            (0, 2, 5, Some(2)),
            (1, 3, 2, None),
        ] {
            let mut dm = Number(answer);
            let mut ctx = ExecutionContext::new(crate::ids::ObjectId(99), player, &mut dm);
            assert_eq!(
                choose_bounded_payment_amount(&game, &mut ctx, player, min, available, "Pay"),
                expected
            );
        }
    }
}

#[cfg(test)]
mod selection_policy_tests {
    use super::*;
    struct Answer(Vec<usize>, bool);
    impl crate::decision::DecisionMaker for Answer {
        fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> { self.0.clone() }
        fn awaiting_choice(&self) -> bool { self.1 }
    }
    #[test]
    fn typed_selection_keeps_strict_first_empty_and_pending_contracts_distinct() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        for (answer, pending, first, strict_error) in [
            (vec![1], false, Some(20), false),
            (vec![99, 0], false, None, true),
            (vec![0, 1], false, Some(10), true),
            (vec![], false, None, true),
            (vec![0], true, None, false),
        ] {
            for policy in [SingleChoicePolicy::First, SingleChoicePolicy::Strict] {
                let mut dm = Answer(answer.clone(), pending);
                let mut ctx = ExecutionContext::new(crate::ids::ObjectId(99), PlayerId(0), &mut dm);
                let result = select_typed_option(&game, &mut ctx, PlayerId(1), "Choose", &[(10, "ten".into()), (20, "twenty".into())], policy);
                if matches!(policy, SingleChoicePolicy::Strict) && strict_error { assert!(result.is_err()); }
                else { assert_eq!(result.unwrap(), first); }
            }
        }
    }
}
