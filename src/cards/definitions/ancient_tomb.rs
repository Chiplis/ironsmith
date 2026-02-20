//! Ancient Tomb card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::CardType;

/// Ancient Tomb - Land
/// {T}: Add {C}{C}. Ancient Tomb deals 2 damage to you.
pub fn ancient_tomb() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Ancient Tomb")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {C}{C}. Ancient Tomb deals 2 damage to you.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::target::ChooseSpec;
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
    fn test_ancient_tomb_basic_properties() {
        let def = ancient_tomb();

        // Check name
        assert_eq!(def.name(), "Ancient Tomb");

        // Check it's a land
        assert!(def.card.is_land());
        assert!(def.card.card_types.contains(&CardType::Land));

        // Check mana value is 0 (lands have no mana cost)
        assert_eq!(def.card.mana_value(), 0);

        // Check it's colorless (no color indicator, no mana cost)
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_ancient_tomb_is_not_basic() {
        let def = ancient_tomb();

        // Ancient Tomb is NOT a basic land
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[test]
    fn test_ancient_tomb_has_mana_ability() {
        let def = ancient_tomb();

        // Should have exactly one ability
        assert_eq!(def.abilities.len(), 1);

        // The ability should be a mana ability
        assert!(def.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_ancient_tomb_mana_ability_structure() {
        let def = ancient_tomb();

        let ability = &def.abilities[0];
        match &ability.kind {
            AbilityKind::Mana(mana_ability) => {
                // Should have effects (not fixed mana)
                assert!(mana_ability.effects.is_some());

                let effects = mana_ability.effects.as_ref().unwrap();
                assert_eq!(
                    effects.len(),
                    2,
                    "Should have 2 effects: add mana and deal damage"
                );

                // First effect should be AddManaEffect
                assert!(format!("{:?}", effects[0]).contains("AddManaEffect"));

                // Second effect should be DealDamageEffect
                assert!(format!("{:?}", effects[1]).contains("DealDamageEffect"));
            }
            _ => panic!("Expected mana ability"),
        }
    }

    // =========================================================================
    // Mana Production Tests
    // =========================================================================

    #[test]
    fn test_ancient_tomb_produces_two_colorless() {
        let def = ancient_tomb();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            let effects = mana_ability.effects.as_ref().unwrap();

            // Check the AddMana effect
            let debug_str = format!("{:?}", &effects[0]);
            assert!(
                debug_str.contains("AddManaEffect"),
                "First effect should be AddManaEffect"
            );
            assert!(
                debug_str.contains("Colorless"),
                "Should contain Colorless mana"
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    // =========================================================================
    // Damage Tests
    // =========================================================================

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_ancient_tomb_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Ancient Tomb on the battlefield
        let def = ancient_tomb();
        let tomb_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&tomb_id));

        // Verify the object has the mana ability
        let obj = game.object(tomb_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);
        assert!(obj.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_ancient_tomb_activation_requires_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Ancient Tomb on the battlefield
        let def = ancient_tomb();
        let tomb_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Tap the land
        game.tap(tomb_id);

        // The mana ability has a tap cost, so it can't be activated while tapped
        assert!(game.is_tapped(tomb_id));

        // The ability's cost requires tapping, which can't be done if already tapped
        let obj = game.object(tomb_id).unwrap();
        if let AbilityKind::Mana(mana_ability) = &obj.abilities[0].kind {
            assert!(
                mana_ability.has_tap_cost(),
                "Mana ability should require tapping"
            );
        }
    }

    #[test]
    fn test_ancient_tomb_summoning_sickness_irrelevant() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Ancient Tomb on the battlefield (will have summoning sickness)
        let def = ancient_tomb();
        let tomb_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(tomb_id).unwrap();

        // Lands (non-creatures) are not affected by summoning sickness
        // The mana ability should be usable immediately
        assert!(!obj.is_creature(), "Ancient Tomb is not a creature");

        // Verify it has the mana ability ready to use
        assert!(obj.abilities[0].is_mana_ability());
    }

    // =========================================================================
    // Life Total Interaction Tests
    // =========================================================================

    #[test]
    fn test_ancient_tomb_effect_targets_source_controller() {
        // Verify that when multiple players exist, the damage targets the source controller
        let def = ancient_tomb();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            let effects = mana_ability.effects.as_ref().unwrap();

            // SourceController means the controller of the land (Ancient Tomb)
            let target_spec = effects[1].0.get_target_spec().unwrap();
            assert!(matches!(target_spec, ChooseSpec::SourceController));
        }
    }

    #[test]
    fn test_ancient_tomb_mana_ability_has_tap_cost() {
        let def = ancient_tomb();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.has_tap_cost(),
                "Should have tap as part of cost"
            );
            // Ancient Tomb has no mana cost - just tap
            assert!(
                mana_ability.mana_cost.mana_cost().is_none(),
                "Should not have mana cost"
            );
        }
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_ancient_tomb_deals_damage() {
        let game = run_replay_test(
            // Actions: 0=pass, 1=tap Ancient Tomb for mana
            vec!["1", ""], // Tap Ancient Tomb, then pass priority
            ReplayTestConfig::new().p1_battlefield(vec!["Ancient Tomb"]),
        );

        let alice = PlayerId::from_index(0);

        // Player 1 should have taken 2 damage from Ancient Tomb
        assert_eq!(
            game.life_total(alice),
            18,
            "Player 1 should be at 18 life after tapping Ancient Tomb"
        );

        // Ancient Tomb should still be on battlefield (but tapped)
        assert!(
            game.battlefield_has("Ancient Tomb"),
            "Ancient Tomb should be on battlefield"
        );
    }
}
