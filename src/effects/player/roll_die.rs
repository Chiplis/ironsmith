use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, helpers::resolve_player_filter};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Roll a die for a player using the game's deterministic RNG.
#[derive(Debug, Clone, PartialEq)]
pub struct RollDieEffect {
    pub player: PlayerFilter,
    pub sides: u32,
}

impl RollDieEffect {
    pub fn new(player: PlayerFilter, sides: u32) -> Self {
        Self { player, sides }
    }
}

impl EffectExecutor for RollDieEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let _player = resolve_player_filter(game, &self.player, ctx)?;
        if self.sides == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let mut faces: Vec<u32> = (1..=self.sides).collect();
        game.shuffle_slice(&mut faces);
        Ok(EffectOutcome::count(faces[0] as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Comparison, Effect, EffectId, EffectPredicate};
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;

    #[test]
    fn roll_die_is_deterministic_for_a_seed_and_consumes_randomness() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(7);

        let before = game.irreversible_random_count();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = execute_effect(
            &mut game,
            &Effect::roll_die(20, PlayerFilter::You),
            &mut ctx,
        )
        .expect("die roll should resolve");
        let rolled = outcome.as_count().expect("die roll should produce a count");

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "die rolls should consume irreversible randomness"
        );
        assert!(
            (1..=20).contains(&rolled),
            "expected a valid d20 result, got {rolled}"
        );
    }

    #[test]
    fn roll_die_outcome_drives_value_based_if_result_branches() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(2);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = execute_effect(
            &mut game,
            &Effect::with_id(0, Effect::roll_die(20, PlayerFilter::You)),
            &mut ctx,
        )
        .expect("die roll should resolve");
        let rolled = outcome.as_count().expect("die roll should produce a count");

        execute_effect(
            &mut game,
            &Effect::if_then(
                EffectId(0),
                EffectPredicate::Value(Comparison::BetweenInclusive(
                    rolled.saturating_sub(1).max(1),
                    rolled,
                )),
                vec![Effect::gain_life(3)],
            ),
            &mut ctx,
        )
        .expect("if-result branch should resolve");

        execute_effect(
            &mut game,
            &Effect::if_then(
                EffectId(0),
                EffectPredicate::Value(Comparison::BetweenInclusive(
                    rolled.saturating_add(1),
                    rolled.saturating_add(2),
                )),
                vec![Effect::gain_life(5)],
            ),
            &mut ctx,
        )
        .expect("non-matching if-result branch should resolve");

        assert_eq!(game.player(alice).unwrap().life, 23);
    }
}
