use super::*;

#[test]
fn typed_move_surface_preserves_put_return_and_actor_agreement() {
    let put = crate::effects::MoveToZoneEffect::new(ChooseSpec::Source, Zone::Hand, false)
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
        .with_actor_surface(PlayerFilter::Opponent);
    let put_text = describe_effect(&Effect::new(put));
    assert!(put_text.starts_with("An opponent puts "), "{put_text}");
    assert!(put_text.contains(" into "), "{put_text}");

    let returned = crate::effects::MoveToZoneEffect::new(ChooseSpec::Source, Zone::Hand, false)
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
        .with_actor_surface(PlayerFilter::Opponent);
    let return_text = describe_effect(&Effect::new(returned));
    assert!(
        return_text.starts_with("An opponent returns "),
        "{return_text}"
    );
    assert!(return_text.contains(" to "), "{return_text}");

    let you_put = crate::effects::MoveToZoneEffect::new(ChooseSpec::Source, Zone::Exile, false)
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
        .with_actor_surface(PlayerFilter::You);
    let you_put_text = describe_effect(&Effect::new(you_put));
    assert!(you_put_text.starts_with("You put "), "{you_put_text}");
    assert!(you_put_text.ends_with(" into exile"), "{you_put_text}");
}

#[test]
fn typed_move_surface_preserves_plural_tagged_sets_and_contextual_actor() {
    let tag = TagKey::from("exiled_set");
    let move_set =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Battlefield, false)
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
            .with_target_plural_surface()
            .with_actor_surface(PlayerFilter::Active)
            .with_destination_player_surface(PlayerFilter::Active)
            .with_destination_player_reference_surface(
                ironsmith_core::DestinationPlayerReferenceSurface::Pronoun,
            )
            .under_owner_control();

    let text = describe_effect(&Effect::new(move_set));
    assert!(text.starts_with("That player puts "), "{text}");
    assert!(text.contains("them onto the battlefield"), "{text}");
    assert!(text.contains("their owners' control"), "{text}");
}

#[test]
fn bulk_battlefield_move_retains_printed_put_surface() {
    let put_all = crate::effects::ReturnAllToBattlefieldEffect::new(
        ObjectFilter::creature().in_zone(Zone::Graveyard),
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
    .under_you_control();

    let text = describe_effect_list(&[Effect::new(put_all)]);
    assert!(text.starts_with("Put all creature cards"), "{text}");
    assert!(text.contains("onto the battlefield"), "{text}");
    assert!(text.ends_with("under your control"), "{text}");
}
