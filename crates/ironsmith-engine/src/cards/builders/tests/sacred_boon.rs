#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::GameState;

fn creature_definition(name: &str, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, toughness))
        .build()
}

fn resolve_prevention_spell_then_damage(
    card_name: &str,
    damage_amounts: &[u32],
) -> (GameState, ObjectId) {
    let definition = parse_oracle_card_definition(card_name);
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(
        debug.contains("event_value_from_prior_prevention: true"),
        "the parser must link {card_name}'s delayed amount to its prevention shield: {debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let protected = game.create_object_from_definition(
        &creature_definition("Protected Creature", 10),
        alice,
        Zone::Battlefield,
    );
    let damage_source = game.create_object_from_definition(
        &creature_definition("Damage Source", 2),
        bob,
        Zone::Battlefield,
    );

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(protected)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .unwrap_or_else(|error| panic!("{card_name} should resolve: {error}"));

    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    for &amount in damage_amounts {
        let processed = crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            damage_source,
            crate::events::DamageTarget::Object(protected),
            amount,
            false,
            crate::events::cause::EventCause::effect(),
        );
        let keywords = crate::rules::damage::source_damage_keywords(&game, damage_source, None);
        for assignment in processed.assignments {
            crate::rules::damage::apply_processed_damage_assignment(
                &mut game,
                damage_source,
                assignment.target,
                assignment.amount,
                keywords,
                crate::events::cause::EventCause::effect(),
            );
        }
    }

    (game, protected)
}

fn resolve_next_end_step(game: &mut GameState, expected_prevented: i32) {
    let alice = PlayerId::from_index(0);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    let entries = crate::triggers::check_delayed_triggers(game, &event);
    assert_eq!(entries.len(), 1, "Sacred Boon should trigger once");
    assert_eq!(
        entries[0].event_value_amount,
        Some(expected_prevented),
        "the delayed trigger must carry the shield's accumulated prevented damage"
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("Sacred Boon's delayed trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(game)
        .expect("Sacred Boon's delayed counter effect should resolve");
}

#[test]
fn sacred_boon_partial_prevention_puts_one_toughness_counter() {
    let (mut game, protected) = resolve_prevention_spell_then_damage("Sacred Boon", &[1]);
    assert_eq!(game.damage_on(protected), 0);

    resolve_next_end_step(&mut game, 1);

    assert_eq!(
        game.counter_count(protected, CounterType::PlusZeroPlusOne),
        1
    );
}

#[test]
fn sacred_boon_accumulates_multiple_events_and_caps_at_three() {
    let (mut game, protected) = resolve_prevention_spell_then_damage("Sacred Boon", &[2, 2]);
    assert_eq!(
        game.damage_on(protected),
        1,
        "the shield should prevent 2 then its remaining 1 damage"
    );
    assert!(
        game.effect_store.prevention_effects.shields().is_empty(),
        "the exhausted shield may be removed before the delayed trigger"
    );

    resolve_next_end_step(&mut game, 3);

    assert_eq!(
        game.counter_count(protected, CounterType::PlusZeroPlusOne),
        3
    );
}

#[test]
fn scars_uses_the_original_creature_target_and_actual_prevented_amount() {
    let (mut game, protected) =
        resolve_prevention_spell_then_damage("Scars of the Veteran", &[2, 3]);
    assert_eq!(game.damage_on(protected), 0);

    resolve_next_end_step(&mut game, 5);

    assert_eq!(
        game.counter_count(protected, CounterType::PlusZeroPlusOne),
        5,
        "the delayed payload must affect the protected creature, not the spell source"
    );
}

#[test]
fn scars_does_not_schedule_creature_counters_for_a_player_target() {
    let definition = parse_oracle_card_definition("Scars of the Veteran");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Player(bob)]),
    );

    crate::game_loop::resolve_stack_entry(&mut game).expect("Scars should resolve");

    assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    assert!(
        game.effect_store.delayed_triggers.is_empty(),
        "the creature-only follow-up must not be scheduled for a player target"
    );
}
