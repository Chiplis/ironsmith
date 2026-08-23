//! Card definition for Mind Bend.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Mind Bend {U}
/// Instant
/// Change the text of target spell or permanent by replacing all instances of one color word
/// with another or one basic land type with another. (This effect lasts indefinitely.)
pub fn mind_bend() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mind Bend")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Change the text of target spell or permanent by replacing all instances of one color \
             word with another or one basic land type with another. (This effect lasts indefinitely.)",
        )
        .expect("Card text should be supported")
}
