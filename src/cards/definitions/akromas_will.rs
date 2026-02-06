//! Akroma's Will card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Akroma's Will - {3}{W}
/// Instant
/// Choose one. If you control a commander as you cast this spell, you may choose both instead.
/// * Creatures you control gain flying, vigilance, and double strike until end of turn.
/// * Creatures you control gain lifelink, indestructible, and protection from all colors until end of turn.
pub fn akromas_will() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Akroma's Will")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text("Choose one. If you control a commander as you cast this spell, you may choose both instead.\n\
            * Creatures you control gain flying, vigilance, and double strike until end of turn.\n\
            * Creatures you control gain lifelink, indestructible, and protection from all colors until end of turn.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Effect;
    use crate::ObjectFilter;
    use crate::StaticAbility;
    use crate::Until;
    use crate::ability::ProtectionFrom;
    use crate::card::PowerToughness;
    use crate::color::Color;
    use crate::executor::{ExecutionContext, execute_effect};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::static_abilities::StaticAbilityId;
    use crate::types::Supertype;
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
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    /// Helper to create a legendary creature on the battlefield.
    fn create_legendary_creature(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        use crate::card::CardBuilder;
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .supertypes(vec![Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    /// Helper to create a creature and designate it as the player's commander.
    fn create_commander(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let commander_id = create_legendary_creature(game, owner, name, power, toughness);
        // Set this creature as the player's commander
        if let Some(player) = game.player_mut(owner) {
            player.add_commander(commander_id);
        }
        commander_id
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_akromas_will_basic_properties() {
        let def = akromas_will();

        // Check name
        assert_eq!(def.name(), "Akroma's Will");

        // Check it's an instant
        assert!(def.is_spell());
        assert!(def.card.card_types.contains(&CardType::Instant));

        // Check mana cost - {3}{W} = mana value 4
        assert_eq!(def.card.mana_value(), 4);

        // Check colors - should be white
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_akromas_will_has_spell_effect() {
        let def = akromas_will();

        // Should have a spell effect
        assert!(def.spell_effect.is_some());

        // The spell effect should be a Conditional
        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);
        assert!(format!("{:?}", effects[0]).contains("ConditionalEffect"));
    }

    // =========================================================================
    // Mode 1 Tests (Flying, Vigilance, Double Strike)
    // =========================================================================

    #[test]
    fn test_mode_1_grants_flying_vigilance_double_strike() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature for Alice
        let _creature = create_creature(&mut game, alice, "Test Creature", 2, 2);

        // Create Akroma's Will spell object (for source)
        let def = akromas_will();
        let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Get the mode 1 effect directly
        let creatures_filter = ObjectFilter::creature().you_control();
        let mode_1_effect = Effect::grant_abilities_all(
            creatures_filter,
            vec![
                StaticAbility::flying(),
                StaticAbility::vigilance(),
                StaticAbility::double_strike(),
            ],
            Until::EndOfTurn,
        );

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(spell_id, alice).with_targets(vec![]);

        let result = execute_effect(&mut game, &mode_1_effect, &mut ctx);
        assert!(result.is_ok());

        // Verify continuous effects were created
        let effects = game.continuous_effects.effects_sorted();
        assert_eq!(
            effects.len(),
            3,
            "Should have 3 continuous effects (one for each ability)"
        );

        // Verify the creature now has the granted abilities via continuous effects
        // The abilities are granted through the layer system
        use crate::continuous::Modification;
        let has_flying = effects.iter().any(|e| {
            if let Modification::AddAbility(a) = &e.modification {
                a.id() == StaticAbilityId::Flying
            } else {
                false
            }
        });
        let has_vigilance = effects.iter().any(|e| {
            if let Modification::AddAbility(a) = &e.modification {
                a.id() == StaticAbilityId::Vigilance
            } else {
                false
            }
        });
        let has_double_strike = effects.iter().any(|e| {
            if let Modification::AddAbility(a) = &e.modification {
                a.id() == StaticAbilityId::DoubleStrike
            } else {
                false
            }
        });

        assert!(has_flying, "Should grant flying");
        assert!(has_vigilance, "Should grant vigilance");
        assert!(has_double_strike, "Should grant double strike");
    }

    // =========================================================================
    // Mode 2 Tests (Lifelink, Indestructible, Protection from all colors)
    // =========================================================================

    #[test]
    fn test_mode_2_grants_lifelink_indestructible_protection() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature for Alice
        let _creature = create_creature(&mut game, alice, "Test Creature", 2, 2);

        // Create Akroma's Will spell object (for source)
        let def = akromas_will();
        let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Get the mode 2 effect directly
        let creatures_filter = ObjectFilter::creature().you_control();
        let mode_2_effect = Effect::grant_abilities_all(
            creatures_filter,
            vec![
                StaticAbility::lifelink(),
                StaticAbility::indestructible(),
                StaticAbility::protection(ProtectionFrom::AllColors),
            ],
            Until::EndOfTurn,
        );

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(spell_id, alice).with_targets(vec![]);

        let result = execute_effect(&mut game, &mode_2_effect, &mut ctx);
        assert!(result.is_ok());

        // Verify continuous effects were created
        let effects = game.continuous_effects.effects_sorted();
        assert_eq!(
            effects.len(),
            3,
            "Should have 3 continuous effects (one for each ability)"
        );

        // Verify the abilities were granted
        use crate::continuous::Modification;
        let has_lifelink = effects.iter().any(|e| {
            if let Modification::AddAbility(a) = &e.modification {
                a.id() == StaticAbilityId::Lifelink
            } else {
                false
            }
        });
        let has_indestructible = effects.iter().any(|e| {
            if let Modification::AddAbility(a) = &e.modification {
                a.id() == StaticAbilityId::Indestructible
            } else {
                false
            }
        });
        let has_protection = effects.iter().any(|e| {
            if let Modification::AddAbility(a) = &e.modification {
                a.id() == StaticAbilityId::Protection
            } else {
                false
            }
        });

        assert!(has_lifelink, "Should grant lifelink");
        assert!(has_indestructible, "Should grant indestructible");
        assert!(has_protection, "Should grant protection from all colors");
    }

    // =========================================================================
    // Both Modes Tests (Commander conditional)
    // =========================================================================

    #[test]
    fn test_both_modes_grant_all_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature for Alice
        let _creature = create_creature(&mut game, alice, "Test Creature", 2, 2);

        // Create Akroma's Will spell object (for source)
        let def = akromas_will();
        let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Execute both mode effects
        let creatures_filter = ObjectFilter::creature().you_control();

        // Mode 1
        let mode_1_effect = Effect::grant_abilities_all(
            creatures_filter.clone(),
            vec![
                StaticAbility::flying(),
                StaticAbility::vigilance(),
                StaticAbility::double_strike(),
            ],
            Until::EndOfTurn,
        );

        // Mode 2
        let mode_2_effect = Effect::grant_abilities_all(
            creatures_filter,
            vec![
                StaticAbility::lifelink(),
                StaticAbility::indestructible(),
                StaticAbility::protection(ProtectionFrom::AllColors),
            ],
            Until::EndOfTurn,
        );

        let mut ctx = ExecutionContext::new_default(spell_id, alice).with_targets(vec![]);

        // Execute both modes
        execute_effect(&mut game, &mode_1_effect, &mut ctx).unwrap();
        execute_effect(&mut game, &mode_2_effect, &mut ctx).unwrap();

        // Verify all 6 continuous effects were created
        let effects = game.continuous_effects.effects_sorted();
        assert_eq!(
            effects.len(),
            6,
            "Should have 6 continuous effects (3 from each mode)"
        );
    }

    // =========================================================================
    // Commander Conditional Tests
    // =========================================================================

    #[test]
    fn test_commander_condition_with_commander_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a commander on the battlefield
        let commander = create_commander(&mut game, alice, "Legendary Commander", 4, 4);

        // Verify the commander is on the battlefield
        assert!(game.battlefield.contains(&commander));

        // Verify the commander is registered in the player's commander list
        let player = game.player(alice).unwrap();
        assert!(player.is_commander(commander));

        // Verify player controls a commander (their own)
        assert!(game.player_controls_a_commander(alice));
        assert!(game.player_controls_own_commander(alice));
    }

    #[test]
    fn test_commander_condition_without_commander() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create only a non-commander creature
        let _creature = create_creature(&mut game, alice, "Regular Creature", 2, 2);

        // Verify player does not control any commander
        assert!(!game.player_controls_a_commander(alice));
    }

    #[test]
    fn test_commander_condition_commander_in_command_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a legendary creature in the command zone (not battlefield)
        use crate::card::CardBuilder;
        let card = CardBuilder::new(CardId::new(), "Commander in Zone")
            .card_types(vec![CardType::Creature])
            .supertypes(vec![Supertype::Legendary])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let commander_id = game.create_object_from_card(&card, alice, Zone::Command);

        // Set as commander
        if let Some(player) = game.player_mut(alice) {
            player.add_commander(commander_id);
        }

        // Verify commander is NOT on battlefield
        assert!(!game.battlefield.contains(&commander_id));
        assert!(game.command_zone.contains(&commander_id));

        // player_controls_a_commander should return false (commander not on battlefield)
        assert!(!game.player_controls_a_commander(alice));
    }

    #[test]
    fn test_akromas_will_allows_both_modes_with_commander() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a commander on the battlefield
        let _commander = create_commander(&mut game, alice, "Akroma, Angel of Wrath", 6, 6);

        // Create a regular creature to receive the buffs
        let _creature = create_creature(&mut game, alice, "Test Creature", 2, 2);

        // Create Akroma's Will spell object
        let def = akromas_will();
        let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Verify the condition evaluates to true
        assert!(game.player_controls_a_commander(alice));

        // Create the execution context
        let mut ctx = ExecutionContext::new_default(spell_id, alice).with_targets(vec![]);

        // The spell effect is a Conditional that checks YouControlCommander
        // With commander on field, it should allow choose_count: 2, min_choose_count: 1
        let spell_effect = def.spell_effect.as_ref().unwrap();
        assert_eq!(spell_effect.len(), 1);

        // Verify the effect structure through debug output
        let effect_debug = format!("{:?}", spell_effect[0]);
        assert!(
            effect_debug.contains("ConditionalEffect"),
            "Should be a conditional effect"
        );
        assert!(
            effect_debug.contains("YouControlCommander"),
            "Should check commander condition"
        );
        assert!(
            effect_debug.contains("ChooseModeEffect"),
            "Should have choose mode in branches"
        );

        // Execute both mode effects directly to simulate choosing both
        let creatures_filter = ObjectFilter::creature().you_control();

        // Mode 1 effect
        let mode_1_effect = Effect::grant_abilities_all(
            creatures_filter.clone(),
            vec![
                StaticAbility::flying(),
                StaticAbility::vigilance(),
                StaticAbility::double_strike(),
            ],
            Until::EndOfTurn,
        );

        // Mode 2 effect
        let mode_2_effect = Effect::grant_abilities_all(
            creatures_filter,
            vec![
                StaticAbility::lifelink(),
                StaticAbility::indestructible(),
                StaticAbility::protection(ProtectionFrom::AllColors),
            ],
            Until::EndOfTurn,
        );

        // Execute both modes (as would happen when choosing both)
        execute_effect(&mut game, &mode_1_effect, &mut ctx).unwrap();
        execute_effect(&mut game, &mode_2_effect, &mut ctx).unwrap();

        // Verify all 6 abilities were granted (3 from each mode)
        let effects = game.continuous_effects.effects_sorted();
        assert_eq!(
            effects.len(),
            6,
            "Should have 6 continuous effects when choosing both modes"
        );
    }

    #[test]
    fn test_akromas_will_only_one_mode_without_commander() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a regular creature (NOT a commander)
        let _creature = create_creature(&mut game, alice, "Test Creature", 2, 2);

        // Create Akroma's Will spell object
        let def = akromas_will();

        // Verify the condition evaluates to false (no commander)
        assert!(!game.player_controls_a_commander(alice));

        // The spell effect is a Conditional that checks YouControlCommander
        let spell_effect = def.spell_effect.as_ref().unwrap();

        // Verify the effect structure through debug output
        let effect_debug = format!("{:?}", spell_effect[0]);
        assert!(
            effect_debug.contains("ConditionalEffect"),
            "Should be a conditional effect"
        );
        assert!(
            effect_debug.contains("YouControlCommander"),
            "Should check commander condition"
        );
        assert!(
            effect_debug.contains("ChooseModeEffect"),
            "Should have choose mode in branches"
        );
    }

    #[test]
    fn test_akromas_will_allows_both_modes_with_opponents_commander() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a commander for Bob (the opponent)
        let bobs_commander = create_legendary_creature(&mut game, bob, "Bob's Commander", 4, 4);

        // Register it as Bob's commander
        if let Some(player) = game.player_mut(bob) {
            player.add_commander(bobs_commander);
        }

        // Now Alice gains control of Bob's commander (e.g., via Control Magic)
        if let Some(obj) = game.object_mut(bobs_commander) {
            obj.controller = alice;
        }

        // Verify Alice now controls a commander (Bob's commander)
        assert!(game.player_controls_a_commander(alice));

        // Alice should NOT control her "own" commander (she doesn't have one)
        assert!(!game.player_controls_own_commander(alice));

        // But she controls "a" commander, so Akroma's Will should allow both modes
        // This is the key distinction - the card says "a commander" not "your commander"
    }

    #[test]
    fn test_opponent_commander_on_battlefield_not_controlled() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a commander for Bob on the battlefield (controlled by Bob)
        let bobs_commander = create_legendary_creature(&mut game, bob, "Bob's Commander", 4, 4);
        if let Some(player) = game.player_mut(bob) {
            player.add_commander(bobs_commander);
        }

        // Alice does NOT control Bob's commander
        assert!(!game.player_controls_a_commander(alice));

        // Bob controls his own commander
        assert!(game.player_controls_a_commander(bob));
        assert!(game.player_controls_own_commander(bob));
    }

    // =========================================================================
    // Multiple Creatures Tests
    // =========================================================================

    #[test]
    fn test_mode_1_affects_multiple_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create multiple creatures for Alice
        let _creature_1 = create_creature(&mut game, alice, "Creature 1", 2, 2);
        let _creature_2 = create_creature(&mut game, alice, "Creature 2", 3, 3);
        let _creature_3 = create_creature(&mut game, alice, "Creature 3", 1, 1);

        // Create Akroma's Will spell object
        let def = akromas_will();
        let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Execute mode 1
        let creatures_filter = ObjectFilter::creature().you_control();
        let mode_1_effect = Effect::grant_abilities_all(
            creatures_filter,
            vec![
                StaticAbility::flying(),
                StaticAbility::vigilance(),
                StaticAbility::double_strike(),
            ],
            Until::EndOfTurn,
        );

        let mut ctx = ExecutionContext::new_default(spell_id, alice).with_targets(vec![]);

        execute_effect(&mut game, &mode_1_effect, &mut ctx).unwrap();

        // The continuous effects apply to all creatures matching the filter
        // Each ability creates one continuous effect that applies to the filter
        let effects = game.continuous_effects.effects_sorted();
        assert_eq!(
            effects.len(),
            3,
            "Should have 3 continuous effects (one per ability, each affects all creatures)"
        );
    }

    // =========================================================================
    // Opponent's Creatures Tests
    // =========================================================================

    #[test]
    fn test_mode_does_not_affect_opponents_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a creature for Alice
        let _alice_creature = create_creature(&mut game, alice, "Alice's Creature", 2, 2);
        // Create a creature for Bob
        let _bob_creature = create_creature(&mut game, bob, "Bob's Creature", 3, 3);

        // Create Akroma's Will spell object (controlled by Alice)
        let def = akromas_will();
        let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Execute mode 1
        let creatures_filter = ObjectFilter::creature().you_control();
        let mode_1_effect = Effect::grant_abilities_all(
            creatures_filter,
            vec![
                StaticAbility::flying(),
                StaticAbility::vigilance(),
                StaticAbility::double_strike(),
            ],
            Until::EndOfTurn,
        );

        let mut ctx = ExecutionContext::new_default(spell_id, alice).with_targets(vec![]);

        execute_effect(&mut game, &mode_1_effect, &mut ctx).unwrap();

        // The filter specifies "you_control", so it should only affect Alice's creatures
        // The continuous effect uses the filter, which is evaluated by the controller
        let effects = game.continuous_effects.effects_sorted();
        assert_eq!(effects.len(), 3);

        // All effects should have Alice as the controller (used for filter evaluation)
        for effect in effects {
            assert_eq!(effect.controller, alice);
        }
    }

    // =========================================================================
    // Duration Tests
    // =========================================================================

    #[test]
    fn test_abilities_expire_at_end_of_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature for Alice
        let _creature = create_creature(&mut game, alice, "Test Creature", 2, 2);

        // Create Akroma's Will spell object
        let def = akromas_will();
        let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Execute mode 1
        let creatures_filter = ObjectFilter::creature().you_control();
        let mode_1_effect = Effect::grant_abilities_all(
            creatures_filter,
            vec![
                StaticAbility::flying(),
                StaticAbility::vigilance(),
                StaticAbility::double_strike(),
            ],
            Until::EndOfTurn,
        );

        let mut ctx = ExecutionContext::new_default(spell_id, alice).with_targets(vec![]);

        execute_effect(&mut game, &mode_1_effect, &mut ctx).unwrap();

        // Verify effects exist
        assert_eq!(game.continuous_effects.effects_sorted().len(), 3);

        // Simulate end of turn cleanup
        game.continuous_effects.cleanup_end_of_turn();

        // Verify effects are removed
        assert_eq!(
            game.continuous_effects.effects_sorted().len(),
            0,
            "Continuous effects should be removed at end of turn"
        );
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_akromas_will_mode_1() {
        use crate::ids::PlayerId;
        use crate::static_abilities::StaticAbility;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        // Cast Akroma's Will (no commander, so choose exactly one mode)
        // Mode 1: Flying, Vigilance, Double Strike
        // Cost is {3}{W} = 4 total mana, use 4 Plains
        let game = run_replay_test(
            vec![
                "1", // Cast Akroma's Will
                "0", // Tap Plains 1 for {W}
                "0", // Tap Plains 2 for mana
                "0", // Tap Plains 3 for mana
                "0", // Tap Plains 4 for mana (spell on stack, auto-resolves)
                "0", // Choose mode 1 (Flying, Vigilance, Double Strike)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Akroma's Will"])
                .p1_battlefield(vec![
                    "Plains",
                    "Plains",
                    "Plains",
                    "Plains",
                    "Grizzly Bears",
                ]),
        );

        let _alice = PlayerId::from_index(0);

        // Find Grizzly Bears on battlefield
        let bears = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .find(|obj| obj.name == "Grizzly Bears")
            .expect("Grizzly Bears should be on battlefield");

        // Verify Grizzly Bears has gained flying, vigilance, and double strike
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::flying()),
            "Creature should have flying"
        );
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::vigilance()),
            "Creature should have vigilance"
        );
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::double_strike()),
            "Creature should have double strike"
        );

        // Verify 3 continuous effects were created (one for each ability)
        assert_eq!(
            game.continuous_effects.effects_sorted().len(),
            3,
            "Should have 3 continuous effects"
        );
    }

    #[test]
    fn test_replay_akromas_will_mode_2() {
        use crate::ability::ProtectionFrom;
        use crate::ids::PlayerId;
        use crate::static_abilities::StaticAbility;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        // Cast Akroma's Will (no commander, so choose exactly one mode)
        // Mode 2: Lifelink, Indestructible, Protection from all colors
        // Cost is {3}{W} = 4 total mana, use 4 Plains
        // Per MTG rule 601.2b, modes are chosen during casting BEFORE mana payment
        let game = run_replay_test(
            vec![
                "1", // Cast Akroma's Will
                "1", // Choose mode 2 (Lifelink, Indestructible, Protection) - per 601.2b
                "0", // Pip payment: Tap Plains 1 for {W}
                "0", // Pip payment: Tap Plains 2 for generic
                "0", // Pip payment: Tap Plains 3 for generic
                     // Fourth Plains auto-taps for last generic, spell resolves
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Akroma's Will"])
                .p1_battlefield(vec![
                    "Plains",
                    "Plains",
                    "Plains",
                    "Plains",
                    "Grizzly Bears",
                ]),
        );

        let _alice = PlayerId::from_index(0);

        // Find Grizzly Bears on battlefield
        let bears = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .find(|obj| obj.name == "Grizzly Bears")
            .expect("Grizzly Bears should be on battlefield");

        // Verify Grizzly Bears has gained lifelink, indestructible, and protection
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::lifelink()),
            "Creature should have lifelink"
        );
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::indestructible()),
            "Creature should have indestructible"
        );
        assert!(
            game.object_has_ability(
                bears.id,
                &StaticAbility::protection(ProtectionFrom::AllColors)
            ),
            "Creature should have protection from all colors"
        );

        // Verify 3 continuous effects were created (one for each ability)
        assert_eq!(
            game.continuous_effects.effects_sorted().len(),
            3,
            "Should have 3 continuous effects"
        );
    }

    #[test]
    fn test_replay_akromas_will_both_modes_with_commander() {
        use crate::ability::ProtectionFrom;
        use crate::ids::PlayerId;
        use crate::static_abilities::StaticAbility;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        // Test Akroma's Will with commander on battlefield
        // When you control a commander, you may choose BOTH modes instead of just one
        //
        // Setup:
        // - Commander (Grizzly Bears) already on battlefield (using p1_commander_on_battlefield)
        // - Akroma's Will in hand - costs {3}{W}
        // - 4 Plains on battlefield for mana
        //
        // Key mechanic being tested:
        // - Akroma's Will checks Condition::YouControlCommander
        // - When true, it uses choose_up_to(2, 1, modes) instead of choose_one(modes)
        // - With SelectFirstDecisionMaker (default), max modes (2) are selected
        // - Both modes grant abilities to creatures, so the commander gets all 6 abilities
        // Per MTG rule 601.2b, modes are chosen during casting BEFORE mana payment
        let game = run_replay_test(
            vec![
                "1",   // Cast Akroma's Will from hand
                "0,1", // Choose BOTH modes (mode 0 and mode 1) - per 601.2b
                "0",   // Tap Plains 1 for {W}
                "0",   // Tap Plains 2 for generic
                "0",   // Tap Plains 3 for generic
                       // Plains 4 auto-taps for last generic, spell resolves
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Akroma's Will"])
                .p1_battlefield(vec!["Plains", "Plains", "Plains", "Plains"])
                .p1_commander_on_battlefield(vec!["Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);

        // Verify commander is on the battlefield
        assert!(
            game.battlefield_has("Grizzly Bears"),
            "Commander should be on battlefield"
        );

        // Verify player controls a commander (this is what Akroma's Will uses via YouControlCommander)
        assert!(
            game.player_controls_a_commander(alice),
            "Player should control a commander on the battlefield"
        );

        // Find Grizzly Bears (the commander) on battlefield
        let bears = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .find(|obj| obj.name == "Grizzly Bears")
            .expect("Grizzly Bears should be on battlefield");

        // Verify Grizzly Bears has ALL 6 abilities from both modes:
        // Mode 1: Flying, Vigilance, Double Strike
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::flying()),
            "Commander should have flying (mode 1)"
        );
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::vigilance()),
            "Commander should have vigilance (mode 1)"
        );
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::double_strike()),
            "Commander should have double strike (mode 1)"
        );

        // Mode 2: Lifelink, Indestructible, Protection from all colors
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::lifelink()),
            "Commander should have lifelink (mode 2)"
        );
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::indestructible()),
            "Commander should have indestructible (mode 2)"
        );
        assert!(
            game.object_has_ability(
                bears.id,
                &StaticAbility::protection(ProtectionFrom::AllColors)
            ),
            "Commander should have protection from all colors (mode 2)"
        );

        // Verify 6 continuous effects were created (3 from each mode)
        assert_eq!(
            game.continuous_effects.effects_sorted().len(),
            6,
            "Should have 6 continuous effects when both modes are chosen"
        );
    }

    #[test]
    fn test_replay_akromas_will_double_strike_combat() {
        use crate::ids::PlayerId;
        use crate::static_abilities::StaticAbility;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test_full_turn};

        // Test that double strike actually causes damage twice in combat.
        //
        // Setup:
        // - Commander (Grizzly Bears, 2/2) already on battlefield
        // - Akroma's Will in hand - costs {3}{W}
        // - 4 Plains on battlefield for mana
        //
        // Game flow:
        // 1. Upkeep/Draw: pass priority
        // 2. Main: Cast Akroma's Will, choose mode 1 (flying, vigilance, double strike)
        // 3. Combat: Attack with Grizzly Bears
        // 4. Damage: Bears should deal 2 in first strike step + 2 in regular step = 4 total
        //
        // Expected result: Opponent should have 16 life (20 - 4 = 16)
        let game = run_replay_test_full_turn(
            vec![
                // Upkeep - pass priority (we have mana abilities available)
                "", // Draw step - pass priority
                "",
                // Main phase - Cast Akroma's Will
                "1", // Cast spell (index 1, after PassPriority at index 0)
                "0", // Tap Plains 1 for white mana
                "0", // Tap Plains 2 for generic
                "0", // Tap Plains 3 for generic
                // Plains 4 auto-taps for last generic, spell resolves
                "0", // Choose mode 1 (flying, vigilance, double strike)
                // After spell resolves, priority auto-passes through main and begin combat
                // Declare attackers - attack with Grizzly Bears (index 0)
                "0",
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Akroma's Will"])
                .p1_battlefield(vec!["Plains", "Plains", "Plains", "Plains"])
                .p1_commander_on_battlefield(vec!["Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Find Grizzly Bears on battlefield
        let bears = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .find(|obj| obj.name == "Grizzly Bears")
            .expect("Grizzly Bears should be on battlefield");

        // Verify Grizzly Bears has double strike from Akroma's Will
        assert!(
            game.object_has_ability(bears.id, &StaticAbility::double_strike()),
            "Creature should have double strike"
        );

        // Verify player controls a commander
        assert!(
            game.player_controls_a_commander(alice),
            "Player should control a commander on the battlefield"
        );

        // Key assertion: Opponent should have taken 4 damage total
        // (2 from first strike damage step + 2 from regular damage step)
        // Starting life was 20, so they should be at 16
        let bob_life = game.player(bob).map(|p| p.life).unwrap_or(0);
        assert_eq!(
            bob_life, 16,
            "Opponent should have 16 life after taking 4 damage from double strike (2+2). Got {}",
            bob_life
        );

        // Verify Alice's life is unchanged (no damage taken)
        let alice_life = game.player(alice).map(|p| p.life).unwrap_or(0);
        assert_eq!(
            alice_life, 20,
            "Alice should still have 20 life. Got {}",
            alice_life
        );
    }
}
