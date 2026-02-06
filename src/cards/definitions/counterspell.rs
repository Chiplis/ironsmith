//! Counterspell card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Counterspell - {U}{U}
/// Instant
/// Counter target spell.
pub fn counterspell() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Counterspell")
        .parse_text(
            "Mana cost: {U}{U}\n\
             Type: Instant\n\
             Counter target spell.",
        )
        .expect("Counterspell text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_counterspell() {
        let def = counterspell();
        assert_eq!(def.name(), "Counterspell");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 2);
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Counterspell countering a spell on the stack.
    ///
    /// Counterspell: {U}{U} instant
    /// Counter target spell.
    ///
    /// Scenario: Alice casts Llanowar Elves, Bob responds with Counterspell.
    /// Expected: Llanowar Elves is countered, goes to Alice's graveyard instead of battlefield.
    #[test]
    fn test_replay_counterspell_counters_creature() {
        let game = run_replay_test(
            vec![
                // Alice's turn - cast creature
                "2", // Tap Forest for mana
                "1", // Cast Llanowar Elves (goes on stack)
                // Now Bob has priority to respond
                "1", // Bob: Cast Counterspell
                "0", // Bob: Target Llanowar Elves on stack
                "0", // Bob: Tap first Island for mana
                "0", // Bob: Tap second Island for mana (auto-passes handle resolution)
                     // Counterspell resolves first (LIFO), countering Llanowar Elves
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Llanowar Elves"])
                .p1_battlefield(vec!["Forest"])
                .p2_hand(vec!["Counterspell"])
                .p2_battlefield(vec!["Island", "Island"]),
        );

        // Llanowar Elves should NOT be on battlefield (was countered)
        assert!(
            !game.battlefield_has("Llanowar Elves"),
            "Llanowar Elves should have been countered"
        );

        // Llanowar Elves should be in Alice's graveyard
        let alice = PlayerId::from_index(0);
        let alice_player = game.player(alice).unwrap();
        let elves_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Llanowar Elves")
                .unwrap_or(false)
        });
        assert!(
            elves_in_gy,
            "Llanowar Elves should be in Alice's graveyard (countered)"
        );

        // Counterspell should be in Bob's graveyard
        let bob = PlayerId::from_index(1);
        let bob_player = game.player(bob).unwrap();
        let cs_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Counterspell")
                .unwrap_or(false)
        });
        assert!(cs_in_gy, "Counterspell should be in Bob's graveyard");
    }
}
