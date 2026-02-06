//! Doom Blade card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Doom Blade - {1}{B}
/// Instant
/// Destroy target nonblack creature.
pub fn doom_blade() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Doom Blade")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Instant])
        // Note: Using ChooseSpec::target() to indicate this is a TARGET (can fizzle, checks hexproof)
        .parse_text("Destroy target nonblack creature.")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_doom_blade() {
        let def = doom_blade();
        assert_eq!(def.name(), "Doom Blade");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 2);
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Doom Blade destroying a nonblack creature.
    ///
    /// Doom Blade: {1}{B} instant
    /// Destroy target nonblack creature.
    ///
    /// Scenario: Alice casts Doom Blade targeting Bob's Grizzly Bears (green creature).
    /// Expected: Grizzly Bears is destroyed and goes to Bob's graveyard.
    #[test]
    fn test_replay_doom_blade_destroys_creature() {
        let game = run_replay_test(
            vec![
                "1", // Cast Doom Blade
                "0", // Target Grizzly Bears (Bob's creature)
                "0", // Tap first Swamp for mana
                "0", // Tap second Swamp for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Doom Blade"])
                .p1_battlefield(vec!["Swamp", "Swamp"])
                .p2_battlefield(vec!["Grizzly Bears"]),
        );

        let bob = PlayerId::from_index(1);

        // Grizzly Bears should NOT be on battlefield anymore
        assert!(
            !game.battlefield_has("Grizzly Bears"),
            "Grizzly Bears should have been destroyed"
        );

        // Grizzly Bears should be in Bob's graveyard
        let bob_player = game.player(bob).unwrap();
        let bears_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(bears_in_gy, "Grizzly Bears should be in Bob's graveyard");

        // Doom Blade should be in Alice's graveyard (after resolving)
        let alice = PlayerId::from_index(0);
        let alice_player = game.player(alice).unwrap();
        let blade_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Doom Blade")
                .unwrap_or(false)
        });
        assert!(blade_in_gy, "Doom Blade should be in Alice's graveyard");
    }
}
