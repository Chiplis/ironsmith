#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::ExecutionContext;
use crate::ids::CardId;
use crate::object::ObjectKind;
use crate::types::CardType;

fn enters_event(object: ObjectId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            object,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn theoretical_duplication_copies_only_an_opponents_nontoken_creature_this_turn() {
    let definition = parse_oracle_card_definition("Theoretical Duplication");
    let spell_effect = definition
        .spell_effect
        .as_ref()
        .expect("Theoretical Duplication should have a spell program");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(spell, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell,
        spell_effect,
        None,
        &[],
    )
    .expect("Theoretical Duplication should install its temporary trigger");

    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    let delayed = &game.effect_store.delayed_triggers[0];
    assert_eq!(
        delayed.expires_at_turn,
        Some(game.turn.turn_number),
        "the temporary trigger must expire with the current turn"
    );

    let creature = |name: &str, token: bool| {
        let builder = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3));
        if token {
            builder.token().build()
        } else {
            builder.build()
        }
    };
    let your_creature = game.create_object_from_definition(
        &creature("Your Nontoken", false),
        alice,
        Zone::Battlefield,
    );
    let opponent_token = game.create_object_from_definition(
        &creature("Opponent Token", true),
        bob,
        Zone::Battlefield,
    );
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &enters_event(your_creature)).is_empty(),
        "your own nontoken creature must not trigger the delayed ability"
    );
    assert!(
        crate::triggers::check_delayed_triggers(&mut game, &enters_event(opponent_token))
            .is_empty(),
        "an opponent's token creature must not trigger the delayed ability"
    );

    let opponent_creature = game.create_object_from_definition(
        &creature("Opponent Nontoken", false),
        bob,
        Zone::Battlefield,
    );
    let entries =
        crate::triggers::check_delayed_triggers(&mut game, &enters_event(opponent_creature));
    assert_eq!(entries.len(), 1);
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("the delayed copy trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("the delayed copy trigger should resolve");

    let copies = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| {
            object.name == "Opponent Nontoken"
                && matches!(object.kind, ObjectKind::Token)
                && game.controller_of(object) == alice
        })
        .count();
    assert_eq!(copies, 1, "Alice should create exactly one token copy");
}
