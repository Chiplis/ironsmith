//! Pay energy effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::executor_trait::CostValidationError;
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::{ChooseSpec, PlayerFilter};

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
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        if game
            .player(player_id)
            .is_some_and(|player| player.energy_counters >= amount)
            && let Some((removed, event)) = game.remove_player_counters_with_source(
                player_id,
                CounterType::Energy,
                amount,
                Some(ctx.source),
                Some(ctx.controller),
            )
        {
            return Ok(EffectOutcome::count(removed as i32).with_event(event));
        }

        Ok(EffectOutcome::from_result(EffectResult::Impossible))
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

    fn cost_description(&self) -> Option<String> {
        if matches!(self.player, ChooseSpec::Player(PlayerFilter::You))
            && let Value::Fixed(amount) = self.amount
        {
            let symbols: String = (0..amount.max(0)).map(|_| "{E}").collect();
            return Some(format!("Pay {}", symbols));
        }
        None
    }
}

impl CostExecutableEffect for PayEnergyEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        let payer = resolve_player_from_spec(game, &self.player, &ctx).map_err(|_| {
            CostValidationError::Other("unable to resolve player for energy cost".to_string())
        })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventKind;
    use crate::ids::PlayerId;
    use crate::target::{ChooseSpec, PlayerFilter};

    #[test]
    fn pay_energy_effect_emits_markers_changed_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice exists")
            .energy_counters = 4;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = PayEnergyEffect::new(2, ChooseSpec::Player(PlayerFilter::You))
            .execute(&mut game, &mut ctx)
            .expect("pay energy should resolve");

        assert_eq!(game.player(alice).expect("alice exists").energy_counters, 2);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::MarkersChanged),
            "paying energy should emit MarkersChangedEvent"
        );
    }
}
