use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, helpers::resolve_player_filter};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Flip a coin for a player using the game's deterministic RNG.
#[derive(Debug, Clone, PartialEq)]
pub struct FlipCoinEffect {
    pub player: PlayerFilter,
}

impl FlipCoinEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for FlipCoinEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let _player = resolve_player_filter(game, &self.player, ctx)?;
        let mut faces = [true, false];
        game.shuffle_slice(&mut faces);
        Ok(EffectOutcome::count(i32::from(faces[0])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Effect, EffectId, EffectPredicate};
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;

    #[test]
    fn flip_coin_is_deterministic_for_a_seed_and_marks_random_usage() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(7);

        let before = game.irreversible_random_count();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = execute_effect(&mut game, &Effect::flip_coin(PlayerFilter::You), &mut ctx)
            .expect("coin flip should resolve");

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "coin flips should consume irreversible randomness"
        );
        assert_eq!(
            outcome.as_count(),
            Some(0),
            "seeded coin flip should stay deterministic"
        );
    }

    #[test]
    fn flip_coin_outcome_drives_if_result_branches() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(2);

        let mut ctx = ExecutionContext::new_default(source, alice);
        execute_effect(
            &mut game,
            &Effect::with_id(0, Effect::flip_coin(PlayerFilter::You)),
            &mut ctx,
        )
        .expect("coin flip should resolve");

        execute_effect(
            &mut game,
            &Effect::if_then(
                EffectId(0),
                EffectPredicate::Happened,
                vec![Effect::gain_life(3)],
            ),
            &mut ctx,
        )
        .expect("if-result branch should resolve");

        assert_eq!(
            game.player(alice).unwrap().life,
            23,
            "winning the seeded coin flip should take the happened branch"
        );
    }
}
