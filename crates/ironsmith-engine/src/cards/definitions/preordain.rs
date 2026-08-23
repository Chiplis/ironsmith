//! Preordain card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Preordain - {U}
/// Sorcery
/// Scry 2, then draw a card.
pub fn preordain() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Preordain")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Scry 2, then draw a card.")
        .expect("Card text should be supported")
}
