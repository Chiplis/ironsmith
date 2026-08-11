#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn vanilla_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn three_player_game() -> crate::GameState {
    crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    )
}

#[test]
fn trespassers_curse_drains_the_enchanted_player_and_gains_its_controllers_life() {
    let definition = parse_oracle_card_definition("Trespasser's Curse");
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
        .expect("Trespasser's Curse should compile to a typed zone-change trigger");
    assert_eq!(
        trigger.to,
        crate::triggers::ZonePattern::Specific(Zone::Battlefield)
    );
    assert_eq!(trigger.object_filter.card_types, vec![CardType::Creature]);
    assert_eq!(
        trigger.object_filter.controller,
        Some(PlayerFilter::TaggedPlayer(crate::tag::TagKey::from(
            "enchanted"
        )))
    );
    let ability_debug = format!("{:#?}", definition.abilities);
    assert!(
        ability_debug.contains("AliasedControllerOf(")
            && ability_debug.contains("\"triggering\"")
            && ability_debug.contains("GainLifeEffect")
            && ability_debug.contains("player: Player(\n")
            && ability_debug.contains("You,"),
        "the life-loss actor must be the triggering creature's aliased controller and the gain actor must be the Curse's controller: {ability_debug}"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = three_player_game();
    let curse = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(curse, crate::object::AttachmentTarget::Player(bob)));
    let entering =
        game.create_object_from_definition(&vanilla_creature("Bob Creature"), bob, Zone::Hand);

    let entered = game
        .move_object_with_etb_processing(entering, Zone::Battlefield)
        .expect("Bob's creature should enter the battlefield")
        .new_id;
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue
            .entries
            .iter()
            .filter(|entry| entry.source == curse)
            .count(),
        1,
        "Trespasser's Curse should trigger once for Bob's entering creature"
    );

    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Trespasser's Curse's trigger should go on the stack");
    game.effect_store.continuous_effects.add_effect(
        crate::continuous::ContinuousEffect::gain_control(curse, alice, entered, charlie)
            .until(crate::Until::Forever),
    );
    assert_eq!(
        game.current_controller(entered),
        Some(charlie),
        "the entering creature should have a different controller before resolution"
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Trespasser's Curse's trigger should resolve");

    assert_eq!(game.life_total(bob), 19, "the enchanted player loses life");
    assert_eq!(
        game.life_total(alice),
        21,
        "the Curse's controller gains life"
    );
    assert_eq!(
        game.life_total(charlie),
        20,
        "the controller alias must remain the player from the ETB event"
    );
}

#[test]
fn trespassers_curse_ignores_other_players_creatures_and_noncreatures() {
    let definition = parse_oracle_card_definition("Trespasser's Curse");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = three_player_game();
    let curse = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(curse, crate::object::AttachmentTarget::Player(bob)));

    let alice_creature =
        game.create_object_from_definition(&vanilla_creature("Alice Creature"), alice, Zone::Hand);
    let charlie_creature = game.create_object_from_definition(
        &vanilla_creature("Charlie Creature"),
        charlie,
        Zone::Hand,
    );
    let bob_artifact_definition = CardDefinitionBuilder::new(CardId::new(), "Bob Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let bob_artifact =
        game.create_object_from_definition(&bob_artifact_definition, bob, Zone::Hand);

    for object in [alice_creature, charlie_creature, bob_artifact] {
        game.move_object_with_etb_processing(object, Zone::Battlefield)
            .expect("the negative-case permanent should enter the battlefield");
    }
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);

    assert!(
        queue.entries.iter().all(|entry| entry.source != curse),
        "Trespasser's Curse requires a creature entering under the enchanted player's control"
    );
    assert_eq!(game.life_total(alice), 20);
    assert_eq!(game.life_total(bob), 20);
    assert_eq!(game.life_total(charlie), 20);
}
