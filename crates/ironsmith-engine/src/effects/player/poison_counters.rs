//! Poison counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::{CompletedEffectOutputs, CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::PlayerFilter;

/// Effect that gives a player poison counters.
///
/// # Fields
///
/// * `count` - How many poison counters to add (can be fixed or variable)
/// * `player` - Which player receives the poison counters
///
/// # Example
///
/// ```ignore
/// // Give yourself 2 poison counters (e.g., from a cost)
/// let effect = PoisonCountersEffect::you(2);
///
/// // Give a specific player 3 poison counters
/// let effect = PoisonCountersEffect::new(3, PlayerFilter::Specific(opponent_id));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PoisonCountersEffect {
    /// How many poison counters to add.
    pub count: Value,
    /// Which player receives the counters.
    pub player: PlayerFilter,
}

impl PoisonCountersEffect {
    /// Create a new poison counters effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you get poison counters.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for PoisonCountersEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn cost_description(&self) -> Option<String> {
        match self.count {
            Value::Fixed(count) => Some(format!("Get {count} poison counters")),
            _ => Some("Get poison counters".to_string()),
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::player_counters::PlayerCountersEffect::new(
            CounterType::Poison, self.count.clone(), self.player.clone(),
        ).execute_selected_with_outputs(game, ctx)
    }
}

impl CostExecutableEffect for PoisonCountersEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        _source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        let player = match self.player {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(player) => player,
            _ => return Err(CostValidationError::Other("poison cost requires a known payer".into())),
        };
        if game.player(player).is_some()
            && (matches!(self.count, Value::Fixed(0)) || game.can_get_poison_counters(player))
        {
            Ok(())
        } else {
            Err(CostValidationError::Other("payer cannot get poison counters".into()))
        }
    }
}

#[cfg(test)]
mod cost_tests {
    use super::*;

    #[test]
    fn poison_cost_respects_the_payers_counter_prohibition() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let cost = PoisonCountersEffect::you(5);
        assert!(crate::costs::Cost::try_effect(crate::effect::Effect::new(cost.clone())).is_ok());
        assert!(CostExecutableEffect::can_execute_as_cost(&cost, &game, source, alice).is_ok());
        game.effect_store.cant_effects.cant_get_poison_counters.insert(alice);
        assert!(CostExecutableEffect::can_execute_as_cost(&cost, &game, source, alice).is_err());
        assert!(CostExecutableEffect::can_execute_as_cost(&PoisonCountersEffect::you(0), &game, source, alice).is_ok());
    }
}
