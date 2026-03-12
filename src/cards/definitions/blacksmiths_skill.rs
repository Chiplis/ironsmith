//! Blacksmith's Skill card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Blacksmith's Skill - {W}
/// Instant
/// Target permanent gains hexproof and indestructible until end of turn.
/// If it's an artifact creature, it gets +2/+2 until end of turn.
pub fn blacksmiths_skill() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Blacksmith's Skill")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target permanent gains hexproof and indestructible until end of turn. \
             If it's an artifact creature, it gets +2/+2 until end of turn.",
        )
        .expect("Card text should be supported")
}
