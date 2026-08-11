#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "When this creature enters, it deals 1 damage to each opponent and 1 damage to each creature your opponents control.";

fn durable_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 4))
        .build()
}

fn entering_event(game: &crate::GameState, object: ObjectId) -> crate::triggers::TriggerEvent {
    let mut snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(object).expect("Dagger Caster should exist"),
        game,
    );
    snapshot.zone = Zone::Stack;
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            object,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn exact_shared_source_surface_and_all_opponent_recipients_execute_end_to_end() {
    let definition = parse_oracle_card_definition("Dagger Caster");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let mut game = crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let dagger = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let alice_creature = game.create_object_from_definition(
        &durable_creature("Alice Creature"),
        alice,
        Zone::Battlefield,
    );
    let bob_creature = game.create_object_from_definition(
        &durable_creature("Bob Creature"),
        bob,
        Zone::Battlefield,
    );
    let charlie_creature = game.create_object_from_definition(
        &durable_creature("Charlie Creature"),
        charlie,
        Zone::Battlefield,
    );

    let event = entering_event(&game, dagger);
    let entries = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == dagger)
        .collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        1,
        "Dagger Caster should have one ETB trigger"
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Dagger Caster's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Dagger Caster's trigger should resolve");

    assert_eq!(game.life_total(alice), 20);
    assert_eq!(game.life_total(bob), 19);
    assert_eq!(game.life_total(charlie), 19);
    assert_eq!(game.damage_on(alice_creature), 0);
    assert_eq!(game.damage_on(bob_creature), 1);
    assert_eq!(game.damage_on(charlie_creature), 1);
    assert_eq!(
        game.damage_on(dagger),
        0,
        "the caster itself is not an opponent-controlled creature"
    );
}
