#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

// Canonical compiled lines intentionally omit parenthetical reminder text.
const AEON_ENGINE_ORACLE: &str =
    "This artifact enters tapped.\n{T}, Exile this artifact: Reverse the game's turn order.";

const FIRE_MAGIC_ORACLE: &str = "Tiered\n• Fire — {0} — Fire Magic deals 1 damage to each creature.\n• Fira — {2} — Fire Magic deals 2 damage to each creature.\n• Firaga — {5} — Fire Magic deals 3 damage to each creature.";

#[test]
fn aeon_engine_and_fire_magic_round_trip_their_exact_oracle_text() {
    for (name, oracle) in [
        ("Aeon Engine", AEON_ENGINE_ORACLE),
        ("Fire Magic", FIRE_MAGIC_ORACLE),
    ] {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition).join("\n"),
            oracle,
            "{name} should render exactly: {definition:#?}"
        );
    }
}

#[test]
fn aeon_engine_activation_reverses_a_four_player_game_after_paying_its_costs() {
    let definition = parse_oracle_card_definition("Aeon Engine");
    let mut game = crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Cara".to_string(),
            "Dan".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let dan = PlayerId::from_index(3);
    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(bob);
    let source = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
    let source_stable_id = game.object(source).expect("Aeon Engine exists").stable_id;
    game.tap(source);
    game.untap(source);

    let ability_index = game
        .object(source)
        .expect("Aeon Engine exists")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Aeon Engine has an activated ability");
    let action = crate::decision::compute_legal_actions(&game, bob)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility {
                    source: candidate,
                    ability_index: candidate_index,
                } if *candidate == source && *candidate_index == ability_index
            )
        })
        .expect("Aeon Engine's untapped activation should be legal");
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(action),
        &mut decisions,
    )
    .expect("Aeon Engine activation should pay its costs and reach the stack");

    let exiled_source = game
        .find_object_by_stable_id(source_stable_id)
        .expect("Aeon Engine remains tracked after paying the exile cost");
    assert_eq!(
        game.object(exiled_source)
            .expect("Aeon Engine remains tracked")
            .zone,
        Zone::Exile,
        "exiling Aeon Engine is an activation cost"
    );
    assert_eq!(game.turn_store.turn_order, vec![alice, bob, cara, dan]);
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Aeon Engine's ability should resolve");
    assert_eq!(game.turn.active_player, bob);
    assert_eq!(game.turn_store.turn_order, vec![dan, cara, bob, alice]);
    game.next_turn();
    assert_eq!(game.turn.active_player, alice);
}
