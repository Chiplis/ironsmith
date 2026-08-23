use super::*;

fn all_mountains() -> ChooseSpec {
    let mut mountains = ObjectFilter::default();
    mountains.zone = Some(Zone::Battlefield);
    mountains.card_types = vec![CardType::Land];
    mountains.subtypes = vec![Subtype::Mountain];
    ChooseSpec::All(mountains)
}

#[test]
fn permanent_plural_basic_land_type_setting_uses_static_characteristic_surface() {
    let effect = Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::fixed(
        all_mountains(),
        Subtype::Plains,
        Until::Forever,
    ));

    assert_eq!(describe_effect(&effect), "All Mountains are Plains");
}

#[test]
fn temporary_plural_basic_land_type_setting_remains_a_transformation() {
    let effect = Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::fixed(
        all_mountains(),
        Subtype::Plains,
        Until::EndOfTurn,
    ));

    assert_eq!(
        describe_effect(&effect),
        "All Mountains become Plains until end of turn"
    );
}
