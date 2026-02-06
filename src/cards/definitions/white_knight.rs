//! White Knight card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// White Knight - {W}{W}
/// Creature — Human Knight (2/2)
/// First strike, protection from black
pub fn white_knight() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "White Knight")
        .parse_text(
            "Mana cost: {W}{W}\n\
             Type: Creature — Human Knight\n\
             Power/Toughness: 2/2\n\
             First strike, protection from black",
        )
        .expect("White Knight text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_white_knight() {
        let def = white_knight();
        assert_eq!(def.name(), "White Knight");
        assert_eq!(def.abilities.len(), 2);
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting White Knight (creature with first strike and protection from black).
    ///
    /// White Knight: {W}{W} creature 2/2
    /// First strike, protection from black
    #[test]
    fn test_replay_white_knight_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast White Knight
                "0", // Tap first Plains
                "0", // Tap second Plains (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["White Knight"])
                .p1_battlefield(vec!["Plains", "Plains"]),
        );

        // White Knight should be on the battlefield
        assert!(
            game.battlefield_has("White Knight"),
            "White Knight should be on battlefield after casting"
        );
    }
}
