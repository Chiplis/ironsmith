use super::*;

#[test]
fn trailing_if_combat_restriction_tracks_battlefield_presence() {
    let oracle = "This creature can't attack or block if an enchantment is on the battlefield.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Presence Gate Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(oracle)
            .unwrap();
    assert!(definition.spell_effect.is_none());
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [oracle]
    );
    for opponent_controls in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let other = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other")
            .card_types(vec![CardType::Creature])
            .build();
        let friend = game.create_object_from_card(&other, alice, Zone::Battlefield);
        let enemy = game.create_object_from_card(&other, bob, Zone::Battlefield);
        let enchantment = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Enchantment")
            .card_types(vec![CardType::Enchantment])
            .build();
        let mut object = game.create_object_from_card(
            &enchantment,
            if opponent_controls { bob } else { alice },
            Zone::Hand,
        );
        for zone in [
            Zone::Hand,
            Zone::Battlefield,
            Zone::Graveyard,
            Zone::Battlefield,
            Zone::Exile,
        ] {
            if game.object(object).unwrap().zone != zone {
                object = game.move_object_by_effect(object, zone).unwrap();
            }
            game.refresh_continuous_state();
            let allowed = zone != Zone::Battlefield;
            assert_eq!(game.can_attack(source), allowed, "{zone:?}");
            assert_eq!(game.can_block_attacker(source, enemy), allowed);
            assert!(game.can_attack(friend));
            assert!(game.can_block_attacker(friend, enemy));
        }
    }
}

#[test]
fn trailing_if_counter_condition_remains_bound_to_source() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter If Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "This creature can't attack or block if it has five or more +1/+1 counters on it.",
            )
            .unwrap();
    assert!(definition.spell_effect.is_none());
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other")
        .card_types(vec![CardType::Creature])
        .build();
    let enemy = game.create_object_from_card(&card, bob, Zone::Battlefield);
    game.add_counters(enemy, CounterType::PlusOnePlusOne, 10);
    for count in [0, 4, 5, 6, 2] {
        game.remove_counters(source, CounterType::PlusOnePlusOne, 100, None, None);
        game.add_counters(source, CounterType::PlusOnePlusOne, count);
        game.refresh_continuous_state();
        assert_eq!(game.can_attack(source), count < 5, "count={count}");
        assert_eq!(game.can_block_attacker(source, enemy), count < 5);
    }
}
