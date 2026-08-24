//! Island basic land card definition.

use super::CardDefinitionBuilder;
use crate::ability::Ability;
use crate::cards::CardDefinition;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::types::{CardType, Subtype, Supertype};

/// Island - Basic Land — Island
pub fn basic_island() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Island")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Island])
        .with_ability(Ability::mana(
            TotalCost::from_cost(Cost::tap()),
            vec![ManaSymbol::Blue],
        ))
        .build()
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_basic_island() {
        let def = basic_island();
        assert!(def.card.is_land());
        assert!(def.card.has_supertype(Supertype::Basic));
        assert!(def.abilities.iter().any(|a| a.is_mana_ability()));
    }
}
