//! Lose life effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::events::LifeLossEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::TriggerEvent;

/// Effect that causes a player to lose life.
///
/// Note: Losing life is different from taking damage:
/// - Damage can be prevented
/// - Losing life cannot be prevented (except by effects that prevent life total changes)
/// - Damage causes loss of life, but loss of life is not damage
///
/// # Fields
///
/// * `amount` - The amount of life to lose (can be fixed or variable)
/// * `player` - Which player loses life (as a ChooseSpec)
///
/// # Example
///
/// ```ignore
/// // Lose 2 life (like Dark Confidant trigger)
/// let effect = LoseLifeEffect {
///     amount: Value::Fixed(2),
///     player: ChooseSpec::Player(PlayerFilter::You),
/// };
///
/// // Target player loses 3 life
/// let effect = LoseLifeEffect {
///     amount: Value::Fixed(3),
///     player: ChooseSpec::target_player(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LoseLifeEffect {
    /// The amount of life to lose.
    pub amount: Value,
    /// Which player loses life.
    pub player: ChooseSpec,
}

impl LoseLifeEffect {
    /// Create a new lose life effect.
    pub fn new(amount: impl Into<Value>, player: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    /// Create a new lose life effect from a PlayerFilter (convenience).
    pub fn with_filter(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self::new(amount, ChooseSpec::Player(player))
    }

    /// Create an effect where you lose life.
    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, ChooseSpec::Player(PlayerFilter::You))
    }

    /// Create an effect where target player loses life.
    pub fn target_player(amount: impl Into<Value>) -> Self {
        Self::new(amount, ChooseSpec::target_player())
    }

    fn life_per_card_in_hand_multiplier(amount: &Value) -> Option<u32> {
        match amount {
            Value::CardsInHand(PlayerFilter::You) => Some(1),
            Value::Add(lhs, rhs) => Some(
                Self::life_per_card_in_hand_multiplier(lhs)?
                    + Self::life_per_card_in_hand_multiplier(rhs)?,
            ),
            _ => None,
        }
    }
}

impl EffectExecutor for LoseLifeEffect {
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

        // Check if player's life total can change (Platinum Emperion, etc.)
        if !game.can_change_life_total(player_id) {
            return Ok(EffectOutcome::prevented());
        }

        if let Some(p) = game.player_mut(player_id) {
            p.lose_life(amount);
        }

        // Create the trigger event only if life was actually lost
        let outcome = EffectOutcome::count(amount as i32);
        if amount > 0 {
            let event = TriggerEvent::new_with_provenance(
                LifeLossEvent::from_effect(player_id, amount),
                ctx.provenance,
            );
            Ok(outcome.with_event(event))
        } else {
            Ok(outcome)
        }
    }

    fn pay_life_amount(&self) -> Option<u32> {
        // Only report pay_life_amount for "you" effects (used in cost checking)
        if matches!(self.player, ChooseSpec::Player(PlayerFilter::You))
            && let Value::Fixed(n) = self.amount
        {
            return Some(n as u32);
        }
        None
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        // Only return spec if it's a target (requires selection during casting)
        if self.player.is_target() {
            Some(&self.player)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "player to lose life"
    }

    fn cost_description(&self) -> Option<String> {
        // Only provide cost description for "you" effects (used as costs)
        if matches!(self.player, ChooseSpec::Player(PlayerFilter::You)) {
            if let Value::Fixed(n) = self.amount {
                return Some(format!("Pay {} life", n));
            }

            if let Some(per_card) = Self::life_per_card_in_hand_multiplier(&self.amount) {
                if per_card == 1 {
                    return Some("Pay 1 life for each card in your hand".to_string());
                }
                return Some(format!("Pay {per_card} life for each card in your hand"));
            }
        }
        None
    }
}

impl CostExecutableEffect for LoseLifeEffect {
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        if reason.is_cast_or_ability_payment()
            && game.player_cant_pay_life_to_cast_or_activate(controller)
        {
            return Err(CostValidationError::NotEnoughLife);
        }

        crate::effects::CostExecutableEffect::can_execute_as_cost(self, game, source, controller)
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        let is_you = matches!(self.player, ChooseSpec::Player(PlayerFilter::You));
        if !is_you {
            return Ok(());
        }

        let amount = match &self.amount {
            Value::Fixed(n) => (*n).max(0) as u32,
            _ => {
                let ctx = ExecutionContext::new_default(source, controller);
                let resolved = resolve_value(game, &self.amount, &ctx).map_err(|err| {
                    CostValidationError::Other(format!("Unable to resolve life cost: {err}"))
                })?;
                resolved.max(0) as u32
            }
        };

        if amount == 0 {
            return Ok(());
        }

        if let Some(player) = game.player(controller) {
            if player.life < amount as i32 {
                return Err(CostValidationError::NotEnoughLife);
            }
        } else {
            return Err(CostValidationError::Other("Player not found".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lose_life_cost_description_fixed() {
        let effect = LoseLifeEffect::you(2);
        assert_eq!(effect.cost_description().as_deref(), Some("Pay 2 life"));
    }

    #[test]
    fn test_lose_life_cost_description_per_card_in_hand() {
        let effect = LoseLifeEffect::you(Value::CardsInHand(PlayerFilter::You));
        assert_eq!(
            effect.cost_description().as_deref(),
            Some("Pay 1 life for each card in your hand")
        );
    }

    #[test]
    fn test_lose_life_cost_description_scaled_per_card_in_hand() {
        let effect = LoseLifeEffect::you(Value::Add(
            Box::new(Value::CardsInHand(PlayerFilter::You)),
            Box::new(Value::CardsInHand(PlayerFilter::You)),
        ));
        assert_eq!(
            effect.cost_description().as_deref(),
            Some("Pay 2 life for each card in your hand")
        );
    }
}
