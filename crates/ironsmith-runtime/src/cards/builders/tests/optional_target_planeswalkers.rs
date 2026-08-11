#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn main_phase_game() -> (crate::GameState, PlayerId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    (game, alice)
}

fn creature(name: &str, flying: bool) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3));
    if flying {
        builder
            .parse_text("Flying")
            .expect("flying creature should parse")
    } else {
        builder.build()
    }
}

fn planeswalker(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .build()
}

fn named_planeswalker_definition(name: &str) -> CardDefinition {
    let mut definition = parse_oracle_card_definition(name);
    definition.card.loyalty = Some(match name {
        "Arlinn Kord" => 3,
        "Mu Yanling, Sky Dancer" => 2,
        "Ajani Steadfast" => 4,
        "Ajani, the Greathearted" => 5,
        "Ajani Unyielding" => 4,
        _ => unreachable!("unexpected named planeswalker"),
    });
    definition
}

fn activate_loyalty(
    game: &mut crate::GameState,
    controller: PlayerId,
    source: ObjectId,
    ability_index: usize,
    selected_targets: Option<Vec<crate::game_state::Target>>,
) -> Vec<crate::game_state::Target> {
    let action = crate::decision::compute_legal_actions(game, controller)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source: candidate, ability_index: candidate_index }
                    if *candidate == source && *candidate_index == ability_index
            )
        })
        .unwrap_or_else(|| panic!("loyalty ability {ability_index} should be legal"));

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        game,
        &mut trigger_queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(action),
        &mut decisions,
    )
    .expect("loyalty activation should start");
    if let Some(selected_targets) = selected_targets {
        assert!(
            matches!(
                progress,
                crate::decision::GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Targets(_)
                )
            ),
            "targeted loyalty activation should request its targets: {progress:?}"
        );
        crate::game_loop::apply_priority_response_with_dm(
            game,
            &mut trigger_queue,
            &mut state,
            &crate::game_loop::PriorityResponse::Targets(selected_targets),
            &mut decisions,
        )
        .expect("zero-or-one target selection should be accepted");
    }

    let targets = game
        .stack
        .last()
        .expect("loyalty ability should be on the stack")
        .targets
        .clone();
    crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
        .expect("loyalty ability should resolve");
    targets
}

fn assert_optional_first_loyalty_target(name: &str) {
    let definition = named_planeswalker_definition(name);
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("card should have a loyalty ability");
    assert!(
        activated.choices.iter().any(|choice| {
            let count = choice.count();
            choice.is_target() && count.min == 0 && count.max == Some(1)
        }),
        "{name} must declare an optional zero-or-one target: {:#?}",
        activated.choices
    );
    assert!(
        canonical_compiled_lines(&definition)[0]
            .to_ascii_lowercase()
            .contains("up to one target creature"),
        "{name} compiled text must retain the optional target"
    );
}

#[test]
fn named_optional_target_planeswalkers_preserve_zero_or_one_target_structure() {
    for name in ["Arlinn Kord", "Mu Yanling, Sky Dancer", "Ajani Steadfast"] {
        assert_optional_first_loyalty_target(name);
    }
}

fn assert_optional_pump_decline_and_positive(name: &str, expected_power: i32, flying: bool) {
    let definition = named_planeswalker_definition(name);

    let (mut decline_game, alice) = main_phase_game();
    let source = decline_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let candidate = decline_game.create_object_from_definition(
        &creature("Available Target", flying),
        alice,
        Zone::Battlefield,
    );
    let targets = activate_loyalty(&mut decline_game, alice, source, 0, Some(Vec::new()));
    assert!(
        targets.is_empty(),
        "{name} must allow its controller to choose zero targets"
    );
    assert_eq!(
        decline_game.calculated_power(candidate),
        Some(3),
        "declining {name}'s optional target must leave the available creature unchanged"
    );
    if flying {
        assert!(
            decline_game.object_has_static_ability_id(candidate, StaticAbilityId::Flying),
            "declining {name}'s optional target must not remove flying"
        );
    }

    let (mut positive_game, alice) = main_phase_game();
    let source = positive_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let candidate = positive_game.create_object_from_definition(
        &creature("Chosen Target", flying),
        alice,
        Zone::Battlefield,
    );
    let targets = activate_loyalty(
        &mut positive_game,
        alice,
        source,
        0,
        Some(vec![crate::game_state::Target::Object(candidate)]),
    );
    assert_eq!(
        targets,
        vec![crate::game_state::Target::Object(candidate)],
        "{name} should accept one legal target"
    );
    assert_eq!(
        positive_game.calculated_power(candidate),
        Some(expected_power),
        "{name}'s selected target must receive its printed power modification"
    );
    match name {
        "Arlinn Kord" => {
            assert!(
                positive_game.object_has_static_ability_id(candidate, StaticAbilityId::Vigilance)
            );
            assert!(positive_game.object_has_static_ability_id(candidate, StaticAbilityId::Haste));
        }
        "Mu Yanling, Sky Dancer" => {
            assert!(!positive_game.object_has_static_ability_id(candidate, StaticAbilityId::Flying))
        }
        "Ajani Steadfast" => {
            for ability in [
                StaticAbilityId::FirstStrike,
                StaticAbilityId::Vigilance,
                StaticAbilityId::Lifelink,
            ] {
                assert!(positive_game.object_has_static_ability_id(candidate, ability));
            }
        }
        _ => unreachable!("unexpected optional-target planeswalker"),
    }
}

#[test]
fn arlinn_kord_optional_target_can_be_declined_or_receive_the_bonus() {
    assert_optional_pump_decline_and_positive("Arlinn Kord", 5, false);
}

#[test]
fn mu_yanling_optional_target_can_be_declined_or_receive_the_penalty() {
    assert_optional_pump_decline_and_positive("Mu Yanling, Sky Dancer", 1, true);
}

#[test]
fn ajani_steadfast_optional_target_can_be_declined_or_receive_the_bonus() {
    assert_optional_pump_decline_and_positive("Ajani Steadfast", 4, false);
}

#[test]
fn ajani_steadfast_minus_two_counters_every_controlled_member_of_both_sets() {
    let definition = named_planeswalker_definition("Ajani Steadfast");
    let rendered = canonical_compiled_lines(&definition)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each creature you control")
            && rendered.contains("each other planeswalker you control"),
        "Ajani's -2 must retain both complete affected sets: {rendered}"
    );

    let (mut game, alice) = main_phase_game();
    let bob = PlayerId::from_index(1);
    let ajani = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ally_a =
        game.create_object_from_definition(&creature("Ally A", false), alice, Zone::Battlefield);
    let ally_b =
        game.create_object_from_definition(&creature("Ally B", false), alice, Zone::Battlefield);
    let enemy =
        game.create_object_from_definition(&creature("Enemy", false), bob, Zone::Battlefield);
    let ally_walker_a = game.create_object_from_definition(
        &planeswalker("Ally Walker A"),
        alice,
        Zone::Battlefield,
    );
    let ally_walker_b = game.create_object_from_definition(
        &planeswalker("Ally Walker B"),
        alice,
        Zone::Battlefield,
    );
    let enemy_walker =
        game.create_object_from_definition(&planeswalker("Enemy Walker"), bob, Zone::Battlefield);
    let loyalty_before =
        [ally_walker_a, ally_walker_b].map(|id| game.counter_count(id, CounterType::Loyalty));
    let ajani_before = game.counter_count(ajani, CounterType::Loyalty);
    assert_eq!(
        ajani_before, 4,
        "Ajani should enter with four loyalty counters"
    );
    let targets = activate_loyalty(&mut game, alice, ajani, 1, None);
    assert!(targets.is_empty(), "Ajani's −2 does not target");

    for ally in [ally_a, ally_b] {
        assert_eq!(game.counter_count(ally, CounterType::PlusOnePlusOne), 1);
    }
    assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
    for (walker, before) in [ally_walker_a, ally_walker_b]
        .into_iter()
        .zip(loyalty_before)
    {
        assert_eq!(game.counter_count(walker, CounterType::Loyalty), before + 1);
    }
    assert_eq!(game.counter_count(enemy_walker, CounterType::Loyalty), 3);
    assert_eq!(
        game.counter_count(ajani, CounterType::Loyalty),
        ajani_before - 2,
        "'other planeswalker' must exclude Ajani himself"
    );
}

fn assert_ajani_complete_counter_sets(
    name: &str,
    ability_index: usize,
    counters_per_member: u32,
    loyalty_cost: u32,
) {
    let definition = named_planeswalker_definition(name);
    let rendered = canonical_compiled_lines(&definition)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each creature you control")
            && rendered.contains("each other planeswalker you control"),
        "{name} must retain both complete affected sets: {rendered}"
    );

    let (mut game, alice) = main_phase_game();
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let current_loyalty = game.counter_count(source, CounterType::Loyalty);
    if current_loyalty < loyalty_cost {
        game.add_counters(source, CounterType::Loyalty, loyalty_cost - current_loyalty);
    }
    let source_loyalty_before = game.counter_count(source, CounterType::Loyalty);
    let ally_a =
        game.create_object_from_definition(&creature("Ally A", false), alice, Zone::Battlefield);
    let ally_b =
        game.create_object_from_definition(&creature("Ally B", false), alice, Zone::Battlefield);
    let enemy =
        game.create_object_from_definition(&creature("Enemy", false), bob, Zone::Battlefield);
    let ally_walker_a = game.create_object_from_definition(
        &planeswalker("Ally Walker A"),
        alice,
        Zone::Battlefield,
    );
    let ally_walker_b = game.create_object_from_definition(
        &planeswalker("Ally Walker B"),
        alice,
        Zone::Battlefield,
    );
    let enemy_walker =
        game.create_object_from_definition(&planeswalker("Enemy Walker"), bob, Zone::Battlefield);
    let allied_loyalty_before =
        [ally_walker_a, ally_walker_b].map(|id| game.counter_count(id, CounterType::Loyalty));

    let targets = activate_loyalty(&mut game, alice, source, ability_index, None);
    assert!(
        targets.is_empty(),
        "{name}'s complete-set ability does not target"
    );

    for ally in [ally_a, ally_b] {
        assert_eq!(
            game.counter_count(ally, CounterType::PlusOnePlusOne),
            counters_per_member,
            "{name} must affect every controlled creature"
        );
    }
    assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
    for (walker, before) in [ally_walker_a, ally_walker_b]
        .into_iter()
        .zip(allied_loyalty_before)
    {
        assert_eq!(
            game.counter_count(walker, CounterType::Loyalty),
            before + counters_per_member,
            "{name} must affect every other controlled planeswalker"
        );
    }
    assert_eq!(game.counter_count(enemy_walker, CounterType::Loyalty), 3);
    assert_eq!(
        game.counter_count(source, CounterType::Loyalty),
        source_loyalty_before - loyalty_cost,
        "{name} must pay its printed loyalty cost without countering itself"
    );
}

#[test]
fn ajani_the_greathearted_minus_two_counters_every_controlled_member_of_both_sets() {
    assert_ajani_complete_counter_sets("Ajani, the Greathearted", 2, 1, 2);
}

#[test]
fn ajani_unyielding_minus_nine_counters_every_controlled_member_of_both_sets() {
    assert_ajani_complete_counter_sets("Ajani Unyielding", 2, 5, 9);
}

#[test]
fn ajani_unyielding_plus_two_keeps_only_nonland_permanents() {
    let definition = named_planeswalker_definition("Ajani Unyielding");
    let rendered = canonical_compiled_lines(&definition)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("all nonland permanent cards revealed this way into your hand"),
        "Ajani's +2 must render its executable permanent-card restriction: {rendered}"
    );

    let (mut game, alice) = main_phase_game();
    let ajani = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for (name, card_type) in [
        ("Revealed Creature", CardType::Creature),
        ("Revealed Instant", CardType::Instant),
        ("Revealed Land", CardType::Land),
    ] {
        let card = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .build();
        game.create_object_from_definition(&card, alice, Zone::Library);
    }

    activate_loyalty(&mut game, alice, ajani, 0, None);

    let player = game.player(alice).expect("Alice should remain in the game");
    let hand_names = player
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(hand_names, vec!["Revealed Creature"]);

    let library_names = player
        .library
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(library_names.len(), 2);
    assert!(library_names.contains(&"Revealed Instant"));
    assert!(library_names.contains(&"Revealed Land"));
}
