use super::*;

fn compile(text: &str) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Shift Probe")
        .parse_text(format!("Type: Instant\n{text}"))
        .unwrap_or_else(|error| panic!("{text}: {error:?}"))
}

/// A custom keyword expanding to "Noncreature, nonartifact spells cost {1}
/// less to cast until end of turn" after "..., then" once compiled to a
/// static cost reduction, dropping the damage and its target. The modifier
/// is a one-shot effect that follows the damage.
#[test]
fn sequenced_spell_cost_modifier_keeps_the_preceding_effect() {
    for text in [
        "This deals 3 damage to target creature, then noncreature, nonartifact spells cost {1} less to cast until end of turn.",
        "This deals 3 damage to target creature. Noncreature, nonartifact spells cost {1} less to cast until end of turn.",
    ] {
        let definition = compile(text);
        assert!(definition.abilities.is_empty(), "{text}: {:?}", definition.abilities);
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("DealDamageEffect"), "{text}: {debug}");
        assert!(debug.contains("GrantNextSpellCostReductionEffect"), "{text}: {debug}");
        assert!(debug.contains("excluded_card_types: [Creature, Artifact]"), "{text}: {debug}");
    }
}

/// With an explicit duration and no damage, the modifier is still a
/// resolving effect rather than a permanent static ability.
#[test]
fn spell_cost_modifier_with_duration_is_an_effect() {
    let definition = compile("Noncreature spells cost {1} less to cast until end of turn.");
    assert!(definition.abilities.is_empty(), "{:?}", definition.abilities);
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("GrantNextSpellCostReductionEffect"), "{debug}");
}
