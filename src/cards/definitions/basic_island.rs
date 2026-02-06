//! Island basic land card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::{CardType, Subtype, Supertype};

/// Island - Basic Land — Island
pub fn basic_island() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Island")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Island])
        .parse_text("{T}: Add {U}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_island() {
        let def = basic_island();
        assert!(def.card.is_land());
        assert!(def.card.has_supertype(Supertype::Basic));
        assert!(def.abilities.iter().any(|a| a.is_mana_ability()));
    }
}
