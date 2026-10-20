use super::*;

fn render(text: &str, name: &str, types: Vec<CardType>) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(types)
        .parse_text(text)
        .expect("cast-linked exile replacement should compile");
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn cast_result_replacement_retains_the_exact_spell_reference() {
    let text = "Target opponent mills nine cards, then you may cast an instant or sorcery spell from that player's graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead.";
    let rendered = render(text, "Cast result probe", vec![CardType::Sorcery]);
    assert!(rendered.contains("If that spell would be put into a graveyard, exile it instead"), "{rendered}");
    assert!(!rendered.contains("If you do, if "), "{rendered}");
    assert!(!rendered.contains("the next time spell"), "{rendered}");
}

#[test]
fn cast_from_your_graveyard_replacement_preserves_its_cast_path() {
    for (text, types) in [
        ("Whenever you cast an instant or sorcery spell from your hand, you may cast up to one target card of the other type from your graveyard. If a spell cast from your graveyard this way would be put into your graveyard, exile it instead.", vec![CardType::Creature]),
        ("Whenever this creature attacks, mill four cards. You may cast an instant or sorcery spell from your graveyard with mana value 4 or less without paying its mana cost. If that spell would be put into your graveyard, exile it instead.", vec![CardType::Creature]),
    ] {
        let rendered = render(text, "Graveyard cast result probe", types);
        assert!(rendered.contains("would be put into your graveyard, exile it instead"), "{rendered}");
        assert!(!rendered.contains("If you do, if "), "{rendered}");
        assert!(!rendered.contains("the next time spell"), "{rendered}");
    }
}

#[test]
fn replacement_for_chosen_spells_retains_the_collection_and_owner() {
    let text = "You may cast up to two instant and/or sorcery spells with total mana value 6 or less from your graveyard and/or hand without paying their mana costs. If those spells would be put into your graveyard, exile them instead. Exile this spell.";
    let rendered = render(text, "Chosen spells probe", vec![CardType::Instant]);
    assert!(rendered.contains("If those spells would be put into your graveyard, exile them instead"), "{rendered}");
    assert!(!rendered.contains("Then if those spells"), "{rendered}");
    assert!(!rendered.contains("a spell or ability you own"), "{rendered}");
}

#[test]
fn untagged_replacement_does_not_claim_to_follow_a_particular_cast() {
    let replacement = crate::effects::RegisterFutureZoneReplacementEffect::new(
        ObjectFilter::spell(),
        Some(crate::zone::Zone::Stack),
        Some(crate::zone::Zone::Graveyard),
        crate::zone::Zone::Exile,
        crate::effects::ReplacementApplyMode::OneShot,
    );
    let rendered = describe_effect(&Effect::new(replacement));
    assert!(!rendered.contains("that spell"), "{rendered}");
    assert!(rendered.contains("graveyard"), "{rendered}");
}
