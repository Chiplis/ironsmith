//! Card definition for Windswept Heath.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Windswept Heath card definition.
///
/// Windswept Heath
/// Land
/// {T}, Pay 1 life, Sacrifice Windswept Heath: Search your library for a Forest or Plains card,
/// put it onto the battlefield, then shuffle.
pub fn windswept_heath() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Windswept Heath")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}, Pay 1 life, Sacrifice Windswept Heath: Search your library for a Forest or Plains card, put it onto the battlefield, then shuffle.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_windswept_heath_basic_properties() {
        let def = windswept_heath();
        assert_eq!(def.name(), "Windswept Heath");
        assert!(def.card.is_land());
        assert_eq!(def.card.mana_value(), 0);
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_windswept_heath_has_activated_ability() {
        let def = windswept_heath();
        assert_eq!(def.abilities.len(), 1);
        assert!(matches!(&def.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[test]
    fn test_windswept_heath_ability_costs() {
        let def = windswept_heath();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            assert!(activated.has_tap_cost());
            assert_eq!(activated.life_cost_amount(), Some(1));
            assert!(activated.has_sacrifice_self_cost());
        }
    }

    #[test]
    fn test_windswept_heath_search_filter() {
        let def = windswept_heath();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            let debug_str = format!("{:?}", activated.effects[0]);
            assert!(debug_str.contains("Forest"));
            assert!(debug_str.contains("Plains"));
        }
    }

    #[test]
    fn test_windswept_heath_not_mana_ability() {
        let def = windswept_heath();
        assert!(!def.abilities[0].is_mana_ability());
    }
}
