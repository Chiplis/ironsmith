//! Exchange life totals effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::event_processor::process_life_gain_with_event;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::TriggerEvent;

/// Effect that exchanges life totals between two players.
///
/// Used by cards like "Exchange life totals with target player."
/// Both players' life totals are simultaneously set to what
/// the other player's life total was.
///
/// # Fields
///
/// * `player1` - First player in the exchange (usually the controller)
/// * `player2` - Second player in the exchange (usually target opponent)
///
/// # Example
///
/// ```ignore
/// // Exchange life totals with target player
/// let effect = ExchangeLifeTotalsEffect::with_target();
///
/// // Or with a specific opponent
/// let effect = ExchangeLifeTotalsEffect::with_opponent();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeLifeTotalsEffect {
    /// First player in the exchange.
    pub player1: PlayerFilter,
    /// Second player in the exchange.
    pub player2: PlayerFilter,
    /// Target spec if this effect targets players.
    pub target_spec: Option<ChooseSpec>,
    /// Whether the exchange participants come from the first two player targets.
    pub use_two_player_targets: bool,
}

impl ExchangeLifeTotalsEffect {
    /// Create a new exchange life totals effect.
    pub fn new(player1: PlayerFilter, player2: PlayerFilter) -> Self {
        let use_two_player_targets = matches!(
            (&player1, &player2),
            (PlayerFilter::Target(_), PlayerFilter::Target(_))
        );
        let target_spec = if use_two_player_targets {
            Some(ChooseSpec::target_player().with_count(crate::effect::ChoiceCount::exactly(2)))
        } else {
            match (&player1, &player2) {
                (PlayerFilter::Target(inner), _) | (_, PlayerFilter::Target(inner)) => {
                    Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
                }
                _ => None,
            }
        };

        Self {
            player1,
            player2,
            target_spec,
            use_two_player_targets,
        }
    }

    /// Create an effect that exchanges life totals with target opponent.
    pub fn with_opponent() -> Self {
        Self::new(PlayerFilter::You, PlayerFilter::Opponent)
    }

    /// Create an effect that exchanges life totals with target player.
    pub fn with_target() -> Self {
        Self::new(
            PlayerFilter::You,
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
        )
    }

    /// Create an effect that exchanges life totals of two targeted players.
    pub fn between_two_targets() -> Self {
        Self::new(
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
        )
    }
}

impl EffectExecutor for ExchangeLifeTotalsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (player1_id, player2_id) = if self.use_two_player_targets {
            ctx.resolve_two_player_targets()
                .ok_or(ExecutionError::InvalidTarget)?
        } else {
            (
                resolve_player_filter(game, &self.player1, ctx)?,
                resolve_player_filter(game, &self.player2, ctx)?,
            )
        };

        let life1 = game.player(player1_id).map(|p| p.life).unwrap_or(0);
        let life2 = game.player(player2_id).map(|p| p.life).unwrap_or(0);

        if life1 == life2 {
            return Ok(EffectOutcome::resolved());
        }

        let player1_can_exchange = if life2 > life1 {
            game.can_gain_life(player1_id)
        } else {
            game.can_lose_life(player1_id)
        };
        let player2_can_exchange = if life1 > life2 {
            game.can_gain_life(player2_id)
        } else {
            game.can_lose_life(player2_id)
        };

        if !player1_can_exchange || !player2_can_exchange {
            return Ok(EffectOutcome::prevented());
        }

        let mut outcome = EffectOutcome::resolved();

        if life2 > life1 {
            let gained = process_life_gain_with_event(game, player1_id, (life2 - life1) as u32);
            if gained > 0
                && let Some(player) = game.player_mut(player1_id)
            {
                player.gain_life(gained);
            }
            if gained > 0 {
                outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                    crate::events::LifeGainEvent::new(player1_id, gained),
                    ctx.provenance,
                ));
            }
        } else if life1 > life2 {
            let lost = (life1 - life2) as u32;
            if let Some(player) = game.player_mut(player1_id) {
                player.lose_life(lost);
            }
            if lost > 0 {
                outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                    crate::events::LifeLossEvent::from_effect(player1_id, lost),
                    ctx.provenance,
                ));
            }
        }

        if life1 > life2 {
            let gained = process_life_gain_with_event(game, player2_id, (life1 - life2) as u32);
            if gained > 0
                && let Some(player) = game.player_mut(player2_id)
            {
                player.gain_life(gained);
            }
            if gained > 0 {
                outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                    crate::events::LifeGainEvent::new(player2_id, gained),
                    ctx.provenance,
                ));
            }
        } else if life2 > life1 {
            let lost = (life2 - life1) as u32;
            if let Some(player) = game.player_mut(player2_id) {
                player.lose_life(lost);
            }
            if lost > 0 {
                outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                    crate::events::LifeLossEvent::from_effect(player2_id, lost),
                    ctx.provenance,
                ));
            }
        }

        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target_spec.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player whose life total is exchanged"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventKind;
    use crate::executor::ResolvedTarget;
    use crate::ids::PlayerId;

    #[test]
    fn exchange_life_totals_emits_gain_and_loss_events() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.player_mut(alice).expect("alice exists").life = 10;
        game.player_mut(bob).expect("bob exists").life = 20;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let outcome = ExchangeLifeTotalsEffect::with_target()
            .execute(&mut game, &mut ctx)
            .expect("exchange should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 20);
        assert_eq!(game.player(bob).expect("bob exists").life, 10);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::LifeGain),
            "exchanging life totals should emit at least one LifeGainEvent"
        );
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::LifeLoss),
            "exchanging life totals should emit at least one LifeLossEvent"
        );
    }

    #[test]
    fn exchange_life_totals_two_targets_uses_both_targeted_players() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.player_mut(alice).expect("alice exists").life = 30;
        game.player_mut(bob).expect("bob exists").life = 12;
        game.player_mut(cara).expect("cara exists").life = 5;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Player(bob),
            ResolvedTarget::Player(cara),
        ]);
        let outcome = ExchangeLifeTotalsEffect::between_two_targets()
            .execute(&mut game, &mut ctx)
            .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.player(alice).expect("alice exists").life, 30);
        assert_eq!(game.player(bob).expect("bob exists").life, 5);
        assert_eq!(game.player(cara).expect("cara exists").life, 12);
    }

    #[test]
    fn exchange_life_totals_is_all_or_nothing_when_player_cant_gain_life() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.player_mut(alice).expect("alice exists").life = 10;
        game.player_mut(bob).expect("bob exists").life = 20;
        game.cant_effects.add_cant_gain_life(alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let outcome = ExchangeLifeTotalsEffect::with_target()
            .execute(&mut game, &mut ctx)
            .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Prevented);
        assert_eq!(game.player(alice).expect("alice exists").life, 10);
        assert_eq!(game.player(bob).expect("bob exists").life, 20);
    }

    #[test]
    fn exchange_life_totals_is_all_or_nothing_when_player_cant_lose_life() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.player_mut(alice).expect("alice exists").life = 20;
        game.player_mut(bob).expect("bob exists").life = 10;
        game.cant_effects.add_life_total_cant_change(alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let outcome = ExchangeLifeTotalsEffect::with_target()
            .execute(&mut game, &mut ctx)
            .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Prevented);
        assert_eq!(game.player(alice).expect("alice exists").life, 20);
        assert_eq!(game.player(bob).expect("bob exists").life, 10);
    }
}
