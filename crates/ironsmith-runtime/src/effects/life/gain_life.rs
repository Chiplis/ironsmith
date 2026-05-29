//! Gain life effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::helpers::{
    resolve_player_filter_to_list, resolve_player_from_spec, resolve_value,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::cause::CauseType;
use crate::events::LifeGainEvent;
use crate::events::processing::process_life_gain_with_event;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::GainLifeEffect;

fn candidate_life_gain_players(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<Vec<PlayerId>, ExecutionError> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => candidate_life_gain_players(game, spec, ctx),
        ChooseSpec::Player(filter)
        | ChooseSpec::EachPlayer(filter)
        | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            let filter_ctx = ctx.filter_context(game);
            resolve_player_filter_to_list(game, filter, &filter_ctx, ctx)
        }
        _ => resolve_player_from_spec(game, spec, ctx).map(|player| vec![player]),
    }
}

fn legal_life_gain_cost_recipients(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<Vec<PlayerId>, ExecutionError> {
    let mut players = candidate_life_gain_players(game, spec, ctx)?;
    players.retain(|player| *player != ctx.controller && game.can_gain_life(*player));
    players.sort_by_key(|player| player.0);
    players.dedup();
    Ok(players)
}

fn resolve_life_gain_cost_recipient(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &mut ExecutionContext,
) -> Result<PlayerId, ExecutionError> {
    let candidates = legal_life_gain_cost_recipients(game, spec, ctx)?;
    if let Some(chosen) = ctx.targets.iter().find_map(|target| match target {
        crate::effects::ResolvedTarget::Player(player) if candidates.contains(player) => {
            Some(*player)
        }
        _ => None,
    }) {
        return Ok(chosen);
    }

    match candidates.as_slice() {
        [player] => Ok(*player),
        [] => Err(ExecutionError::Impossible(
            "no legal life-gain cost recipient".to_string(),
        )),
        _ => {
            let options = candidates
                .iter()
                .filter_map(|player_id| {
                    game.player(*player_id)
                        .map(|player| (player.name.clone(), *player_id))
                })
                .collect::<Vec<_>>();
            crate::decisions::ask_choose_one(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                ctx.source,
                &options,
            )
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "life-gain cost recipient choice is pending".to_string(),
                )
            })
        }
    }
}

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
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = if ctx.cause.cause_type == CauseType::Cost {
            resolve_life_gain_cost_recipient(game, &self.player, ctx)?
        } else {
            resolve_player_from_spec(game, &self.player, ctx)?
        };
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

impl CostExecutableEffect for GainLifeEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        let candidates = legal_life_gain_cost_recipients(game, &self.player, &ctx).map_err(|err| {
            CostValidationError::Other(format!("unable to choose life-gain recipient: {err:?}"))
        })?;
        if candidates.is_empty() {
            return Err(CostValidationError::Other(
                "no legal player can gain life for this cost".to_string(),
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
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::LifeGain),
            "life gain should emit a LifeGainEvent"
        );
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
