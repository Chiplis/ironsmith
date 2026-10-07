// Authored / UNRUN. The full body uses real counter producers; checkpoints
// and copied stack entries must retain the native wide event projection.
fn counter_recipient_native_fixture(wide: bool) -> (WasmGame, ObjectId, i64) {
    use ironsmith::effects::{DoubleCountersEffect, EffectExecutor, EffectContext as ExecutionContext, ProliferateEffect};
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    wasm.game.turn.active_player = PlayerId(0);
    wasm.game.turn.priority_player = Some(PlayerId(0));
    let definition = ironsmith_registry_test::compile_to_runtime_definition("All Will Be One",
        "Mana cost: {3}{R}{R}\nType: Enchantment\nWhenever you put one or more counters on a permanent or player, this enchantment deals that much damage to target opponent, creature an opponent controls, or planeswalker an opponent controls.", false).unwrap();
    let source = wasm.game.create_object_from_definition(&definition, PlayerId(0), Zone::Battlefield);
    let card = ironsmith::card::CardBuilder::new(CardId::new(), "Native counter recipient")
        .card_types(vec![CardType::Artifact]).build();
    let recipient = wasm.game.create_object_from_card(&card, PlayerId(1), Zone::Battlefield);
    let initial = if wide { i32::MAX as u32 } else { 1 };
    for kind in [ironsmith::CounterType::Charge, ironsmith::CounterType::Time] {
        wasm.game.object_mut(recipient).unwrap().counters.insert(kind, initial);
    }
    let mut context = ExecutionContext::new_default(source, PlayerId(0));
    let outcome = if wide {
        DoubleCountersEffect::new(None, ironsmith::target::ChooseSpec::SpecificObject(recipient))
            .execute(&mut wasm.game, &mut context).unwrap()
    } else {
        ProliferateEffect::new(1).execute(&mut wasm.game, &mut context).unwrap()
    };
    for event in outcome.events { wasm.game.queue_trigger_event(event.provenance(), event); }
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut wasm.game, &mut wasm.trigger_queue).unwrap();
    assert_eq!(wasm.trigger_queue.entries.len(), 1);
    (wasm, source, i64::from(initial) * 2)
}

fn counter_recipient_native_stack_amount(wasm: &WasmGame) -> i64 {
    let entry = wasm.game.stack.last().unwrap();
    let mut context = ironsmith::effects::EffectContext::new_default(entry.object_id, entry.controller)
        .with_triggering_event(entry.triggering_event.clone().unwrap());
    context.event_value_amount = entry.event_value_amount;
    ironsmith::effects::helpers::resolve_value_wide(&wasm.game,
        &ironsmith::effect::Value::EventValue(ironsmith::effect::EventValueSpec::Amount), &context).unwrap()
}

#[test]
fn counter_recipient_queue_stack_and_copied_stack_survive_native_savepoints() {
    use ironsmith::effects::{EffectExecutor, EffectContext as ExecutionContext};
    let _guard = crate::test_id_counter_guard();
    for wide in [false, true] {
        let (mut wasm, source, expected) = counter_recipient_native_fixture(wide);
        let queued = RuntimeSavepoint::capture(&wasm);
        ironsmith::game_loop::put_triggers_on_stack(&mut wasm.game, &mut wasm.trigger_queue).unwrap();
        assert_eq!(counter_recipient_native_stack_amount(&wasm), expected);
        let stacked = RuntimeSavepoint::capture(&wasm);
        let ability = wasm.game.stack.last().unwrap().ability_id.unwrap();
        ironsmith::effects::CopySpellEffect::single(ironsmith::target::ChooseSpec::SpecificObject(ability))
            .execute(&mut wasm.game, &mut ExecutionContext::new_default(source, PlayerId(0))).unwrap();
        assert_eq!(wasm.game.stack.len(), 2);
        assert_eq!(counter_recipient_native_stack_amount(&wasm), expected);
        let copied = RuntimeSavepoint::capture(&wasm);
        if !wide {
            ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();
            ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();
            assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life, 16);
        }
        copied.restore(&mut wasm);
        assert_eq!(wasm.game.stack.len(), 2);
        assert_eq!(counter_recipient_native_stack_amount(&wasm), expected);
        stacked.restore(&mut wasm);
        assert_eq!(wasm.game.stack.len(), 1);
        assert_eq!(counter_recipient_native_stack_amount(&wasm), expected);
        queued.restore(&mut wasm);
        assert!(wasm.game.stack.is_empty());
        assert_eq!(wasm.trigger_queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack(&mut wasm.game, &mut wasm.trigger_queue).unwrap();
        assert_eq!(counter_recipient_native_stack_amount(&wasm), expected);
        if !wide {
            ironsmith::game_loop::resolve_stack_entry(&mut wasm.game).unwrap();
            assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life, 18);
        }
    }
}
