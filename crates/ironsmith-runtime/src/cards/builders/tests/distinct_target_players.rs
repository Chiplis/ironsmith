#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const BIOMANTIC_MASTERY_ORACLE: &str = "Draw a card for each creature target player controls, then draw a card for each creature another target player controls.";
const CYBERNETICA_DATASMITH_ORACLE: &str = "Protection from Robots\nField Reprogramming — {U}, {T}: Target player draws a card. Another target player creates a 4/4 colorless Robot artifact creature token with \"This token can't block.\"";

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

fn vanilla_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

fn add_creatures(game: &mut crate::GameState, controller: PlayerId, count: usize) {
    for index in 0..count {
        game.create_object_from_definition(
            &vanilla_creature(&format!("Creature {controller:?} {index}")),
            controller,
            Zone::Battlefield,
        );
    }
}

fn add_library_cards(game: &mut crate::GameState, player: PlayerId, count: usize) {
    for index in 0..count {
        let filler =
            CardDefinitionBuilder::new(CardId::new(), format!("Filler {player:?} {index}"))
                .card_types(vec![CardType::Sorcery])
                .build();
        game.create_object_from_definition(&filler, player, Zone::Library);
    }
}

fn draw_count_filter(effect: &Effect) -> &ObjectFilter {
    let draw = effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .expect("expected a draw action");
    let Value::Count(filter) = draw.count.unhinted() else {
        panic!("expected the draw amount to count a filtered object set: {draw:#?}");
    };
    filter
}

fn assert_distinct_player_requirements(
    requirements: &[crate::decision::TargetRequirement],
    first: PlayerId,
    second: PlayerId,
) {
    assert_eq!(requirements.len(), 2, "{requirements:#?}");
    assert_eq!(
        requirements[0].distinct_player_group, requirements[1].distinct_player_group,
        "the two authored target-player clauses must share one distinctness group"
    );
    assert!(
        requirements[0].distinct_player_group.is_some(),
        "the target announcement must enforce different players: {requirements:#?}"
    );
    let contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                legal_target_sets: requirement.legal_target_sets.clone(),
                aggregate_constraint: requirement.aggregate_constraint.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
                distinct_player_group: requirement.distinct_player_group,
            },
        )
        .collect::<Vec<_>>();
    let first = crate::game_state::Target::Player(first);
    let second = crate::game_state::Target::Player(second);
    assert!(
        !crate::targeting::validate_flat_target_assignment(&contexts, &[first, first]),
        "the same player cannot satisfy both target clauses"
    );
    assert!(crate::targeting::validate_flat_target_assignment(
        &contexts,
        &[first, second],
    ));
}

fn resolve_with_player_targets(
    game: &mut crate::GameState,
    source: ObjectId,
    controller: PlayerId,
    program: &crate::resolution::ResolutionProgram,
    requirements: &[crate::decision::TargetRequirement],
    players: [PlayerId; 2],
) {
    let flat_targets = players
        .into_iter()
        .map(crate::game_state::Target::Player)
        .collect::<Vec<_>>();
    let assignments =
        super::shard_17::target_assignments_for_requirements(requirements, &flat_targets);
    let resolved_targets = players
        .into_iter()
        .map(crate::effects::ResolvedTarget::Player)
        .collect::<Vec<_>>();
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, controller, &mut decisions)
        .with_targets(resolved_targets)
        .with_target_assignments(assignments.clone());
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        controller,
        source,
        program,
        None,
        &assignments,
    )
    .expect("the parser-backed effect should resolve");
}

#[test]
fn biomantic_mastery_announces_two_distinct_players_and_counts_each_selected_battlefield() {
    let definition = parse_oracle_card_definition("Biomantic Mastery");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = three_player_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    add_creatures(&mut game, bob, 1);
    add_creatures(&mut game, charlie, 3);
    add_library_cards(&mut game, alice, 8);

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Biomantic Mastery should have a spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one Biomantic Mastery resolution segment: {program:#?}");
    };
    let [sequence_effect] = segment.default_effects.as_slice() else {
        panic!("expected one comma-then sequence: {segment:#?}");
    };
    let sequence = sequence_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("Biomantic Mastery should retain its comma-then sequence");
    let [_, first_draw, _, second_draw] = sequence.effects.as_slice() else {
        panic!("expected two target/count-draw pairs: {sequence:#?}");
    };
    let first_count = draw_count_filter(first_draw);
    let second_count = draw_count_filter(second_draw);
    assert!(!first_count.other && !second_count.other);
    assert!(matches!(
        first_count.controller.as_ref(),
        Some(PlayerFilter::Target(inner)) if matches!(inner.as_ref(), PlayerFilter::Any)
    ));
    assert!(matches!(
        second_count.controller.as_ref(),
        Some(PlayerFilter::Target(inner))
            if inner
                .relative_target_exclusion_base()
                .is_some_and(|base| matches!(base, PlayerFilter::Any))
    ));
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        program,
        alice,
        Some(source),
        None,
    );
    assert_distinct_player_requirements(&requirements, bob, charlie);

    let alice_hand_before = game.player(alice).unwrap().hand.len();
    let bob_hand_before = game.player(bob).unwrap().hand.len();
    let charlie_hand_before = game.player(charlie).unwrap().hand.len();
    resolve_with_player_targets(
        &mut game,
        source,
        alice,
        program,
        &requirements,
        [bob, charlie],
    );
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        alice_hand_before + 4
    );
    assert_eq!(game.player(bob).unwrap().hand.len(), bob_hand_before);
    assert_eq!(
        game.player(charlie).unwrap().hand.len(),
        charlie_hand_before
    );
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        BIOMANTIC_MASTERY_ORACLE
    );
}

#[test]
fn cybernetica_datasmith_draws_for_first_target_and_gives_nonblocking_robot_to_another() {
    let definition = parse_oracle_card_definition("Cybernetica Datasmith");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Cybernetica Datasmith should have Field Reprogramming");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = three_player_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let attacker = game.create_object_from_definition(
        &vanilla_creature("Unrestricted Attacker"),
        alice,
        Zone::Battlefield,
    );
    add_library_cards(&mut game, bob, 2);

    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &activated.effects,
        alice,
        Some(source),
        None,
    );
    assert_distinct_player_requirements(&requirements, bob, charlie);

    let bob_hand_before = game.player(bob).unwrap().hand.len();
    let charlie_hand_before = game.player(charlie).unwrap().hand.len();
    resolve_with_player_targets(
        &mut game,
        source,
        alice,
        &activated.effects,
        &requirements,
        [bob, charlie],
    );
    assert_eq!(game.player(bob).unwrap().hand.len(), bob_hand_before + 1);
    assert_eq!(
        game.player(charlie).unwrap().hand.len(),
        charlie_hand_before
    );

    let robots = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                matches!(object.kind, crate::object::ObjectKind::Token)
                    && game.calculated_subtypes(*id).contains(&Subtype::Robot)
            })
        })
        .collect::<Vec<_>>();
    let [robot] = robots.as_slice() else {
        panic!("expected exactly one Robot token: {robots:#?}");
    };
    assert_eq!(game.controller_of_id(*robot), Some(charlie));
    assert_eq!(game.current_power(*robot), Some(4));
    assert_eq!(game.current_toughness(*robot), Some(4));
    assert!(
        game.calculated_card_types(*robot)
            .contains(&CardType::Artifact)
    );
    assert!(
        game.calculated_card_types(*robot)
            .contains(&CardType::Creature)
    );
    assert!(
        game.object_has_static_ability_id(
            *robot,
            crate::static_abilities::StaticAbilityId::CantBlock,
        )
    );
    assert!(
        !crate::rules::can_block(
            game.object(attacker)
                .expect("attacker should remain in play"),
            game.object(*robot).expect("Robot should remain in play"),
            &game,
        ),
        "the created Robot must not be a legal blocker"
    );
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        CYBERNETICA_DATASMITH_ORACLE
    );
}
