#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Target creature you control gets +1/+0 and gains deathtouch until end of turn. Whenever a creature dealt damage by that creature dies this turn, its controller loses 2 life. (Any amount of damage a creature with deathtouch deals to a creature is enough to destroy it.)";

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .build()
}

fn damage_event(source: ObjectId, target: ObjectId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Object(target),
            1,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn dies_event(
    object: ObjectId,
    snapshot: crate::snapshot::ObjectSnapshot,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            object,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn touch_of_moonglove_keeps_the_previous_damager_and_independent_dying_victim() {
    let definition = parse_oracle_card_definition("Touch of Moonglove");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let schedule = definition
        .spell_effect
        .as_ref()
        .expect("Touch should have a spell program")
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .expect("Touch should register one delayed death watcher");
    let dies = schedule
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .expect("the watcher should remain a typed death trigger");

    assert!(schedule.target_tag.is_some());
    assert!(!schedule.one_shot);
    assert_eq!(
        schedule.duration,
        ironsmith_core::DelayedTriggerDuration::EndOfTurn
    );
    assert_eq!(dies.object_filter.card_types, [CardType::Creature]);
    assert_eq!(
        dies.object_filter.dealt_damage_by_source_this_turn,
        Some(ironsmith_core::DamagedBySource::ThisCreature)
    );
}

#[test]
fn touch_triggers_only_for_a_victim_damaged_by_the_targeted_creature() {
    let definition = parse_oracle_card_definition("Touch of Moonglove");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let watched_damager =
        game.create_object_from_definition(&creature("Watched Damager"), alice, Zone::Battlefield);
    let other_damager =
        game.create_object_from_definition(&creature("Other Damager"), alice, Zone::Battlefield);
    let matching_victim =
        game.create_object_from_definition(&creature("Matching Victim"), bob, Zone::Battlefield);
    let unrelated_victim =
        game.create_object_from_definition(&creature("Unrelated Victim"), bob, Zone::Battlefield);

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(watched_damager)]),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Touch should resolve and register its delayed watcher");
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert_eq!(
        game.effect_store.delayed_triggers[0].target_objects,
        [watched_damager]
    );

    game.record_turn_history_event(&damage_event(watched_damager, matching_victim));
    game.record_turn_history_event(&damage_event(other_damager, unrelated_victim));

    let unrelated_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(unrelated_victim)
            .expect("unrelated victim exists"),
        &game,
    );
    game.move_object_by_effect(unrelated_victim, Zone::Graveyard)
        .expect("unrelated victim should move");
    assert!(
        crate::triggers::check_delayed_triggers(
            &mut game,
            &dies_event(unrelated_victim, unrelated_snapshot),
        )
        .is_empty()
    );

    let matching_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(matching_victim)
            .expect("matching victim exists"),
        &game,
    );
    game.move_object_by_effect(matching_victim, Zone::Graveyard)
        .expect("matching victim should move");
    let entries = crate::triggers::check_delayed_triggers(
        &mut game,
        &dies_event(matching_victim, matching_snapshot),
    );
    assert_eq!(entries.len(), 1);

    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Touch's delayed ability should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Touch's delayed ability should resolve");
    assert_eq!(game.life_total(bob), 18);
}
