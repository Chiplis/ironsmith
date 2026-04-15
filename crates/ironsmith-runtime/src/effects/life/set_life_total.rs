//! Set life total effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::event_processor::process_life_gain_with_event;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

/// Effect that sets a player's life total to a specific value.
///
/// This is different from gaining or losing life:
/// - If the new total is higher, the player gains the difference
/// - If the new total is lower, the player loses the difference
/// - Used by cards like "Your life total becomes 10"
///
/// # Fields
///
/// * `amount` - The life total to set (can be fixed or variable)
/// * `player` - Which player's life total changes
///
/// # Example
///
/// ```ignore
/// // Set life total to 10 (like Sorin Markov's ability)
/// let effect = SetLifeTotalEffect {
///     amount: Value::Fixed(10),
///     player: PlayerFilter::Opponent,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SetLifeTotalEffect {
    /// The life total to set.
    pub amount: Value,
    /// Which player's life total changes.
    pub player: PlayerFilter,
}

impl SetLifeTotalEffect {
    /// Create a new set life total effect.
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    /// Create an effect that sets your life total.
    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }

    /// Create an effect that sets an opponent's life total.
    pub fn opponent(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::Opponent)
    }
}

impl EffectExecutor for SetLifeTotalEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?;

        let current = game.player(player_id).map(|p| p.life).unwrap_or(amount);
        if amount == current {
            return Ok(EffectOutcome::resolved());
        }

        if amount > current {
            let gained = process_life_gain_with_event(game, player_id, (amount - current) as u32);
            if gained > 0
                && let Some(player) = game.player_mut(player_id)
            {
                player.gain_life(gained);
            }
            if gained > 0 {
                return Ok(EffectOutcome::count(gained as i32).with_event(
                    TriggerEvent::new_with_provenance(
                        crate::events::LifeGainEvent::new(player_id, gained),
                        ctx.provenance,
                    ),
                ));
            }
            return Ok(EffectOutcome::resolved());
        }

        let lost = (current - amount) as u32;
        if !game.can_change_life_total(player_id) {
            return Ok(EffectOutcome::from_status(
                crate::effect::OutcomeStatus::Prevented,
            ));
        }
        if let Some(player) = game.player_mut(player_id) {
            player.lose_life(lost);
        }
        if lost > 0 {
            Ok(
                EffectOutcome::count(lost as i32).with_event(TriggerEvent::new_with_provenance(
                    crate::events::LifeLossEvent::from_effect(player_id, lost),
                    ctx.provenance,
                )),
            )
        } else {
            Ok(EffectOutcome::resolved())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventKind;
    use crate::ids::PlayerId;

    #[test]
    fn set_life_total_emits_life_gain_event_when_total_increases() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).expect("alice exists").life = 10;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = SetLifeTotalEffect::you(15)
            .execute(&mut game, &mut ctx)
            .expect("set life total should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 15);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::LifeGain),
            "raising life total should emit a LifeGainEvent"
        );
    }

    #[test]
    fn set_life_total_emits_life_loss_event_when_total_decreases() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).expect("alice exists").life = 10;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = SetLifeTotalEffect::you(4)
            .execute(&mut game, &mut ctx)
            .expect("set life total should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 4);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::LifeLoss),
            "lowering life total should emit a LifeLossEvent"
        );
    }
}
