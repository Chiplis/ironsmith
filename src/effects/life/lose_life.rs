//! Lose life effect implementation.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
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
}

impl EffectExecutor for LoseLifeEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        _source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        // Only validate for "you" (controller) effects
        let is_you = matches!(self.player, ChooseSpec::Player(PlayerFilter::You));
        if !is_you {
            return Ok(());
        }

        // Get the amount (use base value for validation)
        let amount = match &self.amount {
            Value::Fixed(n) => *n as u32,
            Value::X => 0, // X is usually 0 at validation time
            _ => 0,
        };

        if amount == 0 {
            return Ok(());
        }

        // Check if player has enough life
        if let Some(player) = game.player(controller) {
            if player.life < amount as i32 {
                return Err(CostValidationError::NotEnoughLife);
            }
        } else {
            return Err(CostValidationError::Other("Player not found".to_string()));
        }

        Ok(())
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
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        if let Some(p) = game.player_mut(player_id) {
            p.lose_life(amount);
        }

        // Create the trigger event only if life was actually lost
        let outcome = EffectOutcome::count(amount as i32);
        if amount > 0 {
            let event = TriggerEvent::new(LifeLossEvent::from_effect(player_id, amount));
            Ok(outcome.with_event(event))
        } else {
            Ok(outcome)
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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
        if matches!(self.player, ChooseSpec::Player(PlayerFilter::You))
            && let Value::Fixed(n) = self.amount
        {
            return Some(format!("Pay {} life", n));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    fn new_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_lose_life_fixed_amount() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = LoseLifeEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(5));
        assert_eq!(game.player(alice).unwrap().life, 15);
    }

    #[test]
    fn test_lose_life_zero_amount() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = LoseLifeEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn test_lose_life_negative_becomes_zero() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = LoseLifeEffect::with_filter(Value::Fixed(-5), PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Negative amounts are clamped to 0
        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn test_lose_life_specific_player() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Use SpecificPlayer to target Bob
        let effect = LoseLifeEffect::with_filter(3, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        // Alice (controller) unchanged
        assert_eq!(game.player(alice).unwrap().life, 20);
        // Bob lost life
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    #[test]
    fn test_lose_life_can_go_negative() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = LoseLifeEffect::you(25);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(25));
        assert_eq!(game.player(alice).unwrap().life, -5);
    }

    #[test]
    fn test_lose_life_prevented_by_cant_change() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Add "can't change life total" effect
        game.cant_effects.life_total_cant_change.insert(alice);

        let effect = LoseLifeEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Prevented);
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn test_lose_life_clone_box() {
        let effect = LoseLifeEffect::you(5);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("LoseLifeEffect"));
    }

    #[test]
    fn test_lose_life_returns_event() {
        use crate::events::EventKind;

        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = LoseLifeEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::LifeLoss);
    }

    #[test]
    fn test_lose_life_zero_returns_no_event() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = LoseLifeEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No event when no life lost
        assert!(result.events.is_empty());
    }
}
