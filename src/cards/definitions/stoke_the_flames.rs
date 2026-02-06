//! Card definition for Stoke the Flames.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Stoke the Flames card definition.
///
/// Stoke the Flames {2}{R}{R}
/// Instant
/// Convoke (Your creatures can help cast this spell. Each creature you tap while
/// casting this spell pays for {1} or one mana of that creature's color.)
/// Stoke the Flames deals 4 damage to any target.
pub fn stoke_the_flames() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Stoke the Flames")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text("Convoke\nStoke the Flames deals 4 damage to any target.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn test_stoke_the_flames() {
        let card = stoke_the_flames();
        assert_eq!(card.card.name, "Stoke the Flames");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 4); // {2}{R}{R} = 4
        assert!(card.card.card_types.contains(&CardType::Instant));

        // Check for Convoke ability
        let has_convoke = card.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::Convoke
            } else {
                false
            }
        });
        assert!(has_convoke, "Stoke the Flames should have Convoke");

        // Check spell effect (deal 4 damage)
        assert!(card.spell_effect.is_some());
        let effects = card.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);
    }
}
