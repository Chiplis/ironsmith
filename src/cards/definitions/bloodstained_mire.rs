//! Card definition for Bloodstained Mire.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Bloodstained Mire card definition.
///
/// Bloodstained Mire
/// Land
/// {T}, Pay 1 life, Sacrifice Bloodstained Mire: Search your library for a Swamp or Mountain card,
/// put it onto the battlefield, then shuffle.
pub fn bloodstained_mire() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Bloodstained Mire")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}, Pay 1 life, Sacrifice Bloodstained Mire: Search your library for a Swamp or Mountain card, put it onto the battlefield, then shuffle.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Zone;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{Action, GameScript};
    use crate::types::Subtype;

    /// Helper to create a basic game state for testing.
    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_bloodstained_mire_basic_properties() {
        let def = bloodstained_mire();

        // Check name
        assert_eq!(def.name(), "Bloodstained Mire");

        // Check it's a land
        assert!(def.card.is_land());
        assert!(def.card.card_types.contains(&CardType::Land));

        // Check mana value is 0 (lands have no mana cost)
        assert_eq!(def.card.mana_value(), 0);

        // Check it's colorless (no color indicator, no mana cost)
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_bloodstained_mire_is_not_basic() {
        let def = bloodstained_mire();

        // Bloodstained Mire is NOT a basic land
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[test]
    fn test_bloodstained_mire_has_no_land_subtypes() {
        let def = bloodstained_mire();

        // Fetchlands don't have land subtypes themselves
        assert!(def.card.subtypes.is_empty());
        assert!(!def.card.has_subtype(Subtype::Swamp));
        assert!(!def.card.has_subtype(Subtype::Mountain));
    }

    #[test]
    fn test_bloodstained_mire_has_activated_ability() {
        let def = bloodstained_mire();

        // Should have exactly one ability
        assert_eq!(def.abilities.len(), 1);

        // The ability should be an activated ability (not a mana ability)
        assert!(matches!(&def.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[test]
    fn test_bloodstained_mire_ability_costs() {
        let def = bloodstained_mire();

        let ability = &def.abilities[0];
        if let AbilityKind::Activated(activated) = &ability.kind {
            // Should have 3 cost effects: tap, life payment, sacrifice self
            assert_eq!(
                activated.mana_cost.costs().len(),
                3,
                "Should have 3 cost effects"
            );

            // Check for tap cost
            assert!(activated.has_tap_cost(), "Should have tap cost");

            // Check for life payment cost of 1
            assert_eq!(
                activated.life_cost_amount(),
                Some(1),
                "Should have pay 1 life cost"
            );

            // Check for sacrifice self cost
            assert!(
                activated.has_sacrifice_self_cost(),
                "Should have sacrifice self cost"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_bloodstained_mire_ability_effects() {
        let def = bloodstained_mire();

        let ability = &def.abilities[0];
        if let AbilityKind::Activated(activated) = &ability.kind {
            // Search is compiled as a composed sequence: choose from library, put onto battlefield, then shuffle.
            assert_eq!(activated.effects.len(), 1, "Should have 1 effect");

            let debug_str = format!("{:?}", activated.effects[0]);
            assert!(
                debug_str.contains("SequenceEffect")
                    && debug_str.contains("ChooseObjectsEffect")
                    && debug_str.contains("PutOntoBattlefieldEffect")
                    && debug_str.contains("ShuffleLibraryEffect"),
                "Effect should compose search->put->shuffle, got {debug_str}"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_bloodstained_mire_search_filter() {
        let def = bloodstained_mire();

        let ability = &def.abilities[0];
        if let AbilityKind::Activated(activated) = &ability.kind {
            let debug_str = format!("{:?}", activated.effects[0]);

            // Verify the effect contains the right filter criteria via debug output
            assert!(
                debug_str.contains("ChooseObjectsEffect") && debug_str.contains("is_search: true"),
                "Should include library search choice effect"
            );
            assert!(
                debug_str.contains("Swamp"),
                "Filter should include Swamp subtype"
            );
            assert!(
                debug_str.contains("Mountain"),
                "Filter should include Mountain subtype"
            );
            assert!(debug_str.contains("Land"), "Filter should be for lands");
            assert!(debug_str.contains("Library"), "Should search the library");
            assert!(
                debug_str.contains("PutOntoBattlefieldEffect") && debug_str.contains("Battlefield"),
                "Destination should be battlefield"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_bloodstained_mire_searches_for_correct_types() {
        let def = bloodstained_mire();

        let ability = &def.abilities[0];
        if let AbilityKind::Activated(activated) = &ability.kind {
            let debug_str = format!("{:?}", activated.effects[0]);

            // Should search for Swamp and Mountain
            assert!(debug_str.contains("Swamp"), "Filter should include Swamp");
            assert!(
                debug_str.contains("Mountain"),
                "Filter should include Mountain"
            );

            // Should NOT search for Plains or other land types
            // (This is implicit - the filter only contains the specified subtypes)
        } else {
            panic!("Expected activated ability");
        }
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_bloodstained_mire_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bloodstained Mire on the battlefield
        let def = bloodstained_mire();
        let mire_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&mire_id));

        // Verify the object has the activated ability
        let obj = game.object(mire_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);
        assert!(matches!(&obj.abilities[0].kind, AbilityKind::Activated(_)));
    }

    #[test]
    fn test_bloodstained_mire_not_a_mana_ability() {
        let def = bloodstained_mire();

        // Fetchlands are NOT mana abilities because they don't add mana
        // (they search for lands that produce mana, but the ability itself doesn't)
        assert!(
            !def.abilities[0].is_mana_ability(),
            "Bloodstained Mire ability is not a mana ability"
        );
    }

    #[test]
    fn test_bloodstained_mire_activation_requires_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bloodstained Mire on the battlefield
        let def = bloodstained_mire();
        let mire_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Tap the land
        game.tap(mire_id);

        // The ability has a tap cost, so it can't be activated while tapped
        assert!(game.is_tapped(mire_id));

        // The ability's cost requires tapping
        let obj = game.object(mire_id).unwrap();
        if let AbilityKind::Activated(activated) = &obj.abilities[0].kind {
            assert!(activated.has_tap_cost(), "Ability should require tapping");
        }
    }

    #[test]
    fn test_bloodstained_mire_not_affected_by_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bloodstained Mire on the battlefield
        let def = bloodstained_mire();
        let mire_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(mire_id).unwrap();

        // Lands are not affected by summoning sickness
        assert!(!obj.is_creature(), "Bloodstained Mire is not a creature");
    }

    // =========================================================================
    // GameScript Integration Tests
    // =========================================================================

    #[test]
    fn test_bloodstained_mire_play_from_hand() {
        // Test that Bloodstained Mire can be played as a land
        let result = GameScript::new()
            .player("Alice", &["Bloodstained Mire"])
            .player("Bob", &[])
            .action(Action::PlayLand("Bloodstained Mire"))
            .action(Action::Pass)
            .action(Action::Pass) // Bob passes
            .run();

        let game = result.expect("Game should run successfully");

        // Bloodstained Mire should be on the battlefield
        assert!(
            game.battlefield_has("Bloodstained Mire"),
            "Bloodstained Mire should be on battlefield"
        );

        // Alice's hand should be empty
        assert!(
            !game.hand_has(PlayerId::from_index(0), "Bloodstained Mire"),
            "Bloodstained Mire should not be in hand"
        );
    }

    #[test]
    fn test_bloodstained_mire_with_dual_land_types() {
        // Verify the filter would match lands with both Swamp and Mountain types
        // (like Badlands, Blood Crypt, etc.)
        let def = bloodstained_mire();

        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            // First effect should be SearchLibraryEffect
            let debug_str = format!("{:?}", activated.effects[0]);

            // Both subtypes should be in the filter (OR logic)
            // This means:
            // - A basic Swamp matches (has Swamp subtype)
            // - A basic Mountain matches (has Mountain subtype)
            // - A Badlands (Swamp Mountain) matches (has both)
            // - A Blood Crypt (Swamp Mountain) matches (has both)
            assert!(debug_str.contains("Swamp"), "Filter should contain Swamp");
            assert!(
                debug_str.contains("Mountain"),
                "Filter should contain Mountain"
            );
        }
    }

    // =========================================================================
    // Cost Verification Tests
    // =========================================================================

    #[test]
    fn test_bloodstained_mire_life_cost_amount() {
        let def = bloodstained_mire();

        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            // Check cost_effects for life cost
            assert_eq!(
                activated.life_cost_amount(),
                Some(1),
                "Bloodstained Mire should cost exactly 1 life to activate"
            );
        }
    }

    #[test]
    fn test_bloodstained_mire_no_mana_cost_to_activate() {
        let def = bloodstained_mire();

        if let AbilityKind::Activated(activated) = &def.abilities[0].kind {
            // Should not have any mana cost
            let has_mana_cost = activated.mana_cost.costs().iter().any(|c| c.is_mana_cost());

            assert!(
                !has_mana_cost,
                "Bloodstained Mire activation should not require mana"
            );
        }
    }

    // =========================================================================
    // Comparison with other fetch lands
    // =========================================================================

    #[test]
    fn test_bloodstained_mire_differs_from_arid_mesa() {
        // Bloodstained Mire fetches Swamp/Mountain
        // Arid Mesa fetches Mountain/Plains
        let mire = bloodstained_mire();
        let mesa = crate::cards::arid_mesa();

        // Both should be fetch lands
        assert!(mire.card.is_land());
        assert!(mesa.card.is_land());

        // Both should have exactly one activated ability
        assert_eq!(mire.abilities.len(), 1);
        assert_eq!(mesa.abilities.len(), 1);

        // Get the search effect debug strings to compare filters
        if let (AbilityKind::Activated(mire_act), AbilityKind::Activated(mesa_act)) =
            (&mire.abilities[0].kind, &mesa.abilities[0].kind)
        {
            let mire_filter = format!("{:?}", mire_act.effects[0]);
            let mesa_filter = format!("{:?}", mesa_act.effects[0]);

            // Bloodstained Mire should search for Swamp
            assert!(mire_filter.contains("Swamp"));
            assert!(!mesa_filter.contains("Swamp")); // Mesa doesn't fetch Swamp

            // Arid Mesa should search for Plains
            assert!(mesa_filter.contains("Plains"));
            assert!(!mire_filter.contains("Plains")); // Mire doesn't fetch Plains

            // Both should search for Mountain
            assert!(mire_filter.contains("Mountain"));
            assert!(mesa_filter.contains("Mountain"));
        }
    }
}
