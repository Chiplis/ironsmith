use super::*;

#[test]
fn bounded_x_payment_and_linked_draw_preserve_authored_x_surface() {
    let payment_id = crate::effect::EffectId(7);
    let payment = Effect::with_id(
        payment_id.0,
        Effect::may(vec![Effect::new(
            crate::effects::PayManaEffect::new(
                crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::X]),
                ChooseSpec::Player(PlayerFilter::You),
            )
            .with_x_maximum(Value::EventValue(EventValueSpec::LifeAmount)),
        )]),
    );
    let draw = Effect::if_then(
        payment_id,
        EffectPredicate::Happened,
        vec![Effect::draw(Value::EffectValue(payment_id))],
    );

    assert_eq!(
        describe_effect_list(&[payment, draw]),
        "You may pay {X}, where X is less than or equal to the amount of life you gained. If you do, draw X cards"
    );
}
