use super::*;
use crate::zone::Zone;

fn rendered_counter_replacement(
    replacement_zone: Zone,
    placement: Option<ironsmith_core::ZoneReplacementLibraryPlacement>,
) -> String {
    let tag = TagKey::from("countered_spell");
    let producer = Effect::new(crate::effects::CounterEffect::new(ChooseSpec::target(
        ChooseSpec::Object(ObjectFilter::spell()),
    )))
    .tag(tag.clone());
    let mut replacement = crate::effects::RegisterZoneReplacementEffect::new(
        ChooseSpec::Tagged(tag),
        Some(Zone::Stack),
        Some(Zone::Graveyard),
        replacement_zone,
        crate::effects::ReplacementApplyMode::OneShot,
    );
    if let Some(placement) = placement {
        replacement = replacement.with_library_placement(placement);
    }
    describe_effect_list(&[producer, Effect::new(replacement)])
}

#[test]
fn countered_spell_replacement_preserves_destination_surfaces() {
    use ironsmith_core::ZoneReplacementLibraryPlacement::{Bottom, Top, TopOrBottom};

    assert_eq!(
        rendered_counter_replacement(Zone::Exile, None),
        "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard"
    );
    assert_eq!(
        rendered_counter_replacement(Zone::Hand, None),
        "Counter target spell. If that spell is countered this way, put it into its owner's hand instead of into that player's graveyard"
    );
    assert_eq!(
        rendered_counter_replacement(Zone::Library, Some(Top)),
        "Counter target spell. If that spell is countered this way, put it on top of its owner's library instead of into that player's graveyard"
    );
    assert_eq!(
        rendered_counter_replacement(Zone::Library, Some(Bottom)),
        "Counter target spell. If that spell is countered this way, put it on the bottom of its owner's library instead of into that player's graveyard"
    );
    assert_eq!(
        rendered_counter_replacement(Zone::Library, Some(TopOrBottom)),
        "Counter target spell. If that spell is countered this way, put that card on your choice of the top or bottom of its owner's library instead of into that player's graveyard"
    );
}
