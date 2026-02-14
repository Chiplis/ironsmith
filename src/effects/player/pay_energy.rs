//! Pay energy effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::executor_trait::CostValidationError;
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::ChooseSpec;

/// Effect that asks a player to pay energy counters.
#[derive(Debug, Clone, PartialEq)]
pub struct PayEnergyEffect {
    /// Amount of energy to pay.
    pub amount: Value,
    /// Player who pays the energy.
    pub player: ChooseSpec,
}

impl PayEnergyEffect {
    /// Create a new pay-energy effect.
    pub fn new(amount: impl Into<Value>, player: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }
}

impl EffectExecutor for PayEnergyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        if let Some(player) = game.player_mut(player_id)
            && player.energy_counters >= amount
        {
            player.energy_counters -= amount;
            return Ok(EffectOutcome::count(amount as i32));
        }

        Ok(EffectOutcome::from_result(EffectResult::Impossible))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.player.is_target() {
            Some(&self.player)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "player to pay energy"
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        let payer = resolve_player_from_spec(game, &self.player, &ctx)
            .map_err(|_| CostValidationError::Other("unable to resolve player for energy cost".to_string()))?;
        let needed = resolve_value(game, &self.amount, &ctx)
            .map_err(|_| CostValidationError::Other("unable to resolve energy amount".to_string()))?
            .max(0) as u32;
        let Some(player) = game.player(payer) else {
            return Err(CostValidationError::Other(
                "unable to resolve payer".to_string(),
            ));
        };
        if player.energy_counters >= needed {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "not enough energy counters".to_string(),
            ))
        }
    }
}

