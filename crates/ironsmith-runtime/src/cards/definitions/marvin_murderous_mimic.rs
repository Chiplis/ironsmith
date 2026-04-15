//! Card definition for Marvin, Murderous Mimic.

use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Marvin, Murderous Mimic {2}
/// Legendary Artifact Creature — Toy
/// Marvin has all activated abilities of creatures you control that don't have the same name as this creature.
/// 2/2
pub fn marvin_murderous_mimic() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Marvin, Murderous Mimic")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Toy])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Marvin has all activated abilities of creatures you control that don't have the same name as this creature.",
        )
        .expect("Card text should be supported")
}
