//! Effects that apply rule restrictions ("can't" effects).

use crate::effect::{EffectOutcome, Restriction, Until};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Effect that applies a restriction for a duration.
#[derive(Debug, Clone, PartialEq)]
pub struct CantEffect {
    pub restriction: Restriction,
    pub duration: Until,
}

impl CantEffect {
    pub fn new(restriction: Restriction, duration: Until) -> Self {
        Self {
            restriction,
            duration,
        }
    }

    pub fn until_end_of_turn(restriction: Restriction) -> Self {
        Self::new(restriction, Until::EndOfTurn)
    }
}

impl EffectExecutor for CantEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.add_restriction_effect(
            self.restriction.clone(),
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
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;

    #[test]
    fn cant_effect_blocks_life_gain() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CantEffect::until_end_of_turn(Restriction::gain_life(PlayerFilter::Any));
        effect.execute(&mut game, &mut ctx).expect("execute cant");

        game.update_cant_effects();

        assert!(!game.can_gain_life(PlayerId::from_index(0)));
        assert!(!game.can_gain_life(PlayerId::from_index(1)));
    }
}
