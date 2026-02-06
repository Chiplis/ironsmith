//! Card definition for Reverse Engineer.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;

/// Creates the Reverse Engineer card definition.
///
/// Reverse Engineer {3}{U}{U}
/// Sorcery
/// Improvise (Your artifacts can help cast this spell. Each artifact you tap
/// after you're done activating mana abilities pays for {1}.)
/// Draw three cards.
pub fn reverse_engineer() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Reverse Engineer")
        .parse_text(
            "Mana cost: {3}{U}{U}\n\
             Type: Sorcery\n\
             Improvise\n\
             Draw three cards.",
        )
        .expect("Reverse Engineer text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::static_abilities::StaticAbilityId;
    use crate::types::CardType;

    #[test]
    fn test_reverse_engineer() {
        let card = reverse_engineer();
        assert_eq!(card.card.name, "Reverse Engineer");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 5); // {3}{U}{U} = 5
        assert!(card.card.card_types.contains(&CardType::Sorcery));

        // Check for Improvise ability
        let has_improvise = card.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::Improvise
            } else {
                false
            }
        });
        assert!(has_improvise, "Reverse Engineer should have Improvise");

        // Check spell effect (draw 3)
        assert!(card.spell_effect.is_some());
        let effects = card.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);
    }
}
