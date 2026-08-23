#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn medomai_can_attack_on_normal_turns_but_not_extra_turns() {
    let definition = parse_oracle_card_definition("Medomai the Ageless");
    let compiled = canonical_compiled_lines(&definition);
    assert_eq!(compiled.len(), 3, "{compiled:#?}\n{definition:#?}");
    assert_eq!(compiled[0], "Flying");
    assert!(
        compiled[1].contains("Medomai deals combat damage to a player")
            && compiled[1].contains("take an extra turn after this one"),
        "{compiled:#?}\n{definition:#?}"
    );
    assert!(
        compiled[2].contains("can't attack during extra turns"),
        "{compiled:#?}\n{definition:#?}"
    );
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);

    let medomai = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.remove_summoning_sickness(medomai);

    assert!(!game.turn_store.current_turn_is_extra);
    assert!(
        game.can_attack(medomai),
        "Medomai's restriction must not apply during the initial normal turn"
    );

    game.turn_store.extra_turns.push(alice);
    game.next_turn();

    assert_eq!(game.turn.active_player, alice);
    assert!(game.turn_store.current_turn_is_extra);
    assert!(
        !game.can_attack(medomai),
        "Medomai must not be able to attack during a queued extra turn"
    );

    game.next_turn();

    assert!(!game.turn_store.current_turn_is_extra);
    game.next_turn();

    assert_eq!(game.turn.active_player, alice);
    assert!(!game.turn_store.current_turn_is_extra);
    assert!(
        game.can_attack(medomai),
        "the restriction must stop applying after the extra turn ends"
    );
}
