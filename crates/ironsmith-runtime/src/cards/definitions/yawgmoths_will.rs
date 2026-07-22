//! Yawgmoth's Will card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;

/// Creates the Yawgmoth's Will card definition.
///
/// Yawgmoth's Will {2}{B}
/// Sorcery
/// Until end of turn, you may play lands and cast spells from your graveyard.
/// If a card would be put into your graveyard from anywhere this turn, exile that card instead.
pub fn yawgmoths_will() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Yawgmoth's Will")
        .parse_text(
            "Mana cost: {2}{B}\n\
             Type: Sorcery\n\
             Until end of turn, you may play lands and cast spells from your graveyard. \
             If a card would be put into your graveyard from anywhere this turn, exile \
             that card instead.",
        )
        .unwrap()
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    // ========================================
    // Basic Properties Tests
    // ========================================

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_yawgmoths_will_basic_properties() {
        let def = yawgmoths_will();
        assert_eq!(def.name(), "Yawgmoth's Will");
        assert!(def.is_spell());
        assert!(def.card.is_sorcery());
        assert!(!def.card.is_instant());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_yawgmoths_will_is_black() {
        let def = yawgmoths_will();
        assert!(def.card.colors().contains(Color::Black));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_yawgmoths_will_has_spell_effect() {
        let def = yawgmoths_will();
        assert!(def.spell_effect.is_some());
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 2);
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests basic Yawgmoth's Will casting and resolution.
    ///
    /// After Yawgmoth's Will resolves, cards in graveyard should be castable.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_yawgmoths_will_basic() {
        // Simple test: cast Yawgmoth's Will and verify it resolves
        // Actions: 0=Pass, 1=Cast Yawgmoth's Will, then mana
        let game = run_replay_test(
            vec![
                "1", // Cast Yawgmoth's Will (index 1, spell comes after Pass)
                "0", // Tap first Swamp
                "0", // Tap second Swamp
                "0", // Tap third Swamp
                "",  // Pass priority
                "",  // P2 passes (Yawgmoth's Will resolves)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Yawgmoth's Will"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp"])
                .p1_graveyard(vec!["Lightning Bolt"]),
        );

        let _alice = PlayerId::from_index(0);

        // Yawgmoth's Will should be in EXILE after resolving (it exiles itself due to
        // its own replacement effect - when it would go to graveyard, it's exiled instead)
        let yw_in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Yawgmoth's Will")
                .unwrap_or(false)
        });
        assert!(
            yw_in_exile,
            "Yawgmoth's Will should be in exile after resolving (exiles itself)"
        );
    }

    /// Tests Yawgmoth's Will allowing Force of Will's alternative cost from graveyard.
    ///
    /// After Yawgmoth's Will resolves, Force of Will in graveyard can be cast using
    /// its alternative cost (pay 1 life, exile a blue card from hand).
    ///
    /// This scenario currently has complex interactions with targeting and
    /// cost_effect ordering. The "2 legal targets" at Targets[6] is still
    /// under investigation.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_yawgmoths_will_with_force_of_will() {
        // Setup: P1 casts Yawgmoth's Will, then has Force of Will available from GY
        // P2 casts Lightning Bolt, P1 responds with Force of Will from GY
        //
        // Action order in main phase with 2 cards in hand + 3 lands:
        // 0: PassPriority
        // 1: Play Island (land)
        // 2: Cast Yawgmoth's Will (spell)
        // 3,4,5: Tap Swamps (mana abilities)
        let game = run_replay_test(
            vec![
                // P1's turn: cast Yawgmoth's Will
                // Actions: 0=Pass, 1=YW (Counterspell can't be cast - no blue mana)
                "1", // Decision 0: P1 casts Yawgmoth's Will
                "0", // Decision 1: Pay {B} - tap Swamp
                "0", // Decision 2: Pay {1} - tap Swamp
                "0", // Decision 3: Pay {1} - tap the third Swamp
                // YW on stack; P1 auto-passes and P2 declines to respond.
                "", // Decision 4: P2 passes, so Yawgmoth's Will resolves
                // P1 auto-passes after resolution (Swamps tapped, Counterspell needs UU)
                // P2 gets priority and casts Lightning Bolt.
                "1", // Decision 5: P2 casts Lightning Bolt
                "0", // Decision 6: Target P1
                "0", // Decision 7: Tap Mountain
                // P1 responds with Force of Will from graveyard using alt cost
                "1", // Decision 8: P1 casts Force of Will from GY [alt cost]
                "0", // Decision 9: Target Lightning Bolt on stack
                "0", // Decision 10: Choose the exile-a-blue-card cost
                "0", // Decision 11: Choose Counterspell to exile
                "",  // Decision 12: P1 passes
                "",  // Decision 13: P2 passes (FoW resolves, counters Bolt)
            ],
            ReplayTestConfig::new()
                // Counterspell is a blue card that can be exiled for Force of Will's alt cost
                .p1_hand(vec!["Yawgmoth's Will", "Counterspell"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp"])
                .p1_graveyard(vec!["Force of Will"])
                .p2_hand(vec!["Lightning Bolt"])
                .p2_battlefield(vec!["Mountain"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // P1 should be at 19 life (paid 1 life for Force of Will's alt cost)
        assert_eq!(
            game.life_total(alice),
            19,
            "P1 should be at 19 life after paying for Force of Will"
        );

        // P2 should still be at 20 life (Lightning Bolt was countered)
        assert_eq!(
            game.life_total(bob),
            20,
            "P2 should be at 20 life (Bolt was countered)"
        );
    }

    /// Tests Yawgmoth's Will replacement effect: cards that would go to
    /// graveyard are exiled instead.
    ///
    /// After Yawgmoth's Will resolves, P1 casts Lightning Bolt on their own
    /// creature. The creature should be exiled instead of going to graveyard.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_yawgmoths_will_replacement_effect() {
        let game = run_replay_test(
            vec![
                // P1's turn: cast Yawgmoth's Will
                "1", // Decision 0: Cast Yawgmoth's Will
                "0", // Decision 1: Tap first Swamp
                "0", // Decision 2: Tap second Swamp
                "0", // Decision 3: Tap third Swamp
                "1", // Decision 4: Stop activating mana abilities; leave Mountain untapped
                "",  // Decision 5: Pass priority so Yawgmoth's Will resolves
                // Now cast Lightning Bolt from graveyard on our own creature
                "1", // Decision 6: Cast Lightning Bolt from GY
                "2", // Decision 7: Target Grizzly Bears (0=P1, 1=P2, 2=Bears)
                "0", // Decision 8: Tap Mountain for mana
                "",  // Decision 9: P1 passes priority (P2 auto-passes, Bolt resolves)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Yawgmoth's Will"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp", "Mountain", "Grizzly Bears"])
                .p1_graveyard(vec!["Lightning Bolt"]),
        );

        let alice = PlayerId::from_index(0);

        // Grizzly Bears should be in EXILE (not graveyard) due to replacement effect
        let bears_in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(
            bears_in_exile,
            "Grizzly Bears should be in exile (Yawgmoth's Will replacement)"
        );

        // Lightning Bolt should also be in exile (used from GY with YW active)
        let bolt_in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Lightning Bolt")
                .unwrap_or(false)
        });
        assert!(
            bolt_in_exile,
            "Lightning Bolt should be in exile (cast from GY with Yawgmoth's Will)"
        );

        // Grizzly Bears should NOT be in graveyard
        let bears_in_gy = game
            .player(alice)
            .map(|p| {
                p.graveyard.iter().any(|&id| {
                    game.object(id)
                        .map(|o| o.name == "Grizzly Bears")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        assert!(!bears_in_gy, "Grizzly Bears should NOT be in graveyard");
    }

    /// Negative coverage for Yawgmoth's Will replacement timing boundary:
    /// before Yawgmoth's Will resolves, normal destruction still uses graveyard.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_without_yawgmoths_will_uses_graveyard_normally() {
        let game = run_replay_test(
            vec![
                "1", // Cast Lightning Bolt from hand
                "2", // Target Grizzly Bears (0=P1, 1=P2, 2=Bears)
                "0", // Tap Mountain for mana
                "",  // Pass priority to resolve Bolt
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Lightning Bolt"])
                .p1_battlefield(vec!["Mountain", "Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);

        let bears_in_graveyard = game
            .player(alice)
            .map(|p| {
                p.graveyard.iter().any(|&id| {
                    game.object(id)
                        .map(|o| o.name == "Grizzly Bears")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        assert!(
            bears_in_graveyard,
            "Without Yawgmoth's Will, Grizzly Bears should be in graveyard"
        );
    }
}
