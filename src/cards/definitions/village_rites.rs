//! Village Rites card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Village Rites - {B}
/// Instant
/// As an additional cost to cast this spell, sacrifice a creature.
/// Draw two cards.
pub fn village_rites() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Village Rites")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nDraw two cards.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_village_rites_basic_properties() {
        let def = village_rites();
        assert_eq!(def.name(), "Village Rites");
        assert!(def.is_spell());
        assert!(def.card.is_instant());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_village_rites_is_black() {
        let def = village_rites();
        assert!(def.card.colors().contains(Color::Black));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_village_rites_has_additional_costs() {
        let def = village_rites();
        let costs = def.additional_non_mana_costs();
        assert_eq!(
            costs.len(),
            2,
            "Should have 2 additional costs (choose + sacrifice)"
        );

        let debug_str_0 = format!("{:?}", &costs[0]);
        assert!(
            debug_str_0.contains("ChooseObjectsEffect"),
            "First cost should be choose"
        );

        let debug_str_1 = format!("{:?}", &costs[1]);
        assert!(
            debug_str_1.contains("SacrificeEffect"),
            "Second cost should be sacrifice"
        );
    }

    #[test]
    fn test_village_rites_has_spell_effect() {
        let def = village_rites();
        assert!(def.spell_effect.is_some());

        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);

        // Check it's a draw effect
        let debug_str = format!("{:?}", &effects[0]);
        assert!(
            debug_str.contains("DrawCardsEffect"),
            "Should have draw cards effect"
        );
    }

    #[test]
    fn test_village_rites_oracle_text() {
        let def = village_rites();
        assert!(def.card.oracle_text.contains("sacrifice a creature"));
        assert!(def.card.oracle_text.contains("Draw two cards"));
    }

    // ========================================
    // Replay Tests
    // ========================================

    // Replay coverage is intentionally deferred while replay prompt ordering for
    // non-mana costs + choose-objects is being stabilized.
}
