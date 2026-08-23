use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_any_color_among_lowers_to_the_dynamic_filtered_mana_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Legendary Plaza Variant")
        .parse_text("{T}: Add one mana of any color among legendary permanents you control.")
        .expect("dynamic color-among mana ability should parse");

    let mana_ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_mana_ability() => Some(activated),
            _ => None,
        })
        .expect("expected mana ability");
    let add = mana_ability
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::AddOneManaOfAnyColorAmongEffect>())
        .expect("expected dynamic filtered any-color effect");

    assert_eq!(add.player, PlayerFilter::You);
    assert_eq!(add.filter.controller, Some(PlayerFilter::You));
    assert!(
        add.filter
            .supertypes
            .contains(&crate::types::Supertype::Legendary),
        "expected legendary filter, got {:#?}",
        add.filter
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        ["{T}: Add one mana of any color among legendary permanents you control."]
    );
}
