//! Walker token definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::color::ColorSet;
use crate::ids::CardId;
use crate::card::PowerToughness;
use crate::types::{CardType, Subtype};

/// Creates a Walker token.
///
/// A Walker is a 2/2 black Zombie creature token.
pub fn walker_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Walker")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .color_indicator(ColorSet::BLACK)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[cfg(test)]
mod tests {
    use super::walker_token_definition;
    use crate::card::PowerToughness;
    use crate::types::{CardType, Subtype};

    #[test]
    fn walker_token_has_expected_base_characteristics() {
        let walker = walker_token_definition();
        assert!(walker.card.is_token);
        assert!(walker.card.card_types.contains(&CardType::Creature));
        assert!(walker.card.subtypes.contains(&Subtype::Zombie));
        assert_eq!(walker.card.power_toughness, Some(PowerToughness::fixed(2, 2)));
    }
}
