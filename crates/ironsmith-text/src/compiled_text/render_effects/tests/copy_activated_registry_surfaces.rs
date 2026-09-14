use super::*;

#[test]
fn battlefield_conditioned_copy_activated_lines_stay_static() {
    for (name, card_type, borrowed_type) in [
        ("Mirran Safehouse", CardType::Artifact, CardType::Land),
        ("Necrotic Ooze", CardType::Creature, CardType::Creature),
    ] {
        let subject = card_type.name().to_ascii_lowercase();
        let borrowed = borrowed_type.name().to_ascii_lowercase();
        let oracle = format!(
            "As long as this {subject} is on the battlefield, it has all activated abilities of all {borrowed} cards in all graveyards."
        );
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
            .card_types(vec![card_type])
            .parse_text(&oracle)
            .expect("the static copy-ability registry should own the `as` head");
        assert!(definition.spell_effect.is_none(), "{name}: {definition:#?}");
        assert_eq!(definition.abilities.len(), 1, "{name}: {definition:#?}");
        assert!(
            format!("{:#?}", definition.abilities).contains("CopyActivatedAbilities"),
            "{name}: {:#?}",
            definition.abilities
        );
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(
            rendered.starts_with("As long as this "),
            "{name}: {rendered}"
        );
        assert!(!rendered.contains("It gains"), "{name}: {rendered}");
    }
}

#[test]
fn unrelated_as_long_as_ability_does_not_enter_the_copy_ability_family() {
    let oracle = "As long as this artifact is on the battlefield, it has flying.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy Ability Near Miss")
            .card_types(vec![CardType::Artifact])
            .parse_text(oracle)
            .expect("ordinary conditional keyword grant should still compile");
    assert!(
        !format!("{definition:#?}").contains("CopyActivatedAbilities"),
        "the lexical head alone must not claim an unrelated static line: {definition:#?}"
    );
}

#[test]
fn granted_copy_activated_lines_read_as_one_inheritance_predicate() {
    // Agatha's Soul Cauldron and Hazel's Brewmaster grant the inheritance to a
    // filtered set rather than to the source, and no lexical head enumerates
    // those subjects.
    for (name, subject) in [
        (
            "Counter Cauldron",
            "Creatures you control with +1/+1 counters on them",
        ),
        ("Brewmaster", "Foods you control"),
    ] {
        let oracle = format!(
            "{subject} have all activated abilities of all creature cards exiled with this artifact."
        );
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .parse_text(&oracle)
            .expect("the static copy-ability registry should own an open-ended grant subject");
        assert!(
            format!("{:#?}", definition.abilities).contains("CopyActivatedAbilities"),
            "{name}: {:#?}",
            definition.abilities
        );
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered.trim_end_matches('.'), oracle.trim_end_matches('.'));
        // Ability inheritance is a predicate over the recipients, not quoted
        // ability text they gain.
        assert!(!rendered.contains('"'), "{name}: {rendered}");
    }
}
