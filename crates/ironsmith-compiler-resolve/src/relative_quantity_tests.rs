use super::*;

fn draw_difference() -> EffectAst {
    EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: crate::cards::builders::SubjectVerbSubjectAst {
            role: crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
            player: PlayerAst::You,
        },
        action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
            count: Value::PendingComparisonDifference,
        }),
    })
}

#[test]
fn current_condition_operands_override_earlier_comparison_inside_the_branch() {
    let inner = EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::ValueComparison {
            left: Value::CardsInHand(PlayerFilter::You),
            operator: ironsmith_core::ValueComparisonOperator::LessThan,
            right: Value::Fixed(7),
        },
        if_true: vec![draw_difference()],
        if_false: vec![],
    });
    let imports = ReferenceImports {
        last_value_comparison: Some((Value::Fixed(100), Value::Fixed(2))),
        ..Default::default()
    };
    let outer =
        annotate_effect_sequence(&[inner], &imports, Default::default(), Default::default())
            .unwrap();
    let effect = &outer.effects[0];
    assert_eq!(
        effect.in_env.last_value_comparison,
        RefState::Known((Value::CardsInHand(PlayerFilter::You), Value::Fixed(7)))
    );
    let EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_true, .. }) = &effect.effect
    else {
        panic!("conditional")
    };
    // Lowering re-annotates the body using this local imported environment.
    let body = annotate_effect_sequence(
        if_true,
        &ReferenceImports {
            last_value_comparison: effect.in_env.last_value_comparison.clone().into_option(),
            ..Default::default()
        },
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count }),
        ..
    }) = &body.effects[0].effect
    else {
        panic!("draw")
    };
    assert_eq!(
        count,
        &Value::absolute_difference(Value::CardsInHand(PlayerFilter::You), Value::Fixed(7))
            .with_surface_hint(ValueSurfaceHint::Difference)
    );
}

#[test]
fn chosen_player_comparison_uses_the_lexical_player_or_runtime_iteration() {
    let predicate = PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreCardsInHandThanYou {
        player: PlayerAst::That,
    });
    let mut env = ReferenceEnv::default();
    assert_eq!(
        predicate_comparison_operands(&predicate, &env).unwrap().0,
        Value::CardsInHand(PlayerFilter::IteratedPlayer)
    );
    env.last_player_filter = RefState::Known(PlayerFilter::Specific(
        ironsmith_core::PlayerId::from_index(2),
    ));
    let values = predicate_comparison_operands(&predicate, &env).unwrap();
    assert_eq!(
        values.0,
        Value::CardsInHand(PlayerFilter::Specific(
            ironsmith_core::PlayerId::from_index(2)
        ))
    );
    assert_eq!(values.1, Value::CardsInHand(PlayerFilter::You));
}

#[test]
fn prior_discard_shortfall_is_bound_to_the_discard_id_not_hand_size() {
    let mut surface = ironsmith_core::PriorEffectResultSurface::new(
        PriorEffectAction::Discarded,
        ObjectFilter::default(),
        ironsmith_core::PriorEffectResultActor::Passive,
        ironsmith_core::PriorEffectResultQuantifier::OneOrMore,
    );
    surface.negated = true;
    surface.required_count = Some(2);
    let effects = vec![
        EffectAst::subject_verb_discard(
            PlayerAst::TargetOpponent,
            Value::Fixed(2),
            false,
            false,
            None,
            None,
        ),
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::PriorEffectResult(surface),
            effects: vec![draw_difference()],
        }),
    ];
    let annotated = annotate_effect_sequence(
        &effects,
        &Default::default(),
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let debug = format!("{annotated:?}");
    assert!(!debug.contains("PendingComparison"), "{debug}");
    assert!(
        debug.contains("PriorEffectMetric") && debug.contains("Discarded"),
        "{debug}"
    );
    assert!(!debug.contains("CardsInHand"), "{debug}");
}

#[test]
fn returned_quantity_alias_is_required_and_survives_an_intervening_target() {
    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
        crate::tag::RETURNED_THIS_WAY_QUANTITY_TAG,
    ))));
    assert!(
        crate::reference_helpers::resolve_value_it_tag(&amount, &ReferenceEnv::default()).is_err()
    );
    let target = TargetAst::Object(
        ObjectFilter::creature().in_zone(crate::zone::Zone::Graveyard),
        Some(crate::cards::TextSpan {
            line: 0,
            start: 0,
            end: 10,
        }),
        None,
    );
    let effects = vec![
        EffectAst::subject_verb_return_to_hand(target, false),
        EffectAst::subject_verb_explicit_target_only(TargetAst::Object(
            ObjectFilter::artifact(),
            Some(crate::cards::TextSpan {
                line: 1,
                start: 0,
                end: 10,
            }),
            None,
        )),
        EffectAst::subject_verb_damage(amount.clone(), TargetAst::Player(PlayerFilter::You, None)),
    ];
    let annotated = annotate_effect_sequence(
        &effects,
        &ReferenceImports::default(),
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let first = annotated.effects[0]
        .out_env
        .snapshot_tag_aliases
        .iter()
        .find(|(alias, _)| alias.as_str() == crate::tag::RETURNED_THIS_WAY_QUANTITY_TAG)
        .unwrap()
        .1
        .clone();
    let resolved =
        crate::reference_helpers::resolve_value_it_tag(&amount, &annotated.effects[2].in_env)
            .unwrap();
    assert!(
        matches!(resolved, Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &first))
    );
}

#[test]
fn existential_group_comparisons_do_not_invent_one_compared_opponent() {
    for player in [PlayerAst::Opponent, PlayerAst::Any] {
        let predicate =
            PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreCardsInHandThanYou { player });
        assert!(predicate_comparison_operands(&predicate, &ReferenceEnv::default()).is_none());
    }
}
