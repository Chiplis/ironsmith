//! Gain life effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, CostValidationError, ExecutionContext, ExecutionError};
#[cfg(test)]
use crate::events::LifeGainEvent;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
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
fn prepare_proposal(
    effect: &GainLifeEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<super::life_change::LifeChangeProposal, ExecutionError> {
    let player = resolve_player_from_spec(game, &effect.player, ctx)?;
    let amount = resolve_value(game, &effect.amount, ctx)?.max(0) as u32;
    Ok(super::life_change::LifeChangeProposal::gain(player, amount))
}

impl EffectExecutor for GainLifeEffect {
    fn replacement_original_event(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<Option<crate::events::Event>, ExecutionError> {
        Ok(Some(prepare_proposal(self, game, ctx)?.event(ctx)))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        game.try_update_static_ability_effects(Default::default())
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let event = self.replacement_original_event(game, ctx)?.ok_or_else(|| {
            ExecutionError::InternalError("life gain has no nominal event".into())
        })?;
        super::life_change::execute_life_change_with_outputs(game, ctx, event)
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
        Ok(Box::new(prepare_proposal(self, game, ctx)?))
    }

    fn target_description(&self) -> &'static str {
        "player to gain life"
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }
}

fn validate_gain_cost(
    effect: &GainLifeEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<(), CostValidationError> {
    let recipient = resolve_player_from_spec(game, &effect.player, ctx).map_err(|error| {
        CostValidationError::Other(format!("life-gain cost has no eligible recipient: {error}"))
    })?;
    if !game.can_gain_life(recipient) {
        return Err(CostValidationError::Other(
            "required player can't gain life".to_string(),
        ));
    }
    Ok(())
}

impl CostExecutableEffect for GainLifeEffect {
    fn supports_prepared_payment(&self) -> bool {
        true
    }

    fn accepts_prepared_payment(
        &self,
        proposal: &dyn crate::effects::SimultaneousEffectProposal,
    ) -> bool {
        proposal.nominal_payment_quantity().is_some()
            && proposal.declared_payment_resources().is_empty()
    }

    fn prepare_simultaneous_payment(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let checked = game
            .continuous_query_snapshot()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        validate_gain_cost(self, &checked, ctx).map_err(|error| {
            ExecutionError::Impossible(format!("life-gain payment unavailable: {error:?}"))
        })?;
        Ok(Box::new(prepare_proposal(self, &checked, ctx)?))
    }

    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        _reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        validate_gain_cost(self, game, ctx)
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        validate_gain_cost(self, game, &ctx)
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

#[cfg(test)]
mod replacement_life_observation_identity_contract_tests {
    use super::*;
    use crate::events::{EventKind, life::matchers::WouldGainLifeMatcher};
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};

    fn three_gains() -> (GameState, EffectOutcome, PlayerId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::new(), "Observation source")
                .card_types(vec![crate::types::CardType::Enchantment]).build(),
            alice, crate::zone::Zone::Battlefield,
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(source, alice, WouldGainLifeMatcher::you(),
                ReplacementAction::Additionally(vec![
                    crate::effect::Effect::new(GainLifeEffect::you(2)),
                    crate::effect::Effect::new(GainLifeEffect::you(3)),
                ])),
        );
        let proposal = game.alloc_child_event_provenance(
            crate::provenance::ProvNodeId::default(), EventKind::LifeGain,
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.provenance = proposal;
        let outcome = crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(GainLifeEffect::you(5)), &mut ctx).expect("all three life changes execute");
        assert_eq!(game.player(alice).unwrap().life, 30);
        assert_eq!(outcome.as_count(), Some(5), "the original amount stays separate from additions");
        let mut amounts = outcome.events.iter().filter_map(|event|
            event.downcast::<LifeGainEvent>().map(|gain| gain.amount)).collect::<Vec<_>>();
        amounts.sort_unstable();
        assert_eq!(amounts, vec![2, 3, 5]);
        (game, outcome, alice)
    }

    #[test]
    fn physical_gains_have_distinct_observation_identities() {
        let (_, outcome, _) = three_gains();
        let identities = outcome.events.iter().filter(|event| event.kind() == EventKind::LifeGain)
            .map(|event| event.provenance()).collect::<std::collections::HashSet<_>>();
        assert_eq!(identities.len(), 3, "three physical gains must not share one publication identity");
    }

    #[test]
    fn history_counts_all_physical_gains_from_one_replacement_payload() {
        let (game, _, alice) = three_gains();
        assert_eq!(game.turn_store.turn_history.total_life_gained_for_players(&[alice]), 10,
            "history must retain the original gain and both added gains");
    }

    #[test]
    fn republishing_same_physical_gain_does_not_duplicate_history() {
        let (mut game, outcome, alice) = three_gains();
        for _ in 0..2 {
            for event in &outcome.events {
                game.turn_store.turn_history.stage_event(event, None, None);
            }
        }
        assert_eq!(game.turn_store.turn_history.total_life_gained_for_players(&[alice]), 10);
    }
}

#[cfg(test)]
mod replacement_life_loss_observation_identity_contract_tests {
    use super::*;
    use crate::effects::LoseLifeEffect;
    use crate::events::LifeLossEvent;
    use crate::events::{EventKind, life::matchers::WouldLoseLifeMatcher};
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};

    fn three_losses() -> (GameState, EffectOutcome, PlayerId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::new(), "Observation source")
                .card_types(vec![crate::types::CardType::Enchantment]).build(),
            alice, crate::zone::Zone::Battlefield,
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(source, alice, WouldLoseLifeMatcher::you(),
                ReplacementAction::Additionally(vec![
                    crate::effect::Effect::new(LoseLifeEffect::you(2)),
                    crate::effect::Effect::new(LoseLifeEffect::you(3)),
                ])),
        );
        let proposal = game.alloc_child_event_provenance(
            crate::provenance::ProvNodeId::default(), EventKind::LifeLoss,
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.provenance = proposal;
        let outcome = crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(LoseLifeEffect::you(5)), &mut ctx).expect("all three life changes execute");
        assert_eq!(game.player(alice).unwrap().life, 10);
        assert_eq!(outcome.as_count(), Some(5), "the original amount stays separate from additions");
        let mut amounts = outcome.events.iter().filter_map(|event|
            event.downcast::<LifeLossEvent>().map(|gain| gain.amount)).collect::<Vec<_>>();
        amounts.sort_unstable();
        assert_eq!(amounts, vec![2, 3, 5]);
        (game, outcome, alice)
    }

    #[test]
    fn physical_losses_have_distinct_observation_identities() {
        let (_, outcome, _) = three_losses();
        let identities = outcome.events.iter().filter(|event| event.kind() == EventKind::LifeLoss)
            .map(|event| event.provenance()).collect::<std::collections::HashSet<_>>();
        assert_eq!(identities.len(), 3, "three physical losses must not share one publication identity");
    }

    #[test]
    fn history_counts_all_physical_losses_from_one_replacement_payload() {
        let (game, _, alice) = three_losses();
        assert_eq!(game.turn_store.turn_history.total_life_lost_for_players(&[alice]), 10,
            "history must retain the original loss and both added losses");
    }

    #[test]
    fn republishing_same_physical_loss_does_not_duplicate_history() {
        let (mut game, outcome, alice) = three_losses();
        for _ in 0..2 {
            for event in &outcome.events {
                game.turn_store.turn_history.stage_event(event, None, None);
            }
        }
        assert_eq!(game.turn_store.turn_history.total_life_lost_for_players(&[alice]), 10);
    }
}
