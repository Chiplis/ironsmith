//! Phyrexian Tower card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::{CardType, Supertype};

/// Phyrexian Tower
/// Legendary Land
/// {T}: Add {C}.
/// {T}, Sacrifice a creature: Add {B}{B}.
pub fn phyrexian_tower() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Phyrexian Tower")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Legendary])
        .parse_text("{T}: Add {C}.\n{T}, Sacrifice a creature: Add {B}{B}.")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_phyrexian_tower_basic_properties() {
        let def = phyrexian_tower();
        assert_eq!(def.name(), "Phyrexian Tower");
        assert!(def.card.is_land());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_phyrexian_tower_is_legendary() {
        let def = phyrexian_tower();
        assert!(def.card.has_supertype(Supertype::Legendary));
    }

    #[test]
    fn test_phyrexian_tower_is_not_basic() {
        let def = phyrexian_tower();
        assert!(!def.card.has_supertype(Supertype::Basic));
    }

    #[test]
    fn test_phyrexian_tower_has_two_abilities() {
        let def = phyrexian_tower();
        assert_eq!(def.abilities.len(), 2);
    }

    // ========================================
    // First Ability (Colorless Mana) Tests
    // ========================================

    #[test]
    fn test_first_ability_is_mana_ability() {
        let def = phyrexian_tower();
        let ability = &def.abilities[0];
        assert!(ability.is_mana_ability());
    }

    #[test]
    fn test_first_ability_produces_colorless_mana() {
        let def = phyrexian_tower();
        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::Colorless]);
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_first_ability_requires_tap() {
        let def = phyrexian_tower();
        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    // ========================================
    // Second Ability (Sacrifice for BB) Tests
    // ========================================

    #[test]
    fn test_second_ability_is_mana_ability() {
        let def = phyrexian_tower();
        let ability = &def.abilities[1];
        assert!(ability.is_mana_ability());
    }

    #[test]
    fn test_second_ability_produces_two_black_mana() {
        let def = phyrexian_tower();
        let ability = &def.abilities[1];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(
                mana_ability.mana,
                vec![ManaSymbol::Black, ManaSymbol::Black]
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_second_ability_requires_tap() {
        let def = phyrexian_tower();
        let ability = &def.abilities[1];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_second_ability_requires_creature_sacrifice() {
        let def = phyrexian_tower();
        let ability = &def.abilities[1];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            // Sacrifice is now in cost_effects (not TotalCost) so "dies" triggers fire
            assert!(
                !mana_ability.mana_cost.costs().is_empty(),
                "Should have cost_effects for sacrifice"
            );
            // Should have 3 cost_effects: tap + choose + sacrifice
            assert_eq!(
                mana_ability.mana_cost.costs().len(),
                3,
                "Should have tap + choose + sacrifice effects"
            );

            // Check for tap cost
            assert!(mana_ability.has_tap_cost(), "Should have tap cost");

            let debug_str = format!("{:?}", &mana_ability.mana_cost.costs());
            assert!(
                debug_str.contains("ChooseObjectsEffect"),
                "cost_effects should contain choose objects"
            );
            assert!(
                debug_str.contains("SacrificeEffect"),
                "cost_effects should contain sacrifice"
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_phyrexian_tower_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = phyrexian_tower();
        let tower_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&tower_id));

        // Verify the object has both abilities
        let obj = game.object(tower_id).unwrap();
        assert_eq!(obj.abilities.len(), 2);
    }

    #[test]
    fn test_phyrexian_tower_oracle_text() {
        let def = phyrexian_tower();
        assert!(def.card.oracle_text.contains("Add {C}"));
        assert!(def.card.oracle_text.contains("Sacrifice a creature"));
        assert!(def.card.oracle_text.contains("Add {B}{B}"));
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests Phyrexian Tower's basic mana ability.
    ///
    /// Phyrexian Tower: Legendary Land
    /// {T}: Add {C}.
    #[test]
    fn test_replay_phyrexian_tower_mana() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Tap Phyrexian Tower for colorless mana (mana ability)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Phyrexian Tower"]),
        );

        let alice = PlayerId::from_index(0);

        // Phyrexian Tower should be on battlefield (tapped)
        assert!(
            game.battlefield_has("Phyrexian Tower"),
            "Phyrexian Tower should be on battlefield"
        );

        // Player should have 1 colorless mana in pool
        let player = game.player(alice).unwrap();
        assert_eq!(
            player.mana_pool.colorless, 1,
            "Should have 1 colorless mana from Phyrexian Tower"
        );
    }

    // Tests Phyrexian Tower's sacrifice ability (add BB).
    //
    // {T}, Sacrifice a creature: Add {B}{B}.
    //
    // NOTE: This replay test remains disabled while replay prompt ordering for
    // mana abilities with choice costs is stabilized.

    // #[test]
    // fn test_replay_phyrexian_tower_sacrifice() {
    //     use crate::tests::integration_tests::{run_replay_test, ReplayTestConfig};
    //
    //     let game = run_replay_test(
    //         vec![
    //             // Actions: 0=pass, 1=colorless mana ability, 2=sacrifice mana ability
    //             "2",  // Activate sacrifice mana ability
    //             "0",  // Choose Grizzly Bears to sacrifice
    //         ],
    //         ReplayTestConfig::new()
    //             .p1_battlefield(vec!["Phyrexian Tower", "Grizzly Bears"]),
    //     );
    //
    //     let alice = PlayerId::from_index(0);
    //
    //     // Grizzly Bears should be in graveyard (sacrificed)
    //     let alice_player = game.player(alice).unwrap();
    //     let bears_in_gy = alice_player.graveyard.iter().any(|&id| {
    //         game.object(id).map(|o| o.name == "Grizzly Bears").unwrap_or(false)
    //     });
    //     assert!(bears_in_gy, "Grizzly Bears should be in graveyard after sacrifice");
    //
    //     // Alice should have 2 black mana
    //     assert_eq!(
    //         alice_player.mana_pool.black, 2,
    //         "Alice should have 2 black mana from Phyrexian Tower sacrifice"
    //     );
    //
    //     // Phyrexian Tower should still be on battlefield (but tapped)
    //     assert!(
    //         game.battlefield_has("Phyrexian Tower"),
    //         "Phyrexian Tower should still be on battlefield"
    //     );
    // }
}
