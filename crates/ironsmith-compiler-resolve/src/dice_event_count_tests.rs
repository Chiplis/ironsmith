use super::*;
#[test]
fn bare_die_result_requires_proof_and_preserves_local_producer_precedence() {
    for (grouped, expected) in [
        (false, EventValueSpec::DieResult),
        (true, EventValueSpec::DieBatchTotal),
    ] {
        let mut env = ReferenceEnv {
            dice_event_grouped: Some(grouped),
            ..Default::default()
        };
        let mut value = Value::EventValue(EventValueSpec::Amount)
            .with_surface_hint(ValueSurfaceHint::PriorEffectResult);
        resolve_effect_result_value(&mut value, effect_reference_resolution_state(&env)).unwrap();
        assert_eq!(value.unhinted(), &Value::EventValue(expected));
        env.last_effect_id = RefState::Known(EffectId(7));
        let mut value = Value::EventValue(EventValueSpec::Amount)
            .with_surface_hint(ValueSurfaceHint::PriorEffectResult);
        resolve_effect_result_value(&mut value, effect_reference_resolution_state(&env)).unwrap();
        assert_eq!(value.unhinted(), &Value::EffectValue(EffectId(7)));
        let restored = ReferenceEnv::from_frame(&ReferenceFrame::from_lowering_frame(
            &env.to_lowering_frame(false, false),
        ));
        assert_eq!(restored.dice_event_grouped, Some(grouped));
    }
    let mut value = Value::EventValue(EventValueSpec::Amount)
        .with_surface_hint(ValueSurfaceHint::PriorEffectResult);
    assert!(
        resolve_effect_result_value(
            &mut value,
            effect_reference_resolution_state(&ReferenceEnv::default())
        )
        .is_err()
    );
}
#[test]
fn union_arms_must_supply_the_same_physical_or_grouped_die_result() {
    use ironsmith_compiler_semantic::trigger_references::trigger_die_event_grouped as proof;
    let a = TriggerSpec::PlayerRollsDie {
        player: PlayerFilter::You,
        one_or_more: true,
    };
    assert_eq!(
        proof(&TriggerSpec::Either(
            Box::new(a.clone()),
            Box::new(a.clone())
        )),
        Some(true)
    );
    assert_eq!(
        proof(&TriggerSpec::Either(
            Box::new(a),
            Box::new(TriggerSpec::PlayerRollsResult {
                player: PlayerFilter::You,
                result: 2
            })
        )),
        None
    );
    assert_eq!(
        proof(&TriggerSpec::PlayerRollsNthDie {
            player: PlayerFilter::You,
            ordinal: 3
        }),
        None
    );
}

fn roll_query() -> ironsmith_core::PriorEffectMetricQuery {
    ironsmith_core::PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::Count)
        .with_action(PriorEffectAction::Rolled)
}
fn roll_condition() -> EffectAst {
    EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::ValueComparison {
            left: Value::PendingPriorEffectMetric(roll_query()),
            operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(4),
        },
        if_true: vec![],
        if_false: vec![],
    })
}
#[test]
fn typed_roll_predicate_keeps_the_real_local_producer_across_unrelated_results() {
    let effects = vec![
        EffectAst::subject_verb_roll_die(PlayerAst::You, 6),
        // The latest actual roll replaces the previous roll antecedent.
        EffectAst::subject_verb_roll_die(PlayerAst::You, 20),
        roll_condition(),
    ];
    let mut ids = IdGenContext::default();
    let config = EffectReferenceResolutionConfig {
        dice_event_grouped: Some(false),
        ..Default::default()
    };
    let first = annotate_effect_sequence(
        &effects,
        &ReferenceImports::default(),
        config.clone(),
        ids.clone(),
    )
    .unwrap();
    let id = first.effects[1]
        .assigned_effect_id
        .expect("the second roll must export its scalar result");
    assert_ne!(first.effects[0].assigned_effect_id, Some(id));
    let EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::ValueComparison { left, .. },
        ..
    }) = &first.effects[2].effect
    else {
        panic!("missing predicate")
    };
    assert_eq!(
        left,
        &Value::PriorEffectMetric {
            effect_id: id,
            query: roll_query()
        }
    );
    // Transparent lowering visits the resolved AST again. It must reuse the
    // producer identity already retained by the comparison, not allocate anew.
    ids.next_effect_id = 100;
    let rebound = annotate_effect_sequence(
        &first
            .effects
            .iter()
            .map(|e| e.effect.clone())
            .collect::<Vec<_>>(),
        &ReferenceImports::default(),
        config,
        ids,
    )
    .unwrap();
    assert_eq!(rebound.effects[1].assigned_effect_id, Some(id));

    let env = ReferenceEnv {
        last_effect_id: RefState::Known(EffectId(99)),
        dice_event_grouped: Some(false),
        die_result_producers: std::sync::Arc::new(vec![Some(id)]),
        ..Default::default()
    };
    let mut value = Value::PendingPriorEffectMetric(roll_query());
    resolve_effect_result_value(&mut value, effect_reference_resolution_state(&env)).unwrap();
    assert_eq!(
        value,
        Value::PriorEffectMetric {
            effect_id: id,
            query: roll_query()
        }
    );
    let restored = ReferenceEnv::from_frame(&ReferenceFrame::from_lowering_frame(
        &env.to_lowering_frame(false, false),
    ));
    assert_eq!(restored.die_result_producers, env.die_result_producers);
}
#[test]
fn singular_roll_predicate_requires_local_or_singular_event_evidence() {
    for grouped in [None, Some(true), Some(false)] {
        let env = ReferenceEnv {
            dice_event_grouped: grouped,
            ..Default::default()
        };
        let mut value = Value::PendingPriorEffectMetric(roll_query());
        let result =
            resolve_effect_result_value(&mut value, effect_reference_resolution_state(&env));
        assert_eq!(result.is_ok(), grouped == Some(false));
        if result.is_ok() {
            assert_eq!(value, Value::EventValue(EventValueSpec::DieResult));
        }
    }
    let env = ReferenceEnv {
        dice_event_grouped: Some(false),
        die_result_producers: std::sync::Arc::new(vec![None]),
        ..Default::default()
    };
    assert!(
        resolve_effect_result_value(
            &mut Value::PendingPriorEffectMetric(roll_query()),
            effect_reference_resolution_state(&env)
        )
        .is_err()
    );
}
#[test]
fn grouped_die_predicates_require_proven_grouped_events_and_reject_local_batch_ambiguity() {
    for grouped in [None, Some(false), Some(true)] {
        for local in [vec![], vec![Some(EffectId(4))], vec![None]] {
            let env = ReferenceEnv {
                dice_event_grouped: grouped,
                die_result_producers: std::sync::Arc::new(local.clone()),
                ..Default::default()
            };
            for mut value in [
                Value::EventValue(EventValueSpec::DieResultsAtLeast(10)),
                Value::EventValue(EventValueSpec::DieBatchTotal),
            ] {
                assert_eq!(
                    resolve_effect_result_value(
                        &mut value,
                        effect_reference_resolution_state(&env)
                    )
                    .is_ok(),
                    grouped == Some(true) && local.is_empty()
                );
            }
        }
    }
}
