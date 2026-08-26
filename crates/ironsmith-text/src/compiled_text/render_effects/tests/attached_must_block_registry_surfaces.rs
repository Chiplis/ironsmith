use super::*;

#[test]
fn persistent_attached_must_block_line_stays_static() {
    let oracle = "Enchant creature\nAll creatures able to block enchanted creature do so.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Lure")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(oracle)
        .expect("the attached must-block registry should own the `all creatures` head");

    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("MustBlockSpecificAttacker"), "{debug}");
    assert!(debug.contains("AttachedAbilityGrant"), "{debug}");
    assert!(!debug.contains("EndOfTurn"), "{debug}");
    let spell_debug = format!("{:#?}", definition.spell_effect);
    assert!(spell_debug.contains("AttachToEffect"), "{spell_debug}");
    assert!(
        !spell_debug.contains("MustBlockSpecificAttacker"),
        "{spell_debug}"
    );
    assert!(!spell_debug.contains("EndOfTurn"), "{spell_debug}");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

#[test]
fn target_creature_must_block_line_does_not_enter_the_attached_static_family() {
    let oracle = "All creatures able to block target creature do so.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Must Block Near Miss")
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .expect("the target-creature near miss should remain a spell effect");

    assert!(definition.spell_effect.is_some(), "{definition:#?}");
    assert!(
        !format!("{:#?}", definition.abilities).contains("AttachedAbilityGrant"),
        "{definition:#?}"
    );
}
