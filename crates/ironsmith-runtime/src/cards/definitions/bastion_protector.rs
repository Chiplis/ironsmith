//! Bastion Protector card definition.

use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Bastion Protector - {2}{W}
/// Creature — Human Soldier
/// 3/3
/// Commander creatures you control get +2/+2 and have indestructible.
pub fn bastion_protector() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Bastion Protector")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Commander creatures you control get +2/+2 and have indestructible.")
        .unwrap()
}
