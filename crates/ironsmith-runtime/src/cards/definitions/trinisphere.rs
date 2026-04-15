use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

pub fn trinisphere() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Trinisphere")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "As long as Trinisphere is untapped, each spell that would cost less than three mana to cast costs three mana to cast.",
        )
        .expect("Card text should be supported")
}
