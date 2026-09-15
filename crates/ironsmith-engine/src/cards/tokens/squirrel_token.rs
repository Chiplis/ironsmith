//! Squirrel token definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::color::ColorSet;
use crate::ids::CardId;
use crate::types::{CardType, Subtype};

/// Creates a Squirrel token.
///
/// A Squirrel is a 1/1 green Squirrel creature token.
pub fn squirrel_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Squirrel")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Squirrel])
        .color_indicator(ColorSet::GREEN)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}
