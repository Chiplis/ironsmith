use super::*;

const ESSENCE_VORTEX_SURFACE: &str = "Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated";

fn essence_vortex_shape(payer_tag: TagKey, amount: Value, authored_followup: bool) -> Effect {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    let destroyed_tag = TagKey::from("destroyed_0");
    let destroy = Effect::new(
        crate::effects::DestroyNoRegenerationEffect::with_spec(target)
            .with_creature_destroyed_this_way_surface(authored_followup),
    )
    .tag(destroyed_tag);
    let cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::effect(
        crate::effects::LoseLifeEffect::you(amount),
    ));

    Effect::new(crate::effects::UnlessPaysEffect::new_total_cost(
        vec![destroy],
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(payer_tag)),
        cost,
    ))
}

fn toughness_of_target_creature() -> Value {
    Value::ToughnessOf(Box::new(ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::creature(),
    ))))
}

#[test]
fn dynamic_toughness_life_payment_restores_authored_destroy_rider() {
    let effect = essence_vortex_shape(
        TagKey::from("destroyed_0"),
        toughness_of_target_creature(),
        true,
    );

    assert_eq!(describe_effect(&effect), ESSENCE_VORTEX_SURFACE);
}

#[test]
fn dynamic_destroy_surface_requires_the_exact_reference_graph() {
    let wrong_payer = essence_vortex_shape(
        TagKey::from("other_destroyed"),
        toughness_of_target_creature(),
        true,
    );
    let missing_authored_followup = essence_vortex_shape(
        TagKey::from("destroyed_0"),
        toughness_of_target_creature(),
        false,
    );
    let wrong_basis = essence_vortex_shape(
        TagKey::from("destroyed_0"),
        Value::PowerOf(Box::new(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature(),
        )))),
        true,
    );

    for near_miss in [wrong_payer, missing_authored_followup, wrong_basis] {
        assert_ne!(describe_effect(&near_miss), ESSENCE_VORTEX_SURFACE);
    }
}
