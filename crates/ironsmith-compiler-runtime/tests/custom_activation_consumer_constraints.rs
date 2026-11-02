//! Unregistered integration regressions; depends minimum-actual-mana-colors.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::decision::{LegalAction, compute_legal_actions};
use ironsmith::{GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
#[test]
fn zero_price_preserves_spending_obligation_and_cannot_supply_extra_mana() {
    let minimum = ironsmith_core::mana::ManaSpendingRestriction::AtLeastColors(3);
    let zero = ironsmith_core::ManaCost::new().with_spending_restriction(minimum.clone());
    assert!(
        !zero.is_empty(),
        "zero pips do not erase a minimum actual payment obligation"
    );
    let text = "Type: Artifact\n{3}: You gain 1 life. Spend mana of at least three different colors to activate this ability.";
    for original_definition in source_support::definitions(
        &serde_json::json!({"name":"Independent constrained activation","executable_text":text}),
    )
    .unwrap()
    {
        for dynamic_original in [false, true] {
            let mut definition = original_definition.clone();
            if dynamic_original {
                let ironsmith::ability::AbilityKind::Activated(ability) =
                    &mut definition.abilities[0].kind
                else {
                    panic!()
                };
                let base = ability.mana_cost.mana_cost().unwrap().clone();
                ability.mana_cost = ironsmith::cost::TotalCost::from_costs(vec![
                    ironsmith::costs::Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
                        base,
                        None,
                        None,
                        None,
                        ironsmith_core::DynamicManaDisplayHint::Default,
                    )),
                ]);
            }
            for replacement in [
                ironsmith_core::ManaCost::new(),
                ironsmith_core::ManaCost::from_pips(vec![vec![
                    ironsmith::mana::ManaSymbol::Generic(0),
                ]]),
            ] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                game.turn.active_player = A;
                game.turn.priority_player = Some(A);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.apply_custom_status(source, "charged", true);
                let grant = ironsmith_core::GrantActivationCostAlternativeEffect {
                    target: ironsmith::target::ChooseSpec::SpecificObject(source),
                    status: "charged".into(),
                    replacement_cost: replacement,
                    max_uses_per_ability_each_turn: 1,
                    nonloyalty_only: true,
                };
                ironsmith::effects::execute_effect(
                    &mut game,
                    &ironsmith::Effect::new(grant),
                    &mut ironsmith::effects::EffectContext::new_default(source, A),
                )
                .unwrap();
                let legal = |game: &GameState| {
                    compute_legal_actions(game,A).unwrap().into_iter().any(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index:0}if id==source))
                };
                assert!(
                    !legal(&game),
                    "empty price cannot satisfy three actual colors"
                );
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::White, 1);
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Blue, 1);
                assert!(
                    !legal(&game),
                    "two colors do not satisfy the original or replacement branch"
                );
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Black, 1);
                assert!(
                    legal(&game),
                    "the original three-mana branch remains payable with three colors"
                );
            }
        }
    }
}

#[test]
fn zero_pip_obligations_validate_actual_spending_without_inventing_mana() {
    use ironsmith_core::mana::{ManaCost, ManaSpendingRestriction};
    let game = GameState::new(vec!["A".into(), "B".into()], 20);
    let no_mana = ManaCost::new().with_spending_restriction(ManaSpendingRestriction::NoMana);
    assert!(!no_mana.is_empty());
    assert!(
        game.can_pay_mana_cost(A, None, &no_mana, 0),
        "no actual units satisfies no-mana"
    );
    let three_colors =
        ManaCost::new().with_spending_restriction(ManaSpendingRestriction::AtLeastColors(3));
    assert!(
        !game.can_pay_mana_cost(A, None, &three_colors, 0),
        "zero actual units cannot satisfy a color lower bound"
    );
}

#[test]
fn replacement_keeps_common_consumer_rules_without_combining_exclusive_price_rules() {
    use ironsmith::color::{Color, ColorSet};
    use ironsmith::cost::TotalCost;
    use ironsmith_core::mana::{ManaCost, ManaSpendingRestriction};
    let text = "Type: Artifact\n{1}: You gain 1 life.";
    for original in source_support::definitions(
        &serde_json::json!({"name":"Independent price branches","executable_text":text}),
    )
    .unwrap()
    {
        for common_minimum in [false, true] {
            let mut definition = original.clone();
            let amount = if common_minimum { 3 } else { 1 };
            let price = |color| {
                let mut mana =
                    ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(amount)])
                        .with_spending_restriction(ManaSpendingRestriction::Colors(
                            ColorSet::new().with(color),
                        ));
                if common_minimum {
                    mana =
                        mana.with_spending_restriction(ManaSpendingRestriction::AtLeastColors(3));
                }
                TotalCost::mana(mana)
            };
            let ironsmith::ability::AbilityKind::Activated(ability) =
                &mut definition.abilities[0].kind
            else {
                panic!()
            };
            ability.mana_cost = TotalCost::one_of(vec![price(Color::White), price(Color::Blue)]);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.apply_custom_status(source, "charged", true);
            let replacement = if common_minimum {
                ManaCost::new()
            } else {
                ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(1)])
            };
            let grant = ironsmith_core::GrantActivationCostAlternativeEffect {
                target: ironsmith::target::ChooseSpec::SpecificObject(source),
                status: "charged".into(),
                replacement_cost: replacement,
                max_uses_per_ability_each_turn: 1,
                nonloyalty_only: true,
            };
            ironsmith::effects::execute_effect(
                &mut game,
                &ironsmith::Effect::new(grant),
                &mut ironsmith::effects::EffectContext::new_default(source, A),
            )
            .unwrap();
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Red, 1);
            let legal=compute_legal_actions(&game,A).unwrap().into_iter().any(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index:0}if id==source));
            assert_eq!(
                legal, !common_minimum,
                "exclusive price colors do not combine; common three-color consumer requirement survives"
            );
        }
    }
}

#[test]
fn consumer_rider_rendering_terminates_for_fixed_and_dynamic_prices_without_mutating_obligations() {
    use ironsmith_core::mana::ManaSpendingRestriction;
    for definition in source_support::definitions(&serde_json::json!({
        "name":"Render-only price witness", "executable_text":"Type: Artifact\n{3}: You gain 1 life."
    })).unwrap() {
        for rule in [ManaSpendingRestriction::AtLeastColors(3), ManaSpendingRestriction::GenericManaByMinusOneCounters] {
            for dynamic in [false, true] {
                let mut definition = definition.clone();
                let ironsmith::ability::AbilityKind::Activated(ability) = &mut definition.abilities[0].kind else { panic!() };
                let base = ability.mana_cost.mana_cost().unwrap().clone().with_spending_restriction(rule.clone());
                let component = if dynamic {
                    ironsmith::costs::Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(base, None, None, None, ironsmith_core::DynamicManaDisplayHint::Default))
                } else { ironsmith::costs::Cost::mana(base) };
                ability.mana_cost = ironsmith::cost::TotalCost::from_costs(vec![component]);
                let before = ability.mana_cost.clone();
                let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
                let rider = rule.activation_description();
                assert_eq!(rendered.matches(&rider).count(), 1, "{rendered}");
                let ironsmith::ability::AbilityKind::Activated(ability) = &definition.abilities[0].kind else { panic!() };
                assert_eq!(format!("{:?}", ability.mana_cost), format!("{before:?}"));
                let parts = ability.mana_cost.as_all().unwrap();
                let actual = if dynamic { &parts[0].dynamic_mana_cost_ref().unwrap().base } else { parts[0].mana_cost_ref().unwrap() };
                assert_eq!(actual.spending_restrictions(), &[rule.clone()]);
                assert_eq!(actual.generic_mana_total(), 3);
            }
        }
    }
}
