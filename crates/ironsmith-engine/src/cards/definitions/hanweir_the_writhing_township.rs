//! Hanweir, the Writhing Township card definition.

use super::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::types::{CardType, Subtype, Supertype};

/// Hanweir, the Writhing Township
/// Legendary Creature — Eldrazi Ooze (7/4)
pub fn hanweir_the_writhing_township() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Hanweir, the Writhing Township")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Ooze])
        .power_toughness(PowerToughness::fixed(7, 4))
        .parse_text("Trample\nHaste")
        .expect("Hanweir, the Writhing Township text should be supported")
}
