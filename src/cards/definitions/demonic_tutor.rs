//! Demonic Tutor card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Demonic Tutor
/// {1}{B}
/// Sorcery
/// Search your library for a card, put that card into your hand, then shuffle.
pub fn demonic_tutor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Demonic Tutor")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Search your library for a card, put that card into your hand, then shuffle.")
        .expect("Card text should be supported")
}
