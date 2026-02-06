//! Ornithopter card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Ornithopter - {0}
/// Artifact Creature — Thopter (0/2)
/// Flying
pub fn ornithopter() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Ornithopter")
        .parse_text(
            "Mana cost: {0}\n\
             Type: Artifact Creature — Thopter\n\
             Power/Toughness: 0/2\n\
             Flying",
        )
        .expect("Ornithopter text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_ornithopter() {
        let def = ornithopter();
        assert_eq!(def.name(), "Ornithopter");
        assert_eq!(def.card.mana_value(), 0);
        assert!(def.is_creature());
        assert_eq!(def.abilities.len(), 1);
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Ornithopter for 0 mana.
    ///
    /// Ornithopter: {0} artifact creature
    /// This test verifies that 0-cost spells can be cast without tapping mana.
    #[test]
    fn test_replay_ornithopter_zero_cost() {
        let game = run_replay_test(
            vec![
                "1", // Cast Ornithopter (no mana needed)
                "",  // Pass priority
                "",  // Opponent passes (Ornithopter resolves)
            ],
            ReplayTestConfig::new().p1_hand(vec!["Ornithopter"]),
        );

        // Ornithopter should be on the battlefield
        assert!(
            game.battlefield_has("Ornithopter"),
            "Ornithopter should be on battlefield after casting for 0"
        );
    }
}
