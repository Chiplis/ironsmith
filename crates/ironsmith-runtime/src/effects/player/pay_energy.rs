//! Pay energy effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{NumberSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, ExecutionFact, Value};
use crate::effects::executor_trait::CostValidationError;
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::{ChooseSpec, PlayerFilter};
pub type PayEnergyEffect = ironsmith_core::PayEnergyEffect;
pub type PayAnyEnergyEffect = ironsmith_core::PayAnyEnergyEffect;

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

        Ok(EffectOutcome::impossible())
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

impl EffectExecutor for PayAnyEnergyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        let available = game
            .player(player_id)
            .map(|player| player.energy_counters)
            .unwrap_or(0);

        if available < self.min_amount {
            return Ok(EffectOutcome::count(0));
        }

        let number_spec = if self.min_amount == 0 {
            NumberSpec::up_to(ctx.source, available, "Choose how much {E} to pay")
        } else {
            NumberSpec::range(
                ctx.source,
                self.min_amount,
                available,
                "Choose how much {E} to pay",
            )
        };

        let chosen = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            number_spec,
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let chosen = chosen.clamp(self.min_amount, available);

        if chosen == 0 {
            return Ok(EffectOutcome::count(0).with_execution_fact(ExecutionFact::ChosenNumber(0)));
        }

        if let Some((removed, event)) = game.remove_player_counters_with_source(
            player_id,
            CounterType::Energy,
            chosen,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            return Ok(EffectOutcome::count(removed as i32)
                .with_event(event)
                .with_execution_fact(ExecutionFact::ChosenNumber(removed)));
        }

        Ok(EffectOutcome::count(0))
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
