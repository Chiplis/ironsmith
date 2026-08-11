#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn artifact_definition(name: &str, creature: bool, indestructible: bool) -> CardDefinition {
    let mut card_types = vec![CardType::Artifact];
    if creature {
        card_types.push(CardType::Creature);
    }
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(card_types);
    if creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    if indestructible {
        builder = builder.with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::indestructible(),
        ));
    }
    builder.build()
}

fn queue_outcome_events(game: &mut crate::GameState, outcome: crate::effect::EffectOutcome) {
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}

fn take_jaws_triggers(
    game: &mut crate::GameState,
    jaws: ObjectId,
) -> crate::triggers::TriggerQueue {
    let mut pending = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(game, &mut pending);
    let mut matching = crate::triggers::TriggerQueue::new();
    for entry in pending.entries {
        if entry.source == jaws {
            matching.add(entry);
        }
    }
    matching
}

fn resolve_one_jaws_trigger(game: &mut crate::GameState, jaws: ObjectId) {
    let mut queue = take_jaws_triggers(game, jaws);
    assert_eq!(queue.entries.len(), 1, "expected exactly one Jaws trigger");
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("Jaws' trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(game).expect("Jaws' trigger should resolve");
}

#[test]
fn jaws_keeps_the_passive_sacrifice_or_successful_destruction_union() {
    let definition = parse_oracle_card_definition("Jaws, Relentless Predator");
    assert_eq!(
        unprocessed_compiled_lines(&definition).join("\n"),
        "Trample, haste\nWhenever Jaws deals combat damage to a player, create that many Blood tokens.\nWhenever a noncreature artifact is sacrificed or destroyed, Jaws deals 1 damage to each opponent."
    );

    let union = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::AnyOfTrigger>(),
            _ => None,
        })
        .expect("Jaws should keep the sacrifice/destroy event union");
    let [sacrificed, destroyed] = union.branches.as_slice() else {
        panic!("the event union should have exactly two branches: {union:#?}");
    };
    let sacrificed = sacrificed
        .downcast_ref::<crate::triggers::PermanentSacrificedTrigger>()
        .expect("first branch should be the sacrifice event");
    let destroyed = destroyed
        .downcast_ref::<crate::triggers::PermanentDestroyedTrigger>()
        .expect("second branch should be the successful-destruction event");
    assert_eq!(sacrificed.filter, destroyed.filter);
    assert_eq!(sacrificed.filter.card_types, vec![CardType::Artifact]);
    assert_eq!(
        sacrificed.filter.excluded_card_types,
        vec![CardType::Creature]
    );
}

#[test]
fn jaws_triggers_for_sacrifice_and_successful_destroy_but_not_near_misses() {
    let definition = parse_oracle_card_definition("Jaws, Relentless Predator");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let jaws = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let sacrificed = game.create_object_from_definition(
        &artifact_definition("Sacrificed Relic", false, false),
        alice,
        Zone::Battlefield,
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(jaws, alice);
    let outcome =
        crate::effects::SacrificeTargetEffect::new(ChooseSpec::SpecificObject(sacrificed))
            .execute(&mut game, &mut ctx)
            .expect("the artifact should be sacrificed");
    queue_outcome_events(&mut game, outcome);
    resolve_one_jaws_trigger(&mut game, jaws);
    assert_eq!(game.life_total(bob), 19);

    let destroyed = game.create_object_from_definition(
        &artifact_definition("Destroyed Relic", false, false),
        bob,
        Zone::Battlefield,
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(jaws, alice);
    crate::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(destroyed))
        .execute(&mut game, &mut ctx)
        .expect("the artifact should be destroyed");
    resolve_one_jaws_trigger(&mut game, jaws);
    assert_eq!(game.life_total(bob), 18);

    let indestructible = game.create_object_from_definition(
        &artifact_definition("Indestructible Relic", false, true),
        bob,
        Zone::Battlefield,
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(jaws, alice);
    crate::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(indestructible))
        .execute(&mut game, &mut ctx)
        .expect("attempting to destroy an indestructible artifact should resolve");
    assert!(take_jaws_triggers(&mut game, jaws).entries.is_empty());

    let moved = game.create_object_from_definition(
        &artifact_definition("Moved Relic", false, false),
        bob,
        Zone::Battlefield,
    );
    game.move_object_by_effect(moved, Zone::Graveyard)
        .expect("ordinary movement should succeed");
    assert!(take_jaws_triggers(&mut game, jaws).entries.is_empty());

    let artifact_creature = game.create_object_from_definition(
        &artifact_definition("Artifact Creature", true, false),
        bob,
        Zone::Battlefield,
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(jaws, alice);
    crate::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(artifact_creature))
        .execute(&mut game, &mut ctx)
        .expect("the artifact creature should be destroyed");
    assert!(take_jaws_triggers(&mut game, jaws).entries.is_empty());
    assert_eq!(game.life_total(bob), 18);
}
