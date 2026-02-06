//! Card definition for Treasure Cruise.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Treasure Cruise card definition.
///
/// Treasure Cruise {7}{U}
/// Sorcery
/// Delve (Each card you exile from your graveyard while casting this spell pays for {1}.)
/// Draw three cards.
pub fn treasure_cruise() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Treasure Cruise")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(7)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .delve()
        .with_spell_effect(vec![Effect::draw(3)])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn test_treasure_cruise() {
        let card = treasure_cruise();
        assert_eq!(card.card.name, "Treasure Cruise");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 8); // {7}{U} = 8
        assert!(card.card.card_types.contains(&CardType::Sorcery));

        // Check for Delve ability
        let has_delve = card.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::Delve
            } else {
                false
            }
        });
        assert!(has_delve, "Treasure Cruise should have Delve");

        // Check spell effect (draw 3)
        assert!(card.spell_effect.is_some());
        let effects = card.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);
    }
}
