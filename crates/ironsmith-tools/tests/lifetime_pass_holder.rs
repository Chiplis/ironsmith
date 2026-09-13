//! Frozen atlas observation 25063042: full catalog compilation and event semantics.
use ironsmith::decision::{AutoPassDecisionMaker, SelectFirstDecisionMaker};
use ironsmith::events::other::DieRolledEvent;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{AbilityKind, CardDefinition, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};

fn definition(name: &str) -> CardDefinition {
    let cards = load_card_payloads_by_name(default_cards_path().to_str().unwrap(), name).unwrap();
    assert_eq!(cards.len(), 1);
    let snapshot = compile_authoritative_snapshot_from_payload(&cards[0]);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        !snapshot.has_unimplemented && !snapshot.parse_lossy,
        "{snapshot:#?}"
    );
    compile_definition_from_payload(&cards[0]).unwrap()
}

#[test]
fn lifetime_pass_holder_preserves_graveyard_condition_optionality_and_roll_identity() {
    let definition = definition("\"Lifetime\" Pass Holder");
    let triggers: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|ability| {
            if let AbilityKind::Triggered(triggered) = &ability.kind {
                Some((ability, triggered))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(triggers.len(), 2, "{definition:#?}");
    let (_, returning) = triggers
        .iter()
        .find(|(a, _)| a.functional_zones == vec![Zone::Graveyard])
        .unwrap();
    assert!(returning.intervening_if.is_some());
    assert!(format!("{returning:#?}").contains("DieResult"));
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (player, result, visit, accept, expected) in [
        (alice, 6, true, true, true),
        (alice, 6, true, false, false),
        (alice, 5, true, true, false),
        (alice, 1, true, true, false),
        (alice, 6, false, true, false),
        (bob, 6, true, true, false),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        // Put a competing same-owner card first so an arbitrary graveyard
        // choice cannot accidentally select the correct source.
        let competing = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        // An earlier successful roll must not qualify a later failed roll.
        game.turn_store.turn_history.record_die_roll(alice, 6);
        let mut roll = DieRolledEvent::new(player, source, result, 6);
        if visit {
            roll = roll.for_attraction_visit();
        }
        let event = TriggerEvent::new_with_provenance(roll, Default::default());
        let mut queue = TriggerQueue::new();
        for entry in check_triggers(&game, &event)
            .into_iter()
            .filter(|entry| entry.source == source)
        {
            queue.add(entry);
        }
        assert_eq!(
            queue.entries.len(),
            usize::from(player == alice && result == 6 && visit)
        );
        // Resolution must still inspect the triggering event after another roll.
        game.turn_store.turn_history.record_die_roll(alice, 2);
        if accept {
            ironsmith::game_loop::run_priority_loop_with(
                &mut game,
                &mut queue,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
        } else {
            ironsmith::game_loop::run_priority_loop_with(
                &mut game,
                &mut queue,
                &mut AutoPassDecisionMaker,
            )
            .unwrap();
        }
        let returned: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|object| object.owner == alice && object.name == "\"Lifetime\" Pass Holder")
            .collect();
        assert_eq!(returned.len(), usize::from(expected));
        if expected {
            assert!(game.is_tapped(returned[0].id));
        }
        assert_eq!(game.object(competing).unwrap().zone, Zone::Graveyard);
    }
}

#[test]
fn lifetime_pass_holder_enters_tapped_opens_on_death_and_returns_on_visit_roll() {
    use ironsmith::effect::Effect;
    use ironsmith::game_state::StackEntry;
    use ironsmith::target::ChooseSpec;
    let holder = definition("\"Lifetime\" Pass Holder");
    // This catalog fixture supplies a real, legal Attraction deck for the
    // holder's death trigger and the production precombat-main roll.
    let booth = definition("Information Booth");
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.enable_attractions(vec![(
        alice,
        ironsmith::game_state::AttractionDeckFormat::Limited,
        vec![booth.clone(), booth.clone(), booth],
    )])
    .unwrap();
    let hand = game.create_object_from_definition(&holder, alice, Zone::Hand);
    let entered = game
        .move_object_with_etb_processing(hand, Zone::Battlefield)
        .unwrap();
    assert!(entered.enters_tapped && game.is_tapped(entered.new_id));
    game.push_to_stack(StackEntry::ability(
        entered.new_id,
        alice,
        vec![Effect::new(ironsmith::effects::DestroyEffect::with_spec(
            ChooseSpec::SpecificObject(entered.new_id),
        ))],
    ));
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::run_priority_loop_with(
        &mut game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(
        game.face_up_attractions().len(),
        1,
        "death must open exactly one Attraction"
    );
    assert_eq!(game.attraction_deck(alice).unwrap().len(), 2);
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    // Information Booth draws on a six; supply a card so that draw cannot end
    // the game before the independent return trigger resolves.
    game.create_object_from_definition(&holder, alice, Zone::Library);
    game.force_next_die_roll(6);
    assert_eq!(
        ironsmith::game_loop::roll_to_visit_attractions(&mut game, &mut queue).unwrap(),
        Some(6)
    );
    assert_eq!(
        queue.entries.len(),
        2,
        "the Visit ability and graveyard return both trigger"
    );
    ironsmith::game_loop::run_priority_loop_with(
        &mut game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    let returned = game
        .battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).unwrap().name == holder.name())
        .unwrap();
    assert!(game.is_tapped(returned));
    assert!(game.player(alice).unwrap().graveyard.is_empty());
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
}

#[test]
fn lifetime_pass_holder_return_cannot_follow_a_new_zone_identity() {
    let holder = definition("\"Lifetime\" Pass Holder");
    let alice = PlayerId::from_index(0);
    for zone in [Zone::Hand, Zone::Battlefield, Zone::Exile] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&holder, alice, zone);
        let event = TriggerEvent::new_with_provenance(
            DieRolledEvent::new(alice, source, 6, 6).for_attraction_visit(),
            Default::default(),
        );
        assert!(
            check_triggers(&game, &event).is_empty(),
            "return must only function in graveyard"
        );
    }
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&holder, alice, Zone::Graveyard);
    let event = TriggerEvent::new_with_provenance(
        DieRolledEvent::new(alice, source, 6, 6).for_attraction_visit(),
        Default::default(),
    );
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(&game, &event) {
        queue.add(entry);
    }
    assert_eq!(queue.entries.len(), 1);
    let exiled = game
        .move_object(source, Zone::Exile, ironsmith::events::EventCause::effect())
        .unwrap();
    let new_identity = game
        .move_object(
            exiled,
            Zone::Graveyard,
            ironsmith::events::EventCause::effect(),
        )
        .unwrap();
    ironsmith::game_loop::run_priority_loop_with(
        &mut game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(game.battlefield.is_empty());
    assert_eq!(game.object(new_identity).unwrap().zone, Zone::Graveyard);
}
