//! High Market card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::CardType;

/// Creates the High Market card definition.
///
/// High Market
/// Land
/// {T}: Add {C}.
/// {T}, Sacrifice a creature: You gain 1 life.
pub fn high_market() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "High Market")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {C}.\n{T}, Sacrifice a creature: You gain 1 life.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Zone;
    use crate::ability::{AbilityKind, ActivationTiming};
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_high_market_basic_properties() {
        let def = high_market();
        assert_eq!(def.name(), "High Market");
        assert!(def.card.is_land());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_high_market_is_not_basic() {
        let def = high_market();
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[test]
    fn test_high_market_has_two_abilities() {
        let def = high_market();
        assert_eq!(def.abilities.len(), 2);
    }

    // ========================================
    // First Ability (Mana) Tests
    // ========================================

    #[test]
    fn test_first_ability_is_mana_ability() {
        let def = high_market();

        let ability = &def.abilities[0];
        assert!(ability.is_mana_ability());
    }

    #[test]
    fn test_first_ability_produces_colorless_mana() {
        let def = high_market();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::Colorless]);
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_first_ability_requires_tap() {
        let def = high_market();

        let ability = &def.abilities[0];
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    // ========================================
    // Second Ability (Sacrifice) Tests
    // ========================================

    #[test]
    fn test_second_ability_is_activated_ability() {
        let def = high_market();

        let ability = &def.abilities[1];
        assert!(matches!(ability.kind, AbilityKind::Activated(_)));
    }

    #[test]
    fn test_second_ability_is_not_mana_ability() {
        let def = high_market();

        let ability = &def.abilities[1];
        assert!(!ability.is_mana_ability());
    }

    #[test]
    fn test_second_ability_requires_tap() {
        let def = high_market();

        let ability = &def.abilities[1];
        if let AbilityKind::Activated(activated) = &ability.kind {
            assert!(activated.has_tap_cost());
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_second_ability_requires_creature_sacrifice() {
        let def = high_market();

        let ability = &def.abilities[1];
        if let AbilityKind::Activated(activated) = &ability.kind {
            // Verify sacrifice is in cost_effects (not TotalCost) so "dies" triggers fire
            assert!(
                !activated.mana_cost.costs().is_empty(),
                "Should have cost_effects for sacrifice"
            );
            // Should have 3 cost_effects: tap + choose + sacrifice
            assert_eq!(
                activated.mana_cost.costs().len(),
                3,
                "Should have tap + choose + sacrifice effects"
            );

            // Check for tap cost
            assert!(activated.has_tap_cost(), "Should have tap cost");

            let debug_str = format!("{:?}", &activated.mana_cost.costs());
            assert!(
                debug_str.contains("ChooseObjectsEffect"),
                "cost_effects should contain choose objects"
            );
            assert!(
                debug_str.contains("SacrificeEffect"),
                "cost_effects should contain sacrifice"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_second_ability_gains_life() {
        let def = high_market();

        let ability = &def.abilities[1];
        if let AbilityKind::Activated(activated) = &ability.kind {
            assert_eq!(activated.effects.len(), 1);
            // Check it's a gain life effect
            let effect_debug = format!("{:?}", activated.effects[0]);
            assert!(
                effect_debug.contains("GainLifeEffect"),
                "Should have gain life effect"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_second_ability_instant_speed() {
        let def = high_market();

        let ability = &def.abilities[1];
        if let AbilityKind::Activated(activated) = &ability.kind {
            assert_eq!(activated.timing, ActivationTiming::AnyTime);
        } else {
            panic!("Expected activated ability");
        }
    }

    #[test]
    fn test_second_ability_has_no_targets() {
        let def = high_market();

        let ability = &def.abilities[1];
        if let AbilityKind::Activated(activated) = &ability.kind {
            assert!(activated.choices.is_empty(), "Gain life doesn't target");
        } else {
            panic!("Expected activated ability");
        }
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_high_market_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create High Market on the battlefield
        let def = high_market();
        let market_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&market_id));

        // Verify the object has both abilities
        let obj = game.object(market_id).unwrap();
        assert_eq!(obj.abilities.len(), 2);
    }

    #[test]
    fn test_high_market_oracle_text() {
        let def = high_market();

        assert!(def.card.oracle_text.contains("Add {C}"));
        assert!(def.card.oracle_text.contains("Sacrifice a creature"));
        assert!(def.card.oracle_text.contains("gain 1 life"));
    }

    #[test]
    fn test_high_market_is_permanent() {
        let def = high_market();

        assert!(def.is_permanent());
    }

    #[test]
    fn test_high_market_is_not_spell() {
        let def = high_market();

        // is_spell checks for instant/sorcery
        assert!(!def.is_spell());
    }

    // ========================================
    // Sacrifice Filter Tests
    // ========================================

    #[test]
    fn test_sacrifice_any_creature_type() {
        let def = high_market();

        let ability = &def.abilities[1];
        if let AbilityKind::Activated(activated) = &ability.kind {
            // The sacrifice is now in cost_effects via TapEffect + ChooseObjectsEffect + SacrificeEffect
            // Verify cost_effects has the expected structure
            assert_eq!(
                activated.mana_cost.costs().len(),
                3,
                "Should have tap + choose + sacrifice effects"
            );

            let debug_str = format!("{:?}", &activated.mana_cost.costs());
            assert!(
                debug_str.contains("TapEffect"),
                "First effect should be tap"
            );
            assert!(
                debug_str.contains("ChooseObjectsEffect"),
                "Second effect should be choose objects"
            );
            assert!(
                debug_str.contains("SacrificeEffect"),
                "Third effect should be sacrifice"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    // ========================================
    // Functional/Execution Tests
    // ========================================

    #[test]
    fn test_gain_life_effect_execution() {
        // Test that the gain life effect actually works when executed
        use crate::executor::ExecutionContext;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let initial_life = game.player(alice).unwrap().life;

        // Get the gain life effect from High Market
        let def = high_market();
        if let AbilityKind::Activated(activated) = &def.abilities[1].kind {
            assert_eq!(activated.effects.len(), 1);

            // Execute the effect (Effect wraps Box<dyn EffectExecutor>, so .0 accesses the inner executor)
            let source = game.new_object_id();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let result = activated.effects[0].0.execute(&mut game, &mut ctx);

            assert!(result.is_ok(), "Effect should execute successfully");
            assert_eq!(
                game.player(alice).unwrap().life,
                initial_life + 1,
                "Should have gained 1 life"
            );
        } else {
            panic!("Expected activated ability");
        }
    }

    // Note: can_pay_cost tests removed since sacrifice is now in cost_effects.
    // The game loop checks cost_effects at execution time, not in can_pay_cost.
    // Replay tests verify the full activation flow works correctly.

    #[test]
    fn test_sacrifice_ability_vs_mana_ability() {
        // Verify the sacrifice ability is NOT a mana ability (uses the stack)
        // while the {T}: Add {C} ability IS a mana ability (doesn't use stack)
        let def = high_market();

        // First ability: mana ability
        assert!(
            def.abilities[0].is_mana_ability(),
            "First ability should be mana ability"
        );

        // Second ability: NOT a mana ability (gains life, not adds mana)
        assert!(
            !def.abilities[1].is_mana_ability(),
            "Second ability should NOT be mana ability"
        );
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests High Market's mana ability.
    ///
    /// High Market: Land
    /// {T}: Add {C}.
    #[test]
    fn test_replay_high_market_mana() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Tap High Market for colorless mana (mana ability)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["High Market"]),
        );

        let alice = PlayerId::from_index(0);

        // High Market should be on battlefield (tapped)
        assert!(
            game.battlefield_has("High Market"),
            "High Market should be on battlefield"
        );

        // Player should have 1 colorless mana in pool
        let player = game.player(alice).unwrap();
        assert_eq!(
            player.mana_pool.colorless, 1,
            "Should have 1 colorless mana from High Market"
        );
    }

    /// Tests High Market's sacrifice ability (gain 1 life).
    ///
    /// {T}, Sacrifice a creature: You gain 1 life.
    #[test]
    fn test_replay_high_market_sacrifice() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Actions: 0=pass, 1=mana ability, 2=sacrifice ability
                "2", // Activate sacrifice ability (not mana ability)
                "0", // Choose Grizzly Bears to sacrifice (auto-passes handle resolution)
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["High Market", "Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);

        // Grizzly Bears should be in graveyard (sacrificed)
        let alice_player = game.player(alice).unwrap();
        let bears_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(
            bears_in_gy,
            "Grizzly Bears should be in graveyard after sacrifice"
        );

        // Alice should have gained 1 life
        assert_eq!(
            game.life_total(alice),
            21,
            "Alice should be at 21 life (gained 1 from High Market)"
        );

        // High Market should still be on battlefield (but tapped)
        assert!(
            game.battlefield_has("High Market"),
            "High Market should still be on battlefield"
        );
    }
}
