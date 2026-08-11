#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const END_STEP_LINE: &str = "At the beginning of your end step, if you have at least 15 life more than your starting life total, each player this creature attacked this turn loses the game.";

fn attack_event(attacker: ObjectId, defender: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::new(
            attacker,
            crate::triggers::event::AttackEventTarget::Player(defender),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn end_step_event(player: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn angel_of_destiny_only_defeats_players_it_attacked_this_turn() {
    let definition = parse_oracle_card_definition("Angel of Destiny");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == END_STEP_LINE),
        "Angel's compiled text must retain the source-relative attacked-player filter: {rendered:#?}"
    );
    assert!(
        format!("{:#?}", definition.abilities).contains("AttackedBySourceThisTurn"),
        "Angel must compile an executable attacked-player filter"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()],
        20,
    );
    game.turn.active_player = alice;
    let angel = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let decoy_definition = CardDefinitionBuilder::new(CardId::new(), "Other Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let decoy = game.create_object_from_definition(&decoy_definition, alice, Zone::Battlefield);

    game.record_turn_history_event(&attack_event(angel, bob));
    game.record_turn_history_event(&attack_event(decoy, carol));

    game.player_mut(alice).expect("Alice should exist").life = 34;
    let event = end_step_event(alice);
    assert!(
        crate::triggers::check_triggers(&game, &event)
            .into_iter()
            .all(|entry| entry.source != angel),
        "Angel's intervening-if must fail below 15 life above the starting total"
    );

    game.player_mut(alice).expect("Alice should exist").life = 35;
    let triggers = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == angel)
        .collect::<Vec<_>>();
    assert_eq!(triggers.len(), 1, "Angel's end-step ability should trigger");

    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in triggers {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Angel's end-step trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Angel's end-step trigger should resolve");

    assert!(
        !game.player(bob).expect("Bob should exist").is_in_game(),
        "the player Angel attacked must lose"
    );
    assert!(
        game.player(carol).expect("Carol should exist").is_in_game(),
        "a player attacked only by another creature must not lose"
    );
    assert!(
        game.player(alice).expect("Alice should exist").is_in_game(),
        "Angel's controller must not be included merely because the effect says each player"
    );
}
