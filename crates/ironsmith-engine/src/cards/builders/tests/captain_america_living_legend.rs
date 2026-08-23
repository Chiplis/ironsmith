#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const TRIGGER_LINE: &str = "Whenever a creature you control becomes tapped during your turn, if it's the first time that creature has become tapped this turn, untap it.";

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn condition_contains(condition: &crate::ConditionExpr, expected: &crate::ConditionExpr) -> bool {
    condition == expected
        || matches!(
            condition,
            crate::ConditionExpr::And(left, right)
                if condition_contains(left, expected) || condition_contains(right, expected)
        )
}

fn tap_and_queue(
    game: &mut crate::GameState,
    queue: &mut crate::triggers::TriggerQueue,
    permanent: ObjectId,
) -> usize {
    game.tap(permanent);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::PermanentTappedEvent::new(permanent),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
    let entries = crate::triggers::check_triggers(game, &event);
    let count = entries.len();
    for entry in entries {
        queue.add(entry);
    }
    if count > 0 {
        crate::game_loop::put_triggers_on_stack(game, queue)
            .expect("matching Captain trigger should go on the stack");
    }
    count
}

fn resolve_only_trigger(game: &mut crate::GameState) {
    assert_eq!(game.stack.len(), 1, "expected exactly one Captain trigger");
    crate::game_loop::resolve_stack_entry(game).expect("Captain trigger should resolve");
}

#[test]
fn captain_america_first_tap_is_scoped_to_each_triggering_object_and_turn() {
    let definition = parse_oracle_card_definition("Captain America, Living Legend");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["Vigilance".to_string(), TRIGGER_LINE.to_string()]
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Captain should have a tap trigger");
    let condition = triggered
        .intervening_if
        .as_ref()
        .expect("Captain should preserve both trigger qualifications");
    assert!(condition_contains(
        condition,
        &crate::ConditionExpr::YourTurn
    ));
    assert!(condition_contains(
        condition,
        &crate::ConditionExpr::TriggeringObjectBecameTappedFirstTimeThisTurn
    ));
    assert!(
        !condition_contains(condition, &crate::ConditionExpr::FirstTimeThisTurn),
        "the ordinary ability-wide trigger cap is not per creature: {condition:#?}"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    game.turn.active_player = alice;
    let _captain = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let alpha = game.create_object_from_definition(&creature("Alpha"), alice, Zone::Battlefield);
    let beta = game.create_object_from_definition(&creature("Beta"), alice, Zone::Battlefield);
    let mut queue = crate::triggers::TriggerQueue::new();

    assert_eq!(tap_and_queue(&mut game, &mut queue, alpha), 1);
    resolve_only_trigger(&mut game);
    assert!(!game.is_tapped(alpha), "Alpha's first tap should be undone");

    assert_eq!(
        tap_and_queue(&mut game, &mut queue, alpha),
        0,
        "the same creature's second tap this turn must not trigger"
    );
    assert!(game.is_tapped(alpha));

    assert_eq!(
        tap_and_queue(&mut game, &mut queue, beta),
        1,
        "a different creature's first tap must still trigger"
    );
    resolve_only_trigger(&mut game);
    assert!(!game.is_tapped(beta));

    game.untap(alpha);
    game.turn_store.turn_history.clear_for_new_turn();
    assert_eq!(
        tap_and_queue(&mut game, &mut queue, alpha),
        1,
        "the same creature may trigger again on a later turn"
    );
    resolve_only_trigger(&mut game);

    game.turn_store.turn_history.clear_for_new_turn();
    game.turn.active_player = bob;
    assert_eq!(
        tap_and_queue(&mut game, &mut queue, alpha),
        0,
        "a first tap during an opponent's turn must not trigger"
    );
}
