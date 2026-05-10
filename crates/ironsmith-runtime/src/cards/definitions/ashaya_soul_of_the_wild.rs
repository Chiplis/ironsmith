//! Ashaya, Soul of the Wild card definition.

use super::CardDefinitionBuilder;
use crate::card::{PowerToughness, PtValue};
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Ashaya, Soul of the Wild - {3}{G}{G}
/// Legendary Creature — Elemental
/// Ashaya's power and toughness are each equal to the number of lands you control.
/// Nontoken creatures you control are Forest lands in addition to their other types.
pub fn ashaya_soul_of_the_wild() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Ashaya, Soul of the Wild")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental])
        .power_toughness(PowerToughness::new(PtValue::Star, PtValue::Star))
        .parse_text(
            "Ashaya's power and toughness are each equal to the number of lands you control.\nNontoken creatures you control are Forest lands in addition to their other types.",
        )
        .expect("Ashaya text should be supported")
}
