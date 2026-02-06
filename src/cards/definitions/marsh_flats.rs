//! Card definition for Marsh Flats.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Marsh Flats card definition.
///
/// Marsh Flats
/// Land
/// {T}, Pay 1 life, Sacrifice Marsh Flats: Search your library for a Plains or Swamp card,
/// put it onto the battlefield, then shuffle.
pub fn marsh_flats() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Marsh Flats")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}, Pay 1 life, Sacrifice Marsh Flats: Search your library for a Plains or Swamp card, put it onto the battlefield, then shuffle.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_marsh_flats_basic_properties() {
        let def = marsh_flats();
        assert_eq!(def.name(), "Marsh Flats");
        assert!(def.card.is_land());
        assert_eq!(def.card.mana_value(), 0);
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_marsh_flats_has_activated_ability() {
        let def = marsh_flats();
        assert_eq!(def.abilities.len(), 1);
        assert!(matches!(&def.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[test]
    fn test_marsh_flats_ability_costs() {
        let def = marsh_flats();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            assert!(activated.has_tap_cost());
            assert_eq!(activated.life_cost_amount(), Some(1));
            assert!(activated.has_sacrifice_self_cost());
        }
    }

    #[test]
    fn test_marsh_flats_search_filter() {
        let def = marsh_flats();
        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            let debug_str = format!("{:?}", activated.effects[0]);
            assert!(debug_str.contains("Plains"));
            assert!(debug_str.contains("Swamp"));
        }
    }

    #[test]
    fn test_marsh_flats_not_mana_ability() {
        let def = marsh_flats();
        assert!(!def.abilities[0].is_mana_ability());
    }
}
