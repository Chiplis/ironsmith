//! Card definition for Verdant Catacombs.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Verdant Catacombs card definition.
///
/// Verdant Catacombs
/// Land
/// {T}, Pay 1 life, Sacrifice Verdant Catacombs: Search your library for a Swamp or Forest card,
/// put it onto the battlefield, then shuffle.
pub fn verdant_catacombs() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Verdant Catacombs")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}, Pay 1 life, Sacrifice Verdant Catacombs: Search your library for a Swamp or Forest card, put it onto the battlefield, then shuffle.",
        )
        .unwrap()
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_verdant_catacombs_basic_properties() {
        let def = verdant_catacombs();
        assert_eq!(def.name(), "Verdant Catacombs");
        assert!(def.card.is_land());
        assert_eq!(def.card.mana_value(), 0);
        assert_eq!(def.card.colors().count(), 0);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_verdant_catacombs_has_activated_ability() {
        let def = verdant_catacombs();
        assert_eq!(def.abilities.len(), 1);
        assert!(matches!(&def.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_verdant_catacombs_ability_costs() {
        let def = verdant_catacombs();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            assert!(activated.has_tap_cost());
            assert_eq!(activated.life_cost_amount(), Some(1));
            assert!(activated.has_sacrifice_self_cost());
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_verdant_catacombs_search_filter() {
        let def = verdant_catacombs();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            let debug_str = format!("{:?}", activated.effects[0]);
            assert!(debug_str.contains("Swamp"));
            assert!(debug_str.contains("Forest"));
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_verdant_catacombs_not_mana_ability() {
        let def = verdant_catacombs();
        assert!(!def.abilities[0].is_mana_ability());
    }
}
