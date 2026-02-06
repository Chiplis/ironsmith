//! Typhoid Rats card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Typhoid Rats - {B}
/// Creature — Rat (1/1)
/// Deathtouch
pub fn typhoid_rats() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Typhoid Rats")
        .parse_text(
            "Mana cost: {B}\n\
             Type: Creature — Rat\n\
             Power/Toughness: 1/1\n\
             Deathtouch",
        )
        .expect("Typhoid Rats text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_typhoid_rats() {
        let def = typhoid_rats();
        assert_eq!(def.name(), "Typhoid Rats");
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_deathtouch()
            } else {
                false
            }
        }));
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Typhoid Rats (deathtouch creature).
    ///
    /// Typhoid Rats: {B} creature 1/1
    /// Deathtouch
    #[test]
    fn test_replay_typhoid_rats_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Typhoid Rats
                "0", // Tap Swamp for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Typhoid Rats"])
                .p1_battlefield(vec!["Swamp"]),
        );

        // Typhoid Rats should be on the battlefield
        assert!(
            game.battlefield_has("Typhoid Rats"),
            "Typhoid Rats should be on battlefield after casting"
        );
    }
}
