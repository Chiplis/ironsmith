#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};

fn delayed_entries_to_stack(
    game: &mut crate::GameState,
    event: &crate::triggers::TriggerEvent,
) -> usize {
    let entries = crate::triggers::check_delayed_triggers(game, event);
    let count = entries.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("matching delayed triggers should go on the stack");
    count
}

#[test]
fn acidic_dagger_watches_only_its_target_for_combat_damage_and_leaving() {
    let definition = parse_oracle_card_definition("Acidic Dagger");
    let rendered = canonical_compiled_lines(&definition);
    let joined = rendered.join(" ");
    let joined_lower = joined.to_ascii_lowercase();
    assert!(
        joined_lower
            .contains("whenever target creature deals combat damage to a non-wall creature"),
        "the delayed combat-damage trigger must preserve its target declaration: {rendered:#?}"
    );
    assert!(
        joined_lower.contains("creature leaves the battlefield this turn"),
        "the delayed leave watcher must remain present: {rendered:#?}"
    );

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Acidic Dagger should have an activated ability");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let dagger = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = |name: &str, subtypes: Vec<Subtype>| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    };
    let watched = game.create_object_from_definition(
        &creature("Watched Attacker", vec![]),
        alice,
        Zone::Battlefield,
    );
    let other = game.create_object_from_definition(
        &creature("Other Attacker", vec![]),
        alice,
        Zone::Battlefield,
    );
    let victim = game.create_object_from_definition(
        &creature("Non-Wall Victim", vec![]),
        bob,
        Zone::Battlefield,
    );
    let wall = game.create_object_from_definition(
        &creature("Wall Victim", vec![Subtype::Wall]),
        bob,
        Zone::Battlefield,
    );

    let mut activation_ctx = ExecutionContext::new_default(dagger, alice)
        .with_targets(vec![ResolvedTarget::Object(watched)]);
    for effect in activated.effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut activation_ctx)
            .expect("Acidic Dagger's activated ability should resolve");
    }
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        2,
        "the activation should create both the damage and leave watchers"
    );
    assert!(
        game.effect_store
            .delayed_triggers
            .iter()
            .all(|trigger| trigger.target_objects == vec![watched]),
        "both delayed triggers must watch the announced target"
    );

    let combat_event = |source, target| {
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::DamageEvent::with_cause(
                source,
                crate::events::DamageTarget::Object(target),
                3,
                true,
                crate::events::cause::EventCause::combat_damage(source),
            ),
            crate::provenance::ProvNodeId::default(),
        )
    };
    assert_eq!(
        delayed_entries_to_stack(&mut game, &combat_event(other, victim)),
        0,
        "combat damage by an unselected creature must not trigger the Dagger"
    );
    assert_eq!(
        delayed_entries_to_stack(&mut game, &combat_event(watched, wall)),
        0,
        "combat damage to a Wall must not trigger the Dagger"
    );
    assert_eq!(
        delayed_entries_to_stack(&mut game, &combat_event(watched, victim)),
        1,
        "combat damage by the watched creature to a non-Wall should trigger"
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Acidic Dagger's destroy trigger should resolve");
    assert!(
        !game.battlefield.contains(&victim),
        "the damaged non-Wall creature should be destroyed"
    );

    let watched_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(watched)
            .expect("the watched creature should exist"),
        &game,
    );
    game.move_object_by_effect(watched, Zone::Graveyard);
    let leaves_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            watched,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(watched_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        delayed_entries_to_stack(&mut game, &leaves_event),
        1,
        "the watched creature leaving should trigger the sacrifice"
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Acidic Dagger's sacrifice trigger should resolve");
    assert!(
        !game.battlefield.contains(&dagger),
        "Acidic Dagger should be sacrificed when its targeted creature leaves"
    );
}
