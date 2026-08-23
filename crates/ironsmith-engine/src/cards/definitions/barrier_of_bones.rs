//! Barrier of Bones card definition.

use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Barrier of Bones - {B}
/// Creature - Skeleton Wall (0/3)
/// Defender
/// When Barrier of Bones enters the battlefield, surveil 1.
pub fn barrier_of_bones() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Barrier of Bones")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Skeleton, Subtype::Wall])
        .power_toughness(PowerToughness::fixed(0, 3))
        .parse_text("Defender\nWhen Barrier of Bones enters the battlefield, surveil 1.")
        .expect("Card text should be supported")
}
