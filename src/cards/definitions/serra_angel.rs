//! Serra Angel card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Serra Angel - {3}{W}{W}
/// Creature — Angel (4/4)
/// Flying, vigilance
pub fn serra_angel() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Serra Angel")
        .parse_text(
            "Mana cost: {3}{W}{W}\n\
             Type: Creature — Angel\n\
             Power/Toughness: 4/4\n\
             Flying, vigilance",
        )
        .expect("Serra Angel text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_serra_angel() {
        let def = serra_angel();
        assert_eq!(def.name(), "Serra Angel");
        assert!(def.is_creature());
        assert_eq!(def.card.mana_value(), 5);
        assert_eq!(def.abilities.len(), 2);
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Serra Angel (5 mana creature).
    ///
    /// Serra Angel: {3}{W}{W} creature 4/4
    /// Flying, vigilance
    #[test]
    fn test_replay_serra_angel_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Serra Angel
                "0", // Tap Plains 1
                "0", // Tap Plains 2
                "0", // Tap Plains 3
                "0", // Tap Plains 4
                "0", // Tap Plains 5 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Serra Angel"])
                .p1_battlefield(vec!["Plains", "Plains", "Plains", "Plains", "Plains"]),
        );

        // Serra Angel should be on the battlefield
        assert!(
            game.battlefield_has("Serra Angel"),
            "Serra Angel should be on battlefield after casting"
        );
    }
}
