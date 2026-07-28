use super::*;

#[test]
fn staged_destroy_consult_collection_renders_as_one_oracle_procedure() {
    let destroyed_tag = TagKey::from("destroyed_permanents");
    let revealed_tag = TagKey::from("revealed_for_destroyed");
    let match_tag = TagKey::from("matched_for_destroyed");
    let collection_tag = TagKey::from("matched_for_destroyed__exiled_collection");

    let mut destroy_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    destroy_filter.card_types = vec![CardType::Artifact, CardType::Creature];
    let destroy = Effect::new(crate::effects::DestroyEffect::targets(
        ChooseSpec::Object(destroy_filter),
        ChoiceCount::dynamic_x(),
    ))
    .tag(destroyed_tag.clone());

    let mut consult_filter = ObjectFilter::default();
    consult_filter.card_types = vec![CardType::Artifact, CardType::Creature];
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(TagKey::from("__it__"))),
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        consult_filter,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
        revealed_tag,
        match_tag.clone(),
    ));
    let exile = Effect::move_to_zone(ChooseSpec::Tagged(match_tag), Zone::Exile, false);
    let collected_loop = Effect::for_each_tagged(destroyed_tag.clone(), vec![consult, exile])
        .tag(collection_tag.clone());
    let put_collection =
        Effect::move_to_zone(ChooseSpec::Tagged(collection_tag), Zone::Battlefield, false);
    let shuffle_players = Effect::for_each_controller_of_tagged(
        destroyed_tag,
        vec![Effect::shuffle_library_player(PlayerFilter::IteratedPlayer)],
    );

    assert_eq!(
        describe_effect_list(&[destroy, collected_loop, put_collection, shuffle_players,]),
        "Destroy X target artifacts and/or creatures. For each permanent destroyed this way, its controller reveals cards from the top of their library until an artifact or creature card is revealed and exiles that card. Those players put the exiled cards onto the battlefield, then shuffle"
    );
}
