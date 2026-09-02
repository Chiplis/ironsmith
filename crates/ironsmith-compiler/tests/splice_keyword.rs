use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::static_abilities::StaticAbilityPayload;
use ironsmith_compiler::types::CardType;

#[test]
fn canonical_nonmana_splice_costs_lower_to_typed_total_costs() {
    for (line, expected_surface) in [
        (
            "Splice onto Arcane—Exile four cards from your graveyard. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "Exile four cards from your graveyard",
        ),
        (
            "Splice onto Arcane—Tap an untapped white creature you control. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "Tap an untapped white creature you control",
        ),
        (
            "Splice onto Arcane—An opponent gains 5 life. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "An opponent gains 5 life",
        ),
        (
            "Splice onto Arcane—Sacrifice two Mountains. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "Sacrifice two Mountains",
        ),
        (
            "Splice onto Arcane—Return a blue creature you control to its owner's hand. (As you cast an Arcane spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)",
            "Return a blue creature you control to its owner's hand",
        ),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Nonmana Splice Probe")
            .card_types(vec![CardType::Instant])
            .parse_text(line)
            .unwrap_or_else(|error| panic!("failed to compile {line:?}: {error}"));
        let spec = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => match &static_ability.payload {
                    StaticAbilityPayload::Splice(spec) => Some(spec),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_else(|| {
                panic!(
                    "canonical nonmana splice cost should lower to a typed payload: {line}; abilities: {:#?}",
                    definition.abilities
                )
            });
        assert!(spec.cost.has_non_mana_costs(), "line: {line}");
        assert_ne!(spec.cost.display(), "Free", "line: {line}");
        assert_eq!(spec.cost_surface.as_deref(), Some(expected_surface));
    }
}
