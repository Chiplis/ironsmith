//! Sol Ring card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Sol Ring card definition.
///
/// Sol Ring {1}
/// Artifact
/// {T}: Add {C}{C}.
pub fn sol_ring() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Sol Ring")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Add {C}{C}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};
    use crate::zone::Zone;

    /// Helper to create a basic game state for testing.
    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_sol_ring_basic_properties() {
        let def = sol_ring();

        // Check name
        assert_eq!(def.name(), "Sol Ring");

        // Check it's an artifact
        assert!(def.card.is_artifact());
        assert!(def.card.card_types.contains(&CardType::Artifact));

        // Check mana cost is {1}
        assert_eq!(def.card.mana_value(), 1);

        // Check it's colorless
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_sol_ring_is_not_creature() {
        let def = sol_ring();

        // Sol Ring is not a creature
        assert!(!def.card.is_creature());
    }

    #[test]
    fn test_sol_ring_has_no_subtypes() {
        let def = sol_ring();

        // Sol Ring has no subtypes
        assert!(def.card.subtypes.is_empty());
    }

    #[test]
    fn test_sol_ring_has_mana_ability() {
        let def = sol_ring();

        // Should have exactly one ability
        assert_eq!(def.abilities.len(), 1);

        // The ability should be a mana ability
        assert!(def.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_sol_ring_mana_ability_structure() {
        let def = sol_ring();

        let ability = &def.abilities[0];
        match &ability.kind {
            AbilityKind::Mana(mana_ability) => {
                // Sol Ring uses fixed mana (mana field, not effects)
                assert!(
                    mana_ability.effects.is_none(),
                    "Should use mana field, not effects"
                );

                // Should produce 2 colorless mana
                assert_eq!(mana_ability.mana.len(), 2, "Should produce 2 mana");
                assert!(
                    mana_ability
                        .mana
                        .iter()
                        .all(|m| *m == ManaSymbol::Colorless),
                    "Should produce colorless mana"
                );
            }
            _ => panic!("Expected mana ability"),
        }
    }

    #[test]
    fn test_sol_ring_mana_ability_has_tap_cost() {
        let def = sol_ring();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.has_tap_cost(),
                "Should have tap as part of cost"
            );
            // Sol Ring has no mana cost - just tap
            assert!(
                mana_ability.mana_cost.mana_cost().is_none(),
                "Should not have mana cost to activate"
            );
        }
    }

    // =========================================================================
    // Mana Production Tests
    // =========================================================================

    #[test]
    fn test_sol_ring_produces_two_colorless() {
        let def = sol_ring();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            // Sol Ring uses fixed mana production (mana field)
            assert_eq!(mana_ability.mana.len(), 2, "Should produce 2 mana");
            assert!(
                mana_ability
                    .mana
                    .iter()
                    .all(|m| *m == ManaSymbol::Colorless),
                "Should produce colorless mana"
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_sol_ring_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Sol Ring on the battlefield
        let def = sol_ring();
        let ring_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&ring_id));

        // Verify the object has the mana ability
        let obj = game.object(ring_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);
        assert!(obj.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_sol_ring_activation_requires_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Sol Ring on the battlefield
        let def = sol_ring();
        let ring_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Tap the artifact
        game.tap(ring_id);

        // The mana ability has a tap cost, so it can't be activated while tapped
        assert!(game.is_tapped(ring_id));

        // The ability's cost requires tapping, which can't be done if already tapped
        let obj = game.object(ring_id).unwrap();
        if let AbilityKind::Mana(mana_ability) = &obj.abilities[0].kind {
            assert!(
                mana_ability.has_tap_cost(),
                "Mana ability should require tapping"
            );
        }
    }

    #[test]
    fn test_sol_ring_not_affected_by_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Sol Ring on the battlefield
        let def = sol_ring();
        let ring_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(ring_id).unwrap();

        // Artifacts (non-creatures) are not affected by summoning sickness
        // The mana ability should be usable immediately
        assert!(!obj.is_creature(), "Sol Ring is not a creature");

        // Verify it has the mana ability ready to use
        assert!(obj.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_sol_ring_oracle_text() {
        let def = sol_ring();

        assert!(def.card.oracle_text.contains("Add"));
        assert!(def.card.oracle_text.contains("{C}{C}"));
    }

    // =========================================================================
    // Mana Advantage Tests
    // =========================================================================

    #[test]
    fn test_sol_ring_mana_advantage() {
        let def = sol_ring();

        // Sol Ring costs 1 mana to cast but produces 2 mana
        // This is why it's one of the strongest artifacts
        let casting_cost = def.card.mana_value();
        assert_eq!(casting_cost, 1);

        // The mana field produces 2 mana (two {C})
        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            // Sol Ring uses fixed mana production (mana field)
            assert_eq!(mana_ability.mana.len(), 2, "Should produce 2 mana");
            assert!(
                mana_ability
                    .mana
                    .iter()
                    .all(|m| *m == ManaSymbol::Colorless),
                "Should produce colorless mana"
            );
        }
    }

    #[test]
    fn test_sol_ring_is_permanent() {
        let def = sol_ring();

        // Sol Ring is an artifact, which is a permanent type
        assert!(def.is_permanent());
    }

    #[test]
    fn test_sol_ring_is_not_spell_in_traditional_sense() {
        let def = sol_ring();

        // While Sol Ring is cast as a spell, it's not an "instant or sorcery" spell
        // The is_spell method checks for instant/sorcery types
        assert!(!def.is_spell());
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Sol Ring mana production through actual gameplay.
    ///
    /// Sol Ring: {1} artifact, {T}: Add {C}{C}.
    /// This test verifies Sol Ring can be cast from hand with proper mana payment.
    #[test]
    fn test_replay_sol_ring_casting() {
        let game = run_replay_test(
            // Actions with Island and Sol Ring in hand:
            // After playing Island: 0=pass, 1=cast Sol Ring, 2=tap Island for mana
            vec![
                "1", // Play Island (index 1, after PassPriority at 0)
                "2", // Tap Island for mana (index 2, mana abilities come after spells)
                "1", // Cast Sol Ring (index 1, now we have mana in pool)
                "",  // Pass priority
                "",  // Opponent passes (Sol Ring resolves)
            ],
            ReplayTestConfig::new().p1_hand(vec!["Island", "Sol Ring"]),
        );

        // Sol Ring should be on the battlefield after resolving
        assert!(
            game.battlefield_has("Sol Ring"),
            "Sol Ring should be on battlefield after casting"
        );

        // Island should also be on the battlefield
        assert!(
            game.battlefield_has("Island"),
            "Island should be on battlefield"
        );
    }

    /// Tests that Sol Ring's mana ability works correctly.
    /// Start with Sol Ring already on battlefield, tap it for mana.
    #[test]
    fn test_replay_sol_ring_mana_production() {
        let game = run_replay_test(
            vec![
                "1", // Tap Sol Ring for mana (index 1, after PassPriority at 0)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Sol Ring"]),
        );

        // Sol Ring should be on battlefield (and tapped)
        assert!(
            game.battlefield_has("Sol Ring"),
            "Sol Ring should be on battlefield"
        );

        // Player should have 2 colorless mana in pool
        let alice = PlayerId::from_index(0);
        let player = game.player(alice).unwrap();
        assert_eq!(
            player.mana_pool.colorless, 2,
            "Should have 2 colorless mana from Sol Ring"
        );
    }
}
