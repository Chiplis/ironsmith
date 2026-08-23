#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

struct CombatFixture {
    target_attacker: ObjectId,
    first_target_blocker: ObjectId,
    second_target_blocker: ObjectId,
    decoy_attacker: ObjectId,
    decoy_blocker: ObjectId,
}

fn install_two_blocked_attackers(
    game: &mut crate::GameState,
    attacker_controller: PlayerId,
    blocker_controller: PlayerId,
) -> CombatFixture {
    let target_attacker = game.create_object_from_definition(
        &creature("Target Attacker"),
        attacker_controller,
        Zone::Battlefield,
    );
    let first_target_blocker = game.create_object_from_definition(
        &creature("Relevant Blocker A"),
        blocker_controller,
        Zone::Battlefield,
    );
    let second_target_blocker = game.create_object_from_definition(
        &creature("Relevant Blocker B"),
        blocker_controller,
        Zone::Battlefield,
    );
    let decoy_attacker = game.create_object_from_definition(
        &creature("Decoy Attacker"),
        attacker_controller,
        Zone::Battlefield,
    );
    let decoy_blocker = game.create_object_from_definition(
        &creature("Unrelated Blocker"),
        blocker_controller,
        Zone::Battlefield,
    );
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![
            crate::combat_state::AttackerInfo {
                creature: target_attacker,
                target: crate::combat_state::AttackTarget::Player(blocker_controller),
            },
            crate::combat_state::AttackerInfo {
                creature: decoy_attacker,
                target: crate::combat_state::AttackTarget::Player(blocker_controller),
            },
        ],
        blockers: std::collections::HashMap::from([
            (
                target_attacker,
                vec![first_target_blocker, second_target_blocker],
            ),
            (decoy_attacker, vec![decoy_blocker]),
        ]),
        ..Default::default()
    });
    CombatFixture {
        target_attacker,
        first_target_blocker,
        second_target_blocker,
        decoy_attacker,
        decoy_blocker,
    }
}

fn resolve_targeted_spell(
    game: &mut crate::GameState,
    definition: &CardDefinition,
    controller: PlayerId,
    target: ObjectId,
) -> Vec<crate::decision::TargetRequirement> {
    let source = game.create_object_from_definition(definition, controller, Zone::Stack);
    let program = definition
        .spell_effect
        .as_ref()
        .expect("the instant should have a spell program");
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        program,
        controller,
        Some(source),
        None,
    );
    let flat_targets = vec![crate::game_state::Target::Object(target)];
    let assignments =
        super::shard_17::target_assignments_for_requirements(&requirements, &flat_targets);
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, controller, &mut decisions)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)])
        .with_target_assignments(assignments.clone());
    crate::game_loop::execute_resolution_program(
        game,
        &mut context,
        controller,
        source,
        program,
        None,
        &assignments,
    )
    .expect("the parser-backed spell should resolve");
    requirements
}

fn current_zone(game: &crate::GameState, object: ObjectId) -> Zone {
    let stable = game
        .object(object)
        .expect("fixture object exists")
        .stable_id;
    let current = game
        .find_object_by_stable_id(stable)
        .expect("stable fixture object still exists");
    game.object(current)
        .expect("current fixture object exists")
        .zone
}

#[test]
fn feint_targets_one_attacker_and_taps_only_that_attackers_blockers() {
    let definition = parse_oracle_card_definition("Feint");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let fixture = install_two_blocked_attackers(&mut game, alice, bob);

    let requirements =
        resolve_targeted_spell(&mut game, &definition, alice, fixture.target_attacker);
    assert_eq!(requirements.len(), 1, "Feint has one mandatory target");
    assert_eq!(requirements[0].min_targets, 1);
    assert_eq!(requirements[0].max_targets, Some(1));
    assert!(
        requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(fixture.target_attacker))
    );
    assert!(
        requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(fixture.decoy_attacker))
    );
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(
                fixture.first_target_blocker
            )),
        "Feint may target only an attacking creature"
    );

    assert!(game.is_tapped(fixture.first_target_blocker));
    assert!(game.is_tapped(fixture.second_target_blocker));
    assert!(!game.is_tapped(fixture.target_attacker));
    assert!(!game.is_tapped(fixture.decoy_attacker));
    assert!(!game.is_tapped(fixture.decoy_blocker));

    let prevented_sources = game
        .effect_store
        .prevention_effects
        .shields()
        .iter()
        .filter_map(|shield| shield.damage_filter.from_specific_source)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        prevented_sources,
        std::collections::HashSet::from([
            fixture.target_attacker,
            fixture.first_target_blocker,
            fixture.second_target_blocker,
        ]),
        "Feint must prevent combat damage only from the selected attacker and its blockers"
    );

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Tap all creatures blocking target attacking creature. Prevent all combat damage that would be dealt this turn by that creature and each creature blocking it."
    );
}

#[test]
fn trial_returns_only_creatures_in_combat_with_the_selected_attacker() {
    let definition = parse_oracle_card_definition("Trial");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let fixture = install_two_blocked_attackers(&mut game, alice, bob);

    let first_blocker_stable = game.object(fixture.first_target_blocker).unwrap().stable_id;
    let second_blocker_stable = game
        .object(fixture.second_target_blocker)
        .unwrap()
        .stable_id;
    let requirements =
        resolve_targeted_spell(&mut game, &definition, alice, fixture.target_attacker);
    assert_eq!(requirements.len(), 1, "Trial has one mandatory target");
    assert!(
        requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(fixture.target_attacker))
    );
    assert!(
        requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(
                fixture.first_target_blocker
            )),
        "Trial may target either an attacker or a blocker"
    );

    assert_eq!(
        game.object(game.find_object_by_stable_id(first_blocker_stable).unwrap())
            .unwrap()
            .zone,
        Zone::Hand
    );
    assert_eq!(
        game.object(
            game.find_object_by_stable_id(second_blocker_stable)
                .unwrap()
        )
        .unwrap()
        .zone,
        Zone::Hand
    );
    assert_eq!(
        current_zone(&game, fixture.target_attacker),
        Zone::Battlefield
    );
    assert_eq!(
        current_zone(&game, fixture.decoy_attacker),
        Zone::Battlefield
    );
    assert_eq!(
        current_zone(&game, fixture.decoy_blocker),
        Zone::Battlefield
    );

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Return all creatures blocking or blocked by target creature to their owner's hand."
    );
}

#[test]
fn trial_can_target_a_blocker_and_returns_only_its_attacker() {
    let definition = parse_oracle_card_definition("Trial");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let fixture = install_two_blocked_attackers(&mut game, alice, bob);
    let target_attacker_stable = game.object(fixture.target_attacker).unwrap().stable_id;

    resolve_targeted_spell(&mut game, &definition, alice, fixture.first_target_blocker);

    assert_eq!(
        game.object(
            game.find_object_by_stable_id(target_attacker_stable)
                .unwrap()
        )
        .unwrap()
        .zone,
        Zone::Hand
    );
    assert_eq!(
        current_zone(&game, fixture.first_target_blocker),
        Zone::Battlefield,
        "the selected creature is not part of the returned set"
    );
    assert_eq!(
        current_zone(&game, fixture.second_target_blocker),
        Zone::Battlefield,
        "another blocker of the returned attacker is not itself in combat with the selected blocker"
    );
    assert_eq!(
        current_zone(&game, fixture.decoy_attacker),
        Zone::Battlefield
    );
    assert_eq!(
        current_zone(&game, fixture.decoy_blocker),
        Zone::Battlefield
    );
}
