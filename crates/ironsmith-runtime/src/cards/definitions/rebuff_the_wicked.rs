//! Rebuff the Wicked card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Rebuff the Wicked - {W}
/// Instant
/// Counter target spell that targets a permanent you control.
pub fn rebuff_the_wicked() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Rebuff the Wicked")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell that targets a permanent you control.")
        .unwrap()
}
