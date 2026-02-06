//! Marneus Calgar card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Marneus Calgar - {2}{W}{U}{B}
/// Legendary Creature — Astartes Warrior
/// 3/5
/// Double strike
/// Whenever one or more tokens enter the battlefield under your control, draw a card.
/// {6}: Create two 2/2 white Astartes Warrior creature tokens with vigilance.
pub fn marneus_calgar() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Marneus Calgar")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Astartes, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(3, 5))
        .parse_text(
            "Double strike\n\
             Whenever one or more tokens enter the battlefield under your control, draw a card.\n\
             {6}: Create two 2/2 white Astartes Warrior creature tokens with vigilance.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_marneus_basic_properties() {
        let def = marneus_calgar();
        assert_eq!(def.name(), "Marneus Calgar");
        assert!(def.is_creature());
        assert!(def.card.has_supertype(Supertype::Legendary));
        assert!(def.card.has_subtype(Subtype::Astartes));
        assert!(def.card.has_subtype(Subtype::Warrior));
        assert_eq!(def.card.mana_value(), 5);

        // Double strike (static), triggered draw, activated token creation
        let static_count = def
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Static(_)))
            .count();
        assert_eq!(static_count, 1);

        let triggered_count = def
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .count();
        assert_eq!(triggered_count, 1);

        let activated_count = def
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .count();
        assert_eq!(activated_count, 1);
    }
}
