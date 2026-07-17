use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::static_abilities::StaticAbilityId;
use ironsmith_compiler::types::CardType;

#[test]
fn u036_space_sculptor_lowers_to_typed_keyword() {
    for text in [
        "Space sculptor",
        "Space sculptor (Space Beleren divides the battlefield into alpha, beta, and gamma sectors. If a creature isn't assigned to a sector, its controller assigns it to one. Opponents assign first.)",
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Sector Probe")
            .card_types(vec![CardType::Planeswalker])
            .parse_text(text)
            .unwrap_or_else(|error| panic!("failed to compile {text:?}: {error}"));
        assert!(
            definition.abilities.iter().any(|ability| matches!(
                &ability.kind,
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::SpaceSculptor
            )),
            "{:#?}",
            definition.abilities
        );
    }
}
