//! Cumulative upkeep payment composition.

use crate::decision::{FallbackStrategy, SelectFirstDecisionMaker};
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use ironsmith_core::effect::UpkeepPaymentKind;

#[path = "cumulative_upkeep_action_costs.rs"]
mod action_costs;

pub type CumulativeUpkeepEffect = ironsmith_core::CumulativeUpkeepEffect<Effect>;

fn execute_failure(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<CompletedEffectOutputs, ExecutionError> {
    super::execute_checked_program_with_outputs(game, ctx, effects)
}

fn execute_unpaid_failure(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: crate::ids::PlayerId,
    effects: &[Effect],
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let mut lookback_source_snapshots = game.trigger_source_lookback_snapshots();
    let source_snapshot = game
        .object(ctx.source)
        .map(|object| game.cached_object_snapshot_with_calculated_characteristics(object));
    if let Some(snapshot) = source_snapshot.as_ref()
        && !lookback_source_snapshots
            .iter()
            .any(|candidate| candidate.stable_id == snapshot.stable_id)
    {
        lookback_source_snapshots.push(snapshot.clone());
    }
    let outcome = execute_failure(game, ctx, effects)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let completion = super::publish_keyword_action_completion_receipt(
        game,
        ctx,
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::KeywordActionEvent::new(
                crate::events::other::KeywordActionKind::CumulativeUpkeepNotPaid,
                player,
                ctx.source,
                1,
            )
            .with_snapshot(source_snapshot),
            ctx.provenance,
        )
        .with_lookback_source_snapshots(lookback_source_snapshots),
    )?;
    for event in &completion.outcome.events {
        game.queue_trigger_event(event.provenance(), event.clone());
    }
    let outcome = outcome.append_batch_completion_outputs(completion);
    Ok(outcome)
}

/// The cheapest alternative echo price a battlefield "You may pay {0} rather
/// than pay the echo cost for permanents you control" (Thick-Skinned Goblin)
/// offers for `echo_source` (CR 118.9, 702.30a).
fn echo_cost_alternative(
    game: &GameState,
    echo_source: crate::ids::ObjectId,
) -> Option<crate::mana::ManaCost> {
    use crate::filter::ObjectFilterExt as _;
    let echo_object = game.object(echo_source)?;
    let mut best: Option<crate::mana::ManaCost> = None;
    for &permanent in game.battlefield.iter() {
        let Some(holder) = game.object(permanent) else {
            continue;
        };
        let Some(characteristics) = game.current_characteristics(permanent) else {
            continue;
        };
        for static_ability in characteristics.static_abilities.iter() {
            let Some(ironsmith_core::StaticAbilityPayload::EchoCostAlternative {
                filter,
                replacement_mana_cost,
                ..
            }) = static_ability.compiled_model().map(|model| &model.payload)
            else {
                continue;
            };
            if !static_ability.is_active(game, permanent) {
                continue;
            }
            let filter_ctx = game.filter_context_for(game.controller_of(holder), Some(permanent));
            if !filter.matches(echo_object, &filter_ctx, game) {
                continue;
            }
            if best
                .as_ref()
                .is_none_or(|current| replacement_mana_cost.mana_value() < current.mana_value())
            {
                best = Some(replacement_mana_cost.clone());
            }
        }
    }
    best
}

fn payment_can_complete(
    effects: &[Effect],
    count: usize,
    reason: crate::costs::PaymentReason,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    if let Some(action) = action_costs::ActionCost::read(effects) {
        return action.can_pay(game, ctx, count);
    }
    let mut simulated_game = game.clone();
    let query =
        crate::effects::tokens::resources::TokenQueryScope::new(game.token_creation_limits());
    simulated_game.bind_token_query_meter(query.meter());
    let mut simulated_dm = SelectFirstDecisionMaker;
    let mut simulated_ctx = ExecutionContext::new_default(ctx.source, ctx.controller)
        .with_decision_maker(&mut simulated_dm);
    crate::effects::ExecutionContextCheckpoint::capture(ctx).restore(&mut simulated_ctx);
    simulated_ctx.mana.payment_reason = Some(reason);
    simulated_ctx.prospective_cost_payment = true;

    for _ in 0..count {
        let outcome = match super::sequence::execute_checked_payment_program_with_outputs(
            &mut simulated_game,
            &mut simulated_ctx,
            effects,
        ) {
            Err(ExecutionError::Impossible(_)) => return Ok(false),
            other => other?,
        };
        if outcome.outcome.status.is_failure() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn execute_payment_atomically(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    count: usize,
    reason: crate::costs::PaymentReason,
) -> Result<Option<CompletedEffectOutputs>, ExecutionError> {
    super::compound::execute_optional_transaction(game, ctx, |game, ctx| {
        let previous_reason = ctx.mana.payment_reason;
        ctx.mana.payment_reason = Some(reason);
        if let Some(action) = action_costs::ActionCost::read(effects) {
            let previous_cause = ctx.cause.clone();
            ctx.cause.cause_type = crate::events::cause::CauseType::Cost;
            let result = action.pay(game, ctx, count);
            ctx.cause = previous_cause;
            ctx.mana.payment_reason = previous_reason;
            return result.map(Some);
        }
        let mut outcomes = Vec::new();

        for _ in 0..count {
            let outcome = match super::sequence::execute_checked_payment_program_with_outputs(
                game, ctx, effects,
            ) {
                Err(ExecutionError::Impossible(_)) => return Ok(None),
                other => other?,
            };
            let status = outcome.outcome.status;
            outcomes.push(outcome);
            if ctx.decision_maker.awaiting_choice() || status.is_failure() {
                return Ok(None);
            }
        }

        ctx.mana.payment_reason = previous_reason;
        Ok(Some(CompletedEffectOutputs::from_children(
            outcomes,
            EffectOutcome::aggregate_summing_counts,
        )))
    })
}

impl EffectExecutor for CumulativeUpkeepEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.payment {
            visitor(effect);
        }
        for effect in &self.failure {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(Vec::new())),
            |game, ctx| {
                game.refresh_continuous_state()
                    .map_err(ExecutionError::ContinuousDiscovery)?;
                let player = resolve_player_filter(game, &self.player, ctx)?;
                let count = match self.kind {
                    UpkeepPaymentKind::Echo => 1,
                    UpkeepPaymentKind::Cumulative => resolve_value(
                        game,
                        &crate::effect::Value::CountersOnSource(CounterType::Age),
                        ctx,
                    )?
                    .max(0) as usize,
                };
                let reason = if self.kind == UpkeepPaymentKind::Cumulative {
                    crate::costs::PaymentReason::CumulativeUpkeep
                } else {
                    crate::costs::PaymentReason::Effect
                };
                // CR 118.9: an echo-cost alternative ("you may pay {0} rather
                // than pay the echo cost", Thick-Skinned Goblin) replaces the
                // echo payment. A free alternative is always at least as
                // good; a priced one is offered as a choice when both work.
                let alternative = if self.kind == UpkeepPaymentKind::Echo {
                    echo_cost_alternative(game, ctx.source)
                } else {
                    None
                };
                let alternative_payment = alternative.map(|mana| {
                    if mana.is_empty() {
                        Vec::new()
                    } else {
                        vec![Effect::new(crate::effects::PayManaEffect::new(
                            mana,
                            crate::target::ChooseSpec::SourceController,
                        ))]
                    }
                });
                let payment: std::borrow::Cow<'_, [Effect]> = match alternative_payment {
                    Some(alternative)
                        if alternative.is_empty()
                            || !payment_can_complete(&self.payment, count, reason, game, ctx)? =>
                    {
                        std::borrow::Cow::Owned(alternative)
                    }
                    Some(alternative) => {
                        let use_alternative =
                            payment_can_complete(&alternative, count, reason, game, ctx)?
                                && make_boolean_decision(
                                    game,
                                    &mut ctx.decision_maker,
                                    player,
                                    ctx.source,
                                    "Pay the alternative cost rather than the echo cost?",
                                    FallbackStrategy::Accept,
                                );
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        if use_alternative {
                            std::borrow::Cow::Owned(alternative)
                        } else {
                            std::borrow::Cow::Borrowed(self.payment.as_slice())
                        }
                    }
                    None => std::borrow::Cow::Borrowed(self.payment.as_slice()),
                };
                // A cost of zero still offers a choice (CR 118.5, 702.24a).
                let can_attempt = payment_can_complete(&payment, count, reason, game, ctx)?;
                let wants_to_pay = can_attempt
                    && make_boolean_decision(
                        game,
                        &mut ctx.decision_maker,
                        player,
                        ctx.source,
                        if self.kind == UpkeepPaymentKind::Echo {
                            "Pay echo cost?".into()
                        } else {
                            format!("Pay cumulative upkeep {count} time(s)?")
                        },
                        FallbackStrategy::Accept,
                    );

                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if !wants_to_pay {
                    if game.controller_of_id(ctx.source) != Some(player) {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    return if self.kind == UpkeepPaymentKind::Cumulative {
                        execute_unpaid_failure(game, ctx, player, &self.failure)
                    } else {
                        execute_failure(game, ctx, &self.failure)
                    };
                }

                let source_before_payment =
                    crate::snapshot::ObjectSnapshot::from_object_id(game, ctx.source)
                        .or_else(|| ctx.source_snapshot.clone());
                let Some(mut outcome) =
                    execute_payment_atomically(game, ctx, &payment, count, reason)?
                else {
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    return if self.kind == UpkeepPaymentKind::Cumulative {
                        execute_unpaid_failure(game, ctx, player, &self.failure)
                    } else {
                        execute_failure(game, ctx, &self.failure)
                    };
                };
                game.refresh_continuous_state()
                    .map_err(ExecutionError::ContinuousDiscovery)?;
                // The entire optional keyword cost has now succeeded. A successful
                // zero cost still represents the player's affirmative acknowledgement.
                // The receipt is not emitted by affordability simulation or installments.
                let action = match self.kind {
                    UpkeepPaymentKind::Cumulative => {
                        crate::events::KeywordActionKind::CumulativeUpkeepPaid
                    }
                    UpkeepPaymentKind::Echo => crate::events::KeywordActionKind::EchoCostPaid,
                };
                let snapshot = crate::snapshot::ObjectSnapshot::from_object_id(game, ctx.source)
                    .or(source_before_payment);
                outcome = super::complete_keyword_action_with_outputs(
                    game,
                    ctx,
                    outcome,
                    crate::events::other::KeywordActionEvent::new(action, player, ctx.source, 1)
                        .with_snapshot(snapshot),
                )?;
                crate::effects::runtime::capture_triggers_before_added_program(
                    game,
                    ctx,
                    None,
                    outcome.outcome.events.iter_mut(),
                )?;
                outcome.synchronize_observations();
                Ok(outcome)
            },
        )
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.payment, &self.failure])
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.payment, &self.failure])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.payment, &self.failure], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.payment, &self.failure])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    struct BooleanDecisionMaker {
        response: bool,
    }

    impl DecisionMaker for BooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.response
        }
    }

    struct ScriptedBooleanDecisionMaker {
        responses: Vec<bool>,
        index: usize,
    }

    impl ScriptedBooleanDecisionMaker {
        fn new(responses: Vec<bool>) -> Self {
            Self {
                responses,
                index: 0,
            }
        }
    }

    impl DecisionMaker for ScriptedBooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            let response = self.responses.get(self.index).copied().unwrap_or(false);
            self.index += 1;
            response
        }
    }

    #[derive(Debug, Clone)]
    struct BooleanGatedLoseLifePayment;

    impl EffectExecutor for BooleanGatedLoseLifePayment {
        fn clone_box(&self) -> Box<dyn EffectExecutor> {
            Box::new(self.clone())
        }

        fn execute(
            &self,
            game: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            let accepted = make_boolean_decision(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                ctx.source,
                "Pay 1 life for this age counter?",
                FallbackStrategy::Accept,
            );
            if !accepted {
                return Ok(EffectOutcome::impossible());
            }

            game.player_mut(ctx.controller)
                .expect("controller exists")
                .lose_life(1);
            Ok(EffectOutcome::count(1))
        }
    }

    fn source_with_age_counters(
        game: &mut GameState,
        controller: PlayerId,
        count: u32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Cumulative Permanent")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&card, controller, Zone::Battlefield);
        game.object_mut(source)
            .expect("source exists")
            .add_counters(CounterType::Age, count);
        source
    }

    /// Thick-Skinned Goblin: "You may pay {0} rather than pay the echo cost
    /// for permanents you control" replaces the echo payment (CR 118.9,
    /// 702.30a), so the permanent stays without its echo cost being paid.
    #[test]
    fn echo_cost_alternative_replaces_the_echo_payment() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = source_with_age_counters(&mut game, alice, 0);
        let goblin_card = CardBuilder::new(CardId::new(), "Echo Alternative Holder")
            .card_types(vec![CardType::Creature])
            .build();
        let goblin = game.create_object_from_card(&goblin_card, alice, Zone::Battlefield);
        game.object_mut(goblin)
            .expect("holder exists")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::from_model(
                    crate::static_abilities::CompiledStaticAbility::echo_cost_alternative(
                        crate::target::ObjectFilter::permanent().you_control(),
                        crate::mana::ManaCost::new(),
                        "You may pay {0} rather than pay the echo cost for permanents you control",
                    ),
                ),
            ));
        game.refresh_continuous_state().expect("continuous state");
        assert_eq!(
            echo_cost_alternative(&game, source).map(|mana| mana.is_empty()),
            Some(true)
        );

        let mut dm = BooleanDecisionMaker { response: true };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = CumulativeUpkeepEffect::echo(
            PlayerFilter::You,
            vec![Effect::lose_life_player(3, PlayerFilter::You)],
            vec![Effect::sacrifice_source()],
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("echo resolves");

        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert!(game.battlefield.contains(&source));
    }

    #[test]
    fn cumulative_upkeep_runs_payment_once_per_age_counter() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = source_with_age_counters(&mut game, alice, 2);
        let mut dm = BooleanDecisionMaker { response: true };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![Effect::lose_life_player(1, PlayerFilter::You)],
            vec![Effect::sacrifice_source()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(game.player(alice).expect("alice").life, 18);
        assert!(game.battlefield.contains(&source));
    }

    #[test]
    fn cumulative_upkeep_sacrifices_source_when_declined() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = source_with_age_counters(&mut game, alice, 1);
        let mut dm = BooleanDecisionMaker { response: false };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![Effect::lose_life_player(1, PlayerFilter::You)],
            vec![Effect::sacrifice_source()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert!(!game.battlefield.contains(&source));
        let events = game.take_pending_trigger_events();
        let unpaid = events
            .iter()
            .find_map(|event| {
                event
                    .downcast::<crate::events::other::KeywordActionEvent>()
                    .filter(|event| {
                        event.action
                            == crate::events::other::KeywordActionKind::CumulativeUpkeepNotPaid
                    })
            })
            .expect("declining cumulative upkeep should emit a typed unpaid action");
        assert_eq!(unpaid.player, alice);
        assert_eq!(unpaid.source, source);
        assert!(
            events.iter().any(|event| {
                event
                    .lookback_source_snapshots()
                    .iter()
                    .any(|snapshot| snapshot.object_id == source)
            }),
            "unpaid cumulative upkeep event should retain pre-sacrifice trigger-source LKI"
        );
    }

    #[test]
    fn cumulative_upkeep_declined_after_control_change_does_not_sacrifice_source() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = source_with_age_counters(&mut game, alice, 1);
        game.set_current_controller(source, bob).expect("finite controller fixture must refresh successfully");
        let mut dm = BooleanDecisionMaker { response: false };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![Effect::lose_life_player(1, PlayerFilter::You)],
            vec![Effect::sacrifice_source()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert!(game.battlefield.contains(&source));
        assert_eq!(game.controller_of_id(source), Some(bob));
    }

    #[test]
    fn cumulative_upkeep_rolls_back_partial_payment_before_sacrificing_source() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = source_with_age_counters(&mut game, alice, 2);
        let mut dm = ScriptedBooleanDecisionMaker::new(vec![
            true,  // choose to pay cumulative upkeep
            true,  // first per-counter payment succeeds
            false, // second per-counter payment fails
        ]);
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = CumulativeUpkeepEffect::new(
            PlayerFilter::You,
            vec![Effect::new(BooleanGatedLoseLifePayment)],
            vec![Effect::sacrifice_source()],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(
            game.player(alice).expect("alice").life,
            20,
            "partial cumulative upkeep payments must not be kept"
        );
        assert!(!game.battlefield.contains(&source));
    }
}

#[cfg(test)]
mod resource_failure_tests {
    use super::*;
    #[test]
    fn simulated_payment_exhaustion_does_not_sacrifice_or_report_nonpayment() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = crate::ids::PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Upkeep resource fixture")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, player, crate::zone::Zone::Battlefield);
        let hand = game.create_object_from_card(&card, player, crate::zone::Zone::Hand);
        game.object_mut(source).unwrap().add_counters(CounterType::Age, 1);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
            source, player, crate::events::cards::matchers::WouldDiscardMatcher::you(),
            crate::replacement::ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::new(crate::effects::CreateTokenEffect::you(crate::cards::tokens::treasure_token_definition(), 2))]),
        ));
        game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() });
        game.take_pending_trigger_events(); let next = game.next_object_id_counter();
        let effect = CumulativeUpkeepEffect::new(crate::target::PlayerFilter::You, vec![Effect::discard(1)], vec![Effect::sacrifice_source()]);
        let mut ctx = ExecutionContext::new_default(source, player);
        assert!(matches!(effect.execute(&mut game, &mut ctx), Err(ExecutionError::ResourceLimitExceeded { .. })));
        assert!(game.battlefield.contains(&source)); assert_eq!(game.object(hand).unwrap().zone, crate::zone::Zone::Hand);
        assert_eq!(game.player(player).unwrap().life, 20); assert_eq!(game.next_object_id_counter(), next);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert!(game.take_pending_trigger_events().is_empty());
    }
}
