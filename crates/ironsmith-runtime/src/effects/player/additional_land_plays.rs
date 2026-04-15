use crate::effect::{EffectOutcome, Restriction, Until, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that grants additional land plays for a duration.
#[derive(Debug, Clone, PartialEq)]
pub struct AdditionalLandPlaysEffect {
    pub count: Value,
    pub player: PlayerFilter,
    pub duration: Until,
}

impl AdditionalLandPlaysEffect {
    pub fn new(count: impl Into<Value>, player: PlayerFilter, duration: Until) -> Self {
        Self {
            count: count.into(),
            player,
            duration,
        }
    }
}

impl EffectExecutor for AdditionalLandPlaysEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;
        if count == 0 {
            return Ok(EffectOutcome::resolved());
        }

        game.add_restriction_effect(
            Restriction::additional_land_plays(PlayerFilter::Specific(player), count),
            self.duration.clone(),
            ctx.source,
            ctx.controller,
        );
        game.update_cant_effects();
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionContext;
    use crate::ids::PlayerId;

    #[test]
    fn grants_additional_land_play_until_end_of_turn() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        AdditionalLandPlaysEffect::new(1, PlayerFilter::You, Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .expect("grant additional land play");

        assert_eq!(
            game.player(alice).expect("player").land_plays_per_turn,
            2,
            "effect should increase land plays per turn immediately"
        );
    }

    #[test]
    fn grant_after_first_land_leaves_one_remaining_land_play() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        game.player_mut(alice).expect("player").record_land_play();
        assert!(
            !game.player(alice).expect("player").can_play_land(),
            "normal land play should be exhausted after the first land"
        );

        AdditionalLandPlaysEffect::new(1, PlayerFilter::You, Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .expect("grant additional land play");

        assert!(
            game.player(alice).expect("player").can_play_land(),
            "granting an additional land play should reopen land play eligibility"
        );

        game.player_mut(alice).expect("player").record_land_play();
        assert!(
            !game.player(alice).expect("player").can_play_land(),
            "the second total land play should consume the temporary extra allowance"
        );
    }
}
