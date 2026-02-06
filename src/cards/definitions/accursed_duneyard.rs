//! Accursed Duneyard card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::CardType;

#[cfg(test)]
use crate::target::ObjectFilter;
#[cfg(test)]
use crate::types::Subtype;
#[cfg(test)]
use crate::zone::Zone;

/// Accursed Duneyard
/// Land
/// {T}: Add {C}.
/// {2}, {T}: Regenerate target Shade, Skeleton, Specter, Spirit, Vampire, Wraith, or Zombie.
pub fn accursed_duneyard() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Accursed Duneyard")
        .card_types(vec![CardType::Land])
        //.taps_for(ManaSymbol::Colorless)
        //.with_ability(regenerate_ability)
        .parse_text(
            "{T}: Add {C}.\n{2}, {T}: Regenerate target Shade, Skeleton, Specter, Spirit, Vampire, Wraith, or Zombie.",
        )
        .unwrap()
}

/// Creates a filter for undead creature types that can be targeted by Accursed Duneyard.
/// Matches: Shade, Skeleton, Specter, Spirit, Vampire, Wraith, or Zombie.
#[cfg(test)]
fn undead_creature_filter() -> ObjectFilter {
    ObjectFilter {
        zone: Some(Zone::Battlefield),
        card_types: vec![CardType::Creature],
        subtypes: vec![
            Subtype::Shade,
            Subtype::Skeleton,
            Subtype::Specter,
            Subtype::Spirit,
            Subtype::Vampire,
            Subtype::Wraith,
            Subtype::Zombie,
        ],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::PowerToughness;
    use crate::effect::{Effect, Until};
    use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::target::ChooseSpec;

    /// Helper to create a basic game state for testing.
    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    /// Helper to create an undead creature on the battlefield.
    fn create_undead_creature(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        subtype: Subtype,
    ) -> ObjectId {
        use crate::card::CardBuilder;
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn test_accursed_duneyard_basic_properties() {
        let def = accursed_duneyard();

        // Check name
        assert_eq!(def.name(), "Accursed Duneyard");

        // Check it's a land
        assert!(def.card.is_land());

        // Check mana cost - lands have no mana cost
        assert!(def.card.mana_cost.is_none());

        // Check it's not a creature
        assert!(!def.is_creature());
    }

    #[test]
    fn test_accursed_duneyard_has_mana_ability() {
        let def = accursed_duneyard();

        // Should have at least one mana ability (tap for colorless)
        let has_mana_ability = def.abilities.iter().any(|a| a.is_mana_ability());
        assert!(has_mana_ability, "Should have a mana ability");

        // Check the mana ability produces colorless mana
        let mana_ability = def.abilities.iter().find(|a| a.is_mana_ability()).unwrap();
        if let AbilityKind::Mana(ma) = &mana_ability.kind {
            assert!(
                ma.mana.contains(&ManaSymbol::Colorless),
                "Mana ability should produce colorless mana"
            );
        }
    }

    #[test]
    fn test_accursed_duneyard_has_regenerate_ability() {
        let def = accursed_duneyard();

        // Should have an activated ability for regenerate
        let regenerate_ability = def.abilities.iter().find(|a| {
            if let AbilityKind::Activated(act) = &a.kind {
                act.effects
                    .iter()
                    .any(|e| format!("{:?}", e).contains("RegenerateEffect"))
            } else {
                false
            }
        });

        assert!(
            regenerate_ability.is_some(),
            "Should have a regenerate activated ability"
        );
    }

    #[test]
    fn test_accursed_duneyard_regenerate_cost() {
        let def = accursed_duneyard();

        // Find the regenerate ability
        let regenerate_ability = def
            .abilities
            .iter()
            .find(|a| {
                if let AbilityKind::Activated(act) = &a.kind {
                    act.effects
                        .iter()
                        .any(|e| format!("{:?}", e).contains("RegenerateEffect"))
                } else {
                    false
                }
            })
            .unwrap();

        if let AbilityKind::Activated(act) = &regenerate_ability.kind {
            // Cost should be {2}, {T}
            // Tap is now in cost_effects, not TotalCost
            assert!(act.has_tap_cost(), "Should have tap cost");
            // Mana cost should be {2}
            assert!(
                act.mana_cost
                    .mana_cost()
                    .map(|mc| mc.mana_value() == 2)
                    .unwrap_or(false),
                "Should have mana cost of 2"
            );
        }
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_regenerate_effect_adds_shield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a Zombie creature
        let zombie_id = create_undead_creature(&mut game, alice, "Test Zombie", Subtype::Zombie);

        // Create Accursed Duneyard
        let duneyard_def = accursed_duneyard();
        let duneyard_id =
            game.create_object_from_definition(&duneyard_def, alice, Zone::Battlefield);

        // Verify the zombie has no regeneration shields initially
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(zombie_id),
            0,
            "Zombie should have no regeneration shields initially"
        );

        // Execute the regenerate effect directly
        let regenerate_effect =
            Effect::regenerate(ChooseSpec::SpecificObject(zombie_id), Until::EndOfTurn);

        let mut ctx = ExecutionContext::new_default(duneyard_id, alice)
            .with_targets(vec![ResolvedTarget::Object(zombie_id)]);

        let result = execute_effect(&mut game, &regenerate_effect, &mut ctx);
        assert!(result.is_ok());

        // Verify the zombie now has a regeneration shield
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(zombie_id),
            1,
            "Zombie should have 1 regeneration shield after regenerate"
        );
    }

    #[test]
    fn test_regeneration_shield_prevents_destruction() {
        use crate::effect::EffectResult;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a Zombie creature with a regeneration shield
        let zombie_id = create_undead_creature(&mut game, alice, "Test Zombie", Subtype::Zombie);

        // Apply regeneration via the proper effect (creates replacement effect)
        let regenerate_effect =
            Effect::regenerate(ChooseSpec::SpecificObject(zombie_id), Until::EndOfTurn);
        let mut regen_ctx = ExecutionContext::new_default(zombie_id, alice);
        execute_effect(&mut game, &regenerate_effect, &mut regen_ctx).unwrap();

        // Create a source for the destroy effect
        let duneyard_def = accursed_duneyard();
        let source_id = game.create_object_from_definition(&duneyard_def, alice, Zone::Battlefield);

        // Try to destroy the zombie
        let destroy_effect = Effect::destroy(ChooseSpec::SpecificObject(zombie_id));

        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_targets(vec![ResolvedTarget::Object(zombie_id)]);

        let result = execute_effect(&mut game, &destroy_effect, &mut ctx);
        assert!(result.is_ok());

        // Result should be Replaced (regeneration kicked in)
        assert!(
            matches!(result.unwrap().result, EffectResult::Replaced),
            "Destroy should return Replaced when regeneration is used"
        );

        // Zombie should still be on battlefield
        assert!(
            game.battlefield.contains(&zombie_id),
            "Zombie should still be on battlefield after regeneration"
        );

        // Zombie should be tapped
        assert!(
            game.is_tapped(zombie_id),
            "Zombie should be tapped after regeneration"
        );

        // Zombie should have no damage
        assert_eq!(
            game.damage_on(zombie_id),
            0,
            "Zombie should have no damage after regeneration"
        );

        // Regeneration shield should be used up
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(zombie_id),
            0,
            "Regeneration shield should be consumed"
        );
    }

    #[test]
    fn test_regeneration_shield_works_with_lethal_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a Zombie creature (2/2) with a regeneration shield and lethal damage
        let zombie_id = create_undead_creature(&mut game, alice, "Test Zombie", Subtype::Zombie);

        // Apply regeneration via the proper effect (creates replacement effect)
        let regenerate_effect =
            Effect::regenerate(ChooseSpec::SpecificObject(zombie_id), Until::EndOfTurn);
        let mut regen_ctx = ExecutionContext::new_default(zombie_id, alice);
        execute_effect(&mut game, &regenerate_effect, &mut regen_ctx).unwrap();

        game.mark_damage(zombie_id, 2); // Lethal damage for a 2/2

        // Apply state-based actions
        crate::rules::apply_state_based_actions(&mut game);

        // Zombie should still be on battlefield (regeneration prevented death)
        assert!(
            game.battlefield.contains(&zombie_id),
            "Zombie should still be on battlefield after SBA with regen shield"
        );

        // Regeneration shield should be used up
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(zombie_id),
            0,
            "Regeneration shield should be consumed by SBA"
        );

        // Zombie should be tapped
        assert!(
            game.is_tapped(zombie_id),
            "Zombie should be tapped after regeneration from SBA"
        );

        // Damage should be cleared
        assert_eq!(
            game.damage_on(zombie_id),
            0,
            "Damage should be cleared after regeneration"
        );
    }

    #[test]
    fn test_regeneration_shields_cleared_at_end_of_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature with regeneration shields
        let zombie_id = create_undead_creature(&mut game, alice, "Test Zombie", Subtype::Zombie);

        // Apply regeneration twice via the proper effect (creates 2 replacement effects)
        let regenerate_effect =
            Effect::regenerate(ChooseSpec::SpecificObject(zombie_id), Until::EndOfTurn);
        let mut regen_ctx = ExecutionContext::new_default(zombie_id, alice);
        execute_effect(&mut game, &regenerate_effect, &mut regen_ctx).unwrap();
        execute_effect(&mut game, &regenerate_effect, &mut regen_ctx).unwrap();

        // Verify shields are present
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(zombie_id),
            2
        );

        // Execute cleanup step (end of turn)
        crate::turn::execute_cleanup_step(&mut game);

        // Shields should be cleared
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(zombie_id),
            0,
            "Regeneration shields should be cleared at end of turn"
        );
    }

    #[test]
    fn test_undead_creature_filter() {
        let filter = undead_creature_filter();

        // Should require creature type
        assert!(filter.card_types.contains(&CardType::Creature));

        // Should include all undead subtypes
        assert!(filter.subtypes.contains(&Subtype::Shade));
        assert!(filter.subtypes.contains(&Subtype::Skeleton));
        assert!(filter.subtypes.contains(&Subtype::Specter));
        assert!(filter.subtypes.contains(&Subtype::Spirit));
        assert!(filter.subtypes.contains(&Subtype::Vampire));
        assert!(filter.subtypes.contains(&Subtype::Wraith));
        assert!(filter.subtypes.contains(&Subtype::Zombie));

        // Should require battlefield zone
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn test_regenerate_cannot_target_non_creature() {
        use crate::card::CardBuilder;
        use crate::effect::EffectResult;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a non-creature artifact on the battlefield
        let artifact = CardBuilder::new(CardId::new(), "Test Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);

        // Create Accursed Duneyard as the source
        let duneyard_def = accursed_duneyard();
        let duneyard_id =
            game.create_object_from_definition(&duneyard_def, alice, Zone::Battlefield);

        // Try to execute regenerate effect on the artifact
        let regenerate_effect =
            Effect::regenerate(ChooseSpec::SpecificObject(artifact_id), Until::EndOfTurn);

        let mut ctx = ExecutionContext::new_default(duneyard_id, alice)
            .with_targets(vec![ResolvedTarget::Object(artifact_id)]);

        let result = execute_effect(&mut game, &regenerate_effect, &mut ctx);
        assert!(result.is_ok());

        // Result should be TargetInvalid since artifacts can't be regenerated
        assert!(
            matches!(result.unwrap().result, EffectResult::TargetInvalid),
            "Regenerate should return TargetInvalid for non-creature targets"
        );

        // Artifact should have no regeneration shields
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(artifact_id),
            0,
            "Non-creature should not receive regeneration shield"
        );
    }

    #[test]
    fn test_regeneration_does_not_work_if_creature_loses_creature_type() {
        use crate::effect::EffectResult;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a Zombie creature with a regeneration shield
        let zombie_id = create_undead_creature(&mut game, alice, "Test Zombie", Subtype::Zombie);

        // Apply regeneration via the proper effect (creates replacement effect)
        let regenerate_effect =
            Effect::regenerate(ChooseSpec::SpecificObject(zombie_id), Until::EndOfTurn);
        let mut regen_ctx = ExecutionContext::new_default(zombie_id, alice);
        execute_effect(&mut game, &regenerate_effect, &mut regen_ctx).unwrap();

        // Verify it's currently a creature with a regen shield
        assert!(
            game.object(zombie_id).unwrap().is_creature(),
            "Should start as a creature"
        );
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(zombie_id),
            1,
            "Should have a regeneration shield"
        );

        // Simulate the creature losing its creature type (e.g., from an effect like Opalescence
        // being removed from a creature-enchantment, or a type-changing effect)
        {
            let zombie = game.object_mut(zombie_id).unwrap();
            zombie.card_types.clear();
            zombie.card_types.push(CardType::Enchantment); // Now it's just an enchantment
        }

        // Verify it's no longer a creature
        assert!(
            !game.object(zombie_id).unwrap().is_creature(),
            "Should no longer be a creature after losing creature type"
        );

        // Create a source for the destroy effect
        let duneyard_def = accursed_duneyard();
        let source_id = game.create_object_from_definition(&duneyard_def, alice, Zone::Battlefield);

        // Try to destroy the (now non-creature) permanent
        let destroy_effect = Effect::destroy(ChooseSpec::SpecificObject(zombie_id));

        let mut ctx = ExecutionContext::new_default(source_id, alice)
            .with_targets(vec![ResolvedTarget::Object(zombie_id)]);

        let result = execute_effect(&mut game, &destroy_effect, &mut ctx);
        assert!(result.is_ok());

        // The permanent should be destroyed (moved to graveyard) because regeneration
        // only works on creatures, and it's no longer a creature
        let outcome = result.unwrap();
        assert!(
            matches!(outcome.result, EffectResult::Resolved),
            "Destroy should succeed (not be replaced by regeneration) for non-creatures, got {:?}",
            outcome.result
        );

        // The permanent should no longer be on battlefield
        assert!(
            !game.battlefield.contains(&zombie_id),
            "Non-creature should be destroyed despite having regeneration shield"
        );

        // The player's graveyard should contain the destroyed permanent
        // (Note: zone changes create new object IDs per rule 400.7, so we check the graveyard directly)
        let alice_graveyard = &game.player(alice).unwrap().graveyard;
        assert!(
            !alice_graveyard.is_empty(),
            "Non-creature should be in graveyard after destruction"
        );
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Accursed Duneyard's regenerate ability on a Spirit creature.
    ///
    /// Accursed Duneyard has: {2}, {T}: Regenerate target Spirit/Zombie/etc.
    /// We use Selfless Spirit (a white Spirit) as the target.
    ///
    /// Scenario: Alice activates Accursed Duneyard to regenerate Selfless Spirit.
    /// Expected: Selfless Spirit gains a regeneration shield.
    ///
    /// Note: Abilities with mana costs require mana in pool before they become available.
    /// So we must tap lands first, then the activated ability appears as an option.
    ///
    /// Initial actions for battlefield [Accursed Duneyard, Selfless Spirit, Swamp, Swamp]:
    /// 0=pass, 1=Duneyard mana, 2=Swamp1 mana, 3=Swamp2 mana, 4=Spirit sacrifice
    #[test]
    fn test_replay_accursed_duneyard_regenerate() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Starting state: Accursed Duneyard + Selfless Spirit + 2 Swamps on battlefield
                // Actions (from debug output):
                //   [0] Pass
                //   [1] Mana: Accursed Duneyard
                //   [2] Ability: Selfless Spirit (sacrifice!)
                //   [3] Mana: Swamp
                //   [4] Mana: Swamp
                "3", // Tap first Swamp for mana (adds {B})
                // Actions now:
                //   [0] Pass
                //   [1] Mana: Accursed Duneyard
                //   [2] Ability: Selfless Spirit
                //   [3] Mana: Swamp
                "3", // Tap second Swamp for mana (adds {B}, now have {B}{B} = 2 black)
                // Now with {B}{B} in pool, Duneyard regenerate becomes available
                // Actions now (from debug output):
                //   [0] Pass
                //   [1] Mana: Accursed Duneyard
                //   [2] Ability: Accursed Duneyard (regenerate!)
                //   [3] Ability: Selfless Spirit (sacrifice)
                "2", // Activate Accursed Duneyard's regenerate ability
                "0", // Target Selfless Spirit (the Spirit creature)
                     // Mana is paid from pool, ability goes on stack and resolves via auto-pass
            ],
            ReplayTestConfig::new().p1_battlefield(vec![
                "Accursed Duneyard",
                "Selfless Spirit",
                "Swamp",
                "Swamp",
            ]),
        );

        // Selfless Spirit should still be on battlefield
        assert!(
            game.battlefield_has("Selfless Spirit"),
            "Selfless Spirit should still be on battlefield"
        );

        // Selfless Spirit should have a regeneration shield
        let alice = PlayerId::from_index(0);
        let spirit_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Selfless Spirit" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(spirit_id) = spirit_id {
            assert_eq!(
                game.replacement_effects
                    .count_one_shot_effects_from_source(spirit_id),
                1,
                "Selfless Spirit should have 1 regeneration shield after Accursed Duneyard activation"
            );
        } else {
            panic!("Could not find Selfless Spirit on battlefield");
        }

        // Accursed Duneyard should be tapped (was used for the ability)
        let duneyard_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Accursed Duneyard")
                .unwrap_or(false)
        });

        if let Some(duneyard_id) = duneyard_id {
            assert!(
                game.is_tapped(duneyard_id),
                "Accursed Duneyard should be tapped after activating its ability"
            );
        } else {
            panic!("Could not find Accursed Duneyard on battlefield");
        }
    }

    /// Tests that regeneration from Accursed Duneyard prevents Doom Blade destruction.
    ///
    /// Scenario:
    /// 1. Alice taps lands for mana
    /// 2. Alice activates Accursed Duneyard to regenerate Selfless Spirit
    /// 3. Alice casts Doom Blade targeting Selfless Spirit
    /// 4. Regeneration shield should prevent destruction
    ///
    /// Expected: Selfless Spirit survives, is tapped, and damage is removed.
    ///
    /// Battlefield order: [Swamp, Swamp, Swamp, Swamp, Accursed Duneyard, Selfless Spirit]
    /// Initial actions: 0=pass, 1=cast Doom Blade, 2-5=Swamp mana, 6=Duneyard mana, 7=Spirit sac
    #[test]
    fn test_replay_accursed_duneyard_regenerate_prevents_doom_blade() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Starting state: 4 Swamps + Accursed Duneyard + Selfless Spirit on battlefield
                // Doom Blade in hand
                // Actions (expected):
                //   [0] Pass
                //   [1] Cast: Doom Blade
                //   [2] Mana: Swamp 1
                //   [3] Mana: Swamp 2
                //   [4] Mana: Swamp 3
                //   [5] Mana: Swamp 4
                //   [6] Mana: Accursed Duneyard
                //   [7] Ability: Selfless Spirit (sacrifice)

                // Step 1: Tap 2 Swamps for mana to enable regenerate ({2} cost)
                "2", // Tap Swamp 1 for {B}
                "2", // Tap Swamp 2 for {B} (pool now has {B}{B})
                // Now with {B}{B} in pool, Duneyard regenerate becomes available
                // Expected actions after tapping 2 Swamps:
                //   [0] Pass
                //   [1] Cast: Doom Blade
                //   [2] Mana: Swamp 3
                //   [3] Mana: Swamp 4
                //   [4] Mana: Accursed Duneyard
                //   [5] Ability: Accursed Duneyard (regenerate - now affordable!)
                //   [6] Ability: Selfless Spirit (sacrifice)

                // Step 2: Activate regenerate on Selfless Spirit
                "5", // Activate Duneyard regenerate (costs {2}, pays with {B}{B}, taps Duneyard)
                "0", // Target Selfless Spirit
                // Ability is now on the stack. We must PASS to let it resolve!
                // (Auto-pass only happens when Pass is the ONLY action)
                "", // Alice passes priority - regenerate ability resolves
                // Now Spirit has a regeneration shield. Pool is empty, Duneyard is tapped.

                // Step 3: Cast Doom Blade targeting Selfless Spirit
                // Expected actions after regenerate resolves:
                //   [0] Pass
                //   [1] Cast: Doom Blade
                //   [2] Mana: Swamp 3
                //   [3] Mana: Swamp 4
                //   [4] Ability: Selfless Spirit (sacrifice)
                "1", // Cast Doom Blade
                "0", // Target Selfless Spirit
                "0", // Tap Swamp 3 for {B}
                "0", // Tap Swamp 4 for {B} (spell resolves via auto-pass)
                     // Doom Blade tries to destroy Spirit, but regeneration kicks in!
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Doom Blade"])
                .p1_battlefield(vec![
                    "Swamp",
                    "Swamp",
                    "Swamp",
                    "Swamp",
                    "Accursed Duneyard",
                    "Selfless Spirit",
                ]),
        );

        let alice = PlayerId::from_index(0);

        // Selfless Spirit should STILL be on battlefield (regeneration prevented destruction)
        assert!(
            game.battlefield_has("Selfless Spirit"),
            "Selfless Spirit should survive Doom Blade due to regeneration"
        );

        // Selfless Spirit should be tapped (regeneration taps the creature)
        let spirit_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Selfless Spirit" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(spirit_id) = spirit_id {
            assert!(
                game.is_tapped(spirit_id),
                "Selfless Spirit should be tapped after regeneration"
            );
            // Regeneration shield should be consumed
            assert_eq!(
                game.replacement_effects
                    .count_one_shot_effects_from_source(spirit_id),
                0,
                "Regeneration shield should be consumed after preventing destruction"
            );
        } else {
            panic!("Could not find Selfless Spirit on battlefield");
        }

        // Doom Blade should be in graveyard (it resolved, just didn't destroy)
        let alice_player = game.player(alice).unwrap();
        let blade_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Doom Blade")
                .unwrap_or(false)
        });
        assert!(
            blade_in_gy,
            "Doom Blade should be in graveyard after resolving"
        );

        // Selfless Spirit should NOT be in graveyard
        let spirit_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Selfless Spirit")
                .unwrap_or(false)
        });
        assert!(!spirit_in_gy, "Selfless Spirit should NOT be in graveyard");
    }

    /// Tests Accursed Duneyard's colorless mana ability.
    ///
    /// Scenario: Alice taps Accursed Duneyard for colorless mana.
    #[test]
    fn test_replay_accursed_duneyard_mana() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Cast Sol Ring using Accursed Duneyard + Plains for mana
                // Sol Ring costs {1}
                "1", // Cast Sol Ring
                "0", // Tap Accursed Duneyard for {C} (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Sol Ring"])
                .p1_battlefield(vec!["Accursed Duneyard"]),
        );

        // Sol Ring should be on the battlefield (proof that we could tap for mana)
        assert!(
            game.battlefield_has("Sol Ring"),
            "Sol Ring should be on battlefield after casting with Accursed Duneyard mana"
        );

        // Accursed Duneyard should be tapped
        let duneyard_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Accursed Duneyard")
                .unwrap_or(false)
        });

        if let Some(duneyard_id) = duneyard_id {
            assert!(
                game.is_tapped(duneyard_id),
                "Accursed Duneyard should be tapped after producing mana"
            );
        }
    }
}
