//! Vampiric Tutor card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Vampiric Tutor
/// {B}
/// Instant
/// Search your library for a card, then shuffle and put that card on top. You lose 2 life.
pub fn vampiric_tutor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Vampiric Tutor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Search your library for a card, then shuffle and put that card on top. You lose 2 life.",
        )
        .expect("Card text should be supported")
}
