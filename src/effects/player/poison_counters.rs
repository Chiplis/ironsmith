//! Poison counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
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
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        if let Some(p) = game.player_mut(player_id) {
            p.poison_counters += count;
        }

        Ok(EffectOutcome::count(count as i32))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EffectResult;
    use crate::events::DamageEvent;
    use crate::game_event::DamageTarget;
    use crate::ids::PlayerId;
    use crate::triggers::TriggerEvent;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_poison_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = PoisonCountersEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).unwrap().poison_counters, 3);
    }

    #[test]
    fn test_poison_counters_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = PoisonCountersEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().poison_counters, 0);
    }

    #[test]
    fn test_poison_counters_to_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = PoisonCountersEffect::new(5, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(5));
        assert_eq!(game.player(alice).unwrap().poison_counters, 0);
        assert_eq!(game.player(bob).unwrap().poison_counters, 5);
    }

    #[test]
    fn test_poison_counters_accumulate() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = PoisonCountersEffect::you(2);
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(game.player(alice).unwrap().poison_counters, 4);
    }

    #[test]
    fn test_poison_counters_clone_box() {
        let effect = PoisonCountersEffect::you(3);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PoisonCountersEffect"));
    }

    #[test]
    fn test_poison_counters_damaged_player_from_triggering_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let damage_event =
            TriggerEvent::new(DamageEvent::new(source, DamageTarget::Player(bob), 2, true));
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_triggering_event(damage_event);

        let effect = PoisonCountersEffect::new(2, PlayerFilter::DamagedPlayer);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().poison_counters, 0);
        assert_eq!(game.player(bob).unwrap().poison_counters, 2);
    }
}
