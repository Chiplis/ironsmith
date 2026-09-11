use super::*;

#[test]
fn hand_reveal_search_preserves_the_matching_name_and_empty_hand_alternative() {
    let oracle = "Reveal a card from your hand. Search your library for a card with the same name as that card, reveal it, put it into your hand, then shuffle.\nHellbent — If you have no cards in hand, instead search your library for a card, put it into your hand, then shuffle.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Hand Search Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

#[test]
fn hand_choice_reveal_requires_exact_matching_selection() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Reveal Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Reveal a card from your hand.")
        .unwrap();
    let effects = definition
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects();
    assert!(describe_hand_choice_reveal_prefix(&effects).is_some());
    let choice = effects[0]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .unwrap();
    for variant in 0..6 {
        let mut changed = choice.clone();
        match variant {
            0 => changed.chooser = PlayerFilter::Opponent,
            1 => changed.filter.owner = Some(PlayerFilter::Opponent),
            2 => {
                changed.zone = Some(Zone::Graveyard);
                changed.filter.zone = Some(Zone::Graveyard);
            }
            3 => changed.count.min = 0,
            4 => changed.tag = crate::TagKey::from("different_choice"),
            _ => changed.count.random = true,
        }
        let altered = vec![Effect::new(changed), effects[1].clone()];
        assert!(
            describe_hand_choice_reveal_prefix(&altered).is_none(),
            "variant {variant}"
        );
    }
}
