//! Ugin's Insight card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::effect::{Effect, Value};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ObjectFilter;
use crate::types::CardType;

/// Ugin's Insight - {3}{U}{U}
/// Sorcery
/// Scry X, where X is the greatest mana value among permanents you control,
/// then draw three cards.
pub fn ugins_insight() -> CardDefinition {
    let count = Value::GreatestManaValue(ObjectFilter::permanent().you_control());

    CardDefinitionBuilder::new(CardId::new(), "Ugin's Insight")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::scry(count), Effect::draw(3)])
        .oracle_text("Scry X, where X is the greatest mana value among permanents you control, then draw three cards.")
        .build()
}
