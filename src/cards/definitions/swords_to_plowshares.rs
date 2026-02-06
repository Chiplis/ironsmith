//! Swords to Plowshares card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Swords to Plowshares - {W}
/// Instant
/// Exile target creature. Its controller gains life equal to its power.
pub fn swords_to_plowshares() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Swords to Plowshares")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Exile target creature. Its controller gains life equal to its power.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    #[test]
    fn test_swords_to_plowshares() {
        let def = swords_to_plowshares();
        assert_eq!(def.name(), "Swords to Plowshares");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 1);
        assert!(def.spell_effect.is_some());
        assert_eq!(def.spell_effect.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_swords_to_plowshares_exiles_and_gains_life() {
        // Test that the controller of the exiled creature gains life equal to its power
        use crate::cards::definitions::giant_spider;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Giant Spider (2/4) controlled by Bob on the battlefield
        let spider_def = giant_spider();
        let creature_id = game.create_object_from_definition(&spider_def, bob, Zone::Battlefield);

        // Verify creature power is 2
        assert_eq!(game.object(creature_id).unwrap().power(), Some(2));

        // Set up execution context with the creature as target
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx = ctx.with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Snapshot targets BEFORE executing effects (this is the key!)
        ctx.snapshot_targets(&game);

        // Get the spell effects
        let def = swords_to_plowshares();
        let effects = def.spell_effect.as_ref().unwrap();

        // Bob's starting life
        assert_eq!(game.player(bob).unwrap().life, 20);

        // Execute the exile effect
        let result = execute_effect(&mut game, &effects[0], &mut ctx);
        assert!(result.is_ok(), "Exile effect should succeed");

        // Creature should be in exile (note: zone change creates new object ID per rule 400.7)
        // So we verify by checking that the original ID is gone and the creature is in exile zone
        assert!(
            game.object(creature_id).is_none(),
            "Old ID should no longer exist after zone change"
        );

        // Find the creature in exile by name
        let exiled_creature = game
            .exile
            .iter()
            .find_map(|&id| game.object(id))
            .filter(|obj| obj.name == "Giant Spider");
        assert!(exiled_creature.is_some(), "Giant Spider should be in exile");

        // Execute the life gain effect (should use LKI for power and controller)
        let result = execute_effect(&mut game, &effects[1], &mut ctx);
        assert!(result.is_ok(), "Life gain effect should succeed using LKI");

        // Bob (the creature's controller) should have gained 2 life (Giant Spider's power)
        assert_eq!(
            game.player(bob).unwrap().life,
            22,
            "Bob should have gained 2 life (equal to Giant Spider's power)"
        );

        // Alice (the caster) should not have gained life
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "Alice should not have gained life"
        );
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Swords to Plowshares exiling a creature and its controller gaining life.
    ///
    /// Swords to Plowshares: {W} instant
    /// Exile target creature. Its controller gains life equal to its power.
    ///
    /// Scenario: Alice casts StP targeting Bob's Giant Spider (2/4).
    /// Expected: Spider is exiled, Bob gains 2 life.
    #[test]
    fn test_replay_swords_to_plowshares_exile_life_gain() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Swords to Plowshares (index 1, after PassPriority at 0)
                "0", // Target Giant Spider (Bob's creature)
                "0", // Tap Plains for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Swords to Plowshares"])
                .p1_battlefield(vec!["Plains"])
                .p2_battlefield(vec!["Giant Spider"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Giant Spider should NOT be on battlefield anymore
        assert!(
            !game.battlefield_has("Giant Spider"),
            "Giant Spider should have been exiled"
        );

        // Giant Spider should be in exile
        let spider_in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Giant Spider")
                .unwrap_or(false)
        });
        assert!(spider_in_exile, "Giant Spider should be in exile zone");

        // Bob (the creature's controller) should have gained 2 life (Giant Spider's power)
        assert_eq!(
            game.life_total(bob),
            22,
            "Bob should be at 22 life (gained 2 from Giant Spider's power)"
        );

        // Alice (the caster) should not have gained life
        assert_eq!(
            game.life_total(alice),
            20,
            "Alice should still be at 20 life"
        );
    }

    /// Tests Swords to Plowshares on own creature (self-exile for life gain).
    ///
    /// Scenario: Alice casts StP targeting her own Grizzly Bears (2/2).
    /// Expected: Bears is exiled, Alice gains 2 life.
    #[test]
    fn test_replay_swords_to_plowshares_self_target() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Swords to Plowshares
                "0", // Target own Grizzly Bears
                "0", // Tap Plains for mana
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Swords to Plowshares"])
                .p1_battlefield(vec!["Plains", "Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Grizzly Bears should be in exile
        let bears_in_exile = game.exile.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(bears_in_exile, "Grizzly Bears should be in exile zone");

        // Alice should have gained 2 life (Grizzly Bears' power)
        assert_eq!(
            game.life_total(alice),
            22,
            "Alice should be at 22 life (gained 2 from own creature)"
        );

        // Bob should be unchanged
        assert_eq!(game.life_total(bob), 20, "Bob should still be at 20 life");
    }
}
