//! Vampire Nighthawk card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Vampire Nighthawk - {1}{B}{B}
/// Creature — Vampire Shaman (2/3)
/// Flying, deathtouch, lifelink
pub fn vampire_nighthawk() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Vampire Nighthawk")
        .parse_text(
            "Mana cost: {1}{B}{B}\n\
             Type: Creature — Vampire Shaman\n\
             Power/Toughness: 2/3\n\
             Flying, deathtouch, lifelink",
        )
        .expect("Vampire Nighthawk text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_vampire_nighthawk() {
        let def = vampire_nighthawk();
        assert_eq!(def.name(), "Vampire Nighthawk");
        assert_eq!(def.abilities.len(), 3);
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Vampire Nighthawk (creature with flying, deathtouch, lifelink).
    ///
    /// Vampire Nighthawk: {1}{B}{B} creature 2/3
    /// Flying, deathtouch, lifelink
    #[test]
    fn test_replay_vampire_nighthawk_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Vampire Nighthawk
                "0", // Tap first Swamp
                "0", // Tap second Swamp
                "0", // Tap third Swamp (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Vampire Nighthawk"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp"]),
        );

        // Vampire Nighthawk should be on the battlefield
        assert!(
            game.battlefield_has("Vampire Nighthawk"),
            "Vampire Nighthawk should be on battlefield after casting"
        );
    }
}
