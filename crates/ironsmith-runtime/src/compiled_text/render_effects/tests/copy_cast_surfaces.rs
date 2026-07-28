use super::*;

#[test]
fn sentence_helper_copy_cast_keeps_the_copy_action_explicit() {
    let exiled = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let cast_copy = Effect::new(
        crate::effects::CastTaggedEffect::new(exiled, PlayerFilter::You)
            .as_copy()
            .without_paying_mana_cost(),
    );
    let may_cast_copy = Effect::new(crate::effects::MayEffect::new(vec![cast_copy]));

    assert_eq!(
        describe_effect(&may_cast_copy),
        "Copy it. You may cast the copy without paying its mana cost"
    );
}

#[test]
fn tagged_card_cast_without_copy_does_not_invent_a_copy_action() {
    let exiled = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let cast_card = Effect::new(
        crate::effects::CastTaggedEffect::new(exiled, PlayerFilter::You).without_paying_mana_cost(),
    );
    let may_cast_card = Effect::new(crate::effects::MayEffect::new(vec![cast_card]));

    assert_eq!(
        describe_effect(&may_cast_card),
        "You may cast that card without paying its mana cost"
    );
}

#[test]
fn explicit_copy_effect_and_copy_cast_permission_render_the_copy_only_once() {
    let exiled = TagKey::from("__sentence_helper_exiled_l0_s0_e0");
    let copy = Effect::new(crate::effects::CopySpellEffect::new(
        ChooseSpec::Tagged(exiled.clone()),
        Value::Fixed(1),
    ));
    let cast_copy = Effect::new(
        crate::effects::CastTaggedEffect::new(exiled, PlayerFilter::You)
            .as_copy()
            .without_paying_mana_cost(),
    );
    let may_cast_copy = Effect::new(crate::effects::MayEffect::new(vec![cast_copy]));

    assert_eq!(
        describe_effect_list(&[copy, may_cast_copy]),
        "Copy it. You may cast the copy without paying its mana cost"
    );
}
