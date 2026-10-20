use super::*;

fn parsed_effects(oracle: &str) -> (crate::CardDefinition, Vec<Effect>) {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Searched pair")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("searched pair should compile");
    let effects = &definition.spell_effect.as_ref().unwrap().segments[0].default_effects;
    let mut flattened = Vec::new();
    if let Some(first) = effects.first()
        && let Some(sequence) = first.downcast_ref::<crate::effects::SequenceEffect>()
    {
        flattened.extend(sequence.effects.iter().cloned());
        flattened.extend(effects[1..].iter().cloned());
    } else {
        flattened.extend(effects.iter().cloned());
    }
    (definition, flattened)
}

#[test]
fn searched_revealed_pair_retains_filter_and_both_destinations() {
    for oracle in [
        "Search your library for up to two basic land cards and reveal them. Put one into your hand and the other into your graveyard. Then shuffle.",
        "Search your library for up to two creature cards and reveal them. Put one into your hand and the other into your graveyard. Then shuffle.",
    ] {
        let (definition, effects) = parsed_effects(oracle);
        let refs = effects.iter().collect::<Vec<_>>();
        assert!(
            describe_search_two_split_hand_graveyard_sequence(&refs).is_some(),
            "{effects:#?}"
        );
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            oracle
        );
    }
}

#[test]
fn searched_pair_rejects_a_complement_from_another_set() {
    let oracle = "Search your library for up to two creature cards and reveal them. Put one into your hand and the other into your graveyard. Then shuffle.";
    let (_, mut effects) = parsed_effects(oracle);
    assert_eq!(effects.len(), 7, "{effects:#?}");
    effects[3] = Effect::new(crate::effects::TagMatchingObjectsEffect::new(
        ObjectFilter::default(),
        "other_set",
    ));
    let refs = effects.iter().collect::<Vec<_>>();
    assert!(describe_search_two_split_hand_graveyard_sequence(&refs).is_none());
}
