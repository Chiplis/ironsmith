//! Gain life effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, CostValidationError, ExecutionContext, ExecutionError};
use crate::events::LifeGainEvent;
use crate::events::processing::process_life_gain_with_event;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::GainLifeEffect;

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

        if final_amount > 0 {
            game.gain_life(player_id, final_amount);
        }

        // Create the trigger event only if life was actually gained
        let outcome = EffectOutcome::count(final_amount as i32);
        if final_amount > 0 {
            let mut event = TriggerEvent::new_with_provenance(
                LifeGainEvent::new(player_id, final_amount).with_source(ctx.source),
                ctx.provenance,
            );
            if game.object(ctx.source).is_none()
                && let Some(snapshot) = ctx.source_snapshot.as_ref()
            {
                event = event.with_source_snapshot(snapshot.clone());
            }
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

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Life gain involves no choices; the amount is fixed against the
        // pre-action state and the whole batch commits together.
        let player = resolve_player_from_spec(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        Ok(Box::new(GainLifeProposal { player, amount }))
    }

    fn target_description(&self) -> &'static str {
        "player to gain life"
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }
}

/// One player's part of a simultaneous each-player life gain.
#[derive(Debug)]
struct GainLifeProposal {
    player: crate::ids::PlayerId,
    amount: u32,
}

impl crate::effects::SimultaneousEffectProposal for GainLifeProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let final_amount = process_life_gain_with_event(game, self.player, self.amount);
        if final_amount > 0 {
            game.gain_life(self.player, final_amount);
            let mut event = TriggerEvent::new_with_provenance(
                LifeGainEvent::new(self.player, final_amount).with_source(ctx.source),
                ctx.provenance,
            );
            if game.object(ctx.source).is_none()
                && let Some(snapshot) = ctx.source_snapshot.as_ref()
            {
                event = event.with_source_snapshot(snapshot.clone());
            }
            Ok(EffectOutcome::count(final_amount as i32).with_event(event))
        } else {
            Ok(EffectOutcome::count(0))
        }
    }
}

impl CostExecutableEffect for GainLifeEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        let recipient = resolve_player_from_spec(game, &self.player, &ctx).map_err(|error| {
            CostValidationError::Other(format!("life-gain cost has no eligible recipient: {error}"))
        })?;
        if !game.can_gain_life(recipient) {
            return Err(CostValidationError::Other(
                "required player can't gain life".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventKind;
    use crate::events::life::matchers::WouldGainLifeMatcher;
    use crate::ids::PlayerId;
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};

    fn setup() -> (GameState, PlayerId) {
        (
            crate::tests::test_helpers::setup_two_player_game(),
            PlayerId::from_index(0),
        )
    }

    #[test]
    fn shared_gain_life_payload_executes_in_runtime() {
        let (mut game, alice) = setup();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = GainLifeEffect::you(3)
            .execute(&mut game, &mut ctx)
            .expect("gain life should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 23);
        assert_eq!(outcome.as_count(), Some(3));
        let life_gain = outcome
            .events
            .iter()
            .find(|event| event.kind() == EventKind::LifeGain)
            .and_then(|event| event.downcast::<LifeGainEvent>())
            .expect("life gain should emit a LifeGainEvent");
        assert_eq!(life_gain.source, Some(source));
    }

    #[test]
    fn shared_gain_life_payload_uses_replacement_effects() {
        let (mut game, alice) = setup();
        let source = game.new_object_id();
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldGainLifeMatcher::you(),
                ReplacementAction::Modify(EventModification::Add(2)),
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = GainLifeEffect::you(3)
            .execute(&mut game, &mut ctx)
            .expect("gain life should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 25);
        assert_eq!(outcome.as_count(), Some(5));
    }

    #[test]
    fn shared_gain_life_payload_resolves_speed_value() {
        let (mut game, alice) = setup();
        game.increase_speed(alice, 3);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = GainLifeEffect::you(crate::effect::Value::Speed(
            crate::target::PlayerFilter::You,
        ))
        .execute(&mut game, &mut ctx)
        .expect("speed-based life gain should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 23);
        assert_eq!(outcome.as_count(), Some(3));
    }

    #[test]
    fn shared_gain_life_payload_respects_life_gain_prevention() {
        let (mut game, alice) = setup();
        let source = game.new_object_id();
        game.effect_store
            .replacement_effects
            .add_resolution_effect(ReplacementEffect::cant_gain_life(source, alice));
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = GainLifeEffect::you(3)
            .execute(&mut game, &mut ctx)
            .expect("gain life should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 20);
        assert_eq!(outcome.as_count(), Some(0));
        assert!(
            outcome.events.is_empty(),
            "prevented life gain should not emit a LifeGainEvent"
        );
    }
}
