use super::*;

#[test]
fn bare_x_plus_life_keeps_the_authored_arithmetic_surface() {
    let amount = Value::Add(Box::new(Value::X), Box::new(Value::Fixed(3)));

    assert_eq!(
        describe_effect(&Effect::new(crate::effects::GainLifeEffect::you(amount))),
        "you gain X plus 3 life"
    );
}

#[test]
fn explicit_equal_to_x_plus_life_keeps_the_equal_to_surface() {
    let amount = Value::Add(Box::new(Value::X), Box::new(Value::Fixed(3)))
        .with_surface_hint(ValueSurfaceHint::EqualTo);

    assert_eq!(
        describe_effect(&Effect::new(crate::effects::GainLifeEffect::you(amount))),
        "you gain life equal to X plus 3"
    );
}
