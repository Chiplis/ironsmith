//! Full source programs must retain every reminder operation through artifact transport.
#[path="custom_final_next_wave_support.rs"] mod source_support;

#[test]
fn full_source_growth_candidates_compile_losslessly_and_round_trip() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-growth-actions.json")).unwrap();
    let mut count = 0;
    let mut failures = Vec::new();
    for row in fixture["mechanics"].as_array().unwrap() {
        for card in row["representative_cards"].as_array().unwrap() {
            if card["rewrite_status"] != "frontend_materialized_unverified" {
                failures.push(format!("{}: full source declaration must actually expand ({})", card["name"], card["rewrite_status"]));
                continue;
            }
            match source_support::linked_definitions(card) {
                Ok(pairs) => {
                    for [direct, indirect] in pairs {
                        assert_eq!(ironsmith_text::compiled_text_lines(&direct), ironsmith_text::compiled_text_lines(&indirect));
                        count += 1;
                    }
                }
                Err(error) => failures.push(format!("{}: {error}",card["name"])),
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(count >= 5);
}

#[test]
fn sourced_infamy_status_is_independent_of_existing_or_removed_counters() {
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::effects::{DealDamageEffect, EffectContext, EffectExecutor};
    use ironsmith::target::ChooseSpec;
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::object::CounterType;
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-status-actions.json")).unwrap();
    let row = fixture["mechanics"].as_array().unwrap().iter().find(|row| row["keyword"] == "Infamy").unwrap();
    let card = &row["representative_cards"][0];
    let [direct, indirect] = source_support::definitions(card).unwrap();
    for definition in [direct, indirect] {
        for initial in [0, 2] {
            let a = PlayerId(0); let b = PlayerId(1);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, a, Zone::Battlefield);
            if initial > 0 { game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne, initial); }
            let mut dm = SelectFirstDecisionMaker;
            game.take_pending_trigger_events();
            for (combat, expected) in [(false, initial), (true, initial + 1), (true, 0)] {
                if expected == 0 && combat { game.object_mut(source).unwrap().counters.remove(&CounterType::PlusOnePlusOne); }
                let events = DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(b)).with_combat(combat).execute(&mut game, &mut EffectContext::new_default(source, a)).unwrap().events;
                for event in events { game.queue_trigger_event(Default::default(), event); }
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
                for _ in 0..10 {
                    if game.stack.is_empty() { break; }
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
                }
                assert!(game.stack.is_empty());
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), expected);
            }
        }
    }
}
