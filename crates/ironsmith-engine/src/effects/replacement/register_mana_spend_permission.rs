use crate::effect::{EffectOutcome, Until};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{ActiveManaSpendPermission, GameState, ManaSpendPermissionSource};

pub type RegisterManaSpendPermissionEffect = ironsmith_core::RegisterManaSpendPermissionEffect;
impl EffectExecutor for RegisterManaSpendPermissionEffect {
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        let source = match self.until {
            Until::EndOfTurn => ManaSpendPermissionSource::UntilEndOfTurnEffect {source_id: ctx.source, turn: game.turn.turn_number},
            Until::Forever => ManaSpendPermissionSource::Effect {source_id: ctx.source, expires_end_of_turn: u32::MAX},
            _ => return Err(ExecutionError::UnresolvableValue("unsupported mana-spend permission duration".into())),
        };
        game.effect_store.mana_spend_effects.permissions.push(ActiveManaSpendPermission {
            permission: self.permission.clone(), controller: ctx.controller,
            source,
        });
        game.mark_continuous_state_dirty();
        Ok(EffectOutcome::resolved())
    }
}
