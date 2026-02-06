//! Card definition for Command Tower.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the Command Tower card definition.
///
/// Command Tower
/// Land
/// {T}: Add one mana of any color in your commander's color identity.
pub fn command_tower() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Command Tower")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add one mana of any color in your commander's color identity.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::Color;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::zone::Zone;

    /// Helper to create a basic game state for testing.
    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_command_tower_basic_properties() {
        let def = command_tower();

        // Check name
        assert_eq!(def.name(), "Command Tower");

        // Check it's a land
        assert!(def.card.is_land());
        assert!(def.card.card_types.contains(&CardType::Land));

        // No mana cost (it's a land)
        assert!(def.card.mana_cost.is_none());

        // Check it's colorless (no mana cost)
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_command_tower_mana_value_is_zero() {
        let def = command_tower();

        // Lands have mana value 0
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_command_tower_is_not_creature() {
        let def = command_tower();

        // Command Tower is not a creature
        assert!(!def.card.is_creature());
    }

    #[test]
    fn test_command_tower_has_no_subtypes() {
        let def = command_tower();

        // Command Tower is not a basic land, has no subtypes
        assert!(def.card.subtypes.is_empty());
    }

    #[test]
    fn test_command_tower_has_mana_ability() {
        let def = command_tower();

        // Should have exactly one ability
        assert_eq!(def.abilities.len(), 1);

        // The ability should be a mana ability
        assert!(def.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_command_tower_mana_ability_structure() {
        let def = command_tower();

        let ability = &def.abilities[0];
        match &ability.kind {
            AbilityKind::Mana(mana_ability) => {
                // Should have effects (not fixed mana)
                assert!(mana_ability.effects.is_some());

                let effects = mana_ability.effects.as_ref().unwrap();
                assert_eq!(
                    effects.len(),
                    1,
                    "Should have 1 effect: add mana from commander identity"
                );

                // Effect should be AddManaFromCommanderColorIdentityEffect
                assert!(
                    format!("{:?}", effects[0]).contains("AddManaFromCommanderColorIdentityEffect"),
                    "Effect should be AddManaFromCommanderColorIdentityEffect"
                );
            }
            _ => panic!("Expected mana ability"),
        }
    }

    #[test]
    fn test_command_tower_mana_ability_has_tap_cost() {
        let def = command_tower();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.has_tap_cost(),
                "Should have tap as part of cost"
            );
            // Command Tower has no mana cost - just tap
            assert!(
                mana_ability.mana_cost.mana_cost().is_none(),
                "Should not have mana cost"
            );
        }
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_command_tower_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Command Tower on the battlefield
        let def = command_tower();
        let tower_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&tower_id));

        // Verify the object has the mana ability
        let obj = game.object(tower_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);
        assert!(obj.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_command_tower_activation_requires_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Command Tower on the battlefield
        let def = command_tower();
        let tower_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Tap the land
        game.tap(tower_id);

        // The mana ability has a tap cost, so it can't be activated while tapped
        assert!(game.is_tapped(tower_id));

        // The ability's cost requires tapping, which can't be done if already tapped
        let obj = game.object(tower_id).unwrap();
        if let AbilityKind::Mana(mana_ability) = &obj.abilities[0].kind {
            assert!(
                mana_ability.has_tap_cost(),
                "Mana ability should require tapping"
            );
        }
    }

    #[test]
    fn test_command_tower_not_affected_by_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Command Tower on the battlefield
        let def = command_tower();
        let tower_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(tower_id).unwrap();

        // Lands are not affected by summoning sickness
        // The mana ability should be usable immediately
        assert!(!obj.is_creature(), "Command Tower is not a creature");

        // Verify it has the mana ability ready to use
        assert!(obj.abilities[0].is_mana_ability());
    }

    // =========================================================================
    // Commander Color Identity Tests
    // =========================================================================

    #[test]
    fn test_command_tower_effect_uses_commander_identity() {
        let def = command_tower();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            let effects = mana_ability.effects.as_ref().unwrap();

            // Verify the debug output shows the correct effect type
            let debug_str = format!("{:?}", effects[0]);
            assert!(
                debug_str.contains("AddManaFromCommanderColorIdentityEffect"),
                "Effect should be AddManaFromCommanderColorIdentityEffect"
            );
            assert!(
                debug_str.contains("amount: Fixed(1)"),
                "Should add exactly 1 mana"
            );
        }
    }

    #[test]
    fn test_command_tower_with_no_commander() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        // Alice has no commander set - Command Tower produces no colored mana
        let identity = game.get_commander_color_identity(alice);
        assert!(
            identity.is_empty(),
            "Without a commander, identity should be colorless"
        );
    }

    #[test]
    fn test_command_tower_with_white_black_commander() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a white/black commander
        let commander_card = CardBuilder::new(CardId::new(), "Orzhov Commander")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::White],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let commander_id = game.create_object_from_card(&commander_card, alice, Zone::Command);

        // Set the commander
        if let Some(player) = game.player_mut(alice) {
            player.add_commander(commander_id);
        }

        // Get the color identity
        let identity = game.get_commander_color_identity(alice);

        // Command Tower would produce white or black mana
        assert!(identity.contains(Color::White));
        assert!(identity.contains(Color::Black));
        assert!(!identity.contains(Color::Blue));
        assert!(!identity.contains(Color::Red));
        assert!(!identity.contains(Color::Green));
        assert_eq!(identity.count(), 2);
    }

    #[test]
    fn test_command_tower_with_five_color_commander() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a five-color commander
        let commander_card = CardBuilder::new(CardId::new(), "Five Color Commander")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::White],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Red],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();

        let commander_id = game.create_object_from_card(&commander_card, alice, Zone::Command);

        // Set the commander
        if let Some(player) = game.player_mut(alice) {
            player.add_commander(commander_id);
        }

        // Get the color identity - should include all five colors
        let identity = game.get_commander_color_identity(alice);

        // Command Tower produces any color
        assert!(identity.contains(Color::White));
        assert!(identity.contains(Color::Blue));
        assert!(identity.contains(Color::Black));
        assert!(identity.contains(Color::Red));
        assert!(identity.contains(Color::Green));
        assert_eq!(identity.count(), 5);
    }

    #[test]
    fn test_command_tower_with_colorless_commander() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a colorless commander (generic mana cost only)
        let commander_card = CardBuilder::new(CardId::new(), "Colorless Commander")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(6)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(6, 6))
            .build();

        let commander_id = game.create_object_from_card(&commander_card, alice, Zone::Command);

        // Set the commander
        if let Some(player) = game.player_mut(alice) {
            player.add_commander(commander_id);
        }

        // Get the color identity - should be colorless
        let identity = game.get_commander_color_identity(alice);

        // Command Tower produces no colored mana (colorless identity)
        assert_eq!(
            identity.count(),
            0,
            "Colorless commander has no color identity"
        );
    }

    #[test]
    fn test_command_tower_oracle_text() {
        let def = command_tower();

        assert!(def.card.oracle_text.contains("Add one mana"));
        assert!(def.card.oracle_text.contains("commander's color identity"));
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_command_tower_play() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Command Tower
            ],
            ReplayTestConfig::new().p1_hand(vec!["Command Tower"]),
        );

        // Command Tower should be on the battlefield
        assert!(
            game.battlefield_has("Command Tower"),
            "Command Tower should be on battlefield after playing"
        );
    }
}
