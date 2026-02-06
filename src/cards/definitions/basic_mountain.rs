//! Mountain basic land card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::{CardType, Subtype, Supertype};

/// Mountain - Basic Land — Mountain
pub fn basic_mountain() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mountain")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Mountain])
        .parse_text("{T}: Add {R}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_mountain() {
        let def = basic_mountain();
        assert!(def.card.is_land());
        assert!(def.card.has_supertype(Supertype::Basic));
        assert!(def.abilities.iter().any(|a| a.is_mana_ability()));
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_mountain_play() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Mountain
            ],
            ReplayTestConfig::new().p1_hand(vec!["Mountain"]),
        );

        assert!(
            game.battlefield_has("Mountain"),
            "Mountain should be on battlefield after playing"
        );
    }
}
