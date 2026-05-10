//! Black Lotus card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::ManaCost;
use crate::types::CardType;

/// Black Lotus - {0}
/// Artifact
/// {T}, Sacrifice Black Lotus: Add three mana of any one color.
pub fn black_lotus() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Black Lotus")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}, Sacrifice this artifact: Add three mana of any one color.")
        .expect("Black Lotus text should be supported")
}
