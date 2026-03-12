//! Card definition for Everflowing Chalice.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::mana::ManaCost;
use crate::types::CardType;

/// Creates the Everflowing Chalice card definition.
///
/// Everflowing Chalice {0}
/// Artifact
/// Multikicker {2}
/// Everflowing Chalice enters the battlefield with a charge counter on it
/// for each time it was kicked.
/// {T}: Add {C} for each charge counter on Everflowing Chalice.
pub fn everflowing_chalice() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Everflowing Chalice")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Multikicker {2}\nEverflowing Chalice enters the battlefield with a charge counter on it for each time it was kicked.\n{T}: Add {C} for each charge counter on Everflowing Chalice.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_everflowing_chalice() {
        let card = everflowing_chalice();
        assert_eq!(card.card.name, "Everflowing Chalice");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 0);
        assert!(card.card.card_types.contains(&CardType::Artifact));

        // Should have multikicker
        assert_eq!(card.optional_costs.len(), 1);
        assert_eq!(card.optional_costs[0].label, "Multikicker");
        assert!(card.optional_costs[0].repeatable);

        // Should have 2 abilities: ETB replacement/static ability and mana ability
        assert_eq!(card.abilities.len(), 2);

        // First ability should model the ETB counters as a non-mana static ability.
        assert!(matches!(&card.abilities[0].kind, AbilityKind::Static(_)));

        // Second ability should be a mana ability
        assert!(card.abilities[1].is_mana_ability());
    }
}
