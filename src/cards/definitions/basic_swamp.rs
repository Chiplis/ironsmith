//! Swamp basic land card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::{CardType, Subtype, Supertype};

/// Swamp - Basic Land — Swamp
pub fn basic_swamp() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Swamp")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Swamp])
        .parse_text("{T}: Add {B}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_swamp() {
        let def = basic_swamp();
        assert!(def.card.is_land());
        assert!(def.card.has_supertype(Supertype::Basic));
        assert!(def.abilities.iter().any(|a| a.is_mana_ability()));
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_swamp_play() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Swamp
            ],
            ReplayTestConfig::new().p1_hand(vec!["Swamp"]),
        );

        assert!(
            game.battlefield_has("Swamp"),
            "Swamp should be on battlefield after playing"
        );
    }
}
