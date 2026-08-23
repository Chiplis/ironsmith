#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn creature_definition(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn gruul_spellbreaker_grants_hexproof_only_to_you_and_itself_during_your_turn() {
    let definition = parse_oracle_card_definition("Gruul Spellbreaker");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;

    let spellbreaker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other_alice_creature = game.create_object_from_definition(
        &creature_definition("Other Alice Creature"),
        alice,
        Zone::Battlefield,
    );
    let alice_source = game.create_object_from_definition(
        &creature_definition("Alice Source"),
        alice,
        Zone::Battlefield,
    );
    let bob_source = game.create_object_from_definition(
        &creature_definition("Bob Source"),
        bob,
        Zone::Battlefield,
    );
    game.refresh_continuous_state();

    assert_eq!(
        crate::targeting::can_target_object(&game, spellbreaker, bob_source, bob),
        crate::targeting::TargetingResult::Invalid(
            crate::targeting::TargetingInvalidReason::HasHexproof,
        ),
        "an opponent must not target Gruul Spellbreaker during its controller's turn"
    );
    assert!(
        crate::targeting::can_target_object(&game, spellbreaker, alice_source, alice).is_legal(),
        "hexproof must still permit the controller's source"
    );
    assert!(
        crate::targeting::can_target_object(&game, other_alice_creature, bob_source, bob)
            .is_legal(),
        "the grant must not spread to other creatures Alice controls"
    );
    assert!(
        !game.can_target_player_from_source(alice, bob_source),
        "Alice must have hexproof during her turn"
    );
    assert!(
        game.can_target_player_from_source(alice, alice_source),
        "Alice's own source must still be able to target her"
    );

    game.turn.active_player = bob;
    game.refresh_continuous_state();
    assert!(
        crate::targeting::can_target_object(&game, spellbreaker, bob_source, bob).is_legal(),
        "Gruul Spellbreaker's turn-scoped hexproof must end on Bob's turn"
    );
    assert!(
        game.can_target_player_from_source(alice, bob_source),
        "the controller's turn-scoped hexproof must end on Bob's turn"
    );
}
