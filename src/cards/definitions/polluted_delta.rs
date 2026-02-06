//! Card definition for Polluted Delta.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Polluted Delta card definition.
///
/// Polluted Delta
/// Land
/// {T}, Pay 1 life, Sacrifice Polluted Delta: Search your library for an Island or Swamp card,
/// put it onto the battlefield, then shuffle.
pub fn polluted_delta() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Polluted Delta")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}, Pay 1 life, Sacrifice Polluted Delta: Search your library for an Island or Swamp card, put it onto the battlefield, then shuffle.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_polluted_delta_basic_properties() {
        let def = polluted_delta();
        assert_eq!(def.name(), "Polluted Delta");
        assert!(def.card.is_land());
        assert_eq!(def.card.mana_value(), 0);
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_polluted_delta_has_activated_ability() {
        let def = polluted_delta();
        assert_eq!(def.abilities.len(), 1);
        assert!(matches!(&def.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[test]
    fn test_polluted_delta_ability_costs() {
        let def = polluted_delta();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            // No mana cost - all costs are effect-based (tap, pay life, sacrifice)
            assert!(
                activated.mana_cost.mana_cost().is_none(),
                "Should have no mana cost"
            );

            // All costs are now in TotalCost: tap, pay life, sacrifice
            assert_eq!(
                activated.mana_cost.costs().len(),
                3,
                "Should have 3 costs: tap, pay life, sacrifice"
            );

            let debug_str = format!("{:?}", &activated.mana_cost.costs());
            assert!(debug_str.contains("TapEffect"), "Should have tap");
            assert!(debug_str.contains("LoseLifeEffect"), "Should have pay life");
            assert!(
                debug_str.contains("SacrificeTargetEffect"),
                "Should have sacrifice"
            );
        }
    }

    #[test]
    fn test_polluted_delta_search_filter() {
        let def = polluted_delta();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            let debug_str = format!("{:?}", activated.effects[0]);
            assert!(debug_str.contains("Island"));
            assert!(debug_str.contains("Swamp"));
        }
    }

    #[test]
    fn test_polluted_delta_not_mana_ability() {
        let def = polluted_delta();
        assert!(!def.abilities[0].is_mana_ability());
    }
}
