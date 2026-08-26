use super::*;

fn chosen_type_consult_program(excluded_tag: TagKey) -> Vec<Effect> {
    let all_tag = TagKey::from("revealed_0");
    let match_tag = TagKey::from("consult_match_0");
    let mut consult_filter = ObjectFilter::creature();
    consult_filter.chosen_creature_type = true;
    let mut count_filter = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .in_zone(Zone::Battlefield);
    count_filter.chosen_creature_type = true;

    let choose_type = Effect::new(crate::effects::ChooseCreatureTypeEffect::new(
        PlayerFilter::You,
        Vec::new(),
    ));
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        consult_filter,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(
            Value::Count(count_filter).with_surface_hint(ValueSurfaceHint::WhereXIs),
        ),
        all_tag.clone(),
        match_tag.clone(),
    ));
    let move_matches = Effect::new(
        crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(match_tag),
            Zone::Battlefield,
            false,
        )
        .with_target_plural_surface(),
    );
    let remainder = ObjectFilter::tagged(all_tag).not_tagged(excluded_tag);
    let shuffle_remainder = Effect::new(crate::effects::ShuffleObjectsIntoLibraryEffect::new(
        ChooseSpec::Object(remainder),
        PlayerFilter::You,
    ));

    vec![choose_type, consult, move_matches, shuffle_remainder]
}

#[test]
fn chosen_type_counted_consult_renders_the_linked_revealed_partition() {
    let effects = chosen_type_consult_program(TagKey::from("consult_match_0"));
    assert_eq!(
        describe_effect_list(&effects),
        "Choose a creature type. Reveal cards from the top of your library until you reveal X creature cards of the chosen type, where X is the number of creatures you control of that type. Put those cards onto the battlefield, then shuffle the rest of the revealed cards into your library"
    );
}

#[test]
fn a_different_excluded_collection_does_not_claim_the_revealed_complement_surface() {
    let effects = chosen_type_consult_program(TagKey::from("different_collection"));
    assert_ne!(
        describe_effect_list(&effects),
        "Choose a creature type. Reveal cards from the top of your library until you reveal X creature cards of the chosen type, where X is the number of creatures you control of that type. Put those cards onto the battlefield, then shuffle the rest of the revealed cards into your library"
    );
}
