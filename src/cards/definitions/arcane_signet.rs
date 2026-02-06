//! Card definition for Arcane Signet.

use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Arcane Signet card definition.
///
/// Arcane Signet {2}
/// Artifact
/// {T}: Add one mana of any color in your commander's color identity.
pub fn arcane_signet() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Arcane Signet")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Add one mana of any color in your commander's color identity.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::color::Color;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    /// Helper to create a basic game state for testing.
    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_arcane_signet_basic_properties() {
        let def = arcane_signet();

        // Check name
        assert_eq!(def.name(), "Arcane Signet");

        // Check it's an artifact
        assert!(def.card.is_artifact());
        assert!(def.card.card_types.contains(&CardType::Artifact));

        // Check mana cost is {2}
        assert_eq!(def.card.mana_value(), 2);

        // Check it's colorless
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_arcane_signet_is_not_creature() {
        let def = arcane_signet();

        // Arcane Signet is not a creature
        assert!(!def.card.is_creature());
    }

    #[test]
    fn test_arcane_signet_has_mana_ability() {
        let def = arcane_signet();

        // Should have exactly one ability
        assert_eq!(def.abilities.len(), 1);

        // The ability should be a mana ability
        assert!(def.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_arcane_signet_mana_ability_structure() {
        let def = arcane_signet();

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
    fn test_arcane_signet_mana_ability_has_tap_cost() {
        let def = arcane_signet();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(
                mana_ability.has_tap_cost(),
                "Should have tap as part of cost"
            );
            // Arcane Signet has no mana cost - just tap
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
    fn test_arcane_signet_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Arcane Signet on the battlefield
        let def = arcane_signet();
        let signet_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&signet_id));

        // Verify the object has the mana ability
        let obj = game.object(signet_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);
        assert!(obj.abilities[0].is_mana_ability());
    }

    #[test]
    fn test_arcane_signet_activation_requires_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Arcane Signet on the battlefield
        let def = arcane_signet();
        let signet_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Tap the artifact
        game.tap(signet_id);

        // The mana ability has a tap cost, so it can't be activated while tapped
        assert!(game.is_tapped(signet_id));

        // The ability's cost requires tapping, which can't be done if already tapped
        let obj = game.object(signet_id).unwrap();
        if let AbilityKind::Mana(mana_ability) = &obj.abilities[0].kind {
            assert!(
                mana_ability.has_tap_cost(),
                "Mana ability should require tapping"
            );
        }
    }

    #[test]
    fn test_arcane_signet_not_affected_by_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Arcane Signet on the battlefield
        let def = arcane_signet();
        let signet_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(signet_id).unwrap();

        // Artifacts (non-creatures) are not affected by summoning sickness
        // The mana ability should be usable immediately
        assert!(!obj.is_creature(), "Arcane Signet is not a creature");

        // Verify it has the mana ability ready to use
        assert!(obj.abilities[0].is_mana_ability());
    }

    // =========================================================================
    // Commander Color Identity Tests
    // =========================================================================

    #[test]
    fn test_arcane_signet_effect_uses_commander_identity() {
        let def = arcane_signet();

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
            assert!(
                debug_str.contains("player: You"),
                "Should add mana to controller"
            );
        }
    }

    #[test]
    fn test_commander_color_identity_with_no_commander() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);

        // Alice has no commander set - color identity should be colorless
        let identity = game.get_commander_color_identity(alice);
        assert!(
            identity.is_empty(),
            "Without a commander, identity should be colorless"
        );
    }

    #[test]
    fn test_commander_color_identity_with_commander() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a commander (a white/black creature)
        use crate::card::{CardBuilder, PowerToughness};
        let commander_card = CardBuilder::new(CardId::new(), "Test Commander")
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

        // Should be white and black
        assert!(identity.contains(Color::White));
        assert!(identity.contains(Color::Black));
        assert!(!identity.contains(Color::Blue));
        assert!(!identity.contains(Color::Red));
        assert!(!identity.contains(Color::Green));
        assert_eq!(identity.count(), 2);
    }

    #[test]
    fn test_commander_color_identity_from_oracle_text() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a commander with mana symbols in oracle text
        // Like a green creature with "{T}: Add {R}" in text
        use crate::card::CardBuilder;
        let commander_card = CardBuilder::new(CardId::new(), "Mana Dork Commander")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .card_types(vec![CardType::Creature])
            .oracle_text("{T}: Add {R}.")
            .build();

        let commander_id = game.create_object_from_card(&commander_card, alice, Zone::Command);

        // Set the commander
        if let Some(player) = game.player_mut(alice) {
            player.add_commander(commander_id);
        }

        // Get the color identity - should include both green (from mana cost) and red (from text)
        let identity = game.get_commander_color_identity(alice);

        assert!(
            identity.contains(Color::Green),
            "Should include green from mana cost"
        );
        assert!(
            identity.contains(Color::Red),
            "Should include red from oracle text"
        );
        assert_eq!(identity.count(), 2);
    }

    #[test]
    fn test_commander_color_identity_five_color() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a five-color commander
        use crate::card::{CardBuilder, PowerToughness};
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

        assert!(identity.contains(Color::White));
        assert!(identity.contains(Color::Blue));
        assert!(identity.contains(Color::Black));
        assert!(identity.contains(Color::Red));
        assert!(identity.contains(Color::Green));
        assert_eq!(identity.count(), 5);
    }

    /// Tests casting Arcane Signet.
    ///
    /// Arcane Signet: {2} artifact
    /// {T}: Add one mana of any color in your commander's color identity.
    #[test]
    fn test_replay_arcane_signet_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Tap Sol Ring for mana (adds 2 colorless)
                "1", // Cast Arcane Signet
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Arcane Signet"])
                .p1_battlefield(vec!["Sol Ring"]),
        );

        // Arcane Signet should be on the battlefield
        assert!(
            game.battlefield_has("Arcane Signet"),
            "Arcane Signet should be on battlefield after casting"
        );
    }
}
