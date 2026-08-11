#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn vanilla_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn land(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

#[test]
fn burning_sands_makes_the_dead_creatures_controller_choose_and_sacrifice_a_land() {
    let definition = parse_oracle_card_definition("Burning Sands");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Whenever a creature dies, its controller sacrifices a land of their choice."
    );

    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::ZoneChangeTrigger>(
            ),
            _ => None,
        })
        .expect("Burning Sands should compile to a typed zone-change trigger");
    assert_eq!(
        trigger.from,
        crate::triggers::ZonePattern::Specific(Zone::Battlefield)
    );
    assert_eq!(
        trigger.to,
        crate::triggers::ZonePattern::Specific(Zone::Graveyard)
    );
    assert_eq!(trigger.object_filter.card_types, vec![CardType::Creature]);
    let ability_debug = format!("{:#?}", definition.abilities);
    assert!(
        ability_debug.contains("ChooseObjectsEffect")
            && ability_debug.contains("SacrificePlayerEffect")
            && ability_debug.contains("ControllerOf(")
            && ability_debug.contains("\"triggering\""),
        "the chooser and sacrifice actor must derive from the triggering creature: {ability_debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let burning_sands = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let alice_land =
        game.create_object_from_definition(&land("Alice Land"), alice, Zone::Battlefield);
    let bob_land = game.create_object_from_definition(&land("Bob Land"), bob, Zone::Battlefield);
    let victim = game.create_object_from_definition(
        &vanilla_creature("Bob Creature"),
        bob,
        Zone::Battlefield,
    );

    game.move_object_by_effect(victim, Zone::Graveyard)
        .expect("Bob's creature should die");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue
            .entries
            .iter()
            .filter(|entry| entry.source == burning_sands)
            .count(),
        1,
        "Burning Sands should trigger exactly once for Bob's dead creature"
    );

    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Burning Sands' trigger should go on the stack");
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Bob should be able to choose and sacrifice Bob's land");

    assert!(
        game.battlefield.contains(&alice_land),
        "Burning Sands' controller must not sacrifice a land for Bob's creature"
    );
    assert!(
        !game.battlefield.contains(&bob_land),
        "the dead creature's controller must sacrifice their land"
    );
    assert!(
        game.player(bob)
            .expect("Bob exists")
            .graveyard
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Bob Land")),
        "Bob's chosen land should be in Bob's graveyard"
    );
}

#[test]
fn burning_sands_ignores_creature_cards_outside_the_battlefield_and_noncreatures_that_die() {
    let definition = parse_oracle_card_definition("Burning Sands");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let burning_sands = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let discarded_creature = game.create_object_from_definition(
        &vanilla_creature("Discarded Creature"),
        bob,
        Zone::Hand,
    );
    let destroyed_land =
        game.create_object_from_definition(&land("Destroyed Land"), bob, Zone::Battlefield);

    game.move_object_by_effect(discarded_creature, Zone::Graveyard)
        .expect("the creature card should move from hand to graveyard");
    game.move_object_by_effect(destroyed_land, Zone::Graveyard)
        .expect("the land should move from the battlefield to the graveyard");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);

    assert!(
        queue
            .entries
            .iter()
            .all(|entry| entry.source != burning_sands),
        "Burning Sands requires a creature's battlefield-to-graveyard transition"
    );
}
