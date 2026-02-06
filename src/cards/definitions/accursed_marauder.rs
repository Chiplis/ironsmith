//! Accursed Marauder card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Accursed Marauder - {1}{B}
/// Creature — Zombie Warrior
/// 2/1
/// When this creature enters, each player sacrifices a nontoken creature of their choice.
pub fn accursed_marauder() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Accursed Marauder")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "When this creature enters, each player sacrifices a nontoken creature of their choice.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::PowerToughness;
    use crate::color::Color;
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::zone::Zone;

    /// Helper to create a basic game state for testing.
    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    /// Helper to create a simple creature on the battlefield.
    fn create_creature(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        use crate::card::CardBuilder;
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn test_accursed_marauder_basic_properties() {
        let def = accursed_marauder();

        // Check name
        assert_eq!(def.name(), "Accursed Marauder");

        // Check it's a creature
        assert!(def.is_creature());

        // Check mana cost - {1}{B} = mana value 2
        assert_eq!(def.card.mana_value(), 2);

        // Check colors - should be black
        assert!(def.card.colors().contains(Color::Black));
        assert_eq!(def.card.colors().count(), 1);

        // Check subtypes
        assert!(def.card.has_subtype(Subtype::Zombie));
        assert!(def.card.has_subtype(Subtype::Warrior));

        // Check power/toughness
        let pt = def.card.power_toughness.unwrap();
        assert_eq!(pt.power.base_value(), 2);
        assert_eq!(pt.toughness.base_value(), 1);
    }

    #[test]
    fn test_accursed_marauder_has_etb_trigger() {
        let def = accursed_marauder();

        // Should have exactly one ability (the ETB trigger)
        assert_eq!(def.abilities.len(), 1);

        // Check that it's a triggered ability with ETB trigger (now using Trigger struct)
        let ability = &def.abilities[0];
        match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                assert!(
                    triggered.trigger.display().contains("enters"),
                    "Should trigger on entering battlefield"
                );

                // Check that the effect is present (now uses declarative ForPlayersEffect composition)
                assert_eq!(triggered.effects.len(), 1);
                let debug_str = format!("{:?}", &triggered.effects[0]);
                assert!(debug_str.contains("ForPlayersEffect"));
            }
            _ => panic!("Expected triggered ability"),
        }
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_accursed_marauder_etb_sacrifices_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a creature for Alice
        let alice_creature = create_creature(&mut game, alice, "Alice's Bear", 2, 2);

        // Create a creature for Bob
        let bob_creature = create_creature(&mut game, bob, "Bob's Bear", 2, 2);

        // Create Accursed Marauder on Alice's side
        let marauder_def = accursed_marauder();
        let marauder_id =
            game.create_object_from_definition(&marauder_def, alice, Zone::Battlefield);

        // Verify initial state - all creatures are on battlefield
        assert!(
            game.battlefield.contains(&alice_creature),
            "Alice's creature should be on battlefield initially"
        );
        assert!(
            game.battlefield.contains(&bob_creature),
            "Bob's creature should be on battlefield initially"
        );
        assert!(
            game.battlefield.contains(&marauder_id),
            "Marauder should be on battlefield"
        );

        // Extract the EachPlayerSacrifices effect from the ability
        let effect = &marauder_def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &effect.kind {
            let sacrifice_effect = &triggered.effects[0];

            // Set up execution context
            let mut ctx = ExecutionContext::new_default(marauder_id, alice).with_targets(vec![]);

            // Execute the effect
            let result = execute_effect(&mut game, sacrifice_effect, &mut ctx);
            assert!(result.is_ok(), "Effect execution should succeed");

            // Verify that 2 creatures were sacrificed (one from each player)
            if let Ok(outcome) = result {
                if let crate::effect::EffectResult::Count(count) = outcome.result {
                    assert_eq!(
                        count, 2,
                        "Should sacrifice exactly 2 creatures (one from each player)"
                    );
                } else {
                    panic!("Expected Count result from EachPlayerSacrifices");
                }
            } else {
                panic!("Effect execution failed");
            }

            // Verify Alice's creature is now in the graveyard
            let alice_player = game.player(alice).expect("Alice should exist");
            assert!(
                alice_player.graveyard.iter().any(|&id| {
                    game.object(id)
                        .map_or(false, |obj| obj.name == "Alice's Bear")
                }),
                "Alice's creature should be in graveyard after sacrifice"
            );

            // Verify Bob's creature is now in the graveyard
            let bob_player = game.player(bob).expect("Bob should exist");
            assert!(
                bob_player.graveyard.iter().any(|&id| {
                    game.object(id)
                        .map_or(false, |obj| obj.name == "Bob's Bear")
                }),
                "Bob's creature should be in graveyard after sacrifice"
            );

            // Verify Marauder is still on battlefield (it's a nontoken creature but
            // Alice would sacrifice her only other creature)
            assert!(
                game.battlefield.contains(&marauder_id),
                "Marauder should still be on battlefield"
            );
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_accursed_marauder_etb_does_not_sacrifice_tokens() {
        use crate::object::ObjectKind;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a nontoken creature for Alice
        let _alice_creature = create_creature(&mut game, alice, "Alice's Bear", 2, 2);

        // Create a token creature for Bob (should NOT be sacrificed)
        let bob_token = create_creature(&mut game, bob, "Bob's Token", 1, 1);
        // Mark it as a token
        if let Some(obj) = game.object_mut(bob_token) {
            obj.kind = ObjectKind::Token;
        }

        // Create Accursed Marauder
        let marauder_def = accursed_marauder();
        let marauder_id =
            game.create_object_from_definition(&marauder_def, alice, Zone::Battlefield);

        // Extract and execute the effect
        let effect = &marauder_def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &effect.kind {
            let sacrifice_effect = &triggered.effects[0];

            let mut ctx = ExecutionContext::new_default(marauder_id, alice).with_targets(vec![]);

            let result = execute_effect(&mut game, sacrifice_effect, &mut ctx);
            assert!(result.is_ok());

            // Only Alice's creature should be sacrificed (Bob has no nontoken creatures)
            if let Ok(outcome) = result {
                if let crate::effect::EffectResult::Count(count) = outcome.result {
                    assert_eq!(
                        count, 1,
                        "Should only sacrifice 1 creature (Alice's nontoken)"
                    );
                }
            }

            // Alice's creature should be in graveyard
            let alice_player = game.player(alice).expect("Alice should exist");
            assert!(
                alice_player.graveyard.iter().any(|&id| {
                    game.object(id)
                        .map_or(false, |obj| obj.name == "Alice's Bear")
                }),
                "Alice's creature should be in graveyard"
            );

            // Bob's token should still be on battlefield
            assert!(
                game.battlefield.contains(&bob_token),
                "Token should not be sacrificed"
            );
        }
    }

    #[test]
    fn test_accursed_marauder_etb_with_no_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Accursed Marauder with no other creatures on battlefield
        let marauder_def = accursed_marauder();
        let marauder_id =
            game.create_object_from_definition(&marauder_def, alice, Zone::Battlefield);

        // Extract and execute the effect
        let effect = &marauder_def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &effect.kind {
            let sacrifice_effect = &triggered.effects[0];

            let mut ctx = ExecutionContext::new_default(marauder_id, alice).with_targets(vec![]);

            let result = execute_effect(&mut game, sacrifice_effect, &mut ctx);
            assert!(result.is_ok());

            // No creatures should be sacrificed (only the marauder exists, and
            // each player chooses which creature to sacrifice - if they have none, nothing happens)
            if let Ok(outcome) = result {
                if let crate::effect::EffectResult::Count(count) = outcome.result {
                    // The marauder itself could be sacrificed by Alice
                    assert!(
                        count <= 1,
                        "At most 1 creature sacrificed (the marauder itself by Alice)"
                    );
                }
            }
        }
    }

    /// Tests casting Accursed Marauder with another creature to sacrifice.
    ///
    /// Accursed Marauder: {1}{B} creature 2/1
    /// When this creature enters, each player sacrifices a nontoken creature.
    ///
    /// Note: If cast with no other creatures, the Marauder itself gets sacrificed
    /// because it's the only nontoken creature Alice controls when the ETB resolves.
    /// So we give Alice a Grizzly Bears to sacrifice instead.
    #[test]
    fn test_replay_accursed_marauder_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Accursed Marauder (index 1, since play land is 0)
                "0", // Tap Swamp 1
                "0", // Tap Swamp 2
                     // ETB trigger resolves automatically via auto-pass
                     // The EachPlayerSacrifices effect finds creatures to sacrifice
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Accursed Marauder"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);

        // Accursed Marauder should be on the battlefield (it stays because Bears was sacrificed)
        assert!(
            game.battlefield_has("Accursed Marauder"),
            "Accursed Marauder should be on battlefield after casting"
        );

        // Grizzly Bears should be in graveyard (sacrificed to the ETB trigger)
        let alice_player = game.player(alice).unwrap();
        let bears_in_graveyard = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(
            bears_in_graveyard,
            "Grizzly Bears should be in graveyard after sacrifice"
        );

        // Verify Marauder P/T
        let marauder_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Accursed Marauder" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(marauder_id) = marauder_id {
            assert_eq!(
                game.calculated_power(marauder_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(marauder_id),
                Some(1),
                "Should have 1 toughness"
            );
        } else {
            panic!("Could not find Accursed Marauder on battlefield");
        }
    }
}
