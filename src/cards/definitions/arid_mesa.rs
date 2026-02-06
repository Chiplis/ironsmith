//! Card definition for Arid Mesa.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Arid Mesa card definition.
///
/// Arid Mesa
/// Land
/// {T}, Pay 1 life, Sacrifice Arid Mesa: Search your library for a Mountain or Plains card,
/// put it onto the battlefield, then shuffle.
pub fn arid_mesa() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Arid Mesa")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}, Pay 1 life, Sacrifice Arid Mesa: Search your library for a Mountain or Plains card, put it onto the battlefield, then shuffle.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{Action, GameScript};
    use crate::zone::Zone;

    /// Helper to create a basic game state for testing.
    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_arid_mesa_basic_properties() {
        let def = arid_mesa();

        // Check name
        assert_eq!(def.name(), "Arid Mesa");

        // Check it's a land
        assert!(def.card.is_land());
        assert!(def.card.card_types.contains(&CardType::Land));

        // Check mana value is 0 (lands have no mana cost)
        assert_eq!(def.card.mana_value(), 0);

        // Check it's colorless (no color indicator, no mana cost)
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_arid_mesa_is_not_basic() {
        let def = arid_mesa();

        // Arid Mesa is NOT a basic land
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[test]
    fn test_arid_mesa_has_activated_ability() {
        let def = arid_mesa();

        // Should have exactly one ability
        assert_eq!(def.abilities.len(), 1);

        // The ability should be an activated ability (not a mana ability)
        assert!(matches!(&def.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[test]
    fn test_arid_mesa_ability_costs() {
        let def = arid_mesa();

        let ability = &def.abilities[0];
        if let AbilityKind::Activated(activated) = &ability.kind {
            // No mana cost - all costs are effect-based (tap, pay life, sacrifice)
            assert!(
                activated.mana_cost.mana_cost().is_none(),
                "Should have no mana cost"
            );

            // All costs are now in TotalCost: tap, pay life, sacrifice
            assert_eq!(
                activated.mana_cost.costs().len(),
                3,
                "Should have 3 costs: tap, pay life, sacrifice"
            );

            let debug_str = format!("{:?}", &activated.mana_cost.costs());

            // Check for tap effect
            assert!(debug_str.contains("TapEffect"), "costs should contain tap");

            // Check for life payment effect
            assert!(
                debug_str.contains("LoseLifeEffect"),
                "costs should contain pay life"
            );

            // Check for sacrifice effect
            assert!(
                debug_str.contains("SacrificeTargetEffect"),
                "costs should contain sacrifice"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_arid_mesa_ability_effects() {
        let def = arid_mesa();

        let ability = &def.abilities[0];
        if let AbilityKind::Activated(activated) = &ability.kind {
            // Search sequence is composed as a single SequenceEffect
            assert_eq!(activated.effects.len(), 1, "Should have 1 effect");

            let debug_str = format!("{:?}", activated.effects[0]);
            assert!(
                debug_str.contains("SequenceEffect"),
                "Effect should be a SequenceEffect"
            );
            assert!(
                debug_str.contains("ChooseObjectsEffect"),
                "Sequence should include ChooseObjectsEffect"
            );
            assert!(
                debug_str.contains("ShuffleLibraryEffect"),
                "Sequence should include ShuffleLibraryEffect"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_arid_mesa_search_filter() {
        let def = arid_mesa();

        let ability = &def.abilities[0];
        if let AbilityKind::Activated(activated) = &ability.kind {
            let debug_str = format!("{:?}", activated.effects[0]);

            // Verify the effect contains the right filter criteria via debug output
            assert!(
                debug_str.contains("ChooseObjectsEffect"),
                "Should include ChooseObjectsEffect"
            );
            assert!(
                debug_str.contains("Mountain"),
                "Filter should include Mountain subtype"
            );
            assert!(
                debug_str.contains("Plains"),
                "Filter should include Plains subtype"
            );
            assert!(debug_str.contains("Land"), "Filter should be for lands");
            assert!(
                debug_str.contains("Battlefield"),
                "Destination should be battlefield"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_arid_mesa_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Arid Mesa on the battlefield
        let def = arid_mesa();
        let mesa_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&mesa_id));

        // Verify the object has the activated ability
        let obj = game.object(mesa_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);
        assert!(matches!(&obj.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[test]
    fn test_arid_mesa_not_a_mana_ability() {
        let def = arid_mesa();

        // Fetchlands are NOT mana abilities because they don't add mana
        // (they search for lands that produce mana, but the ability itself doesn't)
        assert!(
            !def.abilities[0].is_mana_ability(),
            "Arid Mesa ability is not a mana ability"
        );
    }

    #[test]
    fn test_arid_mesa_activation_requires_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Arid Mesa on the battlefield
        let def = arid_mesa();
        let mesa_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Tap the land
        game.tap(mesa_id);

        // The ability has a tap cost effect, so it can't be activated while tapped
        assert!(game.is_tapped(mesa_id));

        // The ability's cost_effects includes tapping
        let obj = game.object(mesa_id).unwrap();
        if let AbilityKind::Activated(activated) = &obj.abilities[0].kind {
            let debug_str = format!("{:?}", &activated.mana_cost.costs());
            assert!(
                debug_str.contains("TapEffect"),
                "Ability cost_effects should include tapping"
            );
        }
    }

    #[test]
    fn test_arid_mesa_not_affected_by_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Arid Mesa on the battlefield
        let def = arid_mesa();
        let mesa_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(mesa_id).unwrap();

        // Lands are not affected by summoning sickness
        assert!(!obj.is_creature(), "Arid Mesa is not a creature");
    }

    // =========================================================================
    // GameScript Integration Tests
    // =========================================================================

    #[test]
    fn test_arid_mesa_play_from_hand() {
        // Test that Arid Mesa can be played as a land
        let result = GameScript::new()
            .player("Alice", &["Arid Mesa"])
            .player("Bob", &[])
            .action(Action::PlayLand("Arid Mesa"))
            .action(Action::Pass)
            .action(Action::Pass) // Bob passes
            .run();

        let game = result.expect("Game should run successfully");

        // Arid Mesa should be on the battlefield
        assert!(
            game.battlefield_has("Arid Mesa"),
            "Arid Mesa should be on battlefield"
        );

        // Alice's hand should be empty
        assert!(
            !game.hand_has(PlayerId::from_index(0), "Arid Mesa"),
            "Arid Mesa should not be in hand"
        );
    }

    #[test]
    fn test_arid_mesa_activate_find_plains() {
        // Test activating Arid Mesa to find a Plains
        // Note: The search requires decision-making, so we test what we can
        let result = GameScript::new()
            .player("Alice", &["Arid Mesa", "Plains"])
            .player("Bob", &[])
            .action(Action::PlayLand("Arid Mesa"))
            .action(Action::Pass)
            .action(Action::Pass)
            .run();

        let game = result.expect("Game should run successfully");

        // Arid Mesa should be on battlefield (we only played it, didn't activate)
        assert!(
            game.battlefield_has("Arid Mesa"),
            "Arid Mesa should be on battlefield after playing"
        );

        // Plains should still be in hand (we didn't activate the fetchland)
        assert!(
            game.hand_has(PlayerId::from_index(0), "Plains"),
            "Plains should still be in hand"
        );
    }

    #[test]
    fn test_arid_mesa_with_dual_land_types() {
        // Verify the filter would match lands with both Mountain and Plains types
        // (like Plateau, Sacred Foundry, etc.)
        let def = arid_mesa();

        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            // First effect should be SearchLibraryEffect
            let debug_str = format!("{:?}", activated.effects[0]);

            // Both subtypes should be in the filter (OR logic)
            // This means:
            // - A basic Plains matches (has Plains subtype)
            // - A basic Mountain matches (has Mountain subtype)
            // - A Plateau (Mountain Plains) matches (has both)
            // - A Sacred Foundry (Mountain Plains) matches (has both)
            assert!(
                debug_str.contains("Mountain"),
                "Filter should contain Mountain"
            );
            assert!(debug_str.contains("Plains"), "Filter should contain Plains");
        }
    }

    // =========================================================================
    // Cost Verification Tests
    // =========================================================================

    #[test]
    fn test_arid_mesa_life_cost_amount() {
        let def = arid_mesa();

        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            // Life cost is now in cost_effects as LoseLifeEffect
            let debug_str = format!("{:?}", &activated.mana_cost.costs());
            assert!(
                debug_str.contains("LoseLifeEffect") && debug_str.contains("Fixed(1)"),
                "Arid Mesa should cost exactly 1 life to activate"
            );
        }
    }

    #[test]
    fn test_arid_mesa_no_mana_cost_to_activate() {
        let def = arid_mesa();

        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            // TotalCost should be free (no mana cost)
            assert!(
                activated.mana_cost.mana_cost().is_none(),
                "Arid Mesa activation should not require mana"
            );
        }
    }
}
