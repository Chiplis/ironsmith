use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Creates an isolated child game and stores parent-game continuation effects
/// for the participants who do not win that child game.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlaySubgameEffect {
    pub nonwinner_effects: Vec<Effect>,
}

impl PlaySubgameEffect {
    pub fn new(nonwinner_effects: Vec<Effect>) -> Self {
        Self { nonwinner_effects }
    }
}

impl EffectExecutor for PlaySubgameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.begin_subgame(
            ctx.cause.source.or(Some(ctx.source)),
            ctx.controller,
            self.nonwinner_effects.clone(),
        )
        .map_err(ExecutionError::Impossible)?;
        Ok(EffectOutcome::resolved())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.nonwinner_effects {
            visitor(effect);
        }
    }
}
