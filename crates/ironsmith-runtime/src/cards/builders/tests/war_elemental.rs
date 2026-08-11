#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn entering_event(game: &crate::GameState, object: ObjectId) -> crate::triggers::TriggerEvent {
    let mut snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(object).expect("War Elemental should exist"),
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

fn resolve_etb_trigger(game: &mut crate::GameState, war_elemental: ObjectId) {
    let event = entering_event(game, war_elemental);
    let matching = crate::triggers::check_triggers(game, &event)
        .into_iter()
        .filter(|entry| entry.source == war_elemental)
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "War Elemental should have one ETB trigger"
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in matching {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("War Elemental's ETB trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(game)
        .expect("War Elemental's ETB trigger should resolve");
}

fn setup() -> (crate::GameState, PlayerId, PlayerId, ObjectId, StableId) {
    let definition = parse_oracle_card_definition("War Elemental");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let war_elemental = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let stable_id = game
        .object(war_elemental)
        .expect("War Elemental should exist")
        .stable_id;
    (game, alice, bob, war_elemental, stable_id)
}

#[test]
fn war_elemental_does_not_treat_non_damage_life_loss_as_damage() {
    let (mut game, _alice, bob, war_elemental, stable_id) = setup();
    let life_loss = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::life::LifeLossEvent::from_effect(bob, 2),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&life_loss);

    resolve_etb_trigger(&mut game, war_elemental);

    let moved = game
        .find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .expect("War Elemental should remain identifiable");
    assert_eq!(
        moved.zone,
        Zone::Graveyard,
        "ordinary life loss must not satisfy 'was dealt damage'"
    );
}

#[test]
fn war_elemental_survives_after_an_opponent_was_actually_dealt_damage() {
    let (mut game, alice, bob, war_elemental, stable_id) = setup();
    let damage_source_definition = CardDefinitionBuilder::new(CardId::new(), "Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let damage_source =
        game.create_object_from_definition(&damage_source_definition, alice, Zone::Battlefield);
    let damage = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            damage_source,
            crate::events::DamageTarget::Player(bob),
            2,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&damage);

    resolve_etb_trigger(&mut game, war_elemental);

    let current = game
        .find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .expect("War Elemental should remain identifiable");
    assert_eq!(current.zone, Zone::Battlefield);
}
