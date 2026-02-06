//! Scrubland card definition (Original Dual Land).

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::{CardType, Subtype};

/// Creates the Scrubland card definition.
///
/// Scrubland
/// Land — Plains Swamp
/// {T}: Add {W} or {B}.
pub fn scrubland() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Scrubland")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains, Subtype::Swamp])
        .parse_text("{T}: Add {W} or {B}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::types::Supertype;
    use crate::zone::Zone;

    /// Helper to create a basic game state for testing.
    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_scrubland_basic_properties() {
        let def = scrubland();

        // Check name
        assert_eq!(def.name(), "Scrubland");

        // Check it's a land
        assert!(def.card.is_land());
        assert!(def.card.card_types.contains(&CardType::Land));

        // Check mana value is 0 (lands have no mana cost)
        assert_eq!(def.card.mana_value(), 0);

        // Check it's colorless (color identity comes from mana abilities, not from the card)
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_scrubland_is_not_basic() {
        let def = scrubland();

        // Scrubland is NOT a basic land (no Basic supertype)
        assert!(!def.card.has_supertype(Supertype::Basic));
    }

    #[test]
    fn test_scrubland_has_plains_subtype() {
        let def = scrubland();

        // Scrubland has Plains subtype
        assert!(def.card.has_subtype(Subtype::Plains));
    }

    #[test]
    fn test_scrubland_has_swamp_subtype() {
        let def = scrubland();

        // Scrubland has Swamp subtype
        assert!(def.card.has_subtype(Subtype::Swamp));
    }

    #[test]
    fn test_scrubland_is_dual_typed() {
        let def = scrubland();

        // Should have exactly two subtypes
        assert_eq!(def.card.subtypes.len(), 2);
        assert!(def.card.subtypes.contains(&Subtype::Plains));
        assert!(def.card.subtypes.contains(&Subtype::Swamp));
    }

    // =========================================================================
    // Mana Ability Tests
    // =========================================================================

    #[test]
    fn test_scrubland_has_two_mana_abilities() {
        let def = scrubland();

        // Should have exactly two abilities (both mana abilities)
        assert_eq!(def.abilities.len(), 2);

        // Both should be mana abilities
        assert!(def.abilities.iter().all(|a| a.is_mana_ability()));
    }

    #[test]
    fn test_scrubland_can_produce_white() {
        let def = scrubland();

        // Find the mana ability that produces white
        let white_ability = def.abilities.iter().find(|a| {
            if let AbilityKind::Mana(mana_ability) = &a.kind {
                mana_ability.mana.contains(&ManaSymbol::White)
            } else {
                false
            }
        });

        assert!(
            white_ability.is_some(),
            "Should have ability to produce white mana"
        );
    }

    #[test]
    fn test_scrubland_can_produce_black() {
        let def = scrubland();

        // Find the mana ability that produces black
        let black_ability = def.abilities.iter().find(|a| {
            if let AbilityKind::Mana(mana_ability) = &a.kind {
                mana_ability.mana.contains(&ManaSymbol::Black)
            } else {
                false
            }
        });

        assert!(
            black_ability.is_some(),
            "Should have ability to produce black mana"
        );
    }

    #[test]
    fn test_scrubland_mana_abilities_have_tap_cost() {
        let def = scrubland();

        // Both mana abilities should have tap as cost
        for ability in &def.abilities {
            if let AbilityKind::Mana(mana_ability) = &ability.kind {
                assert!(
                    mana_ability.has_tap_cost(),
                    "Each mana ability should have tap as cost"
                );
            }
        }
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_scrubland_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Scrubland on the battlefield
        let def = scrubland();
        let land_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&land_id));

        // Verify the object has the mana abilities
        let obj = game.object(land_id).unwrap();
        assert_eq!(obj.abilities.len(), 2);
        assert!(obj.abilities.iter().all(|a| a.is_mana_ability()));
    }

    #[test]
    fn test_scrubland_activation_requires_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Scrubland on the battlefield
        let def = scrubland();
        let land_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Tap the land
        game.tap(land_id);

        // The mana abilities have tap costs, so they can't be activated while tapped
        assert!(game.is_tapped(land_id));

        // Verify both abilities require tapping
        let obj = game.object(land_id).unwrap();
        for ability in &obj.abilities {
            if let AbilityKind::Mana(mana_ability) = &ability.kind {
                assert!(
                    mana_ability.has_tap_cost(),
                    "Mana ability should require tapping"
                );
            }
        }
    }

    #[test]
    fn test_scrubland_not_affected_by_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Scrubland on the battlefield
        let def = scrubland();
        let land_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(land_id).unwrap();

        // Lands are not affected by summoning sickness
        assert!(!obj.is_creature(), "Scrubland is not a creature");

        // Mana abilities are usable immediately
        assert!(obj.abilities.iter().all(|a| a.is_mana_ability()));
    }

    #[test]
    fn test_scrubland_oracle_text() {
        let def = scrubland();

        assert!(def.card.oracle_text.contains("Add"));
        assert!(def.card.oracle_text.contains("{W}"));
        assert!(def.card.oracle_text.contains("{B}"));
    }

    // =========================================================================
    // Rules Interaction Tests
    // =========================================================================

    #[test]
    fn test_scrubland_can_be_found_by_fetchlands() {
        // Scrubland can be found by any fetchland that searches for Plains or Swamp
        let def = scrubland();

        // It has Plains subtype (can be found by Flooded Strand, Marsh Flats, etc.)
        assert!(def.card.has_subtype(Subtype::Plains));

        // It has Swamp subtype (can be found by Bloodstained Mire, Polluted Delta, etc.)
        assert!(def.card.has_subtype(Subtype::Swamp));
    }

    #[test]
    fn test_scrubland_is_not_a_creature() {
        let def = scrubland();

        // Original dual lands are not creatures
        assert!(!def.is_creature());
    }

    #[test]
    fn test_scrubland_is_a_permanent() {
        let def = scrubland();

        // Lands are permanents
        assert!(def.is_permanent());
    }

    #[test]
    fn test_scrubland_is_not_a_spell() {
        let def = scrubland();

        // Lands are not spells (can't be countered normally)
        // is_spell checks for instant/sorcery
        assert!(!def.is_spell());
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    /// Tests tapping Scrubland for white mana.
    ///
    /// Scrubland: Land — Plains Swamp
    /// {T}: Add {W} or {B}.
    #[test]
    fn test_replay_scrubland_tap_for_white() {
        let game = run_replay_test(
            vec![
                "1", // Activate first mana ability (white)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Scrubland"]),
        );

        // Find the Scrubland and check it's tapped
        let alice = PlayerId::from_index(0);
        let scrubland_id = game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                game.object(id)
                    .map(|obj| obj.name == "Scrubland" && obj.controller == alice)
                    .unwrap_or(false)
            })
            .expect("Should find Scrubland");

        assert!(
            game.is_tapped(scrubland_id),
            "Scrubland should be tapped after activating mana ability"
        );

        // Check that white mana was added
        let alice_player = game.player(alice).unwrap();
        assert!(
            alice_player.mana_pool.white >= 1,
            "Should have white mana in pool"
        );
    }

    /// Tests tapping Scrubland for black mana.
    #[test]
    fn test_replay_scrubland_tap_for_black() {
        let game = run_replay_test(
            vec![
                "2", // Activate second mana ability (black)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Scrubland"]),
        );

        // Check that black mana was added
        let alice = PlayerId::from_index(0);
        let alice_player = game.player(alice).unwrap();
        assert!(
            alice_player.mana_pool.black >= 1,
            "Should have black mana in pool"
        );
    }
}
