use super::*;

#[test]
fn independently_optional_hand_casts_render_as_any_number() {
    let filter = ObjectFilter::nonland()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::You);
    let cast = Effect::cast_tagged("__it__", PlayerFilter::You, false, false, true, None);
    let effect = Effect::new(crate::effects::ForEachObject::new(
        filter,
        vec![Effect::may_single(cast)],
    ));

    assert_eq!(
        describe_effect(&effect),
        "you may cast any number of spells from your hand without paying their mana costs"
    );
}
