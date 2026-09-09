use super::*;

#[test]
fn counter_placement_condition_tracks_actor_kind_and_turn_after_recipient_leaves() {
    let oracle = "As long as you've put one or more +1/+1 counters on a creature this turn, this creature has trample and lifelink.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter History Creature")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .parse_text(oracle).unwrap();
    assert!(definition.spell_effect.is_none(), "a historical condition must not become a spell action: {definition:#?}");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert_eq!(rendered, "This creature has trample and lifelink as long as you've put one or more +1/+1 counters on a creature this turn.");
    for own_action in [false, true] {
        for creature_recipient in [false, true] {
            for plus_counter in [false, true] {
                let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let recipient = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counter Recipient")
                    .card_types(vec![if creature_recipient { CardType::Creature } else { CardType::Artifact }])
                    .power_toughness(crate::card::PowerToughness::fixed(3, 3)).build();
                // A creature controlled by an opponent still qualifies if you put the counters on it.
                let target = game.create_object_from_card(&recipient, bob, Zone::Battlefield);
                for keyword in [crate::static_abilities::StaticAbilityId::Trample, crate::static_abilities::StaticAbilityId::Lifelink] {
                    assert!(!game.current_has_static_ability_id(source, keyword));
                }
                let event = game.add_counters_with_source(target,
                    if plus_counter { crate::object::CounterType::PlusOnePlusOne } else { crate::object::CounterType::MinusOneMinusOne },
                    1, Some(if own_action { source } else { target }), Some(if own_action { alice } else { bob })).unwrap();
                game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event);
                crate::game_loop::drain_pending_trigger_events(&mut game, &mut crate::triggers::TriggerQueue::new());
                let expected = own_action && creature_recipient && plus_counter;
                for keyword in [crate::static_abilities::StaticAbilityId::Trample, crate::static_abilities::StaticAbilityId::Lifelink] {
                    assert_eq!(game.current_has_static_ability_id(source, keyword), expected,
                        "actor={own_action}, creature={creature_recipient}, plus={plus_counter}");
                }
                game.move_object_by_effect(target, Zone::Graveyard).unwrap();
                assert_eq!(game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Trample), expected,
                    "history must survive the counter recipient leaving the battlefield");
                game.next_turn();
                assert!(!game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Trample));
                assert!(!game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Lifelink));
            }
        }
    }
}

#[test]
fn counter_placement_condition_sums_only_matching_players_placements() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Two Counter History")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .parse_text("As long as you have put two or more +1/+1 counters on a creature this turn, this creature has trample.").unwrap();
    assert!(definition.spell_effect.is_none());
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for (actor, amount, enabled) in [(Some(bob), 3, false), (None, 3, false), (Some(alice), 1, false), (Some(alice), 1, true)] {
        let event = game.add_counters_with_source(source, crate::object::CounterType::PlusOnePlusOne, amount,
            actor.map(|_| source), actor).unwrap();
        game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event);
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut crate::triggers::TriggerQueue::new());
        assert_eq!(game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Trample), enabled,
            "actor={actor:?}, amount={amount}");
    }
}
