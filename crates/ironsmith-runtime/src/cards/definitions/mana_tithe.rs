//! Mana Tithe card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Mana Tithe - {W}
/// Instant
/// Counter target spell unless its controller pays {1}.
pub fn mana_tithe() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mana Tithe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell unless its controller pays {1}.")
        .expect("Card text should be supported")
}
