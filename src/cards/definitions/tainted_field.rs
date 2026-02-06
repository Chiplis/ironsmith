//! Tainted Field card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Tainted Field card definition.
///
/// Tainted Field
/// Land
/// {T}: Add {C}.
/// {T}: Add {W} or {B}. Activate only if you control a Swamp.
pub fn tainted_field() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Tainted Field")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {C}.\n{T}: Add {W} or {B}. Activate only if you control a Swamp.")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{AbilityKind, ManaAbilityCondition};
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::object::Object;
    use crate::special_actions::{SpecialAction, can_perform_check};
    use crate::types::Subtype;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_land(
        game: &mut GameState,
        name: &str,
        subtypes: Vec<Subtype>,
        owner: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Land])
            .subtypes(subtypes)
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_tainted_field_basic_properties() {
        let def = tainted_field();
        assert_eq!(def.name(), "Tainted Field");
        assert!(def.card.is_land());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_tainted_field_is_not_basic() {
        let def = tainted_field();
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[test]
    fn test_tainted_field_has_three_mana_abilities() {
        let def = tainted_field();
        // {C} unconditional, {W} conditional, {B} conditional
        assert_eq!(def.abilities.len(), 3);
        assert!(def.abilities.iter().all(|a| a.is_mana_ability()));
    }

    // ========================================
    // First Ability (Colorless Mana) Tests
    // ========================================

    #[test]
    fn test_first_ability_produces_colorless_mana() {
        let def = tainted_field();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::Colorless]);
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_first_ability_is_unconditional() {
        let def = tainted_field();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.activation_condition.is_none(),
                "First ability (colorless) should be unconditional"
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_first_ability_requires_tap() {
        let def = tainted_field();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    // ========================================
    // Second Ability (White Mana) Tests
    // ========================================

    #[test]
    fn test_second_ability_produces_white_mana() {
        let def = tainted_field();

        let ability = &def.abilities[1];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::White]);
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_second_ability_has_swamp_condition() {
        let def = tainted_field();

        let ability = &def.abilities[1];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.activation_condition.is_some(),
                "White ability should have activation condition"
            );

            if let Some(ManaAbilityCondition::ControlLandWithSubtype(subtypes)) =
                &mana_ability.activation_condition
            {
                assert!(subtypes.contains(&Subtype::Swamp));
                assert!(
                    !subtypes.contains(&Subtype::Plains),
                    "Should only require Swamp, not Plains"
                );
            } else {
                panic!("Expected ControlLandWithSubtype condition");
            }
        } else {
            panic!("Expected mana ability");
        }
    }

    // ========================================
    // Third Ability (Black Mana) Tests
    // ========================================

    #[test]
    fn test_third_ability_produces_black_mana() {
        let def = tainted_field();

        let ability = &def.abilities[2];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::Black]);
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_third_ability_has_swamp_condition() {
        let def = tainted_field();

        let ability = &def.abilities[2];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.activation_condition.is_some(),
                "Black ability should have activation condition"
            );

            if let Some(ManaAbilityCondition::ControlLandWithSubtype(subtypes)) =
                &mana_ability.activation_condition
            {
                assert!(subtypes.contains(&Subtype::Swamp));
            } else {
                panic!("Expected ControlLandWithSubtype condition");
            }
        } else {
            panic!("Expected mana ability");
        }
    }

    // ========================================
    // Activation Condition Tests
    // ========================================

    #[test]
    fn test_colorless_ability_can_activate_without_swamp() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tainted Field on the battlefield (no other lands)
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Should be able to activate the colorless mana ability (index 0)
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 0,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_ok(),
            "Should be able to activate colorless mana ability without Swamp"
        );
    }

    #[test]
    fn test_white_ability_cannot_activate_without_swamp() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tainted Field on the battlefield (no other lands)
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Should NOT be able to activate the white mana ability (index 1)
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 1,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_err(),
            "Should NOT be able to activate white mana ability without Swamp"
        );
    }

    #[test]
    fn test_black_ability_cannot_activate_without_swamp() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tainted Field on the battlefield (no other lands)
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Should NOT be able to activate the black mana ability (index 2)
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 2,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_err(),
            "Should NOT be able to activate black mana ability without Swamp"
        );
    }

    #[test]
    fn test_colored_abilities_can_activate_with_swamp() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tainted Field and a Swamp
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _swamp_id = create_land(&mut game, "Swamp", vec![Subtype::Swamp], alice);

        // Should be able to activate both white and black mana abilities
        let white_action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 1,
        };
        let black_action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 2,
        };

        assert!(
            can_perform_check(&white_action, &game, alice).is_ok(),
            "Should be able to activate white with Swamp"
        );
        assert!(
            can_perform_check(&black_action, &game, alice).is_ok(),
            "Should be able to activate black with Swamp"
        );
    }

    #[test]
    fn test_plains_does_not_enable_colored_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tainted Field and a Plains (but no Swamp!)
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _plains_id = create_land(&mut game, "Plains", vec![Subtype::Plains], alice);

        // Should NOT be able to activate colored mana abilities
        let white_action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 1,
        };
        let black_action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 2,
        };

        assert!(
            can_perform_check(&white_action, &game, alice).is_err(),
            "Plains should not enable white ability"
        );
        assert!(
            can_perform_check(&black_action, &game, alice).is_err(),
            "Plains should not enable black ability"
        );
    }

    #[test]
    fn test_dual_land_with_swamp_enables_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tainted Field and Scrubland (which has Swamp subtype)
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _scrubland_id = create_land(
            &mut game,
            "Scrubland",
            vec![Subtype::Plains, Subtype::Swamp],
            alice,
        );

        // Should be able to activate colored mana abilities (Scrubland has Swamp)
        let white_action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 1,
        };
        assert!(
            can_perform_check(&white_action, &game, alice).is_ok(),
            "Dual land with Swamp should enable abilities"
        );
    }

    #[test]
    fn test_opponent_swamp_does_not_enable_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Tainted Field for Alice
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Bob controls a Swamp (not Alice)
        let _bob_swamp = create_land(&mut game, "Swamp", vec![Subtype::Swamp], bob);

        // Alice should NOT be able to activate the colored mana abilities
        let white_action = SpecialAction::ActivateManaAbility {
            permanent_id: field_id,
            ability_index: 1,
        };
        let result = can_perform_check(&white_action, &game, alice);
        assert!(
            result.is_err(),
            "Opponent's Swamp should not enable abilities"
        );
    }

    // ========================================
    // Tap State Tests
    // ========================================

    #[test]
    fn test_cannot_activate_if_already_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tainted Field and tap it
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.tap(field_id);

        // Create a Swamp to enable the colored abilities
        let _swamp_id = create_land(&mut game, "Swamp", vec![Subtype::Swamp], alice);

        // Should NOT be able to activate any ability while tapped
        for index in 0..3 {
            let action = SpecialAction::ActivateManaAbility {
                permanent_id: field_id,
                ability_index: index,
            };
            assert!(can_perform_check(&action, &game, alice).is_err());
        }
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_tainted_field_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tainted Field on the battlefield
        let def = tainted_field();
        let field_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&field_id));

        // Verify the object has all three mana abilities
        let obj = game.object(field_id).unwrap();
        assert_eq!(obj.abilities.len(), 3);
        assert!(obj.abilities.iter().all(|a| a.is_mana_ability()));
    }

    #[test]
    fn test_oracle_text() {
        let def = tainted_field();

        assert!(def.card.oracle_text.contains("Add {C}"));
        assert!(def.card.oracle_text.contains("Add {W} or {B}"));
        assert!(def.card.oracle_text.contains("control a Swamp"));
    }

    // ========================================
    // Replay Tests
    // ========================================

    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    /// Tests tapping Tainted Field for colorless mana (unconditional).
    ///
    /// Tainted Field: Land
    /// {T}: Add {C}.
    /// {T}: Add {W} or {B}. Activate only if you control a Swamp.
    #[test]
    fn test_replay_tainted_field_colorless_mana() {
        let game = run_replay_test(
            vec![
                "1", // Activate first ability (colorless, unconditional)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Tainted Field"]),
        );

        // Check colorless mana
        let alice = PlayerId::from_index(0);
        let alice_player = game.player(alice).unwrap();
        assert!(
            alice_player.mana_pool.colorless >= 1,
            "Should have colorless mana in pool"
        );
    }

    /// Tests tapping Tainted Field with a Swamp to enable colored mana.
    #[test]
    fn test_replay_tainted_field_colored_mana_with_swamp() {
        let game = run_replay_test(
            vec![
                "2", // Activate second ability (white, conditional - requires Swamp)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Tainted Field", "Swamp"]),
        );

        // Check white mana
        let alice = PlayerId::from_index(0);
        let alice_player = game.player(alice).unwrap();
        assert!(
            alice_player.mana_pool.white >= 1,
            "Should have white mana in pool"
        );
    }
}
