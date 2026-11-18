use crate::types::Subtype;
use super::*;

fn tapped_query() -> Value {
    Value::PendingPriorEffectMetric(
        ironsmith_core::PriorEffectMetricQuery::new(
            EffectMetricSource::AffectedObjects,
            EffectMetric::Count,
        )
        .with_action(PriorEffectAction::Tapped)
        .with_filter(ObjectFilter::creature().with_subtype(Subtype::Goblin)),
    )
}
fn consumer() -> EffectAst {
    EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
            count: tapped_query(),
        }),
    )
}
fn count(effect: &EffectAst) -> &Value {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count }),
        ..
    }) = effect
    else {
        panic!("{effect:?}");
    };
    count
}
#[test]
fn cost_lookup_sees_antecedents_in_order_without_treating_them_as_aliases() {
    let mut env = ReferenceEnv::default();
    let role = SnapshotAntecedent::SourceSuperseded;
    // A named alias can use the old marker's spelling independently.
    let alias = "__source_superseded_antecedent__";
    env.snapshot_bindings
        .add_alias((alias.into(), "tap_cost_1".into()));
    env.snapshot_bindings
        .set_antecedent(role, Some("tap_cost_2".into()));

    assert_eq!(
        env.snapshot_bindings.lookup(alias).unwrap().as_str(),
        "tap_cost_1"
    );
    assert_eq!(tap_cost_tag_from_env(&env).unwrap().as_str(), "tap_cost_2");
    assert_eq!(cost_tag_index_from_env(&env, "tap_cost_"), Some(2));

    let mut branch = env.clone();
    branch
        .snapshot_bindings
        .replace_alias((alias.into(), "tap_cost_3".into()));
    assert_eq!(
        tap_cost_tag_from_env(&branch).unwrap().as_str(),
        "tap_cost_3"
    );
    assert_eq!(
        branch.snapshot_bindings.antecedent(role),
        env.snapshot_bindings.antecedent(role)
    );
    assert_eq!(cost_tag_index_from_env(&env, "tap_cost_"), Some(2));
}

#[test]
fn imported_tap_groups_keep_the_exact_namespace_and_authored_predicate() {
    for tag in ["tapped_0", "tap_cost_2"] {
        let annotated = annotate_effect_sequence(
            &[consumer()],
            &ReferenceImports::with_last_object_tag(tag),
            Default::default(),
            Default::default(),
        )
        .unwrap();
        let Value::Count(filter) = count(&annotated.effects[0].effect) else {
            panic!("{annotated:?}");
        };
        assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
        assert_eq!(filter.subtypes, vec![Subtype::Goblin]);
        assert_eq!(filter.zone, None);
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == tag
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
        );
    }
}
#[test]
fn imported_tap_group_survives_unrelated_memory_but_local_tap_producer_wins() {
    let imports = ReferenceImports::with_last_object_tag("tapped_0");
    let destroy =
        EffectAst::subject_verb_destroy(TargetAst::Object(ObjectFilter::artifact(), None, None));
    let annotated = annotate_effect_sequence(
        &[destroy, consumer()],
        &imports,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert!(
        matches!(count(&annotated.effects[1].effect), Value::Count(filter)
        if filter.tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == "tapped_0"))
    );
    let tap = EffectAst::subject_verb_tap(TargetAst::Object(ObjectFilter::creature(), None, None));
    let annotated = annotate_effect_sequence(
        &[tap, consumer()],
        &imports,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert!(
        matches!(count(&annotated.effects[1].effect), Value::PriorEffectMetric { effect_id: EffectId(0), query }
        if query.action == Some(PriorEffectAction::Tapped))
    );
}
#[test]
fn missing_tap_group_and_different_metric_domains_are_not_guessed() {
    for tag in [
        "sacrifice_cost_0",
        "unrelated_object",
        "tapped_this_way_group",
        "tap_cost_no_index",
    ] {
        assert!(
            annotate_effect_sequence(
                &[consumer()],
                &ReferenceImports::with_last_object_tag(tag),
                Default::default(),
                Default::default()
            )
            .is_err()
        );
    }
    let env = ReferenceEnv::from_imports(
        &ReferenceImports::with_last_object_tag("tapped_0"),
        false,
        false,
        false,
        None,
    );
    for query in [
        ironsmith_core::PriorEffectMetricQuery::new(
            EffectMetricSource::Outcome,
            EffectMetric::Count,
        )
        .with_action(PriorEffectAction::Tapped),
        ironsmith_core::PriorEffectMetricQuery::new(
            EffectMetricSource::AffectedObjects,
            EffectMetric::Count,
        )
        .with_action(PriorEffectAction::Milled),
        ironsmith_core::PriorEffectMetricQuery::new(
            EffectMetricSource::AffectedObjects,
            EffectMetric::Count,
        )
        .with_action(PriorEffectAction::Tapped)
        .with_player(PlayerFilter::You),
    ] {
        let mut value = Value::PendingPriorEffectMetric(query);
        assert!(
            resolve_effect_result_value(&mut value, effect_reference_resolution_state(&env))
                .is_err()
        );
    }
}

#[test]
fn tapped_power_is_a_live_object_alias_with_local_body_precedence_over_a_cost() {
    let value = Value::PowerOf(Box::new(ChooseSpec::Tagged(
        crate::tag::PRIOR_TAPPED_OBJECT_QUANTITY_TAG.into(),
    )));
    let consumer = || {
        EffectAst::subject_verb_damage(value.clone(), TargetAst::Player(PlayerFilter::You, None))
    };
    let imports = ReferenceImports::with_last_object_tag("tap_cost_2");
    let imported = annotate_effect_sequence(
        &[consumer()],
        &imports,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let bound = crate::reference_helpers::resolve_value_it_tag(&value, &imported.effects[0].in_env)
        .unwrap();
    assert!(
        matches!(bound,Value::PowerOf(spec) if matches!(spec.base(),ChooseSpec::Tagged(tag) if tag.as_str()=="tap_cost_2"))
    );
    let tap = EffectAst::subject_verb_tap(TargetAst::Object(ObjectFilter::creature(), None, None));
    let unrelated = EffectAst::subject_verb_explicit_target_only(TargetAst::Object(
        ObjectFilter::artifact(),
        None,
        None,
    ));
    let body = annotate_effect_sequence(
        &[tap, unrelated, consumer()],
        &imports,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let concrete = &body.effects[0]
        .out_env
        .snapshot_bindings
        .aliases()
        .find(|(alias, _)| alias.as_str() == crate::tag::PRIOR_TAPPED_OBJECT_QUANTITY_TAG)
        .unwrap()
        .1;
    assert_ne!(concrete.as_str(), "tap_cost_2");
    let bound =
        crate::reference_helpers::resolve_value_it_tag(&value, &body.effects[2].in_env).unwrap();
    assert!(
        matches!(bound,Value::PowerOf(spec) if matches!(spec.base(),ChooseSpec::Tagged(tag) if tag==concrete))
    );
    assert!(
        crate::reference_helpers::resolve_value_it_tag(&value, &ReferenceEnv::default()).is_err()
    );
}
