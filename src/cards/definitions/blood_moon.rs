//! Card definition for Blood Moon.

use crate::ability::Ability;
use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::types::CardType;

/// Creates the Blood Moon card definition.
///
/// Blood Moon {2}{R}
/// Enchantment
/// Nonbasic lands are Mountains.
///
/// Blood Moon applies in two layers:
/// - Layer 4: Changes land subtypes to Mountain
/// - Layer 6: Removes all abilities (Mountains have intrinsic mana ability)
pub fn blood_moon() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Blood Moon")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::blood_moon()))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_blood_moon() {
        let card = blood_moon();
        assert_eq!(card.card.name, "Blood Moon");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 3); // 2R = 3
        assert!(card.card.card_types.contains(&CardType::Enchantment));

        // Should have one ability: BloodMoon
        assert_eq!(card.abilities.len(), 1);

        let ability = &card.abilities[0];
        if let AbilityKind::Static(s) = &ability.kind {
            assert_eq!(s.id(), crate::static_abilities::StaticAbilityId::BloodMoon);
        } else {
            panic!("Expected static ability");
        }
    }
}
