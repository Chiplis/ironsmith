//! Blood Artist card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Blood Artist - Creature — Vampire
/// {1}{B}
/// 0/1
/// Whenever Blood Artist or another creature dies, target player loses 1 life
/// and you gain 1 life.
pub fn blood_artist() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Blood Artist")
        .parse_text(
            "Mana cost: {1}{B}\n\
             Type: Creature — Vampire\n\
             Power/Toughness: 0/1\n\
             Whenever Blood Artist or another creature dies, target player loses 1 life and you gain 1 life.",
        )
        .expect("Blood Artist text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::card::PowerToughness;
    use crate::color::Color;
    use crate::effect::EffectResult;
    use crate::events::zones::ZoneChangeEvent;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ChooseSpec;
    use crate::triggers::{TriggerEvent, check_triggers};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        subtypes: Vec<Subtype>,
        owner: PlayerId,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn dies_event(object_id: ObjectId, snapshot: ObjectSnapshot) -> TriggerEvent {
        TriggerEvent::new(ZoneChangeEvent::new(
            object_id,
            Zone::Battlefield,
            Zone::Graveyard,
            Some(snapshot),
        ))
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_blood_artist_basic_properties() {
        let def = blood_artist();
        assert_eq!(def.name(), "Blood Artist");
        assert!(def.is_creature());
        assert!(!def.card.is_land());
        assert_eq!(def.card.mana_value(), 2);
    }

    #[test]
    fn test_blood_artist_is_vampire() {
        let def = blood_artist();
        assert!(def.card.has_subtype(Subtype::Vampire));
    }

    #[test]
    fn test_blood_artist_power_toughness() {
        use crate::card::PtValue;
        let def = blood_artist();
        let pt = def.card.power_toughness.as_ref().unwrap();
        assert_eq!(pt.power, PtValue::Fixed(0));
        assert_eq!(pt.toughness, PtValue::Fixed(1));
    }

    #[test]
    fn test_blood_artist_is_black() {
        let def = blood_artist();
        assert!(def.card.colors().contains(Color::Black));
        assert!(!def.card.colors().contains(Color::White));
    }

    #[test]
    fn test_blood_artist_mana_cost() {
        let def = blood_artist();
        assert_eq!(def.card.mana_value(), 2);
        // {1}{B} = 1 generic + 1 black
    }

    #[test]
    fn test_blood_artist_has_one_ability() {
        let def = blood_artist();
        assert_eq!(def.abilities.len(), 1);
    }

    // ========================================
    // Triggered Ability Structure Tests
    // ========================================

    #[test]
    fn test_ability_is_triggered() {
        let def = blood_artist();
        let ability = &def.abilities[0];
        assert!(matches!(ability.kind, AbilityKind::Triggered(_)));
    }

    #[test]
    fn test_trigger_condition_is_creature_dies() {
        let def = blood_artist();
        let ability = &def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            // Check the trigger via display text
            assert!(triggered.trigger.display().contains("dies"));
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_trigger_has_two_effects() {
        let def = blood_artist();
        let ability = &def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            assert_eq!(triggered.effects.len(), 2);
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_trigger_requires_target_player() {
        use crate::target::PlayerFilter;
        let def = blood_artist();
        let ability = &def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            assert_eq!(triggered.choices.len(), 1);
            // Should be Target(Player(Any)) since it's "target player"
            assert!(triggered.choices[0].is_target());
            assert!(matches!(
                triggered.choices[0].inner(),
                ChooseSpec::Player(PlayerFilter::Any)
            ));
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_ability_functions_on_battlefield() {
        let def = blood_artist();
        let ability = &def.abilities[0];
        assert!(ability.functions_in(&Zone::Battlefield));
        assert!(!ability.functions_in(&Zone::Graveyard));
    }

    // ========================================
    // Trigger Detection Tests
    // ========================================

    #[test]
    fn test_triggers_when_another_creature_dies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist on the battlefield
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create another creature that will die
        let victim_id = create_creature(&mut game, "Victim", vec![], alice, 1, 1);

        // Create snapshot of the dying creature
        let snapshot = ObjectSnapshot::from_object(game.object(victim_id).unwrap(), &game);

        // Simulate death event
        let event = dies_event(victim_id, snapshot);

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Blood Artist should trigger when another creature dies"
        );
        assert_eq!(triggered[0].source, blood_artist_id);
    }

    #[test]
    fn test_triggers_when_blood_artist_itself_dies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist on the battlefield
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create snapshot of Blood Artist before it dies
        let snapshot = ObjectSnapshot::from_object(game.object(blood_artist_id).unwrap(), &game);

        // Simulate Blood Artist dying
        let event = dies_event(blood_artist_id, snapshot);

        // Note: The trigger checks the battlefield, but Blood Artist triggers when it dies
        // because it uses last-known information. In this test, we're checking the trigger
        // detection while Blood Artist is still technically on the battlefield.
        let triggered = check_triggers(&game, &event);

        // Blood Artist should trigger for its own death
        // The triggered ability should fire from the snapshot information
        assert!(
            triggered.len() >= 1,
            "Blood Artist should trigger when it dies"
        );
    }

    #[test]
    fn test_triggers_when_opponent_creature_dies() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Blood Artist for Alice
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a creature for Bob that will die
        let victim_id = create_creature(&mut game, "Bob's Creature", vec![], bob, 2, 2);

        // Create snapshot
        let snapshot = ObjectSnapshot::from_object(game.object(victim_id).unwrap(), &game);

        // Simulate death event
        let event = dies_event(victim_id, snapshot);

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Blood Artist should trigger when opponent's creature dies"
        );
        assert_eq!(triggered[0].source, blood_artist_id);
    }

    #[test]
    fn test_multiple_blood_artists_trigger_separately() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two Blood Artists
        let def = blood_artist();
        let blood_artist1_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let blood_artist2_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a creature that will die
        let victim_id = create_creature(&mut game, "Victim", vec![], alice, 1, 1);

        // Create snapshot
        let snapshot = ObjectSnapshot::from_object(game.object(victim_id).unwrap(), &game);

        // Simulate death event
        let event = dies_event(victim_id, snapshot);

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            2,
            "Both Blood Artists should trigger when a creature dies"
        );

        // Verify both Blood Artists are sources
        let sources: Vec<ObjectId> = triggered.iter().map(|t| t.source).collect();
        assert!(sources.contains(&blood_artist1_id));
        assert!(sources.contains(&blood_artist2_id));
    }

    #[test]
    fn test_does_not_trigger_for_noncreature_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist
        let def = blood_artist();
        let _blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create an artifact that will be destroyed
        let artifact_id = game.new_object_id();
        let artifact_card = CardBuilder::new(CardId::from_raw(artifact_id.0 as u32), "Sol Ring")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_obj = Object::from_card(artifact_id, &artifact_card, alice, Zone::Battlefield);
        game.add_object(artifact_obj);

        // Create snapshot
        let snapshot = ObjectSnapshot::from_object(game.object(artifact_id).unwrap(), &game);

        // Simulate the artifact being destroyed (goes to graveyard)
        let event = dies_event(artifact_id, snapshot);

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            0,
            "Blood Artist should NOT trigger when a non-creature permanent dies"
        );
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_target_player_loses_one_life() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Blood Artist
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Get the effects from the ability
        let ability = &def.abilities[0];
        let effects = if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.effects.clone()
        } else {
            panic!("Expected triggered ability");
        };

        // Execute the first effect (target player loses 1 life)
        // Set up execution context with Bob as the target
        let mut ctx = ExecutionContext::new_default(blood_artist_id, alice);
        ctx.targets
            .push(crate::executor::ResolvedTarget::Player(bob));

        let starting_life = game.player(bob).unwrap().life;
        let result = effects[0].0.execute(&mut game, &mut ctx);

        assert!(result.is_ok());
        assert_eq!(
            game.player(bob).unwrap().life,
            starting_life - 1,
            "Target player should lose 1 life"
        );
    }

    #[test]
    fn test_controller_gains_one_life() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Blood Artist
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Get the effects from the ability
        let ability = &def.abilities[0];
        let effects = if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.effects.clone()
        } else {
            panic!("Expected triggered ability");
        };

        // Execute the second effect (you gain 1 life)
        let mut ctx = ExecutionContext::new_default(blood_artist_id, alice);
        ctx.targets
            .push(crate::executor::ResolvedTarget::Player(bob));

        let starting_life = game.player(alice).unwrap().life;
        let result = effects[1].0.execute(&mut game, &mut ctx);

        assert!(result.is_ok());
        assert_eq!(
            game.player(alice).unwrap().life,
            starting_life + 1,
            "Controller should gain 1 life"
        );
    }

    #[test]
    fn test_both_effects_execute_correctly() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Blood Artist
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Get the effects from the ability
        let ability = &def.abilities[0];
        let effects = if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.effects.clone()
        } else {
            panic!("Expected triggered ability");
        };

        // Set up execution context
        let mut ctx = ExecutionContext::new_default(blood_artist_id, alice);
        ctx.targets
            .push(crate::executor::ResolvedTarget::Player(bob));

        let alice_starting = game.player(alice).unwrap().life;
        let bob_starting = game.player(bob).unwrap().life;

        // Execute both effects
        for effect in &effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        assert_eq!(
            game.player(bob).unwrap().life,
            bob_starting - 1,
            "Target should lose 1 life"
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            alice_starting + 1,
            "Controller should gain 1 life"
        );
    }

    #[test]
    fn test_can_target_self_with_life_loss() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Get the effects from the ability
        let ability = &def.abilities[0];
        let effects = if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.effects.clone()
        } else {
            panic!("Expected triggered ability");
        };

        // Set up execution context with Alice targeting herself
        let mut ctx = ExecutionContext::new_default(blood_artist_id, alice);
        ctx.targets
            .push(crate::executor::ResolvedTarget::Player(alice));

        let starting_life = game.player(alice).unwrap().life;

        // Execute both effects (Alice targets herself)
        for effect in &effects {
            let _ = effect.0.execute(&mut game, &mut ctx);
        }

        // Net result: lose 1, gain 1 = no change
        assert_eq!(
            game.player(alice).unwrap().life,
            starting_life,
            "Targeting self should result in net zero life change"
        );
    }

    // ========================================
    // On Battlefield Tests
    // ========================================

    #[test]
    fn test_blood_artist_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist on the battlefield
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&blood_artist_id));

        // Verify the object has the ability
        let obj = game.object(blood_artist_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);

        // Verify it's the triggered ability
        assert!(matches!(obj.abilities[0].kind, AbilityKind::Triggered(_)));
    }

    #[test]
    fn test_blood_artist_creature_stats() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist on the battlefield
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(blood_artist_id).unwrap();
        assert!(obj.is_creature());
        use crate::card::PtValue;
        assert_eq!(obj.base_power, Some(PtValue::Fixed(0)));
        assert_eq!(obj.base_toughness, Some(PtValue::Fixed(1)));
        assert!(obj.has_subtype(Subtype::Vampire));
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_multiple_creatures_dying_triggers_multiple_times() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create multiple creatures that will die
        let victim1_id = create_creature(&mut game, "Victim 1", vec![], alice, 1, 1);
        let victim2_id = create_creature(&mut game, "Victim 2", vec![], alice, 2, 2);

        // Simulate death of first creature
        let snapshot1 = ObjectSnapshot::from_object(game.object(victim1_id).unwrap(), &game);
        let event1 = dies_event(victim1_id, snapshot1);

        let triggered1 = check_triggers(&game, &event1);
        assert_eq!(triggered1.len(), 1);
        assert_eq!(triggered1[0].source, blood_artist_id);

        // Simulate death of second creature
        let snapshot2 = ObjectSnapshot::from_object(game.object(victim2_id).unwrap(), &game);
        let event2 = dies_event(victim2_id, snapshot2);

        let triggered2 = check_triggers(&game, &event2);
        assert_eq!(triggered2.len(), 1);
        assert_eq!(triggered2[0].source, blood_artist_id);
    }

    #[test]
    fn test_blood_artist_doesnt_trigger_from_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist and move it to the graveyard
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _new_id = game.move_object(blood_artist_id, Zone::Graveyard).unwrap();

        // Create a creature that will die
        let victim_id = create_creature(&mut game, "Victim", vec![], alice, 1, 1);

        // Create snapshot
        let snapshot = ObjectSnapshot::from_object(game.object(victim_id).unwrap(), &game);

        // Simulate death event
        let event = dies_event(victim_id, snapshot);

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            0,
            "Blood Artist in graveyard should not trigger"
        );
    }

    #[test]
    fn test_blood_artist_vampire_creature_type_interaction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create another Vampire that will die
        let vampire_id = create_creature(&mut game, "Vampire", vec![Subtype::Vampire], alice, 2, 1);

        // Create snapshot
        let snapshot = ObjectSnapshot::from_object(game.object(vampire_id).unwrap(), &game);

        // Simulate death event
        let event = dies_event(vampire_id, snapshot);

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Blood Artist should trigger for Vampire dying"
        );
        assert_eq!(triggered[0].source, blood_artist_id);
    }

    // ========================================
    // Edge Case Tests
    // ========================================

    #[test]
    fn test_token_creature_dying_triggers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Blood Artist
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a token creature (using a creature to represent a token for testing purposes)
        // The trigger doesn't care if it's a token or not - just that it's a creature
        let token_id = create_creature(
            &mut game,
            "Zombie Token",
            vec![Subtype::Zombie],
            alice,
            2,
            2,
        );

        // Create snapshot
        let snapshot = ObjectSnapshot::from_object(game.object(token_id).unwrap(), &game);

        // Simulate token dying
        let event = dies_event(token_id, snapshot);

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Blood Artist should trigger when token creature dies"
        );
        assert_eq!(triggered[0].source, blood_artist_id);
    }

    #[test]
    fn test_effect_result_values() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Blood Artist
        let def = blood_artist();
        let blood_artist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let ability = &def.abilities[0];
        let effects = if let AbilityKind::Triggered(triggered) = &ability.kind {
            triggered.effects.clone()
        } else {
            panic!("Expected triggered ability");
        };

        let mut ctx = ExecutionContext::new_default(blood_artist_id, alice);
        ctx.targets
            .push(crate::executor::ResolvedTarget::Player(bob));

        // First effect should return Count(1) for 1 life lost
        let result1 = effects[0].0.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result1.result, EffectResult::Count(1));

        // Second effect should return Count(1) for 1 life gained
        let result2 = effects[1].0.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result2.result, EffectResult::Count(1));
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests Blood Artist triggered ability when a creature dies.
    ///
    /// Blood Artist: Whenever Blood Artist or another creature dies,
    /// target player loses 1 life and you gain 1 life.
    ///
    /// Scenario: Cast Doom Blade on Grizzly Bears, Blood Artist triggers,
    /// target opponent with the life loss.
    #[test]
    fn test_replay_blood_artist_creature_dies() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Starting state: Blood Artist + Grizzly Bears + 2 Swamps on battlefield
                // Doom Blade in hand. Starting actions:
                // 0=pass, 1=cast Doom Blade
                "1", // Cast Doom Blade (priority action)
                "0", // Target Grizzly Bears for Doom Blade
                // Pip-by-pip mana payment: tap first Swamp, rest auto-selects
                "0", // Tap first Swamp for generic pip (auto-passes handle resolution)
                // Blood Artist trigger goes on stack after Doom Blade resolves
                "1", // Target Player 2 (Bob) for Blood Artist trigger (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Doom Blade"])
                .p1_battlefield(vec!["Blood Artist", "Grizzly Bears", "Swamp", "Swamp"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // First check if Grizzly Bears died - this verifies Doom Blade resolved
        let alice_gy = game.player(alice).unwrap();
        let bears_in_gy = alice_gy.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });

        // Also check if Doom Blade is in graveyard (after resolving)
        let blade_in_gy = alice_gy.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Doom Blade")
                .unwrap_or(false)
        });

        // Debug: Check both life totals
        let alice_life = game.life_total(alice);
        let bob_life = game.life_total(bob);

        // Check Bears first to see if Doom Blade even resolved
        assert!(
            bears_in_gy,
            "Grizzly Bears should be in graveyard (Doom Blade resolved). Doom Blade in gy: {}, Alice life: {}, Bob life: {}",
            blade_in_gy, alice_life, bob_life
        );

        // Blood Artist should still be on battlefield
        assert!(
            game.battlefield_has("Blood Artist"),
            "Blood Artist should still be on battlefield"
        );

        // Target player (Bob) should have lost 1 life
        assert_eq!(
            bob_life, 19,
            "Bob should be at 19 life (lost 1 from Blood Artist). Alice life: {}",
            alice_life
        );

        // Blood Artist's controller (Alice) should have gained 1 life
        assert_eq!(
            alice_life, 21,
            "Alice should be at 21 life (gained 1 from Blood Artist). Bob life: {}",
            bob_life
        );
    }

    /// Tests Blood Artist trigger when targeting self (net zero life change).
    #[test]
    fn test_replay_blood_artist_target_self() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Doom Blade
                "0", // Target Grizzly Bears
                "0", // Tap first Swamp for mana
                "0", // Tap second Swamp (auto-passes handle resolution)
                "0", // Target self (Player 1) for Blood Artist trigger (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Doom Blade"])
                .p1_battlefield(vec!["Blood Artist", "Grizzly Bears", "Swamp", "Swamp"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // When targeting self: lose 1, gain 1 = net 0
        assert_eq!(
            game.life_total(alice),
            20,
            "Alice should be at 20 life (lost 1, gained 1 = net 0)"
        );

        // Bob should be unchanged
        assert_eq!(game.life_total(bob), 20, "Bob should still be at 20 life");
    }
}
