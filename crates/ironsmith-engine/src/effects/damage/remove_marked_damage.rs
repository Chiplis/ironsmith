//! Physical marked-damage removal. Keyword notifications belong to callers.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;

#[derive(Debug, Clone, PartialEq)]
struct RemoveMarkedDamage {
    object: ObjectId,
    amount: Option<u32>,
}

impl EffectExecutor for RemoveMarkedDamage {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() || game.object(self.object).is_none() {
            return Ok(EffectOutcome::count(0));
        }
        let marked = game.damage_on(self.object);
        let removed = self.amount.unwrap_or(marked).min(marked);
        if removed > 0 {
            game.set_damage_marked(self.object, marked - removed);
        }
        Ok(EffectOutcome::count(removed).with_affected_objects_from_game(game, vec![self.object]))
    }
}

pub(crate) fn remove_marked_damage(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object: ObjectId,
    amount: Option<u32>,
) -> Result<EffectOutcome, ExecutionError> {
    RemoveMarkedDamage { object, amount }.execute_child(game, ctx)
}
