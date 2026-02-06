//! Wall of Roots card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Wall of Roots - {1}{G}
/// Creature — Plant Wall (0/5)
/// Defender
/// Put a -0/-1 counter on Wall of Roots: Add {G}. Activate only once each turn.
pub fn wall_of_roots() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Wall of Roots")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Plant, Subtype::Wall])
        .power_toughness(PowerToughness::fixed(0, 5))
        .parse_text("Defender\nPut a -0/-1 counter on Wall of Roots: Add {G}. Activate only once each turn.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_wall_of_roots_basic_properties() {
        let def = wall_of_roots();
        assert_eq!(def.name(), "Wall of Roots");
        assert_eq!(def.card.mana_value(), 2);
        assert!(def.card.card_types.contains(&CardType::Creature));
        assert!(def.card.subtypes.contains(&Subtype::Plant));
        assert!(def.card.subtypes.contains(&Subtype::Wall));
    }

    #[test]
    fn test_wall_of_roots_has_defender_and_mana_ability() {
        let def = wall_of_roots();
        assert!(
            def.abilities
                .iter()
                .any(|ability| ability.text.as_deref() == Some("Defender"))
        );
        assert!(
            def.abilities
                .iter()
                .any(|ability| matches!(ability.kind, AbilityKind::Mana(_)))
        );
    }
}
