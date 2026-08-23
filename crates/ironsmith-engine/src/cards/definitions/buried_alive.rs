//! Buried Alive card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Buried Alive
/// {2}{B}
/// Sorcery
/// Search your library for up to three creature cards, put them into your graveyard, then shuffle.
pub fn buried_alive() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Buried Alive")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library for up to three creature cards, put them into your graveyard, then shuffle.",
        )
        .expect("Card text should be supported")
}
