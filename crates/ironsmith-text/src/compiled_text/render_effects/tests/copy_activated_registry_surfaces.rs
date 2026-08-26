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
