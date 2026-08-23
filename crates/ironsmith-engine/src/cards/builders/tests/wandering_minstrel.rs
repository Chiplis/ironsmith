#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn all_colors() -> crate::color::ColorSet {
    crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN)
}

#[test]
fn wandering_minstrel_creates_an_all_colors_elemental() {
    let definition = parse_oracle_card_definition("The Wandering Minstrel");
    let rendered = unprocessed_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line
            .contains("create a 2/2 white, blue, black, red, and green Elemental creature token")
            || line.contains("create a 2/2 Elemental creature token that's all colors")),
        "the compiled token must retain all five colors: {rendered:#?}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let minstrel = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for index in 0..5 {
        let town = CardDefinitionBuilder::new(CardId::new(), format!("Town {index}"))
            .card_types(vec![CardType::Land])
            .subtypes(vec![crate::types::Subtype::Town])
            .build();
        game.create_object_from_definition(&town, alice, Zone::Battlefield);
    }

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfCombatEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    let entries = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == minstrel)
        .collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        1,
        "five controlled Towns should satisfy the intervening condition"
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("The Minstrel's Ballad should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("The Minstrel's Ballad should resolve");

    let elementals = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == "Elemental" && game.controller_of(object) == alice
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        elementals.len(),
        1,
        "the trigger should create one Elemental"
    );
    let token_id = elementals[0];
    let token = game.object(token_id).expect("Elemental token should exist");
    assert_eq!(token.colors(), all_colors());
    assert_eq!(game.current_power(token_id), Some(2));
    assert_eq!(game.current_toughness(token_id), Some(2));
}
