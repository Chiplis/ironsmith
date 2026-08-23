#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const BLOCKED_TRIGGER_LINE: &str = "Whenever this creature becomes blocked by a creature, that creature's controller sacrifices it at end of combat. If the player does, they create a 0/2 colorless Wall artifact creature token with defender.";

fn wall_controllers(game: &crate::game_state::GameState) -> Vec<PlayerId> {
    game.battlefield
        .iter()
        .filter_map(|object_id| {
            let object = game.object(*object_id)?;
            (object.name == "Wall").then(|| game.controller_of(object))
        })
        .collect()
}

fn resolve_block_trigger(
    game: &mut crate::game_state::GameState,
    golem: ObjectId,
    blocker: ObjectId,
) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureBlockedEvent::with_snapshots(
            blocker,
            golem,
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(blocker).expect("blocker exists"),
                game,
            ),
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(golem).expect("golem exists"),
                game,
            ),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(game, &event) {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("Basalt Golem's block trigger should go on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry(game).expect("block trigger should resolve");
    }
}

fn resolve_end_of_combat(game: &mut crate::game_state::GameState) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::EndOfCombatEvent::new(),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_delayed_triggers(game, &event) {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(game, &mut queue)
        .expect("the delayed end-of-combat trigger should go on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry(game)
            .expect("the delayed end-of-combat trigger should resolve");
    }
}

#[test]
fn basalt_golem_waits_until_combat_and_rewards_the_blockers_controller_only_on_sacrifice() {
    let definition = parse_oracle_card_definition("Basalt Golem");
    let rendered = canonical_compiled_lines(&definition);

    for blocker_leaves_early in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let golem = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let blocker_definition = CardDefinitionBuilder::new(CardId::new(), "Golem Blocker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 4))
            .build();
        let blocker =
            game.create_object_from_definition(&blocker_definition, bob, Zone::Battlefield);
        let blocker_stable = game.object(blocker).expect("blocker exists").stable_id;

        resolve_block_trigger(&mut game, golem, blocker);
        assert!(
            wall_controllers(&game).is_empty(),
            "scheduling the end-of-combat sacrifice must not create the Wall immediately"
        );
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);

        if blocker_leaves_early {
            game.move_object(
                blocker,
                Zone::Exile,
                crate::events::cause::EventCause::effect(),
            )
            .expect("the blocker should move before end of combat");
        }
        resolve_end_of_combat(&mut game);

        let blocker_zone = game
            .find_object_by_stable_id(blocker_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone)
            .expect("the blocker should remain represented");
        if blocker_leaves_early {
            assert_eq!(blocker_zone, Zone::Exile);
            assert!(
                wall_controllers(&game).is_empty(),
                "a failed sacrifice must not create a Wall"
            );
        } else {
            assert_eq!(blocker_zone, Zone::Graveyard);
            assert_eq!(
                wall_controllers(&game),
                vec![bob],
                "the sacrificed blocker's controller must create the Wall"
            );
        }
    }

    assert!(
        rendered.iter().any(|line| line == BLOCKED_TRIGGER_LINE),
        "Basalt Golem must retain the delayed sacrifice/result relationship: {rendered:#?}"
    );
}
