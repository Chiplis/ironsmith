use super::*;

fn looked_cloak_program(cloak: bool, keep_tag: TagKey) -> Vec<Effect> {
    let looked = TagKey::from("looked_0");
    let selected = TagKey::from("selected_0");
    let look = Effect::new(crate::effects::LookAtTopCardsEffect::new(
        PlayerFilter::You,
        Value::Fixed(5),
        looked.clone(),
    ));
    let mut pool = ObjectFilter::tagged(looked.clone());
    pool.zone = Some(Zone::Library);
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            pool,
            ChoiceCount::exactly(2),
            PlayerFilter::You,
            selected.clone(),
        )
        .in_zone(Zone::Library),
    );
    let mut manifest =
        crate::effects::ManifestObjectsEffect::new(ChooseSpec::Tagged(selected), PlayerFilter::You);
    if cloak {
        manifest = manifest.cloak();
    }
    let remainder = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            looked,
            Some(keep_tag),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );
    vec![look, choose, Effect::new(manifest), remainder]
}

#[test]
fn exact_looked_cloak_partition_uses_the_authored_compact_surface() {
    let effects = looked_cloak_program(true, TagKey::from("selected_0"));
    assert_eq!(
        describe_effect_list(&effects),
        "Look at the top five cards of your library, cloak two of them, and put the rest on the bottom of your library in a random order"
    );
}

#[test]
fn manifest_or_wrong_complement_tag_does_not_claim_the_cloak_partition() {
    let expected = "Look at the top five cards of your library, cloak two of them, and put the rest on the bottom of your library in a random order";
    assert_ne!(
        describe_effect_list(&looked_cloak_program(false, TagKey::from("selected_0"))),
        expected
    );
    assert_ne!(
        describe_effect_list(&looked_cloak_program(
            true,
            TagKey::from("different_selection")
        )),
        expected
    );
}
