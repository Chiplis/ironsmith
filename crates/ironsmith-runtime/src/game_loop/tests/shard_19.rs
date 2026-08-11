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
                aggregate_constraint: requirement.aggregate_constraint.clone(),
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

#[test]
pub(super) fn alternative_activation_cost_locks_and_pays_the_selected_complete_branch() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let generic_branch = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]])),
        crate::costs::Cost::tap(),
    ]);
    let white_branch = crate::cost::TotalCost::from_costs(vec![
        crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::White]])),
        crate::costs::Cost::tap(),
    ]);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Alternative Cost Shard")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::activated(
            crate::cost::TotalCost::one_of(vec![generic_branch, white_branch]),
            vec![Effect::draw(1)],
        ))
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::White, 1);

    let action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility {
                    source: action_source,
                    ability_index: 0,
                } if *action_source == source
            )
        })
        .expect("the payable white branch should make the ability legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut decision_maker = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut decision_maker,
    )
    .expect("activating should prompt for a complete alternative cost");

    let branch_context = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(context),
        ) => context,
        other => panic!("expected an alternative-cost prompt, got {other:?}"),
    };
    assert_eq!(branch_context.options.len(), 2);
    assert!(branch_context.options[0].description.contains("{3}"));
    assert!(!branch_context.options[0].legal);
    assert!(branch_context.options[1].description.contains("{W}"));
    assert!(branch_context.options[1].legal);

    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::NextCostChoice(1),
        &mut decision_maker,
    )
    .expect("the selected white-and-tap branch should lock and begin payment");

    for _ in 0..8 {
        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::ManaPayment(context),
            ) => apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::ManaPaymentPlan(
                    crate::mana_payment::ManaPaymentResponse::Confirm {
                        plan_id: context.plan.id,
                        request_hash: context.plan.request_hash,
                    },
                ),
                &mut decision_maker,
            )
            .expect("the locked white branch mana plan should remain payable"),
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(context),
            ) => {
                let option = context
                    .options
                    .iter()
                    .find(|option| option.legal)
                    .expect("the locked white branch should have a legal payment option");
                assert!(matches!(
                    state
                        .pending_activation
                        .as_ref()
                        .map(|pending| &pending.stage),
                    Some(ActivationStage::ChoosingNextCost)
                ));
                let response = PriorityResponse::NextCostChoice(option.index);
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &response,
                    &mut decision_maker,
                )
                .expect("the locked white branch should remain payable")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            )
            | crate::decision::GameProgress::Continue => break,
            other => panic!("unexpected activation payment progress: {other:?}"),
        };
    }

    assert_eq!(
        game.stack.len(),
        1,
        "the paid ability should be on the stack"
    );
    assert!(
        game.is_tapped(source),
        "the selected branch must pay its tap cost"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.white,
        0,
        "the selected branch must pay one white mana, not three generic mana"
    );
}
