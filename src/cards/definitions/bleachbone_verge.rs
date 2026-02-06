//! Bleachbone Verge card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::CardType;

/// Bleachbone Verge - Land
/// {T}: Add {B}.
/// {T}: Add {W}. Activate only if you control a Plains or a Swamp.
pub fn bleachbone_verge() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Bleachbone Verge")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}: Add {B}.\n{T}: Add {W}. Activate only if you control a Plains or a Swamp.",
        )
        .expect("Card text should be supported")
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
    fn test_bleachbone_verge_basic_properties() {
        let def = bleachbone_verge();
        assert_eq!(def.name(), "Bleachbone Verge");
        assert!(def.card.is_land());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_bleachbone_verge_is_not_basic() {
        let def = bleachbone_verge();
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[test]
    fn test_bleachbone_verge_has_two_mana_abilities() {
        let def = bleachbone_verge();
        assert_eq!(def.abilities.len(), 2);
        assert!(def.abilities[0].is_mana_ability());
        assert!(def.abilities[1].is_mana_ability());
    }

    // ========================================
    // First Ability (Black Mana) Tests
    // ========================================

    #[test]
    fn test_first_ability_produces_black_mana() {
        let def = bleachbone_verge();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::Black]);
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_first_ability_is_unconditional() {
        let def = bleachbone_verge();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.activation_condition.is_none(),
                "First ability should be unconditional"
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_first_ability_requires_tap() {
        let def = bleachbone_verge();

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
        let def = bleachbone_verge();

        let ability = &def.abilities[1];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::White]);
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_second_ability_has_condition() {
        let def = bleachbone_verge();

        let ability = &def.abilities[1];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.activation_condition.is_some(),
                "Second ability should have activation condition"
            );

            if let Some(ManaAbilityCondition::ControlLandWithSubtype(subtypes)) =
                &mana_ability.activation_condition
            {
                assert!(subtypes.contains(&Subtype::Plains));
                assert!(subtypes.contains(&Subtype::Swamp));
            } else {
                panic!("Expected ControlLandWithSubtype condition");
            }
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_second_ability_requires_tap() {
        let def = bleachbone_verge();

        let ability = &def.abilities[1];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    // ========================================
    // Activation Condition Tests
    // ========================================

    #[test]
    fn test_black_ability_can_activate_without_other_lands() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge on the battlefield (no other lands)
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Should be able to activate the black mana ability (index 0)
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 0,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_ok(),
            "Should be able to activate black mana ability without other lands"
        );
    }

    #[test]
    fn test_white_ability_cannot_activate_without_plains_or_swamp() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge on the battlefield (no other lands)
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Should NOT be able to activate the white mana ability (index 1)
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 1,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_err(),
            "Should NOT be able to activate white mana ability without Plains or Swamp"
        );
    }

    #[test]
    fn test_white_ability_can_activate_with_plains() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge and a Plains
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _plains_id = create_land(&mut game, "Plains", vec![Subtype::Plains], alice);

        // Should be able to activate the white mana ability
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 1,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_ok(),
            "Should be able to activate white mana ability with Plains"
        );
    }

    #[test]
    fn test_white_ability_can_activate_with_swamp() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge and a Swamp
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _swamp_id = create_land(&mut game, "Swamp", vec![Subtype::Swamp], alice);

        // Should be able to activate the white mana ability
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 1,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_ok(),
            "Should be able to activate white mana ability with Swamp"
        );
    }

    #[test]
    fn test_white_ability_can_activate_with_dual_land() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge and a dual land with both types
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _dual_id = create_land(
            &mut game,
            "Scrubland",
            vec![Subtype::Plains, Subtype::Swamp],
            alice,
        );

        // Should be able to activate the white mana ability
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 1,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_ok(),
            "Should be able to activate white mana ability with dual land"
        );
    }

    #[test]
    fn test_opponent_plains_does_not_enable_white_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Bleachbone Verge for Alice
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Bob controls a Plains (not Alice)
        let _bob_plains = create_land(&mut game, "Plains", vec![Subtype::Plains], bob);

        // Alice should NOT be able to activate the white mana ability
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 1,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_err(),
            "Opponent's Plains should not enable white mana ability"
        );
    }

    // ========================================
    // Tap State Tests
    // ========================================

    #[test]
    fn test_cannot_activate_if_already_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge and tap it
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.tap(verge_id);

        // Create a Plains to enable the white ability
        let _plains_id = create_land(&mut game, "Plains", vec![Subtype::Plains], alice);

        // Should NOT be able to activate either ability while tapped
        let black_action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 0,
        };
        let white_action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 1,
        };

        assert!(can_perform_check(&black_action, &game, alice).is_err());
        assert!(can_perform_check(&white_action, &game, alice).is_err());
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_bleachbone_verge_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge on the battlefield
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&verge_id));

        // Verify the object has both mana abilities
        let obj = game.object(verge_id).unwrap();
        assert_eq!(obj.abilities.len(), 2);
        assert!(obj.abilities[0].is_mana_ability());
        assert!(obj.abilities[1].is_mana_ability());
    }

    #[test]
    fn test_summoning_sickness_irrelevant_for_lands() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge on the battlefield (will have summoning sickness flag set)
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(verge_id).unwrap();

        // Lands are not creatures, so summoning sickness doesn't affect mana abilities
        assert!(!obj.is_creature());

        // Black ability should be activatable immediately
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 0,
        };
        assert!(can_perform_check(&action, &game, alice).is_ok());
    }

    // ========================================
    // Land Type Edge Cases
    // ========================================

    #[test]
    fn test_forest_does_not_enable_white_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Bleachbone Verge and a Forest
        let def = bleachbone_verge();
        let verge_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _forest_id = create_land(&mut game, "Forest", vec![Subtype::Forest], alice);

        // Should NOT be able to activate the white mana ability (Forest is not Plains or Swamp)
        let action = SpecialAction::ActivateManaAbility {
            permanent_id: verge_id,
            ability_index: 1,
        };
        let result = can_perform_check(&action, &game, alice);
        assert!(
            result.is_err(),
            "Forest should not enable white mana ability"
        );
    }

    #[test]
    fn test_multiple_verges_condition_check() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two Bleachbone Verges (each can be used as a condition for the other!)
        // But wait - Bleachbone Verge itself doesn't have Plains or Swamp subtype
        let def = bleachbone_verge();
        let verge1_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let verge2_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Neither should enable the other's white ability since neither has Plains or Swamp subtype
        let action1 = SpecialAction::ActivateManaAbility {
            permanent_id: verge1_id,
            ability_index: 1,
        };
        let action2 = SpecialAction::ActivateManaAbility {
            permanent_id: verge2_id,
            ability_index: 1,
        };

        assert!(can_perform_check(&action1, &game, alice).is_err());
        assert!(can_perform_check(&action2, &game, alice).is_err());
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_bleachbone_verge_play() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Bleachbone Verge
            ],
            ReplayTestConfig::new().p1_hand(vec!["Bleachbone Verge"]),
        );

        assert!(
            game.battlefield_has("Bleachbone Verge"),
            "Bleachbone Verge should be on battlefield after playing"
        );
    }
}
