use super::*;

#[test]
fn synthetic_target_with_one_value_consumer_folds_into_that_action() {
    let target = ChooseSpec::target_creature();
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target.clone())).tag("targeted_0"),
        Effect::new(crate::effects::GainLifeEffect::you(Value::PowerOf(
            Box::new(target),
        ))),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "You gain life equal to target creature's power"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("you gain life equal to target creature's power")
    );
}

#[test]
fn synthetic_target_with_two_consumers_retains_explicit_tag_identity() {
    let tag = TagKey::from("targeted_0");
    let target = ChooseSpec::target_creature();
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target.clone())).tag(tag.clone()),
        Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            target,
        )),
        Effect::new(crate::effects::DrawCardsEffect::you(Value::PowerOf(
            Box::new(ChooseSpec::Tagged(tag)),
        ))),
    ];

    let rendered = describe_effect_list(&effects);
    assert!(
        rendered.starts_with("Choose target creature"),
        "shared target declaration was lost: {rendered}"
    );
    let lowercase = rendered.to_ascii_lowercase();
    assert!(lowercase.contains("put a +1/+1 counter on target creature"));
    assert!(
        lowercase.contains("draw cards equal to that creature's power")
            || lowercase.contains("draw cards equal to its power")
    );

    let clause = describe_effect_clause_list(&effects).expect("clause rendering");
    assert!(
        clause.starts_with("choose target creature"),
        "shared target declaration was lost in clause rendering: {clause}"
    );
}

#[test]
fn synthetic_target_with_one_attached_object_consumer_folds_the_anchor() {
    let tag = TagKey::from("targeted_0");
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default()
            .with_type(CardType::Land)
            .in_zone(Zone::Battlefield),
    ));
    let mut attached_auras = ObjectFilter::default()
        .with_subtype(Subtype::Aura)
        .in_zone(Zone::Battlefield);
    attached_auras
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::AttachedToTaggedObject,
        });
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(tag),
        Effect::new(crate::effects::DestroyEffect::all(attached_auras)),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Destroy all Auras attached to target land"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("destroy all Auras attached to target land")
    );
}

#[test]
fn synthetic_spell_target_folds_into_controller_and_mana_value_damage() {
    let tag = TagKey::from("targeted_0");
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_spell(),
        ))
        .tag(tag.clone()),
        Effect::deal_damage(
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(tag.clone())))
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
            ChooseSpec::Player(PlayerFilter::ControllerOf(
                crate::filter::ObjectRef::Tagged(tag),
            )),
        ),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Deal damage to target spell's controller equal to that spell's mana value"
    );
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some("deal damage to target spell's controller equal to that spell's mana value")
    );
}

fn conditional_counter_spell_with_mana_value(
    comparison: ironsmith_core::FilterComparison,
) -> Vec<Effect> {
    let tag = TagKey::from("countered_0");
    let target = ChooseSpec::target_spell();
    let mut condition_filter = ObjectFilter::default();
    condition_filter.mana_value = Some(comparison);
    vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target.clone())).tag(tag.clone()),
        Effect::new(crate::effects::ConditionalEffect::if_only(
            Condition::TaggedObjectMatches(tag.clone(), condition_filter),
            vec![Effect::counter(target).tag(tag)],
        )),
    ]
}

#[test]
fn synthetic_counter_spell_fixed_mana_value_gate_uses_target_possessive() {
    let effects = conditional_counter_spell_with_mana_value(
        ironsmith_core::FilterComparison::LessThanOrEqual(2),
    );

    assert_eq!(
        describe_effect_list(&effects),
        "Counter target spell if its mana value is 2 or less"
    );
}

#[test]
fn synthetic_counter_spell_dynamic_mana_value_gate_uses_target_possessive() {
    let effects = conditional_counter_spell_with_mana_value(
        ironsmith_core::FilterComparison::EqualExpr(Box::new(Value::X)),
    );

    assert_eq!(
        describe_effect_list(&effects),
        "Counter target spell if its mana value is X"
    );
}
