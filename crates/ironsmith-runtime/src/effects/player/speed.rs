//! Speed effects for Start your engines and max speed cards.

use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::helpers::{resolve_player_filter_to_list, resolve_value};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::{IncreaseSpeedEffect, ReduceSpeedEffect};

impl EffectExecutor for IncreaseSpeedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        let players =
            resolve_player_filter_to_list(game, &self.player, &ctx.filter_context(game), ctx)?;
        let changed = players
            .into_iter()
            .map(|player| game.increase_speed(player, amount))
            .sum::<u32>();

        Ok(EffectOutcome {
            status: OutcomeStatus::Succeeded,
            value: crate::effect::OutcomeValue::Count(changed as i32),
            events: Vec::new(),
            execution_facts: Vec::new(),
        })
    }
}

impl EffectExecutor for ReduceSpeedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        let players =
            resolve_player_filter_to_list(game, &self.player, &ctx.filter_context(game), ctx)?;
        let changed = players
            .into_iter()
            .map(|player| game.reduce_speed(player, amount, self.minimum))
            .sum::<u32>();

        Ok(EffectOutcome {
            status: OutcomeStatus::Succeeded,
            value: crate::effect::OutcomeValue::Count(changed as i32),
            events: Vec::new(),
            execution_facts: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    #[test]
    fn increase_speed_starts_and_caps_at_four() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = IncreaseSpeedEffect::you(2)
            .execute(&mut game, &mut ctx)
            .expect("increase speed should resolve");

        assert_eq!(game.player_speed(alice), Some(2));
        assert_eq!(outcome.as_count(), Some(2));

        IncreaseSpeedEffect::you(10)
            .execute(&mut game, &mut ctx)
            .expect("increase speed should cap");
        assert_eq!(game.player_speed(alice), Some(4));
    }

    #[test]
    fn reduce_speed_respects_minimum() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.start_engines(alice);
        game.increase_speed(alice, 3);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ReduceSpeedEffect::new(5, ironsmith_core::PlayerFilter::You, 1)
            .execute(&mut game, &mut ctx)
            .expect("reduce speed should resolve");

        assert_eq!(game.player_speed(alice), Some(1));
        assert_eq!(outcome.as_count(), Some(3));
    }
}
