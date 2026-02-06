//! Saw in Half card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Saw in Half - {2}{B}
/// Instant
/// Destroy target creature. If that creature dies this way, its controller
/// creates two tokens that are copies of that creature, except their base
/// power is half that creature's power and their base toughness is half
/// that creature's toughness. Round up each time.
pub fn saw_in_half() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Saw in Half")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target creature. If that creature dies this way, its controller \
             creates two tokens that are copies of that creature, except their base \
             power is half that creature's power and their base toughness is half \
             that creature's toughness. Round up each time.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{EffectOutcome, EffectResult};
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::object::Object;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(
        card_id: u32,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        power: i32,
        toughness: i32,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name, power, toughness);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn execute_saw_in_half(
        game: &mut GameState,
        caster: PlayerId,
        target: Option<ObjectId>,
    ) -> Vec<EffectOutcome> {
        let def = saw_in_half();
        let effects = def
            .spell_effect
            .as_ref()
            .expect("Saw in Half should have effects");
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, caster);
        if let Some(target_id) = target {
            ctx = ctx.with_targets(vec![ResolvedTarget::Object(target_id)]);
        }
        ctx.snapshot_targets(game);

        let mut outcomes = Vec::with_capacity(effects.len());
        for effect in effects {
            outcomes.push(effect.0.execute(game, &mut ctx).unwrap());
        }
        outcomes
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_saw_in_half_basic_properties() {
        let def = saw_in_half();
        assert_eq!(def.name(), "Saw in Half");
        assert!(def.is_spell());
        assert!(!def.is_creature());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_saw_in_half_is_instant() {
        let def = saw_in_half();
        assert!(def.card.is_instant());
    }

    #[test]
    fn test_saw_in_half_has_spell_effect() {
        let def = saw_in_half();
        assert!(def.spell_effect.is_some());
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_saw_in_half_creates_two_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a 4/4 creature controlled by Bob
        let creature_id = create_creature(&mut game, "Big Beast", 4, 4, bob);
        let outcomes = execute_saw_in_half(&mut game, alice, Some(creature_id));

        // Should return the two created tokens
        let token_ids = outcomes
            .iter()
            .find_map(|outcome| {
                if let EffectResult::Objects(ids) = &outcome.result {
                    Some(ids.clone())
                } else {
                    None
                }
            })
            .expect("Expected Objects result from create token effect");

        assert_eq!(token_ids.len(), 2, "Should create exactly two tokens");

        // Both tokens should be on the battlefield under Bob's control
        for token_id in &token_ids {
            let token = game.object(*token_id).expect("Token should exist");
            assert_eq!(
                token.controller, bob,
                "Token should be controlled by original creature's controller"
            );
            assert_eq!(token.zone, Zone::Battlefield);
            assert_eq!(token.name, "Big Beast");
            // 4/4 -> 2/2 (half rounded up)
            assert_eq!(
                token.power(),
                Some(2),
                "Token power should be half (rounded up)"
            );
            assert_eq!(
                token.toughness(),
                Some(2),
                "Token toughness should be half (rounded up)"
            );
        }

        // Original creature should be in graveyard
        assert!(
            game.players[bob.index()].graveyard.iter().any(|&id| {
                game.object(id)
                    .map(|o| o.name == "Big Beast")
                    .unwrap_or(false)
            }),
            "Original creature should be in graveyard"
        );
    }

    #[test]
    fn test_saw_in_half_odd_stats_round_up() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a 5/3 creature - should become two 3/2 tokens
        let creature_id = create_creature(&mut game, "Odd Creature", 5, 3, bob);
        let outcomes = execute_saw_in_half(&mut game, alice, Some(creature_id));
        let token_ids = outcomes
            .iter()
            .find_map(|outcome| {
                if let EffectResult::Objects(ids) = &outcome.result {
                    Some(ids.clone())
                } else {
                    None
                }
            })
            .expect("Expected Objects result from create token effect");

        for token_id in &token_ids {
            let token = game.object(*token_id).expect("Token should exist");
            // 5/3 -> 3/2 (ceil(5/2)=3, ceil(3/2)=2)
            assert_eq!(
                token.power(),
                Some(3),
                "Power 5 should become 3 (rounded up)"
            );
            assert_eq!(
                token.toughness(),
                Some(2),
                "Toughness 3 should become 2 (rounded up)"
            );
        }
    }

    #[test]
    fn test_saw_in_half_1_1_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a 1/1 creature - should become two 1/1 tokens (ceil(1/2) = 1)
        let creature_id = create_creature(&mut game, "Tiny Creature", 1, 1, bob);
        let outcomes = execute_saw_in_half(&mut game, alice, Some(creature_id));
        let token_ids = outcomes
            .iter()
            .find_map(|outcome| {
                if let EffectResult::Objects(ids) = &outcome.result {
                    Some(ids.clone())
                } else {
                    None
                }
            })
            .expect("Expected Objects result from create token effect");

        for token_id in &token_ids {
            let token = game.object(*token_id).expect("Token should exist");
            // 1/1 -> 1/1 (ceil(1/2)=1)
            assert_eq!(token.power(), Some(1));
            assert_eq!(token.toughness(), Some(1));
        }
    }

    #[test]
    fn test_saw_in_half_indestructible() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature_id = create_creature(&mut game, "Darksteel Colossus", 4, 4, bob);

        // Add indestructible
        if let Some(obj) = game.object_mut(creature_id) {
            obj.abilities.push(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::indestructible(),
            ));
        }

        let outcomes = execute_saw_in_half(&mut game, alice, Some(creature_id));

        // Should be protected, no tokens created
        assert_eq!(outcomes[0].result, EffectResult::Protected);

        // Creature should still be on battlefield
        assert!(game.battlefield.contains(&creature_id));
    }

    #[test]
    fn test_saw_in_half_own_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Alice targets her own creature
        let creature_id = create_creature(&mut game, "Sacrifice Target", 6, 6, alice);
        let outcomes = execute_saw_in_half(&mut game, alice, Some(creature_id));
        let token_ids = outcomes
            .iter()
            .find_map(|outcome| {
                if let EffectResult::Objects(ids) = &outcome.result {
                    Some(ids.clone())
                } else {
                    None
                }
            })
            .expect("Expected Objects result from create token effect");

        assert_eq!(token_ids.len(), 2);
        for token_id in &token_ids {
            let token = game.object(*token_id).expect("Token should exist");
            // Tokens go under Alice's control (the creature's controller)
            assert_eq!(token.controller, alice);
            // 6/6 -> 3/3
            assert_eq!(token.power(), Some(3));
            assert_eq!(token.toughness(), Some(3));
        }
    }

    #[test]
    fn test_saw_in_half_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let outcomes = execute_saw_in_half(&mut game, alice, None);
        assert_eq!(outcomes[0].result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_saw_in_half_tokens_have_summoning_sickness() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature_id = create_creature(&mut game, "Test Creature", 4, 4, bob);
        let outcomes = execute_saw_in_half(&mut game, alice, Some(creature_id));
        let token_ids = outcomes
            .iter()
            .find_map(|outcome| {
                if let EffectResult::Objects(ids) = &outcome.result {
                    Some(ids.clone())
                } else {
                    None
                }
            })
            .expect("Expected Objects result from create token effect");

        for token_id in &token_ids {
            assert!(
                game.is_summoning_sick(*token_id),
                "Tokens should have summoning sickness"
            );
        }
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Saw in Half destroying a creature and creating two half-sized token copies.
    ///
    /// Saw in Half: {2}{B} instant
    /// Destroy target creature. If that creature dies this way, its controller
    /// creates two tokens that are copies of that creature, except their base
    /// power is half that creature's power and their base toughness is half
    /// that creature's toughness. Round up each time.
    ///
    /// Scenario: Alice casts Saw in Half targeting Bob's Serra Angel (4/4).
    /// Expected: Serra Angel is destroyed, Bob gets two 2/2 token copies.
    #[test]
    fn test_replay_saw_in_half_creates_tokens() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Saw in Half
                "0", // Target Serra Angel (Bob's creature)
                "0", // Tap first Swamp
                "0", // Tap second Swamp
                "0", // Tap third Swamp (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Saw in Half"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp"])
                .p2_battlefield(vec!["Serra Angel"]),
        );

        let bob = PlayerId::from_index(1);

        // Original Serra Angel should NOT be on battlefield anymore
        // (Note: the original card may have been moved, but no Serra Angel cards should remain)
        let serra_angels_on_bf: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.name == "Serra Angel")
            .collect();

        // Should have exactly 2 Serra Angel tokens on battlefield
        assert_eq!(
            serra_angels_on_bf.len(),
            2,
            "Should have exactly 2 Serra Angel tokens on battlefield"
        );

        // Both tokens should be controlled by Bob
        for token in &serra_angels_on_bf {
            assert_eq!(
                token.controller, bob,
                "Tokens should be controlled by Bob (original creature's controller)"
            );
            // 4/4 -> 2/2 (half rounded up)
            assert_eq!(
                token.power(),
                Some(2),
                "Token power should be 2 (half of 4)"
            );
            assert_eq!(
                token.toughness(),
                Some(2),
                "Token toughness should be 2 (half of 4)"
            );
        }

        // Original Serra Angel should be in Bob's graveyard
        let bob_player = game.player(bob).unwrap();
        let serra_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Serra Angel")
                .unwrap_or(false)
        });
        assert!(
            serra_in_gy,
            "Original Serra Angel should be in Bob's graveyard"
        );
    }

    /// Tests Saw in Half targeting an indestructible creature.
    ///
    /// Scenario: Alice casts Saw in Half targeting Bob's Darksteel Colossus (indestructible 11/11).
    /// Expected: Darksteel Colossus survives, no tokens are created.
    #[test]
    fn test_replay_saw_in_half_indestructible_survives() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Saw in Half
                "0", // Target Darksteel Colossus (Bob's creature)
                "0", // Tap first Swamp
                "0", // Tap second Swamp
                "0", // Tap third Swamp (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Saw in Half"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp"])
                .p2_battlefield(vec!["Darksteel Colossus"]),
        );

        let bob = PlayerId::from_index(1);

        // Darksteel Colossus should still be on battlefield (indestructible)
        let colossus_on_bf: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.name == "Darksteel Colossus")
            .collect();

        assert_eq!(
            colossus_on_bf.len(),
            1,
            "Darksteel Colossus should still be on battlefield (indestructible)"
        );

        // The Colossus should still be controlled by Bob
        assert_eq!(
            colossus_on_bf[0].controller, bob,
            "Darksteel Colossus should still be controlled by Bob"
        );

        // Original stats should be unchanged (11/11)
        assert_eq!(
            colossus_on_bf[0].power(),
            Some(11),
            "Darksteel Colossus should still have 11 power"
        );
        assert_eq!(
            colossus_on_bf[0].toughness(),
            Some(11),
            "Darksteel Colossus should still have 11 toughness"
        );

        // No tokens should have been created (creature didn't die)
        let total_creatures_on_bf = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.is_creature())
            .count();

        assert_eq!(
            total_creatures_on_bf, 1,
            "Only the original Darksteel Colossus should be on battlefield, no tokens"
        );

        // Darksteel Colossus should NOT be in graveyard
        let bob_player = game.player(bob).unwrap();
        let colossus_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Darksteel Colossus")
                .unwrap_or(false)
        });
        assert!(
            !colossus_in_gy,
            "Darksteel Colossus should NOT be in graveyard"
        );
    }
}
