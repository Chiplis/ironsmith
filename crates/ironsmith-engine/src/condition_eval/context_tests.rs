use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::effect::ValueComparisonOperator;
use crate::effects::ResolvedTarget;
use crate::ids::CardId;
use crate::target::ObjectFilter;
use crate::types::CardType;

fn fixture() -> (GameState, PlayerId, PlayerId, ObjectId, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let card = CardBuilder::new(CardId::from_raw(99001), "Condition source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let target = game.create_object_from_card(&card, bob, Zone::Battlefield);
    (game, alice, bob, source, target)
}

#[test]
fn boolean_composition_short_circuits_resolution_errors() {
    let (game, alice, _, source, _) = fixture();
    let exec = ExecutionContext::new_default(source, alice);
    let unresolved = Condition::ValueComparison {
        left: Value::X,
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    };
    assert!(evaluate_condition_resolution(&game, &unresolved, &exec).is_err());
    let yes = Condition::LifeTotalOrGreater(1);
    let no = Condition::LifeTotalOrLess(0);
    for (condition, expected) in [
        (
            Condition::And(Box::new(no), Box::new(unresolved.clone())),
            false,
        ),
        (
            Condition::Or(Box::new(yes), Box::new(unresolved.clone())),
            true,
        ),
    ] {
        assert_eq!(
            evaluate_condition_resolution(&game, &condition, &exec).unwrap(),
            expected
        );
    }
    assert!(
        evaluate_condition_resolution(&game, &Condition::Not(Box::new(unresolved.clone())), &exec)
            .is_err()
    );
    assert!(!evaluate_condition_cast_time(
        &game,
        &unresolved,
        alice,
        source
    ));
    assert!(!evaluate_condition_external(
        &game,
        &unresolved,
        &ExternalEvaluationContext {
            controller: alice,
            source,
            ..Default::default()
        }
    ));
    assert!(
        evaluate_condition_with_mode(
            &game,
            &Condition::YourTurn,
            ConditionEvaluationMode::Resolution,
            None
        )
        .is_err()
    );
}

#[test]
fn source_filter_policy_is_preserved_inside_nested_conditions() {
    let (game, alice, _, source, _) = fixture();
    let condition = Condition::And(
        Box::new(Condition::YourTurn),
        Box::new(Condition::YouControl(ObjectFilter::creature().other())),
    );
    let mut external = ExternalEvaluationContext {
        controller: alice,
        source,
        filter_source: None,
        ..Default::default()
    };
    // An intervening-if caller without a filter source can count its own source.
    assert!(evaluate_condition_external(&game, &condition, &external));
    external.filter_source = Some(source);
    assert!(!evaluate_condition_external(&game, &condition, &external));
    assert!(!evaluate_condition_cast_time(
        &game, &condition, alice, source
    ));
    assert!(
        !evaluate_condition_resolution(
            &game,
            &condition,
            &ExecutionContext::new_default(source, alice)
        )
        .unwrap()
    );
}

#[test]
fn player_control_filters_preserve_you_and_iteration_bindings() {
    let (game, alice, bob, source, _) = fixture();
    let condition = Condition::PlayerControls {
        player: PlayerFilter::Specific(bob),
        filter: ObjectFilter::creature().you_control(),
    };
    // Earlier phases interpret "you" from the selected player's perspective.
    assert!(evaluate_condition_cast_time(
        &game, &condition, alice, source
    ));
    let external = ExternalEvaluationContext {
        controller: alice,
        source,
        ..Default::default()
    };
    assert!(evaluate_condition_external(&game, &condition, &external));
    // Resolution retains the effect controller and separately binds the iteration.
    assert!(
        !evaluate_condition_resolution(
            &game,
            &condition,
            &ExecutionContext::new_default(source, alice)
        )
        .unwrap()
    );
    let iterated = Condition::PlayerControls {
        player: PlayerFilter::IteratedPlayer,
        filter: ObjectFilter::creature(),
    };
    assert!(!evaluate_condition_cast_time(
        &game, &iterated, alice, source
    ));
    assert!(evaluate_condition_external(
        &game,
        &iterated,
        &ExternalEvaluationContext {
            iterated_player: Some(bob),
            ..external
        }
    ));
    let mut exec = ExecutionContext::new_default(source, alice);
    exec.iteration.iterated_player = Some(bob);
    assert!(evaluate_condition_resolution(&game, &iterated, &exec).unwrap());
}

#[test]
fn missing_player_bindings_remain_errors_only_during_resolution() {
    let (game, alice, _, source, _) = fixture();
    let condition = Condition::PlayerHasInitiative {
        player: PlayerFilter::Target(Box::new(PlayerFilter::Any)),
    };
    assert!(!evaluate_condition_cast_time(
        &game, &condition, alice, source
    ));
    assert!(!evaluate_condition_external(
        &game,
        &condition,
        &ExternalEvaluationContext {
            controller: alice,
            source,
            ..Default::default()
        }
    ));
    assert!(
        evaluate_condition_resolution(
            &game,
            &condition,
            &ExecutionContext::new_default(source, alice)
        )
        .is_err()
    );
}

#[test]
fn source_x_and_target_facts_use_their_original_phase() {
    let (mut game, alice, _, source, target) = fixture();
    game.object_mut(source).unwrap().x_value = Some(4);
    let condition = Condition::XValueAtLeast(3);
    assert!(!evaluate_condition_cast_time(
        &game, &condition, alice, source
    ));
    let external = ExternalEvaluationContext {
        controller: alice,
        source,
        ..Default::default()
    };
    assert!(evaluate_condition_external(&game, &condition, &external));
    let mut exec = ExecutionContext::new_default(source, alice);
    exec.x_value = Some(2);
    assert!(!evaluate_condition_resolution(&game, &condition, &exec).unwrap());
    exec.x_value = Some(3);
    assert!(evaluate_condition_resolution(&game, &condition, &exec).unwrap());

    let condition = Condition::TargetMatches(ObjectFilter::creature());
    exec.targets = vec![ResolvedTarget::Object(target)];
    assert!(evaluate_condition_resolution(&game, &condition, &exec).unwrap());
    assert!(!evaluate_condition_cast_time(
        &game, &condition, alice, source
    ));
    assert!(!evaluate_condition_external(&game, &condition, &external));
}

#[test]
fn external_activation_options_do_not_change_other_phases() {
    let (game, alice, _, source, _) = fixture();
    let mut external = ExternalEvaluationContext {
        controller: alice,
        source,
        ..Default::default()
    };
    let timing = Condition::ActivationTiming(crate::ability::ActivationTiming::DuringCombat);
    let limit = Condition::MaxActivationsPerTurn(0);
    for condition in [&timing, &limit] {
        assert!(!evaluate_condition_external(&game, condition, &external));
        assert!(!evaluate_condition_cast_time(
            &game, condition, alice, source
        ));
        assert!(
            !evaluate_condition_resolution(
                &game,
                condition,
                &ExecutionContext::new_default(source, alice)
            )
            .unwrap()
        );
    }
    external.options.ignore_timing = true;
    external.options.ignore_activation_limits = true;
    assert!(evaluate_condition_external(&game, &timing, &external));
    assert!(evaluate_condition_external(&game, &limit, &external));
}

#[test]
fn shared_source_predicates_are_available_even_at_cast_time() {
    let (mut game, alice, _, source, _) = fixture();
    let external = ExternalEvaluationContext {
        controller: alice,
        source,
        ..Default::default()
    };
    let exec = ExecutionContext::new_default(source, alice);
    // These were already handled by the shared core before the old cast-time
    // fallback match. Consolidation must not resurrect that fallback's false.
    for (condition, expected) in [
        (Condition::SourceMatches(ObjectFilter::creature()), true),
        (Condition::SourcePowerAtLeast(3), true),
        (Condition::SourcePowerAtLeast(4), false),
        (Condition::SourceIsInZone(Zone::Battlefield), true),
    ] {
        assert_eq!(
            evaluate_condition_cast_time(&game, &condition, alice, source),
            expected
        );
        assert_eq!(
            evaluate_condition_external(&game, &condition, &external),
            expected
        );
        assert_eq!(
            evaluate_condition_resolution(&game, &condition, &exec).unwrap(),
            expected
        );
    }
    game.is_night = true;
    assert!(evaluate_condition_cast_time(
        &game,
        &Condition::ItIsNight,
        alice,
        source
    ));
    game.is_night = false;
    assert!(!evaluate_condition_cast_time(
        &game,
        &Condition::ItIsNight,
        alice,
        source
    ));
}

#[test]
fn hand_thresholds_preserve_signed_external_and_missing_player_behavior() {
    let (game, alice, _, source, _) = fixture();
    let external = ExternalEvaluationContext {
        controller: alice,
        source,
        ..Default::default()
    };
    let exec = ExecutionContext::new_default(source, alice);
    for (player, count, expected_external, expected_other) in [
        (PlayerFilter::You, -1, true, false),
        (
            PlayerFilter::Specific(PlayerId::from_index(99)),
            0,
            false,
            true,
        ),
    ] {
        let condition = Condition::PlayerCardsInHandOrMore { player, count };
        assert_eq!(
            evaluate_condition_external(&game, &condition, &external),
            expected_external
        );
        assert_eq!(
            evaluate_condition_cast_time(&game, &condition, alice, source),
            expected_other
        );
        assert_eq!(
            evaluate_condition_resolution(&game, &condition, &exec).unwrap(),
            expected_other
        );
    }
}

#[test]
fn trigger_limits_are_checked_at_registration_without_rejecting_resolution() {
    let (mut game, alice, _, source, _) = fixture();
    let identity = TriggerIdentity(99001);
    let external = ExternalEvaluationContext {
        controller: alice,
        source,
        trigger_identity: Some(identity),
        ..Default::default()
    };
    let conditions = [
        Condition::FirstTimeThisTurn,
        Condition::MaxTimesEachTurn(1),
        Condition::DoThisMaxTimesEachTurn(1),
    ];
    for condition in &conditions {
        assert!(evaluate_condition_external(&game, condition, &external));
    }
    game.record_trigger_fired(source, identity);
    let mut exec = ExecutionContext::new_default(source, alice);
    exec.trigger_identity = Some(identity);
    for condition in &conditions {
        assert!(!evaluate_condition_external(&game, condition, &external));
        assert!(evaluate_condition_resolution(&game, condition, &exec).unwrap());
        assert!(evaluate_condition_cast_time(
            &game, condition, alice, source
        ));
    }
}
