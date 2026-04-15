//! Register a one-shot spell-cost reduction for the next matching spell this turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaCost;
use crate::target::{ObjectFilter, PlayerFilter};

#[derive(Debug, Clone, PartialEq)]
pub struct GrantNextSpellCostReductionEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub reduction: ManaCost,
}

impl GrantNextSpellCostReductionEffect {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, reduction: ManaCost) -> Self {
        Self {
            player,
            filter,
            reduction,
        }
    }
}

impl EffectExecutor for GrantNextSpellCostReductionEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        game.add_temporary_spell_cost_reduction(
            player,
            ctx.source,
            self.filter.clone(),
            self.reduction.clone(),
            1,
        );
        Ok(EffectOutcome::resolved())
    }
}
