//! Forest basic land card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::{CardType, Subtype, Supertype};

/// Forest - Basic Land — Forest
pub fn basic_forest() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Forest")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .parse_text("{T}: Add {G}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_forest() {
        let def = basic_forest();
        assert!(def.card.is_land());
        assert!(def.card.has_supertype(Supertype::Basic));
        assert!(def.abilities.iter().any(|a| a.is_mana_ability()));
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_forest_play() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Forest
            ],
            ReplayTestConfig::new().p1_hand(vec!["Forest"]),
        );

        assert!(
            game.battlefield_has("Forest"),
            "Forest should be on battlefield after playing"
        );
    }
}
