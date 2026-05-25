use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase, Step};

#[derive(Debug, Clone, PartialEq)]
pub struct EndTurnEffect {
    pub player: crate::target::PlayerFilter,
}

impl EndTurnEffect {
    pub fn new(player: crate::target::PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for EndTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if player != game.turn.active_player {
            return Ok(EffectOutcome::resolved());
        }

        game.stack.clear();
        if let Some(combat) = game.combat.as_mut() {
            crate::combat_state::end_combat(combat);
        }
        crate::turn::execute_cleanup_step(game);
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::Cleanup);
        game.turn.priority_player = None;
        Ok(EffectOutcome::resolved())
    }
}
