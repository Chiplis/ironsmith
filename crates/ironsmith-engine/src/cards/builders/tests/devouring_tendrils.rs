#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Target creature you control deals damage equal to its power to target creature or planeswalker you don't control. When the permanent you don't control dies this turn, you gain 2 life.";

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
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
fn definite_death_clause_keeps_exact_prior_target_and_one_shot_surface() {
    let definition = parse_oracle_card_definition("Devouring Tendrils");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let spell = definition
        .spell_effect
        .as_ref()
        .expect("Devouring Tendrils should have a spell effect");
    let schedule = spell
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .expect("the spell should register one delayed death watcher");
    let death = schedule
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .expect("the delayed watcher should be a typed zone-change trigger");
    assert!(schedule.one_shot);
    assert!(schedule.until_end_of_turn);
    assert_eq!(
        schedule.duration,
        ironsmith_core::DelayedTriggerDuration::EndOfTurn
    );
    assert!(schedule.target_tag.is_some());
    assert!(
        schedule
            .target_filter
            .as_ref()
            .is_some_and(|filter| filter.demonstrative_antecedent_surface().is_some())
    );
    assert!(death.this_object);
    assert_eq!(
        death.from,
        crate::triggers::ZonePattern::Specific(Zone::Battlefield)
    );
    assert_eq!(
        death.to,
        crate::triggers::ZonePattern::Specific(Zone::Graveyard)
    );
}

#[test]
fn only_the_prior_damage_target_dying_fires_the_life_gain() {
    let definition = parse_oracle_card_definition("Devouring Tendrils");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let damage_source = game.create_object_from_definition(
        &creature("Controlled Damage Source", 1, 4),
        alice,
        Zone::Battlefield,
    );
    let watched = game.create_object_from_definition(
        &creature("Watched Opponent Creature", 2, 4),
        bob,
        Zone::Battlefield,
    );
    let unrelated = game.create_object_from_definition(
        &creature("Unrelated Opponent Creature", 2, 2),
        bob,
        Zone::Battlefield,
    );

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice).with_targets(vec![
            crate::game_state::Target::Object(damage_source),
            crate::game_state::Target::Object(watched),
        ]),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Devouring Tendrils should resolve and register its delayed watcher");

    assert_eq!(game.damage_on(watched), 1);
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    assert_eq!(
        game.effect_store.delayed_triggers[0].target_objects,
        [watched],
        "the delayed watcher must capture only the second spell target"
    );

    let unrelated_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(unrelated).expect("unrelated creature exists"),
        &game,
    );
    game.move_object_by_effect(unrelated, Zone::Graveyard)
        .expect("unrelated creature should move to the graveyard");
    assert!(
        crate::triggers::check_delayed_triggers(
            &mut game,
            &dies_event(unrelated, unrelated_snapshot),
        )
        .is_empty(),
        "an unrelated opponent permanent dying must not fire the watcher"
    );
    assert_eq!(game.life_total(alice), 20);

    let watched_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(watched).expect("watched creature exists"),
        &game,
    );
    game.move_object_by_effect(watched, Zone::Graveyard)
        .expect("watched creature should move to the graveyard");
    let entries =
        crate::triggers::check_delayed_triggers(&mut game, &dies_event(watched, watched_snapshot));
    assert_eq!(
        entries.len(),
        1,
        "the exact prior target dying should fire once"
    );
    assert!(
        game.effect_store.delayed_triggers.is_empty(),
        "the exact watcher should be consumed after its one matching death"
    );

    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("the delayed life-gain trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("the delayed life-gain trigger should resolve");
    assert_eq!(game.life_total(alice), 22);
}
