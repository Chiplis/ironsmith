//! Frozen optional/count-limited coin bodies. All scenarios remain unrun.
use super::*;
use ironsmith::effect::{Effect, EffectId, ExecutionFact, Value};
use ironsmith::effects::{ExecutionError, execute_effect};
use ironsmith_core::{CoinFlipStopCondition, EffectMetric, EffectMetricSource, PriorEffectAction, PriorEffectMetricQuery};

fn loop_definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../../fixtures/optional_coin_loops.json.fixture")).unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    assert_eq!(row["proposed_complete"], true);
    let definitions = program_definitions(name, row["text"].as_str().unwrap());
    for definition in &definitions {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    definitions
}

#[test]
fn gambit_cast_declares_a_target_before_flips_and_rewards_each_threshold_once() {
    for definition in loop_definitions("Fiery Gambit") {
        for wins in [1usize, 2, 3, 4] {
            let mut g = game();
            let creature = object(&mut g, B, Zone::Battlefield, "Saved target", "Type: Creature\nPower/Toughness: 1/10");
            let other = object(&mut g, B, Zone::Battlefield, "Other target", "Type: Creature\nPower/Toughness: 1/10");
            let own_land = object(&mut g, A, Zone::Battlefield, "Own land", "Type: Land");
            let enemy_land = object(&mut g, B, Zone::Battlefield, "Enemy land", "Type: Land");
            g.tap(own_land); g.tap(enemy_land);
            library(&mut g, A, 12);
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices { targets: vec![Target::Object(creature)], booleans: vec![true; wins - 1], ..Default::default() };
            force(&mut g, &vec![H; wins]);
            cast(&mut g, spell, &mut dm);
            assert_eq!(g.stack.last().unwrap().targets, vec![Target::Object(creature)]);
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 0);
            dm.targets = vec![Target::Object(other)];
            settle(&mut g, &mut dm);
            assert_eq!(g.damage_on(creature), 3);
            assert_eq!(g.damage_on(other), 0, "resolution cannot replace the saved target");
            assert_eq!(g.player(B).unwrap().life, if wins >= 2 { 24 } else { 30 });
            assert_eq!(g.player(A).unwrap().hand.len(), if wins >= 3 { 9 } else { 0 });
            assert_eq!(g.is_tapped(own_land), wins < 3);
            assert!(g.is_tapped(enemy_land));
            assert_eq!(dm.option_players, vec![A; wins]);
            assert_eq!(dm.boolean_players, vec![A; wins], "there is no implicit stop after the third win");
        }
    }
}

#[test]
fn gambit_loss_erases_every_reward_and_illegal_saved_target_prevents_any_flip() {
    for definition in loop_definitions("Fiery Gambit") {
        for invalid_target in [false, true] {
            let mut g = game();
            let creature = object(&mut g, B, Zone::Battlefield, "Target", "Type: Creature\nPower/Toughness: 1/10");
            let land = object(&mut g, A, Zone::Battlefield, "Land", "Type: Land");
            g.tap(land); library(&mut g, A, 12);
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices { targets: vec![Target::Object(creature)], accept: true, ..Default::default() };
            force(&mut g, &[H, H, H, T]);
            cast(&mut g, spell, &mut dm);
            if invalid_target { g.move_object_by_effect(creature, Zone::Graveyard).unwrap(); }
            settle(&mut g, &mut dm);
            assert_eq!(g.player(B).unwrap().life, 30);
            assert!(g.player(A).unwrap().hand.is_empty());
            assert!(g.is_tapped(land));
            assert_eq!(g.damage_on(creature), 0);
            assert_eq!(dm.option_players.len(), if invalid_target { 0 } else { 4 });
            assert_eq!(dm.boolean_players.len(), if invalid_target { 0 } else { 3 });
        }
        let mut g = game();
        let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
        assert!(!ironsmith::decision::compute_legal_actions(&g, A).unwrap().iter().any(|action| {
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)
        }), "a creature target is required even before any win");
    }
}

#[test]
fn squee_chosen_zero_full_wins_and_early_loss_have_distinct_native_outcomes() {
    for definition in loop_definitions("Squee's Revenge") {
        for (number, faces, draw, flips) in [
            (0, vec![], 0, 0), (1, vec![H], 2, 1), (3, vec![H, H, H], 6, 3),
            (3, vec![T, H, H], 0, 1), (3, vec![H, T, H], 0, 2),
            (u32::MAX, vec![T, H], 0, 1),
        ] {
            let mut g = game(); library(&mut g, A, 10);
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices { number: Some(number), ..Default::default() };
            force(&mut g, &faces);
            cast(&mut g, spell, &mut dm); settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().hand.len(), draw);
            assert_eq!(dm.option_players, vec![A; flips]);
            assert!(dm.boolean_players.is_empty(), "a chosen count is mandatory until its bound or loss");
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), flips as u32);
            if flips < faces.len() {
                let source = g.new_object_id();
                let next = flip(&mut g, source, A, 1, false, false, &mut dm);
                assert_eq!(next.coin_flip_results().unwrap()[0].face, faces[flips]);
            }
        }
    }
}

#[test]
fn optional_stop_and_counted_call_suspension_restore_the_whole_native_spell() {
    for name in ["Fiery Gambit", "Squee's Revenge"] {
        for definition in loop_definitions(name) {
            for pause in [0, 1, 2] {
                let mut g = game(); library(&mut g, A, 12);
                let creature = object(&mut g, B, Zone::Battlefield, "Saved target", "Type: Creature\nPower/Toughness: 1/10");
                let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
                let mut dm = Choices { number: Some(3), targets: vec![Target::Object(creature)], booleans: vec![true, true, false], ..Default::default() };
                force(&mut g, &[H, H, H]); cast(&mut g, spell, &mut dm);
                if name == "Fiery Gambit" { dm.pause_on_boolean = Some(pause + 1); }
                else if pause == 0 { dm.pause_on_number = true; }
                else { dm.pause_on_option = Some(pause); }
                let random = g.irreversible_random_count();
                let ui = g.ui_effect_events().count();
                resolve_stack_entry_with(&mut g, &mut dm).unwrap();
                assert!(dm.pending); assert_eq!(g.stack.len(), 1);
                assert_eq!(g.irreversible_random_count(), random);
                assert_eq!(g.ui_effect_events().count(), ui);
                assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 0);
                assert_eq!(g.damage_on(creature), 0);
                assert_eq!(g.player(B).unwrap().life, 30);
                assert!(g.player(A).unwrap().hand.is_empty());
                dm.pending = false; dm.pause_on_boolean = None; dm.pause_on_number = false; dm.pause_on_option = None;
                dm.boolean_players.clear(); dm.option_players.clear();
                settle(&mut g, &mut dm);
                assert_eq!(g.player(A).unwrap().hand.len(), if name == "Fiery Gambit" { 9 } else { 6 });
                assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 3);
            }
        }
    }
}

#[test]
fn counted_coin_reads_only_exact_number_evidence_and_preserves_independent_x() {
    let mut g = game(); let source = g.new_object_id();
    let query = PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::Count).with_action(PriorEffectAction::ChosenNumber);
    let mut effect = FlipCoinEffect::new(PlayerFilter::Specific(B));
    effect.repeat_until_loss = true;
    effect.stop_condition = Some(CoinFlipStopCondition::CountReached);
    effect.count_value = Some(Value::PriorEffectMetric { effect_id: EffectId(4), query });
    let mut dm = Choices { number: Some(2), ..Default::default() };
    force(&mut g, &[H, H]);
    let mut ctx = ExecutionContext::new(source, A, &mut dm);
    ctx.x_value = Some(17);
    assert!(matches!(effect.execute(&mut g, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
    ctx.effect_outcomes.insert(EffectId(4), EffectOutcome::count(2));
    assert!(matches!(effect.execute(&mut g, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
    execute_effect(&mut g, &Effect::with_id(4, Effect::new(ironsmith::effects::ChooseNumberEffect::unbounded(PlayerFilter::You))), &mut ctx).unwrap();
    let out = effect.execute(&mut g, &mut ctx).unwrap();
    assert_eq!(ctx.x_value, Some(17));
    let receipt = out.coin_flip_results().unwrap();
    assert_eq!(receipt.len(), 2);
    assert!(receipt.iter().all(|coin| coin.player == B && coin.call == Some(H) && coin.winner == Some(B)));
    assert_eq!(receipt.iter().map(|coin| coin.instruction_ordinal).collect::<Vec<_>>(), [1, 2]);
    drop(ctx);
    assert_eq!(dm.option_players, [B, B]);
}

#[test]
fn unbounded_number_preserves_the_full_host_response_without_clamping() {
    struct Number(u32);
    impl DecisionMaker for Number {
        fn decide_number(&mut self, _: &GameState, context: &ironsmith::decisions::context::NumberContext) -> u32 {
            assert_eq!(context.authored_max, None); assert!(!context.is_x_value); self.0
        }
    }
    let mut g = game(); let source = g.new_object_id();
    let effect = ironsmith::effects::ChooseNumberEffect::unbounded(PlayerFilter::You);
    for value in [0, 7, i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
        let mut dm = Number(value);
        let out = effect.execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm)).unwrap();
        assert!(out.execution_facts.contains(&ExecutionFact::ChosenNumber(value)));
    }
}

#[test]
fn both_new_stops_rollback_partial_receipts_on_capacity_failure() {
    for stop in [CoinFlipStopCondition::ChooseToStop, CoinFlipStopCondition::CountReached] {
        let mut g = game(); let source = g.new_object_id();
        g.turn_store.turn_history.completed_coin_flips_this_turn.insert(A, i32::MAX as u32 - 1);
        force(&mut g, &[H, H]);
        let random = g.irreversible_random_count(); let ui = g.ui_effect_events().count();
        let mut effect = FlipCoinEffect::new(PlayerFilter::You);
        effect.repeat_until_loss = true; effect.stop_condition = Some(stop); effect.count = 2;
        let mut dm = Choices { booleans: vec![true, false], ..Default::default() };
        assert!(matches!(effect.execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm)), Err(ExecutionError::ResourceLimitExceeded { .. })));
        assert_eq!(g.irreversible_random_count(), random); assert_eq!(g.ui_effect_events().count(), ui);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), i32::MAX as u32 - 1);
        g.turn_store.turn_history.completed_coin_flips_this_turn.insert(A, 0);
        dm.boolean_players.clear();
        let out = effect.execute(&mut g, &mut ExecutionContext::new(source, A, &mut dm)).unwrap();
        assert_eq!(out.coin_flip_results().unwrap().len(), 2);
    }
}

#[test]
fn planar_chaos_paid_cast_uses_the_actual_caster_and_exact_triggering_spell() {
    for definition in loop_definitions("Planar Chaos") {
        for caster in [A, B] {
            for face in [H, T] {
                let mut g = game();
                let enchantment = g.create_object_from_definition(&definition, A, Zone::Hand);
                let stable = g.object(enchantment).unwrap().stable_id;
                let mut dm = Choices::default();
                cast(&mut g, enchantment, &mut dm); settle(&mut g, &mut dm);
                let source = g.find_object_by_stable_id(stable).unwrap();
                assert_eq!(g.object(source).unwrap().zone, Zone::Battlefield);
                let spell = object(&mut g, caster, Zone::Hand, "Triggered spell", "Mana cost: {R}\nType: Instant\nYou gain 9 life.");
                let spell_stable = g.object(spell).unwrap().stable_id;
                force(&mut g, &[face]);
                action(&mut g, caster, LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand,
                    casting_method: ironsmith::alternative_cast::CastingMethod::Normal }, &mut dm);
                assert_eq!(g.stack.len(), 2);
                assert!(g.stack.last().unwrap().targets.is_empty(), "the triggering spell is referenced, not targeted");
                // The already-triggered coin instruction survives its source.
                g.move_object_by_effect(source, Zone::Graveyard).unwrap();
                settle(&mut g, &mut dm);
                assert_eq!(dm.option_players, vec![caster]);
                assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(caster), 1);
                assert_eq!(g.player(caster).unwrap().life, if face == H { 39 } else { 30 });
                let finished = g.find_object_by_stable_id(spell_stable).unwrap();
                assert_eq!(g.object(finished).unwrap().zone, Zone::Graveyard);
            }
        }
    }
}

#[test]
fn planar_upkeep_only_its_controller_flips_and_loss_sacrifices_the_enchantment() {
    for definition in loop_definitions("Planar Chaos") {
        for face in [H, T] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let stable = g.object(source).unwrap().stable_id;
            let mut dm = Choices::default();
            g.turn.phase = ironsmith::Phase::Beginning; g.turn.step = Some(ironsmith::game_state::Step::Upkeep);
            for active in [B, A] {
                g.turn.active_player = active;
                let mut queue = TriggerQueue::new(); generate_and_queue_step_triggers(&mut g, &mut queue);
                put_triggers_on_stack_with_dm(&mut g, &mut queue, &mut dm).unwrap();
                assert_eq!(g.stack.len(), if active == A { 1 } else { 0 });
            }
            force(&mut g, &[face]); settle(&mut g, &mut dm);
            assert_eq!(dm.option_players, [A]);
            let current = g.find_object_by_stable_id(stable).unwrap();
            assert_eq!(g.object(current).unwrap().zone, if face == H { Zone::Battlefield } else { Zone::Graveyard });
        }
    }
}

#[test]
fn planar_pending_foreign_caster_call_replays_without_countering_the_wrong_stack_object() {
    for definition in loop_definitions("Planar Chaos") {
        let mut g = game(); g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let spell = object(&mut g, B, Zone::Hand, "Opponent spell", "Mana cost: {R}\nType: Instant\nYou gain 9 life.");
        let mut dm = Choices { pause_on_option: Some(1), ..Default::default() };
        force(&mut g, &[T]);
        action(&mut g, B, LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal }, &mut dm);
        let random = g.irreversible_random_count();
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert!(dm.pending); assert_eq!(g.stack.len(), 2);
        assert_eq!(g.irreversible_random_count(), random);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 0);
        dm.pending = false; dm.pause_on_option = None; dm.option_players.clear();
        settle(&mut g, &mut dm);
        assert_eq!(dm.option_players, [B]); assert_eq!(g.player(B).unwrap().life, 30);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 1);
    }
}

#[test]
fn planar_multiple_casts_keep_distinct_casters_and_triggering_spell_assignments() {
    for definition in loop_definitions("Planar Chaos") {
        let mut g = game(); g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = object(&mut g, A, Zone::Hand, "First spell", "Mana cost: {R}\nType: Instant\nYou gain 1 life.");
        let second = object(&mut g, B, Zone::Hand, "Second spell", "Mana cost: {R}\nType: Instant\nYou gain 9 life.");
        let mut dm = Choices::default();
        cast(&mut g, first, &mut dm);
        action(&mut g, B, LegalAction::CastSpell { spell_id: second, from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal }, &mut dm);
        assert_eq!(g.stack.len(), 4);
        force(&mut g, &[T, H]); settle(&mut g, &mut dm);
        assert_eq!(dm.option_players, [B, A]);
        assert_eq!(g.player(A).unwrap().life, 31); assert_eq!(g.player(B).unwrap().life, 30);
    }
}

#[test]
fn third_person_coin_consumer_requires_a_local_producer_and_retains_unknown_tails() {
    for text in [
        "Type: Sorcery\nIf they lose the flip, you gain 1 life.",
        "Type: Sorcery\nFlip a coin. If they lose the flip {R}, you gain 1 life.",
        "Type: Sorcery\nFlip a coin. If they lose the flip, you gain 1 life except on Tuesdays.",
    ] {
        assert!(compile_to_artifact("Unbound or malformed coin reference", text, false).is_err());
        assert!(compile_to_runtime_definition("Unbound or malformed coin reference", text, false).is_err());
    }
}

#[test]
fn legacy_coin_and_bounded_number_payloads_keep_defaults_and_numeric_json() {
    let mut repeat = ironsmith_core::FlipCoinEffect::new(PlayerFilter::You);
    repeat.repeat_until_loss = true;
    let mut legacy = serde_json::to_value(&repeat).unwrap();
    legacy.as_object_mut().unwrap().remove("stop_condition");
    legacy.as_object_mut().unwrap().remove("loss_action");
    let restored: ironsmith_core::FlipCoinEffect = serde_json::from_value(legacy).unwrap();
    assert!(restored.repeat_until_loss); assert_eq!(restored.stop_condition, None); assert_eq!(restored.loss_action, None);
    assert_eq!(restored.count_value, None); assert_eq!(restored.opponent_results, None);
    let legacy_number = serde_json::json!({ "chooser": serde_json::to_value(PlayerFilter::You).unwrap(), "min": 0, "max": 13 });
    let restored: ironsmith_core::ChooseNumberEffect = serde_json::from_value(legacy_number.clone()).unwrap();
    assert_eq!(restored.max, Some(13));
    assert_eq!(serde_json::to_value(restored).unwrap(), legacy_number);
    let unbounded = ironsmith_core::ChooseNumberEffect::unbounded(PlayerFilter::You);
    let restored: ironsmith_core::ChooseNumberEffect = serde_json::from_value(serde_json::to_value(&unbounded).unwrap()).unwrap();
    assert_eq!(restored, unbounded);
}


#[test]
fn gambit_loss_stops_actual_paid_spliced_text_but_success_and_next_spell_still_resolve() {
    for definition in loop_definitions("Fiery Gambit") {
        for face in [H, T] {
            let mut g = game(); library(&mut g, A, 4);
            let creature = object(&mut g, B, Zone::Battlefield, "Gambit target", "Type: Creature\nPower/Toughness: 1/10");
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let splice = object(&mut g, A, Zone::Hand, "Everdream", "Mana cost: {1}{U}\nType: Instant\nDraw a card.\nSplice onto instant or sorcery {2}{U} (As you cast an instant or sorcery spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)");
            let splice_stable = g.object(splice).unwrap().stable_id;
            let mana = g.player(A).unwrap().mana_pool.total();
            let mut dm = Choices { targets: vec![Target::Object(creature)], ..Default::default() };
            force(&mut g, &[face]); cast(&mut g, spell, &mut dm);
            assert_eq!(g.stack.last().unwrap().spliced_cards, vec![splice_stable]);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 6, "both the spell and splice costs are paid");
            settle(&mut g, &mut dm);
            assert_eq!(g.damage_on(creature), if face == H { 3 } else { 0 });
            assert_eq!(g.player(A).unwrap().hand.len(), if face == H { 2 } else { 1 }, "the appended draw is suppressed after loss");
            assert_eq!(g.object(splice).unwrap().zone, Zone::Hand);
            // A new stack object's resolution has a fresh control-flow scope.
            cast(&mut g, splice, &mut dm); settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().hand.len(), if face == H { 2 } else { 1 });
        }
    }
}


#[test]
fn large_chosen_number_result_predicates_compare_losslessly() {
    let mut g = game(); let source = g.new_object_id();
    let mut dm = Choices { number: Some(u32::MAX), ..Default::default() };
    let mut ctx = ExecutionContext::new(source, A, &mut dm);
    execute_effect(&mut g, &Effect::with_id(1, Effect::new(ironsmith::effects::ChooseNumberEffect::unbounded(PlayerFilter::You))), &mut ctx).unwrap();
    execute_effect(&mut g, &Effect::if_then(EffectId(1),
        ironsmith::effect::EffectPredicate::Value(ironsmith::effect::Comparison::GreaterThan(i32::MAX)),
        vec![Effect::gain_life(2)]), &mut ctx).unwrap();
    execute_effect(&mut g, &Effect::if_then(EffectId(1),
        ironsmith::effect::EffectPredicate::Value(ironsmith::effect::Comparison::LessThan(0)),
        vec![Effect::lose_life(9)]), &mut ctx).unwrap();
    assert_eq!(g.player(A).unwrap().life, 32);
}

fn scales_ability(definition: &CardDefinition) -> usize {
    definition.abilities.iter().position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))).unwrap()
}

#[test]
fn scales_paid_cast_and_activation_keep_two_saved_targets_and_charge_each_repeated_payment() {
    for definition in loop_definitions("Crooked Scales") {
        for (faces, payments, paid, win) in [
            (vec![H], vec![], 0, true), (vec![T], vec![false], 0, false),
            (vec![T, H], vec![true], 1, true), (vec![T, T], vec![true, false], 1, false),
            (vec![T, T, H], vec![true, true], 2, true),
        ] {
            let mut g = game();
            let enemy = object(&mut g, B, Zone::Battlefield, "Saved enemy", "Type: Creature\nPower/Toughness: 2/2");
            let own = object(&mut g, A, Zone::Battlefield, "Saved own", "Type: Creature\nPower/Toughness: 2/2");
            let enemy_stable = g.object(enemy).unwrap().stable_id;
            let own_stable = g.object(own).unwrap().stable_id;
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let source_stable = g.object(spell).unwrap().stable_id;
            let mut dm = Choices::default(); let initial_mana = g.player(A).unwrap().mana_pool.total();
            cast(&mut g, spell, &mut dm); settle(&mut g, &mut dm);
            let source = g.find_object_by_stable_id(source_stable).unwrap();
            assert_eq!(g.player(A).unwrap().mana_pool.total(), initial_mana - 4);
            dm.targets = vec![Target::Object(enemy), Target::Object(own)]; dm.booleans = payments.clone();
            force(&mut g, &faces);
            action(&mut g, A, LegalAction::ActivateAbility { source, ability_index: scales_ability(&definition) }, &mut dm);
            assert!(g.is_tapped(source));
            assert_eq!(g.stack.last().unwrap().targets, vec![Target::Object(enemy), Target::Object(own)]);
            let unchosen = object(&mut g, A, Zone::Battlefield, "Unchosen", "Type: Creature\nPower/Toughness: 2/2");
            dm.targets = vec![Target::Object(unchosen)];
            g.move_object_by_effect(source, Zone::Graveyard).unwrap();
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), initial_mana - 8 - paid * 3);
            assert_eq!(dm.option_players, vec![A; faces.len()]); assert_eq!(dm.boolean_players, vec![A; payments.len()]);
            assert_eq!(g.object(g.find_object_by_stable_id(enemy_stable).unwrap()).unwrap().zone, if win { Zone::Graveyard } else { Zone::Battlefield });
            assert_eq!(g.object(g.find_object_by_stable_id(own_stable).unwrap()).unwrap().zone, if win { Zone::Battlefield } else { Zone::Graveyard });
            assert_eq!(g.object(unchosen).unwrap().zone, Zone::Battlefield);
        }
    }
}

#[test]
fn scales_payment_and_later_call_suspensions_restore_all_resolution_payments_and_coins() {
    for definition in loop_definitions("Crooked Scales") {
        for pause in [0, 1, 2] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let enemy = object(&mut g, B, Zone::Battlefield, "Enemy", "Type: Creature\nPower/Toughness: 2/2");
            let own = object(&mut g, A, Zone::Battlefield, "Own", "Type: Creature\nPower/Toughness: 2/2");
            let mut dm = Choices { targets: vec![Target::Object(enemy), Target::Object(own)], booleans: vec![true, true], ..Default::default() };
            force(&mut g, &[T, T, H]);
            action(&mut g, A, LegalAction::ActivateAbility { source, ability_index: scales_ability(&definition) }, &mut dm);
            let mana = g.player(A).unwrap().mana_pool.total(); let random = g.irreversible_random_count();
            let ui = g.ui_effect_events().count();
            if pause == 0 { dm.pause_on_option = Some(2); } else { dm.pause_on_boolean = Some(pause); }
            resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert!(dm.pending); assert_eq!(g.stack.len(), 1); assert!(g.is_tapped(source));
            assert_eq!(g.player(A).unwrap().mana_pool.total(), mana);
            assert_eq!(g.irreversible_random_count(), random); assert_eq!(g.ui_effect_events().count(), ui);
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 0);
            assert_eq!(g.object(enemy).unwrap().zone, Zone::Battlefield); assert_eq!(g.object(own).unwrap().zone, Zone::Battlefield);
            dm.pending = false; dm.pause_on_boolean = None; dm.pause_on_option = None;
            dm.boolean_players.clear(); dm.option_players.clear();
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 6);
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 3);
        }
    }
}

#[test]
fn scales_unaffordable_payment_and_illegal_targets_never_choose_replacements() {
    for definition in loop_definitions("Crooked Scales") {
        for removed in [0, 1, 2, 3] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let enemy = object(&mut g, B, Zone::Battlefield, "Enemy", "Type: Creature\nPower/Toughness: 2/2");
            let own = object(&mut g, A, Zone::Battlefield, "Own", "Type: Creature\nPower/Toughness: 2/2");
            let enemy_stable = g.object(enemy).unwrap().stable_id; let own_stable = g.object(own).unwrap().stable_id;
            let mut dm = Choices { targets: vec![Target::Object(enemy), Target::Object(own)], accept: true, ..Default::default() };
            action(&mut g, A, LegalAction::ActivateAbility { source, ability_index: scales_ability(&definition) }, &mut dm);
            g.player_mut(A).unwrap().mana_pool = Default::default();
            g.player_mut(A).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless, 2);
            if removed & 1 != 0 { g.move_object_by_effect(enemy, Zone::Graveyard).unwrap(); }
            if removed & 2 != 0 { g.move_object_by_effect(own, Zone::Graveyard).unwrap(); }
            force(&mut g, &[T]); settle(&mut g, &mut dm);
            assert!(dm.boolean_players.is_empty(), "an unaffordable payment is not offered");
            assert_eq!(dm.option_players.len(), if removed == 3 { 0 } else { 1 });
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 2);
            assert_eq!(g.object(g.find_object_by_stable_id(enemy_stable).unwrap()).unwrap().zone,
                if removed & 1 != 0 { Zone::Graveyard } else { Zone::Battlefield });
            assert_eq!(g.object(g.find_object_by_stable_id(own_stable).unwrap()).unwrap().zone, Zone::Graveyard);
        }
    }
}

#[test]
fn repeated_process_requires_fresh_gate_evidence_and_rolls_back_native_work_on_error() {
    use ironsmith::effects::RepeatProcessEffect;
    let mut g = game(); let source = g.new_object_id();
    let condition = EffectId(55);
    let mut dm = Choices::default(); let mut ctx = ExecutionContext::new(source, A, &mut dm);
    ctx.effect_outcomes.insert(condition, EffectOutcome::count(1));
    let missing = RepeatProcessEffect::new(vec![Effect::gain_life(2)], condition, ironsmith::effect::EffectPredicate::Happened);
    assert!(matches!(missing.execute(&mut g, &mut ctx), Err(ExecutionError::IncompleteEvidence(_))));
    assert_eq!(g.player(A).unwrap().life, 30); assert_eq!(ctx.get_outcome(condition).unwrap().as_count(), Some(1));
    let failing = RepeatProcessEffect::new(vec![Effect::gain_life(2), Effect::with_id(condition.0,
        Effect::new(ironsmith::effects::ChooseNumberEffect::new(PlayerFilter::You, 4, 1)))], condition,
        ironsmith::effect::EffectPredicate::Happened);
    assert!(matches!(failing.execute(&mut g, &mut ctx), Err(ExecutionError::Impossible(_))));
    assert_eq!(g.player(A).unwrap().life, 30);
}

#[test]
fn scales_resource_failure_after_paid_loss_restores_mana_history_and_forced_faces() {
    for definition in loop_definitions("Crooked Scales") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let enemy = object(&mut g, B, Zone::Battlefield, "Enemy", "Type: Creature\nPower/Toughness: 2/2");
        let own = object(&mut g, A, Zone::Battlefield, "Own", "Type: Creature\nPower/Toughness: 2/2");
        let enemy_stable = g.object(enemy).unwrap().stable_id;
        let mut dm = Choices { targets: vec![Target::Object(enemy), Target::Object(own)], booleans: vec![true], ..Default::default() };
        action(&mut g, A, LegalAction::ActivateAbility { source, ability_index: scales_ability(&definition) }, &mut dm);
        let mana = g.player(A).unwrap().mana_pool.total();
        let random = g.irreversible_random_count(); let ui = g.ui_effect_events().count();
        g.turn_store.turn_history.completed_coin_flips_this_turn.insert(A, i32::MAX as u32 - 1);
        force(&mut g, &[T, H]);
        assert!(resolve_stack_entry_with(&mut g, &mut dm).is_err());
        assert_eq!(g.stack.len(), 1); assert_eq!(g.player(A).unwrap().mana_pool.total(), mana);
        assert_eq!(g.irreversible_random_count(), random); assert_eq!(g.ui_effect_events().count(), ui);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), i32::MAX as u32 - 1);
        assert_eq!(g.object(own).unwrap().zone, Zone::Battlefield); assert_eq!(g.object(enemy).unwrap().zone, Zone::Battlefield);
        g.turn_store.turn_history.completed_coin_flips_this_turn.insert(A, 0);
        dm.boolean_players.clear(); dm.option_players.clear();
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().mana_pool.total(), mana - 3);
        assert_eq!(dm.option_players, [A, A]);
        assert_eq!(g.object(g.find_object_by_stable_id(enemy_stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(g.object(own).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn mana_clash_uses_only_the_saved_pair_and_repeats_until_the_same_round_is_all_heads() {
    for definition in loop_definitions("Mana Clash") {
        for (faces, alice_life, opponent_life, rounds) in [
            (vec![H, H], 30, 30, 1), (vec![T, H, H, H], 29, 30, 2),
            (vec![H, T, H, H], 30, 29, 2), (vec![T, T, H, H], 29, 29, 2),
            (vec![T, H, H, T, H, H], 29, 29, 3),
        ] {
            let mut g = multiplayer_game();
            let opponent = PlayerId(2);
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices { targets: vec![Target::Player(opponent)], ..Default::default() };
            force(&mut g, &faces); cast(&mut g, spell, &mut dm);
            assert_eq!(g.stack.last().unwrap().targets, vec![Target::Player(opponent)]);
            dm.targets = vec![Target::Player(B)];
            settle(&mut g, &mut dm);
            assert!(dm.option_players.is_empty(), "physical heads/tails do not request calls");
            assert!(dm.boolean_players.is_empty());
            assert_eq!(g.player(A).unwrap().life, alice_life); assert_eq!(g.player(opponent).unwrap().life, opponent_life);
            assert_eq!(g.player(B).unwrap().life, 30); assert_eq!(g.player(PlayerId(3)).unwrap().life, 30);
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), rounds);
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(opponent), rounds);
            assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 0);
        }
    }
}

#[test]
fn mana_clash_prevented_tail_damage_does_not_replace_the_rounds_face_condition() {
    for definition in loop_definitions("Mana Clash") {
        let mut g = game();
        let shield = object(&mut g, A, Zone::Battlefield, "Shield source", "Type: Artifact");
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        execute_effect(&mut g, &Effect::prevent_all_damage_to_target(ChooseSpec::Player(PlayerFilter::Specific(A)), Until::EndOfTurn),
            &mut ExecutionContext::new(shield, A, &mut dm)).unwrap();
        let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
        force(&mut g, &[T, H, H, T, H, H]);
        cast(&mut g, spell, &mut dm); settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().life, 30); assert_eq!(g.player(B).unwrap().life, 29);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 3);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 3);
    }
}

#[test]
fn correlated_tails_damage_is_one_native_simultaneous_instruction() {
    let mut g = game();
    let source = object(&mut g, A, Zone::Battlefield, "Damage source", "Type: Artifact");
    let mut dm = Choices::default(); let mut ctx = ExecutionContext::new(source, A, &mut dm);
    ctx.store_outcome(EffectId(7), EffectOutcome::count(0).with_player_counts(vec![(A, 0), (B, 0)]));
    let followup = ironsmith::effects::IfEffect::if_then(EffectId(7), ironsmith::effect::EffectPredicate::DidNotHappen,
        vec![Effect::deal_damage(1, ChooseSpec::Player(PlayerFilter::IteratedPlayer))]).with_per_player_result(true);
    let out = followup.execute(&mut g, &mut ctx).unwrap();
    let damage = out.events.iter().filter(|event| event.downcast::<ironsmith::events::DamageEvent>().is_some()).collect::<Vec<_>>();
    assert_eq!(damage.len(), 2); assert!(damage[0].simultaneous_batch().is_some());
    assert_eq!(damage[0].simultaneous_batch(), damage[1].simultaneous_batch());
    assert_eq!(g.player(A).unwrap().life, 29); assert_eq!(g.player(B).unwrap().life, 29);
}

#[test]
fn mana_clash_pending_replacement_keep_choice_restores_both_players_and_prior_round_damage() {
    for definition in loop_definitions("Mana Clash") {
        let mut g = game();
        object(&mut g, A, Zone::Battlefield, "Krark's Thumb", "Type: Artifact\nIf you would flip a coin, instead flip two coins and ignore one.");
        let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        // A keeps the first face of each replacement pair; B flips once.
        force(&mut g, &[T, H, T, H, T, H]); cast(&mut g, spell, &mut dm);
        dm.pause_on_option = Some(2);
        let random = g.irreversible_random_count(); let ui = g.ui_effect_events().count();
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert!(dm.pending); assert_eq!(g.stack.len(), 1);
        assert_eq!(g.player(A).unwrap().life, 30); assert_eq!(g.player(B).unwrap().life, 30);
        assert_eq!(g.irreversible_random_count(), random); assert_eq!(g.ui_effect_events().count(), ui);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 0);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 0);
        dm.pending = false; dm.pause_on_option = None; dm.option_players.clear();
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().life, 29); assert_eq!(g.player(B).unwrap().life, 29);
        assert_eq!(dm.option_players, [A, A], "only Thumb keeps choices; neither player calls a face-only flip");
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 2);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 2);
    }
}

#[test]
fn mana_clash_capacity_failure_rolls_back_prior_round_damage_and_recovers_the_exact_pair() {
    for definition in loop_definitions("Mana Clash") {
        let mut g = game(); let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        force(&mut g, &[T, T, H, H]); cast(&mut g, spell, &mut dm);
        g.turn_store.turn_history.completed_coin_flips_this_turn.insert(A, i32::MAX as u32 - 1);
        let random = g.irreversible_random_count();
        assert!(resolve_stack_entry_with(&mut g, &mut dm).is_err());
        assert_eq!(g.stack.len(), 1); assert_eq!(g.irreversible_random_count(), random);
        assert_eq!(g.player(A).unwrap().life, 30); assert_eq!(g.player(B).unwrap().life, 30);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), i32::MAX as u32 - 1);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 0);
        g.turn_store.turn_history.completed_coin_flips_this_turn.insert(A, 0);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().life, 29); assert_eq!(g.player(B).unwrap().life, 29);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(A), 2);
        assert_eq!(g.turn_store.turn_history.completed_coin_flip_count(B), 2);
    }
}
