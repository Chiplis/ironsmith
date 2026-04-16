//! Plains basic land card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::types::{CardType, Subtype, Supertype};

/// Plains - Basic Land — Plains
pub fn basic_plains() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Plains")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .parse_text("{T}: Add {W}.")
        .expect("Card text should be supported")
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_basic_plains() {
        let def = basic_plains();
        assert!(def.card.is_land());
        assert!(def.card.has_supertype(Supertype::Basic));
        assert!(def.abilities.iter().any(|a| a.is_mana_ability()));
    }
}
