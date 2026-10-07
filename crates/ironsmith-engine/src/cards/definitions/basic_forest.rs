//! Forest basic land card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::types::{CardType, Subtype, Supertype};

/// Forest - Basic Land — Forest
pub fn basic_forest() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Forest")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build()
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_basic_forest() {
        let def = basic_forest();
        assert!(def.card.is_land());
        assert!(def.card.has_supertype(Supertype::Basic));
        assert!(def.abilities.is_empty(), "CR 305.6 mana belongs to the current type, not printed text");
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[cfg(ironsmith_runtime_parser_tests)]
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
