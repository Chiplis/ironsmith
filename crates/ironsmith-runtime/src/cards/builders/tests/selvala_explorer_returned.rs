#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::ExecutionContext;
use crate::ids::CardId;
use crate::types::CardType;

#[test]
fn selvala_parley_rewards_only_nonland_cards_revealed_this_way() {
    let definition = parse_oracle_card_definition("Selvala, Explorer Returned");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Selvala should have her activated parley ability");
    let debug = format!("{activated:#?}");
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("excluded_card_types"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let selvala = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let land = CardDefinitionBuilder::new(CardId::new(), "Revealed Land")
        .card_types(vec![CardType::Land])
        .build();
    let creature = CardDefinitionBuilder::new(CardId::new(), "Revealed Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&land, alice, Zone::Library);
    game.create_object_from_definition(&creature, bob, Zone::Library);

    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(selvala, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        selvala,
        &activated.effects,
        None,
        &[],
    )
    .expect("Selvala's parley ability should resolve");

    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.green,
        1,
        "only Bob's nonland reveal should produce green mana"
    );
    assert_eq!(
        game.life_total(alice),
        21,
        "only Bob's nonland reveal should gain life"
    );
    assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 1);
    assert_eq!(game.player(bob).expect("Bob exists").hand.len(), 1);
    let alice_hand_names = game
        .player(alice)
        .expect("Alice exists")
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    let bob_hand_names = game
        .player(bob)
        .expect("Bob exists")
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(alice_hand_names, vec!["Revealed Land"]);
    assert_eq!(bob_hand_names, vec!["Revealed Creature"]);
}
