//! Stroke of Midnight card definition.

use super::CardDefinitionBuilder;
#[cfg(test)]
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;
#[cfg(test)]
use crate::{color::ColorSet, types::Subtype};

/// Creates a 1/1 white Human creature token.
#[cfg(test)]
fn human_token() -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), "Human")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .color_indicator(ColorSet::WHITE)
            .power_toughness(PowerToughness::fixed(1, 1))
            .token()
            .build(),
    )
}

/// Stroke of Midnight - {2}{W}
/// Instant
/// Destroy target nonland permanent. Its controller creates a 1/1 white Human creature token.
pub fn stroke_of_midnight() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Stroke of Midnight")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target nonland permanent. Its controller creates a 1/1 white Human creature token.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_stroke_of_midnight_basic_properties() {
        let def = stroke_of_midnight();
        assert_eq!(def.name(), "Stroke of Midnight");
        assert!(def.is_spell());
        assert!(def.card.is_instant());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_stroke_of_midnight_is_white() {
        let def = stroke_of_midnight();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_stroke_of_midnight_has_spell_effects() {
        let def = stroke_of_midnight();
        assert!(def.spell_effect.is_some());

        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 2);

        // First effect is destroy
        let debug_str = format!("{:?}", &effects[0]);
        assert!(
            debug_str.contains("Destroy"),
            "First effect should be destroy"
        );

        // Second effect is create token
        let debug_str2 = format!("{:?}", &effects[1]);
        assert!(
            debug_str2.contains("CreateToken"),
            "Second effect should create tokens"
        );
    }

    #[test]
    fn test_stroke_of_midnight_oracle_text() {
        let def = stroke_of_midnight();
        assert!(
            def.card
                .oracle_text
                .contains("Destroy target nonland permanent")
        );
        assert!(def.card.oracle_text.contains("1/1 white Human"));
    }

    // ========================================
    // Token Tests
    // ========================================

    #[test]
    fn test_human_token_properties() {
        let token = human_token();
        assert_eq!(token.name(), "Human");
        assert!(token.is_creature());
        assert!(token.card.has_subtype(Subtype::Human));
        assert!(token.card.colors().contains(Color::White));

        let pt = token.card.power_toughness.as_ref().unwrap();
        assert_eq!(pt.power.base_value(), 1);
        assert_eq!(pt.toughness.base_value(), 1);
    }
}
