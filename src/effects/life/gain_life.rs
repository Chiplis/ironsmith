//! Gain life effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::event_processor::process_life_gain_with_event;
use crate::events::LifeGainEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::TriggerEvent;

/// Effect that causes a player to gain life.
///
/// # Fields
///
/// * `amount` - The amount of life to gain (can be fixed or variable)
/// * `player` - Which player gains life (as a ChooseSpec)
///
/// # Example
///
/// ```ignore
/// // Gain 3 life (healing salve style)
/// let effect = GainLifeEffect {
///     amount: Value::Fixed(3),
///     player: ChooseSpec::Player(PlayerFilter::You),
/// };
///
/// // Target player gains 3 life
/// let effect = GainLifeEffect {
///     amount: Value::Fixed(3),
///     player: ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)),
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GainLifeEffect {
    /// The amount of life to gain.
    pub amount: Value,
    /// Which player gains life.
    pub player: ChooseSpec,
}

impl GainLifeEffect {
    /// Create a new gain life effect.
    pub fn new(amount: impl Into<Value>, player: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    /// Create a new gain life effect from a PlayerFilter (convenience).
    pub fn with_filter(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self::new(amount, ChooseSpec::Player(player))
    }

    /// Create an effect where you gain life.
    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, ChooseSpec::Player(PlayerFilter::You))
    }

    /// Create an effect where target player gains life.
    pub fn target_player(amount: impl Into<Value>) -> Self {
        Self::new(amount, ChooseSpec::target_player())
    }
}

impl EffectExecutor for GainLifeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        // Process through replacement effects and check "can't gain life"
        let final_amount = process_life_gain_with_event(game, player_id, amount);

        if final_amount > 0
            && let Some(p) = game.player_mut(player_id)
        {
            p.gain_life(final_amount);
        }

        // Create the trigger event only if life was actually gained
        let outcome = EffectOutcome::count(final_amount as i32);
        if final_amount > 0 {
            let event = TriggerEvent::new(LifeGainEvent::new(player_id, final_amount));
            Ok(outcome.with_event(event))
        } else {
            Ok(outcome)
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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
        "player to gain life"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn new_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_gain_life_fixed_amount() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = GainLifeEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(5));
        assert_eq!(game.player(alice).unwrap().life, 25);
    }

    #[test]
    fn test_gain_life_zero_amount() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = GainLifeEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn test_gain_life_negative_becomes_zero() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = GainLifeEffect::with_filter(Value::Fixed(-5), PlayerFilter::You);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Negative amounts are clamped to 0
        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn test_gain_life_specific_player() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Use SpecificPlayer to target Bob
        let effect = GainLifeEffect::with_filter(3, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        // Alice (controller) unchanged
        assert_eq!(game.player(alice).unwrap().life, 20);
        // Bob gained life
        assert_eq!(game.player(bob).unwrap().life, 23);
    }

    #[test]
    fn test_gain_life_clone_box() {
        let effect = GainLifeEffect::you(5);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("GainLifeEffect"));
    }

    #[test]
    fn test_gain_life_returns_event() {
        use crate::events::EventKind;

        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = GainLifeEffect::you(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::LifeGain);
    }

    #[test]
    fn test_gain_life_zero_returns_no_event() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = GainLifeEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No event when no life gained
        assert!(result.events.is_empty());
    }
}
