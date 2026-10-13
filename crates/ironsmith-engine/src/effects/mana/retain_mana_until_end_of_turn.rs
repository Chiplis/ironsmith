use crate::effect::{EffectOutcome, Restriction, Until};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub type RetainManaUntilEndOfTurnEffect = ironsmith_core::RetainManaUntilEndOfTurnEffect;

impl EffectExecutor for RetainManaUntilEndOfTurnEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut registration = crate::effects::restrictions::ResolvedRestrictions::new(
            vec![Restriction::lose_unspent_mana(self.player.clone(), self.color)],
            Until::EndOfTurn,
        );
        registration.iterated_player = ctx.iteration.iterated_player;
        registration.register(game, ctx);
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_mana_keeps_its_iterated_player_after_the_binding_changes() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = crate::ids::PlayerId(0);
        let b = crate::ids::PlayerId(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, a);
        ctx.iteration.iterated_player = Some(b);
        RetainManaUntilEndOfTurnEffect::new(crate::target::PlayerFilter::IteratedPlayer)
            .execute(&mut game, &mut ctx).unwrap();
        ctx.iteration.iterated_player = Some(a);
        game.update_cant_effects();
        assert!(game.effect_store.cant_effects.retained_mana_scopes(b).unwrap().contains(&None));
        assert!(game.effect_store.cant_effects.retained_mana_scopes(a).is_none());
    }
}
