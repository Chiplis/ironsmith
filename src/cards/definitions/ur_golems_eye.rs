//! Ur-Golem's Eye card definition.
//!
//! A simple artifact with mana value 4 for testing layer system interactions.

use crate::ability::{Ability, AbilityKind, ManaAbility};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;
use crate::zone::Zone;

/// Ur-Golem's Eye - {4}
/// Artifact
/// {T}: Add {C}{C}.
pub fn ur_golems_eye() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Ur-Golem's Eye")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .from_text_with_metadata("{T}: Add {C}{C}.")
            kind: AbilityKind::Mana(ManaAbility {
                mana_cost: crate::ability::merge_cost_effects(
                    TotalCost::free(),
                    vec![Effect::tap_source()],
                ),
                mana: vec![ManaSymbol::Colorless, ManaSymbol::Colorless],
                effects: None,
                activation_condition: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some("{T}: Add {C}{C}.".to_string()),
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ur_golems_eye_basic_properties() {
        let def = ur_golems_eye();
        assert_eq!(def.name(), "Ur-Golem's Eye");
        assert_eq!(def.card.mana_value(), 4);
        assert!(def.card.card_types.contains(&CardType::Artifact));
        assert!(!def.card.card_types.contains(&CardType::Creature));
    }
}
