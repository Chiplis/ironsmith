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
            let event = TriggerEvent::new_with_provenance(
                LifeGainEvent::new(player_id, final_amount),
                ctx.provenance,
            );
            Ok(outcome.with_event(event))
        } else {
            Ok(outcome)
        }
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
