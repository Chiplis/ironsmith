//! Serum Powder card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Serum Powder card definition.
pub fn serum_powder() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Serum Powder")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Any time you could mulligan and Serum Powder is in your hand, you may exile all the cards from your hand, then draw that many cards. (You can do this in addition to taking mulligans.)\n{T}: Add {C}.",
        )
        .expect("Card text should be supported")
}
