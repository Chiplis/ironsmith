//! Actual Siphon source, bounded counter-placement payment.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
fn definitions(id: &str) -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-final-abilities.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let card = &row["representative_cards"][0];
    source_support::definitions(card).unwrap()
}
fn announce(game: &mut GameState, action: ironsmith::decision::LegalAction) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut q = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut q, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
}
fn settle(game: &mut GameState) {
    let mut q = ironsmith::triggers::TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    for _ in 0..64 {
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut q, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut q, &mut dm).unwrap();
        if game.stack.is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
    panic!("did not settle");
}
#[test]
fn siphon_actual_source_places_bounded_counters_instead_of_only_generic_mana() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    use ironsmith::object::CounterType;
    for definition in definitions("msem-919") {
        for mode in ["counters", "ordinary-mana"] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let body = |name: &str, toughness: i32| {
                ironsmith_compiler_runtime::compile_to_runtime_definition(
                    name,
                    &format!("Type: Creature — Human\nPower/Toughness: 1/{toughness}"),
                    false,
                )
                .unwrap()
            };
            let victim = game.create_object_from_definition(
                &body("Destruction target", 5),
                PlayerId(1),
                Zone::Battlefield,
            );
            let damaged = game.create_object_from_definition(
                &body("Damaged contributor", 3),
                A,
                Zone::Battlefield,
            );
            game.mark_damage(damaged, 2);
            game.tap(damaged);
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let stable = game.object(source).unwrap().stable_id;
            let action = |g: &GameState| {
                compute_legal_actions(g,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
            };
            assert!(
                action(&game).is_none(),
                "counter placement cannot replace the colored black pip"
            );
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Black, 1);
            assert!(
                action(&game).is_none(),
                "three toughness with two damage only pays one generic, opponent is ineligible"
            );
            let second = game.create_object_from_definition(
                &body("Second contributor", 2),
                A,
                Zone::Battlefield,
            );
            game.tap(second);
            if mode == "ordinary-mana" {
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 3);
            }
            let chosen =
                action(&game).expect("tapped summoning-sick contributors can pay with counters");
            announce(&mut game, chosen);
            let counters = |g: &GameState, id| {
                g.object(id)
                    .map(|_| g.counter_count(id, CounterType::MinusOneMinusOne))
                    .or_else(|| {
                        g.turn_store
                            .turn_history
                            .source_last_known_snapshot(id)
                            .map(|snapshot| snapshot.counter_count(CounterType::MinusOneMinusOne))
                    })
                    .unwrap()
            };
            let tapped = |g: &GameState, id| {
                g.object(id)
                    .map(|_| g.is_tapped(id))
                    .or_else(|| {
                        g.turn_store
                            .turn_history
                            .source_last_known_snapshot(id)
                            .map(|snapshot| snapshot.tapped)
                    })
                    .unwrap()
            };
            assert!(tapped(&game, damaged));
            assert!(tapped(&game, second));
            if mode == "counters" {
                assert_eq!(counters(&game, damaged), 1);
                assert_eq!(counters(&game, second), 2);
            } else {
                assert_eq!(counters(&game, damaged), 0);
                assert_eq!(counters(&game, second), 0);
            }
            assert_eq!(game.counter_count(victim, CounterType::MinusOneMinusOne), 0);
            settle(&mut game);
            let final_source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(final_source).unwrap().zone, Zone::Graveyard);
        }
    }
}
fn lady_definitions() -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-siphon-next-wave.json"
    ))
    .unwrap();
    let card = &fixture["mechanics"][0]["representative_cards"][1];
    assert_eq!(card["id"], "Lady Kahra Belmord_TPW");
    source_support::definitions(card).unwrap()
}
#[test]
fn actual_lady_source_grants_payment_only_to_first_qualifying_spell_and_resets_each_turn() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    use ironsmith::object::CounterType;
    for definition in lady_definitions() {
        for earlier_high_value in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let lady = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let support = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Toughness support",
                "Type: Enchantment\nCreatures you control get +12/+12.",
                false,
            )
            .unwrap();
            game.create_object_from_definition(&support, A, Zone::Battlefield);
            let spell = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Eligible spell",
                "Mana cost: {2}{B}\nType: Sorcery\nYou gain 1 life.",
                false,
            )
            .unwrap();
            let first = game.create_object_from_definition(&spell, A, Zone::Hand);
            let second = game.create_object_from_definition(&spell, A, Zone::Hand);
            let cast = |g: &GameState, id| {
                compute_legal_actions(g, A).unwrap().into_iter().find(
                    |action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id),
                )
            };
            if earlier_high_value {
                let earlier = ironsmith_compiler_runtime::compile_to_runtime_definition(
                    "Earlier excluded spell",
                    "Mana cost: {4}\nType: Instant\nYou gain 1 life.",
                    false,
                )
                .unwrap();
                let earlier = game.create_object_from_definition(&earlier, A, Zone::Hand);
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ManaSymbol::Colorless, 4);
                let chosen = cast(&game, earlier).unwrap();
                announce(&mut game, chosen);
                settle(&mut game);
            }
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Black, 1);
            let chosen = cast(&game, first).unwrap();
            announce(&mut game, chosen);
            settle(&mut game);
            assert_eq!(game.counter_count(lady, CounterType::MinusOneMinusOne), 2);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Black, 1);
            assert!(
                cast(&game, second).is_none(),
                "later eligible spell has no granted counter substitution"
            );
            game.turn.turn_number += 1;
            game.turn_store.turn_history.clear_for_new_turn();
            let chosen = cast(&game, second).expect("first matching spell grant renews next turn");
            announce(&mut game, chosen);
            settle(&mut game);
            assert_eq!(game.counter_count(lady, CounterType::MinusOneMinusOne), 4);
        }
    }
}
#[test]
fn counter_contributor_can_also_tap_for_colored_mana_in_the_same_payment() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::object::CounterType;
    for definition in definitions("msem-919") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let victim = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Target witness",
            "Type: Creature — Human\nPower/Toughness: 5/5",
            false,
        )
        .unwrap();
        game.create_object_from_definition(&victim, PlayerId(1), Zone::Battlefield);
        let donor = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Mana and counter contributor",
            "Type: Creature — Human\nPower/Toughness: 3/3\n{T}: Add {B}.",
            false,
        )
        .unwrap();
        let donor = game.create_object_from_definition(&donor, A, Zone::Battlefield);
        game.remove_summoning_sickness(donor);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let action = compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
            .expect("same creature can tap for B and receive three counters for generic pips");
        announce(&mut game, action);
        let counters = game
            .object(donor)
            .map(|_| game.counter_count(donor, CounterType::MinusOneMinusOne))
            .or_else(|| {
                game.turn_store
                    .turn_history
                    .source_last_known_snapshot(donor)
                    .map(|snapshot| snapshot.counter_count(CounterType::MinusOneMinusOne))
            })
            .unwrap();
        assert_eq!(counters, 3);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        settle(&mut game);
    }
}

#[test]
fn counter_payment_rechecks_toughness_after_each_cost_instead_of_capping_at_initial_toughness() {
    use ironsmith::mana::ManaSymbol;
    use ironsmith::object::CounterType;
    for definition in definitions("msem-919") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let body = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Counter-responsive contributor",
            "Type: Creature — Human\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        let donor = game.create_object_from_definition(&body, A, Zone::Battlefield);
        let target = game.create_object_from_definition(&body, PlayerId(1), Zone::Battlefield);
        let bonus = ironsmith::continuous::Modification::ModifyPowerToughnessValue {
            power: ironsmith::effect::Value::Fixed(0),
            toughness: ironsmith::effect::Value::Scaled(
                Box::new(ironsmith::effect::Value::CountersOn(
                    Box::new(ironsmith::target::ChooseSpec::SpecificObject(donor)),
                    Some(CounterType::MinusOneMinusOne),
                )),
                2,
            ),
        };
        let effect = ironsmith::Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(donor),
            bonus,
            ironsmith::effect::Until::EndOfTurn,
        ));
        ironsmith::effects::execute_effect(
            &mut game,
            &effect,
            &mut ironsmith::effects::EffectContext::new_default(donor, A),
        )
        .unwrap();
        assert_eq!(game.calculated_toughness(donor), Some(1));
        game.create_object_from_definition(&definition, A, Zone::Hand);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        let action = ironsmith::decision::compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { .. }))
            .expect("three substitutions remain payable after dynamic toughness changes");
        announce(&mut game, action);
        assert_eq!(game.counter_count(donor, CounterType::MinusOneMinusOne), 3);
        assert_eq!(game.calculated_toughness(donor), Some(4));
        assert!(
            game.object(target).is_some(),
            "mana payment does not resolve the spell"
        );
    }
}
