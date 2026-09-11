use super::*;

const TEXT: &str = "When you cast a Bolas planeswalker spell, exile this card from your graveyard. That planeswalker enters with an additional loyalty counter on it.";

#[test]
fn graveyard_cast_entry_counter_follows_the_cast_spell() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Dark Intimations")
            .card_types(vec![CardType::Sorcery])
            .parse_text(TEXT)
            .unwrap();
    assert!(
        format!("{definition:#?}").contains("RegisterNextBatchEnterWithCountersEffect"),
        "{definition:#?}"
    );
    for remove_source in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let walker = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bolas Fixture")
            .card_types(vec![CardType::Planeswalker])
            .subtypes(vec![Subtype::Bolas])
            .loyalty(4)
            .build();
        let spell = game.create_object_from_definition(&walker, alice, Zone::Stack);
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::SpellCastEvent::new_with_snapshot(spell, alice, Zone::Hand, snapshot),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            1,
            "graveyard ability must trigger for its owner's Bolas cast"
        );
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        if remove_source {
            game.move_object_by_effect(source, Zone::Hand).unwrap();
        }
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        if !remove_source {
            assert!(game.exile.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|o| o.name == "Dark Intimations")
            }));
        }
        let unrelated = game.create_object_from_definition(&walker, bob, Zone::Hand);
        let unrelated = game
            .move_object_with_etb_processing(unrelated, Zone::Battlefield)
            .unwrap()
            .new_id;
        assert_eq!(
            game.object(unrelated)
                .unwrap()
                .counters
                .get(&crate::CounterType::Loyalty)
                .copied(),
            Some(4)
        );
        let entered = game
            .move_object_with_etb_processing(spell, Zone::Battlefield)
            .unwrap()
            .new_id;
        assert_eq!(
            game.object(entered)
                .unwrap()
                .counters
                .get(&crate::CounterType::Loyalty)
                .copied(),
            Some(5)
        );
    }
}

#[test]
fn cast_entry_riders_preserve_generic_filters_and_counters() {
    for text in [
        TEXT,
        "When you cast an Elf creature spell, exile this card from your graveyard. That creature enters with two additional +1/+1 counters on it.",
        "Whenever an opponent casts a planeswalker spell, exile this card from your graveyard. That planeswalker enters with three additional loyalty counters on it.",
    ] {
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cast Entry Fixture")
                .card_types(vec![CardType::Sorcery])
                .parse_text(text)
                .unwrap();
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("RegisterNextBatchEnterWithCountersEffect"),
            "{debug}"
        );
        assert!(debug.contains("same_stable_id_tag: Some"), "{debug}");
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            text
        );
    }
}

#[test]
fn ordinary_counter_placement_after_source_exile_is_not_an_entry_rider() {
    let text = "When you cast a Bolas planeswalker spell, exile this card from your graveyard. Put a loyalty counter on the exiled card.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ordinary Counter Fixture")
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap();
    let debug = format!("{definition:#?}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(
        !debug.contains("RegisterNextBatchEnterWithCountersEffect"),
        "{debug}"
    );
}

#[test]
fn graveyard_cast_trigger_preserves_caster_subtype_and_source_zone() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cast Trigger Fixture")
            .card_types(vec![CardType::Sorcery])
            .parse_text(TEXT)
            .unwrap();
    for (source_zone, opponent_cast, subtype, expected) in [
        (Zone::Graveyard, false, Subtype::Bolas, 1),
        (Zone::Graveyard, true, Subtype::Bolas, 0),
        (Zone::Graveyard, false, Subtype::Gideon, 0),
        (Zone::Battlefield, false, Subtype::Bolas, 0),
        (Zone::Exile, false, Subtype::Bolas, 0),
    ] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let caster = if opponent_cast {
            game.players[1].id
        } else {
            alice
        };
        game.create_object_from_definition(&definition, alice, source_zone);
        let walker = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Walker Fixture")
            .card_types(vec![CardType::Planeswalker])
            .subtypes(vec![subtype])
            .loyalty(4)
            .build();
        let spell = game.create_object_from_definition(&walker, caster, Zone::Stack);
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::SpellCastEvent::new_with_snapshot(spell, caster, Zone::Hand, snapshot),
            crate::provenance::ProvNodeId::default(),
        );
        assert_eq!(
            crate::triggers::check_triggers(&game, &event).len(),
            expected,
            "source={source_zone:?} opponent_cast={opponent_cast} subtype={subtype:?}"
        );
    }
}
