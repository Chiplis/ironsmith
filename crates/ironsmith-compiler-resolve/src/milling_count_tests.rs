use super::*;
fn nonland() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter
        .excluded_card_types
        .push(crate::types::CardType::Land);
    filter
}
fn query(filter: ObjectFilter) -> Value {
    Value::PendingPriorEffectMetric(
        ironsmith_core::PriorEffectMetricQuery::new(
            EffectMetricSource::AffectedObjects,
            EffectMetric::Count,
        )
        .with_action(PriorEffectAction::Milled)
        .with_filter(filter),
    )
}
#[test]
fn only_the_exact_filtered_milling_event_count_is_a_valid_fallback() {
    let env = ReferenceEnv {
        allow_life_event_value: true,
        milling_event_filter: Some(std::sync::Arc::new(nonland())),
        ..Default::default()
    };
    let mut value = query(nonland());
    resolve_effect_result_value(&mut value, effect_reference_resolution_state(&env)).unwrap();
    assert_eq!(value, Value::EventValue(EventValueSpec::Amount));
    for filter in [ObjectFilter::default(), ObjectFilter::creature()] {
        let mut mismatch = query(filter);
        assert!(
            resolve_effect_result_value(&mut mismatch, effect_reference_resolution_state(&env))
                .is_err()
        );
    }
    let mut player_scoped = query(nonland());
    let Value::PendingPriorEffectMetric(descriptor) = &mut player_scoped else {
        unreachable!()
    };
    descriptor.player = Some(PlayerFilter::You);
    assert!(
        resolve_effect_result_value(&mut player_scoped, effect_reference_resolution_state(&env))
            .is_err()
    );
    let mut foreign_action = query(nonland());
    let Value::PendingPriorEffectMetric(descriptor) = &mut foreign_action else {
        unreachable!()
    };
    descriptor.action = Some(PriorEffectAction::Discarded);
    assert!(
        resolve_effect_result_value(&mut foreign_action, effect_reference_resolution_state(&env))
            .is_err()
    );
    let mut absent = query(nonland());
    assert!(
        resolve_effect_result_value(
            &mut absent,
            effect_reference_resolution_state(&ReferenceEnv::default())
        )
        .is_err()
    );
}
#[test]
fn a_real_local_mill_producer_still_precedes_the_triggering_event() {
    let mut env = ReferenceEnv {
        milling_event_filter: Some(std::sync::Arc::new(nonland())),
        ..Default::default()
    };
    env.last_effect_id = RefState::Known(EffectId(19));
    let mut value = query(ObjectFilter::creature());
    resolve_effect_result_value(&mut value, effect_reference_resolution_state(&env)).unwrap();
    assert!(
        matches!(value, Value::PriorEffectMetric { effect_id: EffectId(19), query } if query.filter == Some(ObjectFilter::creature()) && query.action == Some(PriorEffectAction::Milled))
    );
    let frame = env.to_lowering_frame(false, false);
    let restored = ReferenceEnv::from_frame(&ReferenceFrame::from_lowering_frame(&frame));
    assert_eq!(restored.milling_event_filter, env.milling_event_filter);
}
#[test]
fn alternatives_must_all_export_the_same_milling_predicate() {
    use ironsmith_compiler_semantic::trigger_references::trigger_milling_event_filter;
    let event = |filter| TriggerSpec::CardsMilled {
        player: PlayerFilter::Any,
        filter,
        one_or_more: true,
        per_player: false,
    };
    let a = event(Some(nonland()));
    assert!(trigger_milling_event_filter(&a).is_some());
    assert!(
        trigger_milling_event_filter(&TriggerSpec::Either(
            Box::new(a.clone()),
            Box::new(a.clone())
        ))
        .is_some()
    );
    assert!(
        trigger_milling_event_filter(&TriggerSpec::Either(
            Box::new(a.clone()),
            Box::new(event(Some(ObjectFilter::creature())))
        ))
        .is_none()
    );
    assert!(
        trigger_milling_event_filter(&TriggerSpec::Either(
            Box::new(a),
            Box::new(TriggerSpec::YouGainLife)
        ))
        .is_none()
    );
}
