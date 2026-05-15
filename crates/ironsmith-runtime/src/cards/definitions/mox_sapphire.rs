//! Mox Sapphire card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::ManaCost;
use crate::types::CardType;

/// Mox Sapphire - {0}
/// Artifact
/// {T}: Add {U}.
pub fn mox_sapphire() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mox Sapphire")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Add {U}.")
        .expect("Mox Sapphire text should be supported")
}
