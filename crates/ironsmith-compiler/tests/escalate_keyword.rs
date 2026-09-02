use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::static_abilities::{StaticAbilityId, StaticAbilityPayload};
use ironsmith_compiler::types::CardType;

#[test]
fn escalate_lowers_mana_and_nonmana_costs_to_typed_payloads() {
    for (line, expected_surface, has_nonmana) in [
        (
            "Escalate {1}{R} (Pay this cost for each mode chosen beyond the first.)",
            "{1}{r}",
            false,
        ),
        (
            "Escalate—Discard a card. (Pay this cost for each mode chosen beyond the first.)",
            "discard a card",
            true,
        ),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Escalate Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(line)
            .unwrap_or_else(|error| panic!("failed to compile {line:?}: {error}"));
        let (ability, spec) = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => match &static_ability.payload {
                    StaticAbilityPayload::Escalate(spec) => Some((static_ability, spec)),
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or_else(|| panic!("Escalate should retain its typed cost: {line}"));

        assert_eq!(ability.id(), StaticAbilityId::Escalate);
        assert_eq!(spec.cost_surface.as_deref(), Some(expected_surface));
        assert_eq!(spec.cost.has_non_mana_costs(), has_nonmana);
        assert_ne!(spec.cost.display(), "Free");
    }
}
