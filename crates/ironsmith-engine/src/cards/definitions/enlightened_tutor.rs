//! Enlightened Tutor card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Enlightened Tutor
/// {W}
/// Instant
/// Search your library for an artifact or enchantment card, reveal it, then shuffle and put that card on top.
pub fn enlightened_tutor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Enlightened Tutor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Search your library for an artifact or enchantment card, reveal it, then shuffle and put that card on top.",
        )
        .expect("Card text should be supported")
}
