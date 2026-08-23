#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE_LINE: &str = "Whenever another creature you control or a land you control is put into a graveyard from the battlefield, put a +1/+1 counter on target creature you control.";

struct TargetExactCreature(ObjectId);

impl crate::decision::DecisionMaker for TargetExactCreature {
    fn decide_targets(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        ctx.requirements
            .iter()
            .filter_map(|requirement| {
                requirement
                    .legal_targets
                    .iter()
                    .find(|target| {
                        matches!(target, crate::game_state::Target::Object(id) if *id == self.0)
                    })
                    .cloned()
            })
            .collect()
    }
}

fn permanent(name: &str, card_types: Vec<CardType>) -> CardDefinition {
    let mut builder =
        CardDefinitionBuilder::new(CardId::new(), name).card_types(card_types.clone());
    if card_types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

fn source_trigger_queue(
    game: &mut crate::GameState,
    source: ObjectId,
) -> crate::triggers::TriggerQueue {
    let mut pending = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(game, &mut pending);
    pending.entries.retain(|entry| entry.source == source);
    pending
}

fn setup_long_feng_game() -> (crate::GameState, PlayerId, PlayerId, ObjectId, ObjectId) {
    let definition = parse_oracle_card_definition("Long Feng, Grand Secretariat");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = game.create_object_from_definition(
        &permanent("Chosen Counter Recipient", vec![CardType::Creature]),
        alice,
        Zone::Battlefield,
    );
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    (game, alice, bob, source, target)
}

fn assert_single_trigger_resolves_onto_target(
    game: &mut crate::GameState,
    source: ObjectId,
    target: ObjectId,
) {
    let mut queue = source_trigger_queue(game, source);
    assert_eq!(
        queue.entries.len(),
        1,
        "Long Feng should trigger exactly once for the matching transition"
    );

    let mut decisions = TargetExactCreature(target);
    crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut decisions)
        .expect("Long Feng's target should be selected");
    crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
        .expect("Long Feng's trigger should resolve");

    assert_eq!(
        game.counter_count(target, CounterType::PlusOnePlusOne),
        1,
        "the chosen controlled creature must receive exactly one +1/+1 counter"
    );
    assert_eq!(
        game.counter_count(source, CounterType::PlusOnePlusOne),
        0,
        "the exact chosen target, rather than Long Feng, must receive the counter"
    );
}

#[test]
fn long_feng_keeps_another_local_to_the_creature_arm() {
    let definition = parse_oracle_card_definition("Long Feng, Grand Secretariat");
    assert_eq!(canonical_compiled_lines(&definition), vec![ORACLE_LINE]);

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
        .expect("Long Feng should compile to a battlefield-to-graveyard trigger");
    let filter = &trigger.object_filter;
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(!filter.other, "the land arm must not inherit another");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");

    let creature = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types == [CardType::Creature])
        .expect("creature arm");
    assert!(creature.other, "{filter:#?}");
    let land = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types == [CardType::Land])
        .expect("land arm");
    assert!(!land.other, "{filter:#?}");
}

#[test]
fn long_feng_does_not_trigger_for_its_own_death() {
    let (mut game, _alice, _bob, source, target) = setup_long_feng_game();
    game.move_object_by_effect(source, Zone::Graveyard)
        .expect("Long Feng should move to the graveyard");

    let queue = source_trigger_queue(&mut game, source);
    assert!(
        queue.entries.is_empty(),
        "another creature excludes the triggering source itself"
    );
    assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 0);
}

#[test]
fn long_feng_triggers_for_another_controlled_creature_and_targets_exactly() {
    let (mut game, alice, _bob, source, target) = setup_long_feng_game();
    let victim = game.create_object_from_definition(
        &permanent("Another Controlled Creature", vec![CardType::Creature]),
        alice,
        Zone::Battlefield,
    );
    game.move_object_by_effect(victim, Zone::Graveyard)
        .expect("the controlled creature should die");

    assert_single_trigger_resolves_onto_target(&mut game, source, target);
}

#[test]
fn long_feng_triggers_for_a_controlled_land_and_targets_exactly() {
    let (mut game, alice, _bob, source, target) = setup_long_feng_game();
    let land = game.create_object_from_definition(
        &permanent("Controlled Land", vec![CardType::Land]),
        alice,
        Zone::Battlefield,
    );
    game.move_object_by_effect(land, Zone::Graveyard)
        .expect("the controlled land should go to the graveyard");

    assert_single_trigger_resolves_onto_target(&mut game, source, target);
}

#[test]
fn long_feng_ignores_opponents_permanents_and_controlled_nonmatching_permanents() {
    let (mut game, alice, bob, source, target) = setup_long_feng_game();
    for (name, controller, card_types) in [
        ("Opponent Creature", bob, vec![CardType::Creature]),
        ("Opponent Land", bob, vec![CardType::Land]),
        ("Controlled Artifact", alice, vec![CardType::Artifact]),
    ] {
        let object = game.create_object_from_definition(
            &permanent(name, card_types),
            controller,
            Zone::Battlefield,
        );
        game.move_object_by_effect(object, Zone::Graveyard)
            .expect("the nonmatching permanent should move to the graveyard");
    }

    let queue = source_trigger_queue(&mut game, source);
    assert!(queue.entries.is_empty());
    assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 0);
}
