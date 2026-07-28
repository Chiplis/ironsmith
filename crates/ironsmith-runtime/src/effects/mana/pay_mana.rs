//! Pay mana effect implementation.

use crate::ability::ActivatedAbilityRuntimeExt as _;
use crate::decision::{DecisionMaker, FallbackStrategy};
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::decisions::{XValueSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::special_actions::{SpecialAction, can_perform, perform};
use crate::target::{ChooseSpec, PlayerFilter};

/// Effect that asks a player to pay a mana cost.
///
/// Returns `Count(1)` for a fixed or externally defined payment. For a bounded
/// player-chosen X payment, returns `Count(X)` and records the chosen number.
pub type PayManaEffect = ironsmith_core::PayManaEffect;

fn payment_reason(ctx: &ExecutionContext<'_>) -> crate::costs::PaymentReason {
    ctx.mana
        .payment_reason
        .unwrap_or(crate::costs::PaymentReason::Effect)
}

fn try_pay_interactively(
    effect: &PayManaEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    x_value: u32,
) -> Result<bool, ExecutionError> {
    const MAX_PAYMENT_STEPS: usize = 32;
    let payment_reason = payment_reason(ctx);

    for _ in 0..MAX_PAYMENT_STEPS {
        let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
            player_id,
            Some(ctx.source),
            &effect.cost,
            payment_reason,
        );
        let can_pay_now = game.can_pay_mana_cost_with_reason(
            player_id,
            Some(ctx.source),
            &adjusted_cost,
            x_value,
            payment_reason,
        );
        let mana_abilities = get_available_mana_abilities(game, player_id, &mut ctx.decision_maker);

        if !can_pay_now && mana_abilities.is_empty() {
            return Ok(false);
        }
        let mut choices = Vec::new();
        let mut options = Vec::new();

        if can_pay_now {
            choices.push(PayManaChoice::PayNow);
            options.push(SelectableOption::new(choices.len() - 1, "Pay mana cost"));
        }

        for (permanent_id, ability_index, description) in mana_abilities {
            choices.push(PayManaChoice::ActivateManaAbility {
                permanent_id,
                ability_index,
            });
            options.push(SelectableOption::new(
                choices.len() - 1,
                format!(
                    "Tap {}: {}",
                    describe_permanent(game, permanent_id),
                    description
                ),
            ));
        }

        if choices.is_empty() {
            return Ok(false);
        }

        let source_name = game
            .object(ctx.source)
            .map(|obj| obj.name.to_string())
            .unwrap_or_else(|| "effect".to_string());
        let decision_ctx =
            SelectOptionsContext::mana_payment(player_id, ctx.source, source_name, options);
        let selected = ctx.decision_maker.decide_options(game, &decision_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        let Some(selected_idx) = selected.first().copied() else {
            if can_pay_now {
                return Ok(game.try_pay_mana_cost_with_reason(
                    player_id,
                    Some(ctx.source),
                    &adjusted_cost,
                    x_value,
                    payment_reason,
                ));
            }
            return Ok(false);
        };
        let Some(choice) = choices.get(selected_idx).copied() else {
            return Ok(false);
        };

        match choice {
            PayManaChoice::PayNow => {
                return Ok(game.try_pay_mana_cost_with_reason(
                    player_id,
                    Some(ctx.source),
                    &adjusted_cost,
                    x_value,
                    payment_reason,
                ));
            }
            PayManaChoice::ActivateManaAbility {
                permanent_id,
                ability_index,
            } => {
                let action = SpecialAction::ActivateManaAbility {
                    permanent_id,
                    ability_index,
                };

                if perform(action, game, player_id, &mut ctx.decision_maker).is_err() {
                    return Ok(false);
                }
            }
        }
    }

    let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
        player_id,
        Some(ctx.source),
        &effect.cost,
        payment_reason,
    );
    Ok(game.try_pay_mana_cost_with_reason(
        player_id,
        Some(ctx.source),
        &adjusted_cost,
        x_value,
        payment_reason,
    ))
}

fn maximum_affordable_bounded_x(
    effect: &PayManaEffect,
    game: &GameState,
    ctx: &ExecutionContext<'_>,
    player_id: PlayerId,
    semantic_maximum: u32,
) -> Option<u32> {
    let reason = payment_reason(ctx);
    let adjusted_cost =
        game.adjust_mana_cost_for_payment_reason(player_id, Some(ctx.source), &effect.cost, reason);
    let view = crate::derived_view::DerivedGameView::new(game);
    let can_pay = |x_value| {
        view.can_potentially_pay_with_reason(
            player_id,
            Some(ctx.source),
            &adjusted_cost,
            x_value,
            reason,
        )
    };
    if !can_pay(0) {
        return None;
    }

    // Paying a mana cost with a larger X cannot require less mana than paying
    // the same cost with a smaller X, so affordability is monotonic.
    let mut lower = 0;
    let mut upper = semantic_maximum;
    while lower < upper {
        let distance = upper - lower;
        let middle = lower + distance / 2 + distance % 2;
        if can_pay(middle) {
            lower = middle;
        } else {
            upper = middle - 1;
        }
    }
    Some(lower)
}

impl EffectExecutor for PayManaEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_from_spec(game, &self.player, ctx)?;
        let bounded_x = if let Some(maximum) = &self.x_maximum {
            let semantic_maximum = resolve_value(game, maximum, ctx)?.max(0) as u32;
            let Some(affordable_maximum) =
                maximum_affordable_bounded_x(self, game, ctx, player_id, semantic_maximum)
            else {
                return Ok(EffectOutcome::impossible());
            };
            let chosen = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                player_id,
                Some(ctx.source),
                XValueSpec::new(ctx.source, affordable_maximum),
                FallbackStrategy::Maximum,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            Some(chosen.min(affordable_maximum))
        } else {
            None
        };
        let x_value = if let Some(chosen) = bounded_x {
            chosen
        } else {
            self.x_value
                .as_ref()
                .map(|value| resolve_value(game, value, ctx))
                .transpose()?
                .unwrap_or(0)
                .max(0) as u32
        };

        if !try_pay_interactively(self, game, ctx, player_id, x_value)? {
            Ok(EffectOutcome::impossible())
        } else if let Some(chosen) = bounded_x {
            Ok(EffectOutcome::count(chosen as i32)
                .with_execution_fact(ExecutionFact::ChosenNumber(chosen))
                .with_execution_fact(ExecutionFact::ManaPaid { x_value: chosen }))
        } else {
            Ok(EffectOutcome::count(1))
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.player.is_target() {
            Some(&self.player)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "player to pay mana"
    }
}

impl CostExecutableEffect for PayManaEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        let player_id = match self.player.inner() {
            ChooseSpec::Player(PlayerFilter::You | PlayerFilter::EffectController) => controller,
            ChooseSpec::Player(PlayerFilter::Specific(player))
            | ChooseSpec::SpecificPlayer(player) => *player,
            ChooseSpec::SourceController => game
                .object(source)
                .map(|object| game.controller_of(object))
                .unwrap_or(controller),
            _ => controller,
        };
        let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
            player_id,
            Some(source),
            &self.cost,
            crate::costs::PaymentReason::Effect,
        );
        let x_value = if self.x_maximum.is_some() {
            // Zero is always inside a bounded-X range. The semantic maximum
            // may depend on trigger context that cost preflight does not have.
            0
        } else {
            let context = ExecutionContext::new_default(source, controller);
            self.x_value
                .as_ref()
                .map(|value| resolve_value(game, value, &context))
                .transpose()
                .map_err(|_| {
                    CostValidationError::Other("unable to resolve mana payment X value".to_string())
                })?
                .unwrap_or(0)
                .max(0) as u32
        };
        if game.can_pay_mana_cost_with_reason(
            player_id,
            Some(source),
            &adjusted_cost,
            x_value,
            crate::costs::PaymentReason::Effect,
        ) {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "not enough mana available to pay cost".to_string(),
            ))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PayManaChoice {
    PayNow,
    ActivateManaAbility {
        permanent_id: ObjectId,
        ability_index: usize,
    },
}

fn get_available_mana_abilities(
    game: &GameState,
    player: PlayerId,
    decision_maker: &mut &mut dyn DecisionMaker,
) -> Vec<(ObjectId, usize, String)> {
    let mut abilities = Vec::new();

    for &permanent_id in &game.battlefield {
        let Some(permanent) = game.object(permanent_id) else {
            continue;
        };

        if game.controller_of(permanent) != player {
            continue;
        }

        for (ability_index, ability) in permanent.abilities.iter().enumerate() {
            let crate::ability::AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.is_runtime_mana_ability(game, permanent_id, player) {
                continue;
            }

            let action = SpecialAction::ActivateManaAbility {
                permanent_id,
                ability_index,
            };
            if can_perform(&action, game, player, decision_maker).is_err() {
                continue;
            }

            abilities.push((
                permanent_id,
                ability_index,
                describe_mana_ability(game, permanent_id, player, &ability.kind),
            ));
        }
    }

    abilities
}

fn describe_mana_ability(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    kind: &crate::ability::AbilityKind,
) -> String {
    use crate::ability::AbilityKind;
    use crate::mana::ManaSymbol;

    if let AbilityKind::Activated(mana_ability) = kind
        && mana_ability.is_runtime_mana_ability(game, source, controller)
    {
        let produced: Vec<&str> = mana_ability
            .inferred_mana_symbols(game, source, controller)
            .iter()
            .map(|symbol| match symbol {
                ManaSymbol::White => "{W}",
                ManaSymbol::Blue => "{U}",
                ManaSymbol::Black => "{B}",
                ManaSymbol::Red => "{R}",
                ManaSymbol::Green => "{G}",
                ManaSymbol::Colorless => "{C}",
                _ => "mana",
            })
            .collect();
        if produced.is_empty() {
            "Add mana".to_string()
        } else {
            format!("Add {}", produced.join(""))
        }
    } else {
        "Add mana".to_string()
    }
}

fn describe_permanent(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|obj| obj.name.to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::static_abilities::StaticAbility;
    use crate::target::PlayerFilter;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_payment_replacement_permanent(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        ability: StaticAbility,
    ) {
        let source = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source, controller, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("static-ability source should exist")
            .abilities_mut()
            .push(Ability::static_ability(ability));
    }

    #[derive(Default)]
    struct ActivateThenPayDecisionMaker {
        mana_payment_prompts: usize,
    }

    impl DecisionMaker for ActivateThenPayDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description.starts_with("Pay mana for") {
                self.mana_payment_prompts += 1;

                // First prompt: activate a mana ability if available.
                if self.mana_payment_prompts == 1
                    && let Some(activation) = ctx
                        .options
                        .iter()
                        .find(|opt| opt.legal && opt.description != "Pay mana cost")
                {
                    return vec![activation.index];
                }

                if let Some(pay) = ctx
                    .options
                    .iter()
                    .find(|opt| opt.legal && opt.description == "Pay mana cost")
                {
                    return vec![pay.index];
                }
            }

            ctx.options
                .iter()
                .filter(|opt| opt.legal)
                .map(|opt| opt.index)
                .take(ctx.min)
                .collect()
        }
    }

    struct ChooseBoundedXDecisionMaker {
        choice: u32,
        offered_maximum: Option<u32>,
    }

    impl ChooseBoundedXDecisionMaker {
        fn new(choice: u32) -> Self {
            Self {
                choice,
                offered_maximum: None,
            }
        }
    }

    impl DecisionMaker for ChooseBoundedXDecisionMaker {
        fn decide_number(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            assert!(ctx.is_x_value);
            self.offered_maximum = Some(ctx.max);
            self.choice.min(ctx.max)
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn pay_mana_effect_activates_mana_ability_then_pays() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let mountain_def = crate::cards::definitions::basic_mountain();
        let mountain_id =
            game.create_object_from_definition(&mountain_def, alice, Zone::Battlefield);

        let mut dm = ActivateThenPayDecisionMaker::default();
        let mut ctx =
            ExecutionContext::new_default(mountain_id, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Red]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(dm.mana_payment_prompts, 2);
        assert!(game.is_tapped(mountain_id));
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            0
        );
    }

    #[test]
    fn pay_mana_effect_is_impossible_without_mana_sources() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Red]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Impossible);
    }

    #[test]
    fn pay_mana_effect_resolves_typed_x_value_from_source_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::new(), "Counter Payment Source")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.add_counters(source, crate::object::CounterType::PlusOnePlusOne, 2);
        game.player_mut(alice)
            .expect("alice should exist")
            .mana_pool
            .red = 2;

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ChooseSpec::Player(PlayerFilter::You),
        )
        .with_x_value(crate::effect::Value::CountersOnSource(
            crate::object::CounterType::PlusOnePlusOne,
        ));

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("counter-defined X payment should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            0,
            "the X payment should spend one generic mana per source counter"
        );
    }

    #[test]
    fn bounded_x_payment_uses_trigger_amount_and_preserves_chosen_x() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.player_mut(alice)
            .expect("alice should exist")
            .mana_pool
            .red = 5;
        let life_gain = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(alice, 3),
            crate::provenance::ProvNodeId::default(),
        );

        let mut dm = ChooseBoundedXDecisionMaker::new(2);
        let mut ctx =
            ExecutionContext::new(source, alice, &mut dm).with_triggering_event(life_gain);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ChooseSpec::Player(PlayerFilter::You),
        )
        .with_x_maximum(crate::effect::Value::EventValue(
            crate::effect::EventValueSpec::LifeAmount,
        ));

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("bounded X payment should execute");

        assert_eq!(dm.offered_maximum, Some(3));
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::ChosenNumber(2))
        );
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            3
        );
    }

    #[test]
    fn bounded_x_payment_only_offers_affordable_values() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.player_mut(alice)
            .expect("alice should exist")
            .mana_pool
            .red = 2;
        let life_gain = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(alice, 5),
            crate::provenance::ProvNodeId::default(),
        );

        let mut dm = ChooseBoundedXDecisionMaker::new(5);
        let mut ctx =
            ExecutionContext::new(source, alice, &mut dm).with_triggering_event(life_gain);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ChooseSpec::Player(PlayerFilter::You),
        )
        .with_x_maximum(crate::effect::Value::EventValue(
            crate::effect::EventValueSpec::LifeAmount,
        ));

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("bounded X payment should execute");

        assert_eq!(dm.offered_maximum, Some(2));
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .mana_pool
                .red,
            0
        );
    }

    #[test]
    fn bounded_x_payment_of_zero_still_counts_as_completed() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let life_gain = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::LifeGainEvent::new(alice, 3),
            crate::provenance::ProvNodeId::default(),
        );

        let mut dm = ChooseBoundedXDecisionMaker::new(0);
        let mut ctx =
            ExecutionContext::new(source, alice, &mut dm).with_triggering_event(life_gain);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::X]),
            ChooseSpec::Player(PlayerFilter::You),
        )
        .with_x_maximum(crate::effect::Value::EventValue(
            crate::effect::EventValueSpec::LifeAmount,
        ));

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("zero is a legal bounded X payment");

        assert_eq!(dm.offered_maximum, Some(0));
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::ManaPaid { x_value: 0 })
        );
        assert!(
            crate::effect::EffectPredicateRuntimeExt::evaluate_outcome(
                &crate::effect::EffectPredicate::Happened,
                &result,
            ),
            "a successful zero-mana payment must satisfy an if-you-do branch"
        );
    }

    #[test]
    fn pay_mana_effect_can_use_krrik_life_for_black() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Krrik Effect Helper",
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        );

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );
    }

    #[test]
    fn pay_mana_effect_still_can_use_krrik_under_yasharn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Krrik Effect Helper",
            StaticAbility::krrik_black_mana_may_be_paid_with_life(),
        );
        add_payment_replacement_permanent(
            &mut game,
            alice,
            "Yasharn Effect Helper",
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
        );

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = PayManaEffect::new(
            ManaCost::from_symbols(vec![ManaSymbol::Black]),
            ChooseSpec::Player(PlayerFilter::You),
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("pay mana effect should execute");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).expect("alice exists").life, 18);
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.total(),
            0
        );
    }
}
