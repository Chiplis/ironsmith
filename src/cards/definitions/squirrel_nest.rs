//! Card definition for Squirrel Nest.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Squirrel Nest {1}{G}{G}
/// Enchantment — Aura
/// Enchant land
/// Enchanted land has "{T}: Create a 1/1 green Squirrel creature token."
pub fn squirrel_nest() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Squirrel Nest")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant land\nEnchanted land has \"{T}: Create a 1/1 green Squirrel creature token.\"",
        )
        .expect("Card text should be supported")
}
