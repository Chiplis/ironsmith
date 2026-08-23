use super::*;

fn counter_text(filter: ObjectFilter) -> String {
    describe_effect(&Effect::new(crate::effects::CounterEffect::new(
        ChooseSpec::target(ChooseSpec::Object(filter)),
    )))
}

#[test]
fn targeted_stack_spell_renders_positive_cast_origin_as_provenance() {
    let filter = ObjectFilter::spell().in_zone(Zone::Graveyard);

    assert_eq!(
        counter_text(filter),
        "Counter target spell cast from a graveyard"
    );
}

#[test]
fn positive_cast_origin_counter_surface_rejects_nonspell_and_stack_near_misses() {
    assert_eq!(counter_text(ObjectFilter::spell()), "Counter target spell");
    assert_eq!(
        counter_text(ObjectFilter::default().in_zone(Zone::Graveyard)),
        "Counter target card in a graveyard"
    );
}
