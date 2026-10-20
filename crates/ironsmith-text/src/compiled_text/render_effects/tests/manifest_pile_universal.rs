use super::*;

#[test]
fn universal_face_down_pile_preserves_all_objects_and_their_provenance() {
    for action in ["manifest", "cloak"] {
        let oracle = format!("Exile all creature cards from target player's graveyard in a face-down pile, shuffle that pile, then {action} those cards.");
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Pile probe")
            .card_types(vec![CardType::Sorcery]).parse_text(&oracle).unwrap();
        let program = format!("{:?}", definition.spell_effect);
        assert!(program.contains("spec: All("), "{program}");
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered, oracle);
    }
}

#[test]
fn pile_compaction_requires_the_same_collection_and_public_exile_semantics() {
    let tag = TagKey::from("pile");
    let exile = crate::effects::ExileEffect::with_spec(ChooseSpec::All(
        ObjectFilter::creature().in_zone(Zone::Graveyard),
    )).with_face_down(true);
    let producer = Effect::new(crate::effects::TaggedEffect::new(tag.clone(), Effect::new(exile.clone())));
    let mut manifest = crate::effects::ManifestObjectsEffect::new(
        ChooseSpec::Tagged(tag), PlayerFilter::You,
    );
    manifest.shuffle = true;
    assert!(effect_lists::describe_face_down_pile_then_manifest(&[
        producer.clone(), Effect::new(manifest.clone()),
    ]).is_some());
    manifest.target = ChooseSpec::Tagged(TagKey::from("unrelated"));
    assert!(effect_lists::describe_face_down_pile_then_manifest(&[
        producer, Effect::new(manifest),
    ]).is_none());
}
