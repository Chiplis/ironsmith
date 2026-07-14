use super::*;

fn two_mode_distinct_opponent_effect() -> Effect {
    let target_opponent = || {
        Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
            ChooseSpec::Player(PlayerFilter::Opponent),
        )))
    };
    let modes = vec![
        crate::effect::EffectMode::new("First mode", vec![target_opponent()]),
        crate::effect::EffectMode::new("Second mode", vec![target_opponent()]),
    ];
    Effect::new(
        crate::effects::ChooseModeEffect::choose_exactly(2, modes)
            .with_distinct_player_targets_per_mode(),
    )
}

#[test]
pub(super) fn modal_distinct_player_rule_requires_a_distinct_legal_assignment() {
    let alice = PlayerId::from_index(0);
    let effect = two_mode_distinct_opponent_effect();

    let two_player_game = setup_game();
    assert!(
        !spell_has_legal_targets_with_modes(
            &two_player_game,
            std::slice::from_ref(&effect),
            alice,
            None,
            Some(&[0, 1]),
        ),
        "one opponent cannot provide distinct targets for two selected modes"
    );

    let three_player_game = setup_three_player_game();
    assert!(spell_has_legal_targets_with_modes(
        &three_player_game,
        std::slice::from_ref(&effect),
        alice,
        None,
        Some(&[0, 1]),
    ));

    let requirements = extract_target_requirements_with_modes(
        &three_player_game,
        std::slice::from_ref(&effect),
        alice,
        None,
        Some(&[0, 1]),
    );
    assert_eq!(requirements.len(), 2, "{requirements:#?}");
    assert_eq!(
        requirements[0].distinct_player_group,
        requirements[1].distinct_player_group
    );
    assert!(requirements[0].distinct_player_group.is_some());

    let contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                legal_target_sets: requirement.legal_target_sets.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
                distinct_player_group: requirement.distinct_player_group,
            },
        )
        .collect::<Vec<_>>();
    let bob = Target::Player(PlayerId::from_index(1));
    let charlie = Target::Player(PlayerId::from_index(2));

    assert!(!crate::targeting::validate_flat_target_assignment(
        &contexts,
        &[bob, bob],
    ));
    assert!(crate::targeting::validate_flat_target_assignment(
        &contexts,
        &[bob, charlie],
    ));
    let autofilled = crate::targeting::normalize_targets_for_requirements(&contexts, Vec::new())
        .expect("the legal target planner should find a distinct assignment");
    assert_eq!(autofilled.len(), 2);
    assert_ne!(autofilled[0], autofilled[1]);
}
