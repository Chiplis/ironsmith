//! Pay energy effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{NumberSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, ExecutionFact, Value};
use crate::effects::executor_trait::CostValidationError;
use crate::effects::helpers::{resolve_nonnegative_u32, resolve_player_from_spec};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::{ChooseSpec, PlayerFilter};
pub type PayEnergyEffect = ironsmith_core::PayEnergyEffect;
pub type PayAnyEnergyEffect = ironsmith_core::PayAnyEnergyEffect;
pub type PayAnyLifeEffect = ironsmith_core::PayAnyLifeEffect;

impl EffectExecutor for PayEnergyEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        let amount = resolve_nonnegative_u32(game, &self.amount, ctx)?;

        if game
            .player(player_id)
            .is_none_or(|player| player.energy_counters < amount)
        {
            return Ok(EffectOutcome::impossible());
        }
        if amount == 0 {
            return Ok(EffectOutcome::count(0));
        }

        if let Some((removed, event)) = game.remove_player_counters_with_source(
            player_id,
            CounterType::Energy,
            amount,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            return Ok(EffectOutcome::count(i64::from(removed)).with_event(event));
        }

        Ok(EffectOutcome::impossible())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.player.is_target() {
            Some(&self.player)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "player to pay energy"
    }

    fn references_cost_x(&self) -> bool {
        matches!(self.amount.unhinted(), Value::X)
    }

    fn max_cost_x(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> Option<u32> {
        if !self.references_cost_x() {
            return None;
        }
        let ctx = ExecutionContext::new_default(source, controller);
        let payer = resolve_player_from_spec(game, &self.player, &ctx).ok()?;
        Some(game.player(payer)?.energy_counters)
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.player, ChooseSpec::Player(PlayerFilter::You))
            && let Value::Fixed(amount) = self.amount
        {
            if amount > 5 {
                let amount_text = u32::try_from(amount)
                    .ok()
                    .and_then(ironsmith_core::cardinal_word)
                    .unwrap_or_else(|| amount.to_string());
                return Some(format!("Pay {amount_text} {{E}}"));
            }
            let symbols: String = (0..amount.max(0)).map(|_| "{E}").collect();
            return Some(format!("Pay {}", symbols));
        }
        None
    }
}

impl CostExecutableEffect for PayEnergyEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller).with_x(0);
        let payer = resolve_player_from_spec(game, &self.player, &ctx).map_err(|_| {
            CostValidationError::Other("unable to resolve player for energy cost".to_string())
        })?;
        let needed = resolve_nonnegative_u32(game, &self.amount, &ctx).map_err(|_| {
            CostValidationError::Other("unable to resolve energy amount".to_string())
        })?;
        let Some(player) = game.player(payer) else {
            return Err(CostValidationError::Other(
                "unable to resolve payer".to_string(),
            ));
        };
        if player.energy_counters >= needed {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "not enough energy counters".to_string(),
            ))
        }
    }
}

impl EffectExecutor for PayAnyEnergyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        let available = game
            .player(player_id)
            .map(|player| player.energy_counters)
            .unwrap_or(0);

        if available < self.min_amount {
            return Ok(EffectOutcome::count(0));
        }

        let number_spec = if self.min_amount == 0 {
            NumberSpec::up_to(ctx.source, available, "Choose how much {E} to pay")
        } else {
            NumberSpec::range(
                ctx.source,
                self.min_amount,
                available,
                "Choose how much {E} to pay",
            )
        };

        let chosen = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            number_spec,
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let chosen = chosen.clamp(self.min_amount, available);

        if chosen == 0 {
            return Ok(EffectOutcome::count(0).with_execution_fact(ExecutionFact::ChosenNumber(0)));
        }

        if let Some((removed, event)) = game.remove_player_counters_with_source(
            player_id,
            CounterType::Energy,
            chosen,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            return Ok(EffectOutcome::count(i64::from(removed))
                .with_event(event)
                .with_execution_fact(ExecutionFact::ChosenNumber(removed)));
        }

        Ok(EffectOutcome::count(0))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.player.is_target() {
            Some(&self.player)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "player to pay energy"
    }
}

impl EffectExecutor for PayAnyLifeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        if !game.can_lose_life(player_id) && self.min_amount > 0 {
            return Ok(EffectOutcome::impossible());
        }
        let available = game
            .player(player_id)
            .map(|player| player.life)
            .unwrap_or(0);
        let available = if game.can_lose_life(player_id) {
            available.max(0) as u32
        } else {
            0
        };

        if available < self.min_amount {
            return Ok(EffectOutcome::count(0));
        }

        let number_spec = if self.min_amount == 0 {
            NumberSpec::up_to(ctx.source, available, "Choose how much life to pay")
        } else {
            NumberSpec::range(
                ctx.source,
                self.min_amount,
                available,
                "Choose how much life to pay",
            )
        };

        let chosen = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            number_spec,
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let chosen = chosen.clamp(self.min_amount, available);

        let Some(outcome) = game.pay_life_with_context(player_id, chosen, ctx)? else {
            return Ok(EffectOutcome::count(0));
        };
        Ok(outcome.with_execution_fact(ExecutionFact::ChosenNumber(chosen)))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
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
        let checked = game
            .continuous_query_snapshot()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let game = &checked;
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        if !game.can_lose_life(player_id) && self.min_amount > 0 {
            return Ok(Box::new(PayAnyLifeProposal {
                player: player_id,
                amount: 0,
                acknowledged: false,
                prepared: None,
            }));
        }
        let available = game
            .player(player_id)
            .map(|player| player.life)
            .unwrap_or(0)
            .max(0) as u32;
        let available = if game.can_lose_life(player_id) {
            available
        } else {
            0
        };
        if available < self.min_amount {
            return Ok(Box::new(PayAnyLifeProposal {
                player: player_id,
                amount: 0,
                acknowledged: false,
                prepared: None,
            }));
        }
        let number_spec = if self.min_amount == 0 {
            NumberSpec::up_to(ctx.source, available, "Choose how much life to pay")
        } else {
            NumberSpec::range(
                ctx.source,
                self.min_amount,
                available,
                "Choose how much life to pay",
            )
        };
        let chosen = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            number_spec,
            FallbackStrategy::Maximum,
        );
        let amount = if ctx.decision_maker.awaiting_choice() {
            0
        } else {
            chosen.clamp(self.min_amount, available)
        };
        Ok(Box::new(PayAnyLifeProposal {
            player: player_id,
            amount,
            acknowledged: !ctx.decision_maker.awaiting_choice(),
            prepared: None,
        }))
    }

    fn target_description(&self) -> &'static str {
        "player to pay life"
    }
}

/// One player's declared life payment for a simultaneous each-player round;
/// the amount was chosen against pre-round state and commits atomically with
/// the other players' payments.
struct PayAnyLifeProposal {
    player: crate::ids::PlayerId,
    amount: u32,
    acknowledged: bool,
    prepared: Option<crate::game_state::PreparedLifePayment>,
}
impl std::fmt::Debug for PayAnyLifeProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PayAnyLifeProposal")
            .field("player", &self.player)
            .field("amount", &self.amount)
            .finish_non_exhaustive()
    }
}
impl crate::effects::SimultaneousEffectProposal for PayAnyLifeProposal {
    fn declared_life_payment(&self) -> Option<(crate::ids::PlayerId, u32)> {
        self.acknowledged.then_some((self.player, self.amount))
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.acknowledged {
            self.prepared = game.prepare_life_payment(self.player, self.amount, ctx, true)?;
        }
        Ok(())
    }
    fn commit_original(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        if !self.acknowledged {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                EffectOutcome::count(0).with_execution_fact(ExecutionFact::ChosenNumber(0)),
            ));
        }
        if self.prepared.is_none() {
            self.prepare_original(game, ctx)?;
        }
        let prepared = self.prepared.take().ok_or_else(|| {
            ExecutionError::UnresolvableValue("prepared life payment is unavailable".into())
        })?;
        let mut receipt = game.commit_life_payment_original(prepared, ctx)?;
        receipt.outcome = receipt
            .outcome
            .with_execution_fact(ExecutionFact::ChosenNumber(self.amount));
        Ok(receipt)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let outcome = if self.acknowledged {
            game.pay_life_with_context(self.player, self.amount, ctx)?
                .unwrap_or_else(EffectOutcome::impossible)
        } else {
            EffectOutcome::count(0)
        };
        Ok(outcome.with_execution_fact(ExecutionFact::ChosenNumber(self.amount)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventKind;
    use crate::ids::PlayerId;
    use crate::target::{ChooseSpec, PlayerFilter};

    #[test]
    fn pay_energy_effect_emits_markers_changed_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice exists")
            .energy_counters = 4;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = PayEnergyEffect::new(2, ChooseSpec::Player(PlayerFilter::You))
            .execute(&mut game, &mut ctx)
            .expect("pay energy should resolve");

        assert_eq!(game.player(alice).expect("alice exists").energy_counters, 2);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::MarkersChanged),
            "paying energy should emit MarkersChangedEvent"
        );
    }

    #[test]
    fn large_fixed_energy_cost_uses_counted_oracle_surface() {
        let player = ChooseSpec::Player(PlayerFilter::You);
        assert_eq!(
            PayEnergyEffect::new(5, player.clone())
                .cost_description()
                .as_deref(),
            Some("Pay {E}{E}{E}{E}{E}")
        );
        assert_eq!(
            PayEnergyEffect::new(6, player.clone())
                .cost_description()
                .as_deref(),
            Some("Pay six {E}")
        );
        assert_eq!(
            PayEnergyEffect::new(50, player)
                .cost_description()
                .as_deref(),
            Some("Pay fifty {E}")
        );
    }

    #[test]
    fn pay_any_life_respects_cant_lose_life() {
        struct PayMaximum;

        impl crate::decision::DecisionMaker for PayMaximum {
            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                ctx.max
            }
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let prohibition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Life payment prohibition",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::restriction(
                crate::effect::Restriction::LoseLife(PlayerFilter::You),
                "You can't lose life".into(),
            ),
        ))
        .build();
        game.create_object_from_definition(&prohibition, alice, crate::zone::Zone::Battlefield);

        let source = game.new_object_id();
        let mut dm = PayMaximum;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = PayAnyLifeEffect::new(ChooseSpec::Player(PlayerFilter::You), 0)
            .execute(&mut game, &mut ctx)
            .expect("pay any life should resolve");

        assert_eq!(outcome.as_count(), Some(0));
        assert_eq!(game.player(alice).expect("alice exists").life, 20);
    }
}

#[cfg(test)]
mod unsigned_energy_payment_public_contract_tests {
    use crate::effect::{EffectId, Value};
    use crate::effects::{
        CostExecutableEffect, EffectContext, EnergyCountersEffect, PayEnergyEffect, execute_effect,
    };
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::{CardId, Effect, GameState, PlayerId, Zone};
    fn seeded(amount: u32) -> (GameState, crate::ObjectId, PlayerId) {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Energy payment result owner",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(source, alice);
        let out = execute_effect(
            &mut game,
            &Effect::new(EnergyCountersEffect::you(amount)),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(out.as_count(), Some(i64::from(amount)));
        assert_eq!(game.player(alice).unwrap().energy_counters, amount);
        (game, source, alice)
    }
    fn dynamic(amount: u32) {
        let (mut game, source, alice) = seeded(amount);
        let mut ctx = EffectContext::new_default(source, alice);
        // Store a second actual completed instruction's result, rather than a fake receipt.
        let other = PlayerId::from_index(1);
        let out = execute_effect(
            &mut game,
            &Effect::new(EnergyCountersEffect::new(
                amount,
                PlayerFilter::Specific(other),
            )),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(out.as_count(), Some(i64::from(amount)));
        ctx.store_outcome(EffectId(31), out);
        let pay = Effect::new(PayEnergyEffect::new(
            Value::EffectValue(EffectId(31)),
            ChooseSpec::Player(PlayerFilter::You),
        ));
        let out = execute_effect(&mut game, &pay, &mut ctx)
            .expect("actual unsigned receipt must remain usable for energy payment");
        assert_eq!(out.as_count(), Some(i64::from(amount)));
        assert_eq!(game.player(alice).unwrap().energy_counters, 0);
        assert_eq!(game.player(other).unwrap().energy_counters, amount);
        ctx.store_outcome(EffectId(57), out);
        let follow = execute_effect(
            &mut game,
            &Effect::new(EnergyCountersEffect::new(
                Value::EffectValue(EffectId(57)),
                PlayerFilter::You,
            )),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(follow.as_count(), Some(i64::from(amount)));
        assert_eq!(game.player(alice).unwrap().energy_counters, amount);
    }
    #[test]
    fn bounded_energy_receipt_control() {
        dynamic(i32::MAX as u32);
    }
    #[test]
    fn unsigned_energy_receipt_above_signed_boundary() {
        dynamic(i32::MAX as u32 + 1);
    }
    #[test]
    fn unsigned_energy_receipt_at_storage_maximum() {
        dynamic(u32::MAX);
    }
    #[test]
    fn unsigned_static_energy_cost_is_legal_with_actual_available_energy() {
        let (game, source, alice) = seeded(u32::MAX);
        let pay = PayEnergyEffect::new(u32::MAX, ChooseSpec::Player(PlayerFilter::You));
        assert!(
            CostExecutableEffect::can_execute_as_cost(&pay, &game, source, alice).is_ok(),
            "natural energy cost must use available unsigned energy"
        );
    }

    #[test]
    fn unsigned_chosen_energy_payment_reports_actual_removed_count() {
        let (mut game, source, alice) = seeded(u32::MAX);
        let mut ctx = EffectContext::new_default(source, alice);
        let effect = Effect::new(crate::effects::PayAnyEnergyEffect::new(
            ChooseSpec::Player(PlayerFilter::You),
            u32::MAX,
        ));
        let out = execute_effect(&mut game, &effect, &mut ctx)
            .expect("forced unsigned quantity is payable");
        assert_eq!(
            game.player(alice).unwrap().energy_counters,
            0,
            "actual unsigned energy was removed"
        );
        assert_eq!(
            out.as_count(),
            Some(i64::from(u32::MAX)),
            "receipt must report actual unsigned removal without signed wrap"
        );
    }
}
