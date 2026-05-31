//! Register a one-shot spell-cost reduction for the next matching spell this turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type GrantNextSpellCostReductionEffect = ironsmith_core::GrantNextSpellCostReductionEffect;

impl EffectExecutor for GrantNextSpellCostReductionEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if self.applies_to_all_matching_this_turn {
            let amount = self
                .generic_reduction
                .as_ref()
                .map(|value| resolve_value(game, value, ctx))
                .transpose()?
                .unwrap_or(0)
                .max(0);
            game.add_temporary_matching_spell_cost_reduction_this_turn(
                player,
                ctx.source,
                self.filter.clone(),
                crate::effect::Value::Fixed(amount),
            );
        } else {
            game.add_temporary_spell_cost_reduction(
                player,
                ctx.source,
                self.filter.clone(),
                self.reduction.clone(),
                1,
            );
        }
        Ok(EffectOutcome::resolved())
    }
}
