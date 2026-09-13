//! Frozen atlas observation 25070861: verify existing connive support on the full card.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::object::CounterType;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};

fn inspector() -> CardDefinition {
    let payloads =
        load_card_payloads_by_name(default_cards_path().to_str().unwrap(), "Echo Inspector")
            .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(!snapshot.has_unimplemented && !snapshot.parse_lossy);
    compile_definition_from_payload(&payloads[0]).unwrap()
}

#[test]
fn echo_inspector_connives_only_itself_and_counts_nonland_discards() {
    let definition = inspector();
    assert_eq!(
        definition
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .count(),
        1
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for discard_land in [true, false] {
        for leave_before_resolution in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let first = CardDefinitionBuilder::new(CardId::new(), "First discard")
                .card_types(vec![if discard_land {
                    CardType::Land
                } else {
                    CardType::Instant
                }])
                .build();
            let drawn = CardDefinitionBuilder::new(CardId::new(), "Drawn card")
                .card_types(vec![CardType::Instant])
                .build();
            let discard = game.create_object_from_definition(&first, alice, Zone::Hand);
            game.create_object_from_definition(&drawn, alice, Zone::Library);
            game.create_object_from_definition(&drawn, bob, Zone::Library);
            let bystander =
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
            let source = game
                .move_object_with_etb_processing(hand, Zone::Battlefield)
                .unwrap()
                .new_id;
            assert!(game.object_has_static_ability_id(
                source,
                ironsmith::static_abilities::StaticAbilityId::Flying
            ));
            let mut queue = TriggerQueue::new();
            for event in game.take_pending_trigger_events() {
                for entry in check_triggers(&game, &event) {
                    queue.add(entry);
                }
            }
            assert_eq!(queue.entries.len(), 1);
            let departed = leave_before_resolution
                .then(|| game.move_object_by_effect(source, Zone::Graveyard).unwrap());
            ironsmith::game_loop::run_priority_loop_with(
                &mut game,
                &mut queue,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert!(game.player(alice).unwrap().library.is_empty());
            assert_eq!(game.player(alice).unwrap().hand.len(), 1);
            assert_eq!(game.player(bob).unwrap().library.len(), 1);
            assert!(game.object(discard).is_none());
            let bystander_counters = game
                .object(bystander)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0);
            assert_eq!(bystander_counters, 0);
            if let Some(departed) = departed {
                assert_eq!(game.object(departed).unwrap().zone, Zone::Graveyard);
                assert_eq!(
                    game.object(departed)
                        .unwrap()
                        .counters
                        .get(&CounterType::PlusOnePlusOne)
                        .copied()
                        .unwrap_or(0),
                    0
                );
            } else {
                assert_eq!(
                    game.object(source)
                        .unwrap()
                        .counters
                        .get(&CounterType::PlusOnePlusOne)
                        .copied()
                        .unwrap_or(0),
                    u32::from(!discard_land)
                );
            }
        }
    }
}

#[test]
fn echo_inspector_connive_uses_current_or_last_controller() {
    let definition = inspector();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Controller draw")
        .card_types(vec![CardType::Instant])
        .build();
    for depart in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for player in [alice, bob] {
            game.create_object_from_definition(&filler, player, Zone::Hand);
            game.create_object_from_definition(&filler, player, Zone::Library);
        }
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        assert_eq!(queue.entries.len(), 1);
        game.set_current_controller(source, bob);
        if depart {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        }
        ironsmith::game_loop::run_priority_loop_with(
            &mut game,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(
            game.player(alice).unwrap().library.len(),
            1,
            "ability controller must not draw instead of the conniving creature's controller; departed={depart}"
        );
        assert!(
            game.player(bob).unwrap().library.is_empty(),
            "current/last controller must draw; departed={depart}"
        );
    }
}

#[test]
fn echo_inspector_original_entry_does_not_connive_a_later_incarnation() {
    let definition = inspector();
    let alice = PlayerId::from_index(0);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Discard for old entry")
        .card_types(vec![CardType::Instant])
        .build();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&filler, alice, Zone::Hand);
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let source = game
        .move_object_with_etb_processing(hand, Zone::Battlefield)
        .unwrap()
        .new_id;
    let mut queue = TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        game.turn_store
            .turn_history
            .record_event(&event, None, None);
        for entry in check_triggers(&game, &event) {
            queue.add(entry);
        }
    }
    assert_eq!(queue.entries.len(), 1);
    let exiled = game.move_object_by_effect(source, Zone::Exile).unwrap();
    let returned = game
        .move_object_with_etb_processing(exiled, Zone::Battlefield)
        .unwrap()
        .new_id;
    // Isolate the first entry's ability. The new entry has its own independent
    // trigger, already covered by the entry test; retain zone history for LKI.
    for event in game.take_pending_trigger_events() {
        game.turn_store
            .turn_history
            .record_event(&event, None, None);
    }
    ironsmith::game_loop::run_priority_loop_with(
        &mut game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(game.player(alice).unwrap().library.is_empty());
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(
        game.object(returned)
            .unwrap()
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        0,
        "the old entry's connive must not put a counter on a later incarnation"
    );
}
