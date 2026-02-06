//! Card definition for Humility.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Humility card definition.
///
/// Humility {2}{W}{W}
/// Enchantment
/// All creatures lose all abilities and have base power and toughness 1/1.
///
/// Humility applies in two layers:
/// - Layer 6: Removes all abilities from creatures
/// - Layer 7b: Sets power and toughness to 1/1
pub fn humility() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Humility")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text("All creatures lose all abilities and have base power and toughness 1/1.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn test_humility() {
        let card = humility();
        assert_eq!(card.card.name, "Humility");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 4); // 2WW = 4
        assert!(card.card.card_types.contains(&CardType::Enchantment));

        // Parsed into two static abilities:
        // 1) remove all abilities
        // 2) set base power/toughness
        assert_eq!(card.abilities.len(), 2);

        let static_ids: Vec<_> = card
            .abilities
            .iter()
            .filter_map(|ability| {
                if let AbilityKind::Static(s) = &ability.kind {
                    Some(s.id())
                } else {
                    None
                }
            })
            .collect();

        assert!(
            static_ids.contains(&StaticAbilityId::RemoveAllAbilitiesForFilter),
            "Expected remove-all-abilities static ability"
        );
        assert!(
            static_ids.contains(&StaticAbilityId::SetBasePowerToughnessForFilter),
            "Expected base P/T setting static ability"
        );
    }
}
