#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_remove_up_to_counters_choose_zero() {
    use crate::cards::definitions::the_birth_of_meletis;
    use crate::effects::execute_effect;

    // Test that player can choose to remove 0 counters with "up to" effect
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    // Put a saga on battlefield with 2 lore counters
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);
    game.object_mut(saga_id)
        .unwrap()
        .add_counters(CounterType::Lore, 2);

    // Create a decision maker that chooses to remove 0 counters
    struct ChooseZeroDecisionMaker;
    impl DecisionMaker for ChooseZeroDecisionMaker {
        fn decide_number(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            // Choose to remove 0 counters
            0
        }
    }

    let source_id = game.new_object_id();
    let mut dm = ChooseZeroDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_x(3)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(saga_id)])
        .with_decision_maker(&mut dm);

    let effect = Effect::remove_up_to_counters(
        CounterType::Lore,
        Value::X,
        crate::target::ChooseSpec::SpecificObject(saga_id),
    );

    let result = execute_effect(&mut game, &effect, &mut ctx);
    assert!(result.is_ok(), "Effect should succeed");

    // Verify 0 counters were removed
    let removed = result.unwrap().as_count().unwrap_or(-1);
    assert_eq!(
        removed, 0,
        "Should have removed 0 counters (player's choice)"
    );

    // Verify saga still has all 2 lore counters
    let saga = game.object(saga_id).unwrap();
    assert_eq!(
        saga.counters.get(&CounterType::Lore).copied().unwrap_or(0),
        2,
        "Saga should still have all 2 lore counters"
    );
}

#[test]
pub(super) fn test_remove_up_to_any_counters_multiple_types() {
    use crate::effects::execute_effect;

    // Test that RemoveUpToAnyCounters works with multiple counter types
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    // Create a creature with multiple types of counters
    let card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(999), "Test Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

    // Add multiple types of counters
    game.object_mut(creature_id)
        .unwrap()
        .add_counters(CounterType::PlusOnePlusOne, 3);
    game.object_mut(creature_id)
        .unwrap()
        .add_counters(CounterType::Charge, 2);

    // Verify initial state: 5 total counters
    let creature = game.object(creature_id).unwrap();
    assert_eq!(
        creature
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        3
    );
    assert_eq!(
        creature
            .counters
            .get(&CounterType::Charge)
            .copied()
            .unwrap_or(0),
        2
    );

    // Create a decision maker that chooses to remove 4 counters (2 Charge + 2 +1/+1)
    struct ChooseFourDecisionMaker;
    impl DecisionMaker for ChooseFourDecisionMaker {
        fn decide_counters(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::CountersContext,
        ) -> Vec<(CounterType, u32)> {
            // max_total is capped to min(X, total_available_counters) = min(10, 5) = 5
            assert_eq!(
                ctx.max_total, 5,
                "Max should be capped to available counters"
            );
            assert_eq!(
                ctx.available_counters.len(),
                2,
                "Should have 2 counter types"
            );
            // Choose to remove 2 Charge and 2 +1/+1 = 4 total
            vec![(CounterType::Charge, 2), (CounterType::PlusOnePlusOne, 2)]
        }
    }

    let source_id = game.new_object_id();
    let mut dm = ChooseFourDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_x(10) // X=10, but only 5 counters available
        .with_targets(vec![crate::effects::ResolvedTarget::Object(creature_id)])
        .with_decision_maker(&mut dm);

    let effect = Effect::remove_up_to_any_counters(
        Value::X,
        crate::target::ChooseSpec::SpecificObject(creature_id),
    );

    let result = execute_effect(&mut game, &effect, &mut ctx);
    assert!(result.is_ok(), "Effect should succeed");

    // Verify 4 counters were removed
    let removed = result.unwrap().as_count().unwrap_or(0);
    assert_eq!(removed, 4, "Should have removed 4 counters");

    // Verify final state: 1 +1/+1 counter remaining, 0 Charge remaining
    // (We chose to remove 2 Charge and 2 +1/+1)
    let creature = game.object(creature_id).unwrap();
    let charge_remaining = creature
        .counters
        .get(&CounterType::Charge)
        .copied()
        .unwrap_or(0);
    let plus_remaining = creature
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);

    assert_eq!(charge_remaining, 0, "All Charge counters should be removed");
    assert_eq!(plus_remaining, 1, "Should have 1 +1/+1 counter remaining");
}

#[test]
pub(super) fn test_hex_parasite_removes_loyalty_counters() {
    use crate::effects::execute_effect;

    // Test that Hex Parasite can remove loyalty counters from a planeswalker
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    // Create a planeswalker with loyalty counters
    let card =
        crate::card::CardBuilder::new(crate::ids::CardId::from_raw(998), "Test Planeswalker")
            .card_types(vec![CardType::Planeswalker])
            .build();
    let pw_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

    // Add loyalty counters
    game.object_mut(pw_id)
        .unwrap()
        .add_counters(CounterType::Loyalty, 4);

    // Create a decision maker that removes 2 loyalty counters
    struct ChooseTwoDecisionMaker;
    impl DecisionMaker for ChooseTwoDecisionMaker {
        fn decide_counters(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::CountersContext,
        ) -> Vec<(CounterType, u32)> {
            assert_eq!(
                ctx.available_counters.len(),
                1,
                "Should only have Loyalty counters"
            );
            assert_eq!(ctx.available_counters[0].0, CounterType::Loyalty);
            // Choose to remove 2 Loyalty counters
            vec![(CounterType::Loyalty, 2)]
        }
    }

    let source_id = game.new_object_id();
    let mut dm = ChooseTwoDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_x(5)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(pw_id)])
        .with_decision_maker(&mut dm);

    // Use the same effect Hex Parasite uses
    let effect = Effect::remove_up_to_any_counters(
        Value::X,
        crate::target::ChooseSpec::SpecificObject(pw_id),
    );

    let result = execute_effect(&mut game, &effect, &mut ctx);
    assert!(result.is_ok(), "Effect should succeed");

    // Verify 2 loyalty counters were removed
    let removed = result.unwrap().as_count().unwrap_or(0);
    assert_eq!(removed, 2, "Should have removed 2 counters");

    // Verify planeswalker has 2 loyalty remaining
    let pw = game.object(pw_id).unwrap();
    assert_eq!(
        pw.counters.get(&CounterType::Loyalty).copied().unwrap_or(0),
        2,
        "Planeswalker should have 2 loyalty remaining"
    );
}

#[test]
pub(super) fn test_planeswalker_etb_processing_seeds_starting_loyalty_counters() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let chandra = CardBuilder::new(CardId::from_raw(997), "Chandra Nalaar")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(6)
        .build();
    let hand_id = game.create_object_from_card(&chandra, alice, Zone::Hand);
    let result = game
        .move_object_with_etb_processing(hand_id, Zone::Battlefield)
        .expect("planeswalker should enter battlefield");

    let loyalty = game
        .object(result.new_id)
        .and_then(|obj| obj.counters.get(&CounterType::Loyalty).copied())
        .unwrap_or(0);
    assert_eq!(loyalty, 6, "planeswalker should enter with printed loyalty");

    crate::rules::state_based::apply_state_based_actions(&mut game);
    assert!(
        game.object(result.new_id)
            .is_some_and(|obj| obj.zone == Zone::Battlefield),
        "planeswalker should survive state-based actions after entering"
    );
}

#[test]
pub(super) fn test_create_object_on_battlefield_seeds_starting_loyalty_counters() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let gideon = CardBuilder::new(CardId::from_raw(996), "Test Gideon")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    let pw_id = game.create_object_from_card(&gideon, alice, Zone::Battlefield);

    let loyalty = game
        .object(pw_id)
        .and_then(|obj| obj.counters.get(&CounterType::Loyalty).copied())
        .unwrap_or(0);
    assert_eq!(
        loyalty, 4,
        "direct battlefield creation should seed loyalty"
    );

    crate::rules::state_based::apply_state_based_actions(&mut game);
    assert!(
        game.object(pw_id)
            .is_some_and(|obj| obj.zone == Zone::Battlefield),
        "directly created planeswalker should survive state-based actions"
    );
}

// ========================================================================
// Valley Floodcaller Tests
// ========================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_valley_floodcaller_grants_flash_to_sorceries() {
    use crate::cards::definitions::valley_floodcaller;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Give Alice enough mana
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(crate::mana::ManaSymbol::Blue, 5);

    // Create Valley Floodcaller on battlefield
    let floodcaller_def = valley_floodcaller();
    let _floodcaller_id =
        game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

    // Create a sorcery in Alice's hand
    let sorcery = CardBuilder::new(CardId::from_raw(100), "Test Sorcery")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .build();
    let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

    // Check that the sorcery has been granted flash
    let flash_ability = crate::static_abilities::StaticAbility::flash();
    let has_granted_flash = game.effect_store.grant_registry.card_has_granted_ability(
        &game,
        sorcery_id,
        Zone::Hand,
        alice,
        &flash_ability,
    );
    assert!(
        has_granted_flash,
        "Valley Floodcaller should grant flash to sorceries in hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_valley_floodcaller_does_not_grant_flash_to_creatures() {
    use crate::cards::definitions::valley_floodcaller;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Valley Floodcaller on battlefield
    let floodcaller_def = valley_floodcaller();
    let _floodcaller_id =
        game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

    // Create a creature in Alice's hand
    let creature = CardBuilder::new(CardId::from_raw(100), "Test Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Hand);

    // Check that the creature has NOT been granted flash
    let flash_ability = crate::static_abilities::StaticAbility::flash();
    let has_granted_flash = game.effect_store.grant_registry.card_has_granted_ability(
        &game,
        creature_id,
        Zone::Hand,
        alice,
        &flash_ability,
    );
    assert!(
        !has_granted_flash,
        "Valley Floodcaller should NOT grant flash to creatures in hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_valley_floodcaller_flash_grant_removed_when_floodcaller_leaves() {
    use crate::cards::definitions::valley_floodcaller;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Valley Floodcaller on battlefield
    let floodcaller_def = valley_floodcaller();
    let floodcaller_id =
        game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

    // Create a sorcery in Alice's hand
    let sorcery = CardBuilder::new(CardId::from_raw(100), "Test Sorcery")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .build();
    let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

    let flash_ability = crate::static_abilities::StaticAbility::flash();

    // Verify sorcery has flash while Floodcaller is on battlefield
    assert!(
        game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            sorcery_id,
            Zone::Hand,
            alice,
            &flash_ability,
        ),
        "Sorcery should have flash while Floodcaller is on battlefield"
    );

    // Remove Floodcaller from battlefield
    game.move_object_by_effect(floodcaller_id, Zone::Graveyard);

    // Verify sorcery no longer has flash
    assert!(
        !game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            sorcery_id,
            Zone::Hand,
            alice,
            &flash_ability,
        ),
        "Sorcery should NOT have flash after Floodcaller leaves battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_valley_floodcaller_sorcery_castable_during_combat() {
    use crate::cards::definitions::valley_floodcaller;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Give Alice mana
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(crate::mana::ManaSymbol::Blue, 5);

    // Create Valley Floodcaller on battlefield
    let floodcaller_def = valley_floodcaller();
    let _floodcaller_id =
        game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

    // Create a sorcery in Alice's hand
    let sorcery = CardBuilder::new(CardId::from_raw(100), "Draw Spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .build();
    let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

    // Set to combat phase (not main phase)
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);

    // Check that the sorcery can be cast during combat (has flash)
    let actions = compute_legal_actions(&game, alice);
    let can_cast_sorcery = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell { spell_id, .. } if *spell_id == sorcery_id
        )
    });

    assert!(
        can_cast_sorcery,
        "Should be able to cast sorcery during combat thanks to Valley Floodcaller granting flash"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_valley_floodcaller_only_grants_to_controller() {
    use crate::cards::definitions::valley_floodcaller;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Create Valley Floodcaller on Alice's battlefield
    let floodcaller_def = valley_floodcaller();
    let _floodcaller_id =
        game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

    // Create sorceries in both players' hands
    let alice_sorcery = CardBuilder::new(CardId::from_raw(100), "Alice Sorcery")
        .card_types(vec![CardType::Sorcery])
        .build();
    let alice_sorcery_id = game.create_object_from_card(&alice_sorcery, alice, Zone::Hand);

    let bob_sorcery = CardBuilder::new(CardId::from_raw(101), "Bob Sorcery")
        .card_types(vec![CardType::Sorcery])
        .build();
    let bob_sorcery_id = game.create_object_from_card(&bob_sorcery, bob, Zone::Hand);

    let flash_ability = crate::static_abilities::StaticAbility::flash();

    // Alice's sorcery should have flash
    assert!(
        game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            alice_sorcery_id,
            Zone::Hand,
            alice,
            &flash_ability,
        ),
        "Alice's sorcery should have flash from her Floodcaller"
    );

    // Bob's sorcery should NOT have flash (Alice's Floodcaller doesn't grant to opponents)
    assert!(
        !game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            bob_sorcery_id,
            Zone::Hand,
            bob,
            &flash_ability,
        ),
        "Bob's sorcery should NOT have flash from Alice's Floodcaller"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_compute_legal_actions_respects_valley_floodcaller_flash_grant_on_opponents_turn()
{
    use crate::cards::definitions::valley_floodcaller;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);

    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(crate::mana::ManaSymbol::Blue, 2);

    let floodcaller_def = valley_floodcaller();
    let _floodcaller_id =
        game.create_object_from_definition(&floodcaller_def, alice, Zone::Battlefield);

    let sorcery = CardBuilder::new(CardId::from_raw(102), "Opponent Turn Sorcery")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .build();
    let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell { spell_id, from_zone, .. }
                    if *spell_id == sorcery_id && *from_zone == Zone::Hand
            )
        }),
        "Valley Floodcaller should still let Alice cast sorceries at flash timing on Bob's turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn return_to_dust_allows_second_target_on_your_main_phase() {
    use crate::decision::{GameProgress, LegalAction};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::White, 4);

    let return_to_dust = CardDefinitionBuilder::new(CardId::from_raw(100_910), "Return to Dust")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Colorless],
            vec![ManaSymbol::Colorless],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile target artifact or enchantment. If you cast this spell during your main phase, you may exile up to one other target artifact or enchantment.",
        )
        .expect("Return to Dust should parse");

    let spell_id = game.create_object_from_definition(&return_to_dust, alice, Zone::Hand);
    let target_a = CardBuilder::new(CardId::from_raw(100_911), "Target A")
        .card_types(vec![CardType::Artifact])
        .build();
    let target_b = CardBuilder::new(CardId::from_raw(100_912), "Target B")
        .card_types(vec![CardType::Enchantment])
        .build();
    game.create_object_from_card(&target_a, bob, Zone::Battlefield);
    game.create_object_from_card(&target_b, bob, Zone::Battlefield);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("Return to Dust cast should start");

    let targets = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(
            ctx,
        )) => ctx,
        other => panic!("unexpected cast flow state for Return to Dust: {other:?}"),
    };

    assert_eq!(
        targets.requirements.len(),
        2,
        "main phase cast should expose both target branches"
    );
    assert_eq!(targets.requirements[0].min_targets, 1);
    assert_eq!(targets.requirements[0].max_targets, Some(1));
    assert_eq!(targets.requirements[1].min_targets, 0);
    assert_eq!(targets.requirements[1].max_targets, Some(1));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn return_to_dust_does_not_allow_second_target_outside_main_phase() {
    use crate::decision::{GameProgress, LegalAction};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game.turn.priority_player = Some(alice);

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::White, 4);

    let return_to_dust = CardDefinitionBuilder::new(CardId::from_raw(100_913), "Return to Dust")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Colorless],
            vec![ManaSymbol::Colorless],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile target artifact or enchantment. If you cast this spell during your main phase, you may exile up to one other target artifact or enchantment.",
        )
        .expect("Return to Dust should parse");

    let spell_id = game.create_object_from_definition(&return_to_dust, alice, Zone::Hand);
    let target_a = CardBuilder::new(CardId::from_raw(100_914), "Target A")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&target_a, bob, Zone::Battlefield);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("Return to Dust cast should start");

    let targets = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(
            ctx,
        )) => ctx,
        other => panic!("unexpected cast flow state for Return to Dust: {other:?}"),
    };

    assert_eq!(
        targets.requirements.len(),
        1,
        "non-main-phase cast should not expose second target branch"
    );
    assert_eq!(targets.requirements[0].min_targets, 1);
    assert_eq!(targets.requirements[0].max_targets, Some(1));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn return_to_dust_does_not_allow_second_target_on_opponents_turn() {
    use crate::decision::{GameProgress, LegalAction};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = bob;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::White, 4);

    let return_to_dust = CardDefinitionBuilder::new(CardId::from_raw(100_915), "Return to Dust")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Colorless],
            vec![ManaSymbol::Colorless],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile target artifact or enchantment. If you cast this spell during your main phase, you may exile up to one other target artifact or enchantment.",
        )
        .expect("Return to Dust should parse");

    let spell_id = game.create_object_from_definition(&return_to_dust, alice, Zone::Hand);
    let target_a = CardBuilder::new(CardId::from_raw(100_916), "Target A")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&target_a, bob, Zone::Battlefield);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("Return to Dust cast should start");

    let targets = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(
            ctx,
        )) => ctx,
        other => panic!("unexpected cast flow state for Return to Dust: {other:?}"),
    };

    assert_eq!(
        targets.requirements.len(),
        1,
        "opponent-turn cast should not expose second target branch"
    );
    assert_eq!(targets.requirements[0].min_targets, 1);
    assert_eq!(targets.requirements[0].max_targets, Some(1));
}

#[test]
pub(super) fn test_gift_given_event_queues_opponent_gives_gift_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let library_card = CardBuilder::new(CardId::from_raw(9200), "Gift Trigger Draw")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&library_card, alice, Zone::Library);

    let watcher = CardDefinitionBuilder::new(CardId::from_raw(9201), "Gift Watcher")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(Ability::triggered(
            Trigger::player_gives_gift(PlayerFilter::Opponent),
            vec![Effect::target_draws(1, PlayerFilter::You)],
        ))
        .build();
    let watcher_id = game.create_object_from_definition(&watcher, alice, Zone::Battlefield);

    let gift_source = CardBuilder::new(CardId::from_raw(9202), "Gift Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let gift_source_id = game.create_object_from_card(&gift_source, bob, Zone::Stack);

    let effect = Effect::emit_gift_given(PlayerFilter::ChosenPlayer);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(gift_source_id, bob, &mut dm);
    ctx.combat.chosen_player = Some(alice);
    let outcome = execute_effect(&mut game, &effect, &mut ctx).expect("gift event should resolve");

    let event = outcome
        .events
        .into_iter()
        .next()
        .expect("gift event should be emitted");
    let gift_event = event
        .downcast::<crate::events::GiftGivenEvent>()
        .expect("gift emitter should produce a GiftGivenEvent");
    assert_eq!(gift_event.player, bob);
    assert_eq!(gift_event.recipient, alice);
    assert_eq!(gift_event.source, gift_source_id);

    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "opponent-gives-gift trigger should be queued once"
    );
    assert_eq!(trigger_queue.entries[0].source, watcher_id);
    assert_eq!(
        trigger_queue.entries[0].ability.trigger.display(),
        "Whenever an opponent gives a gift"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("gift trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("gift trigger should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        1,
        "the queued gift trigger should resolve using the normal stack path"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn trove_tracker_dies_trigger_draws_a_card() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let tracker = CardDefinitionBuilder::new(CardId::from_raw(93_001), "Trove Tracker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("When this creature dies, draw a card.\nEncore {5}{U}{U}")
        .expect("Trove Tracker text should parse");
    let tracker_id = game.create_object_from_definition(&tracker, alice, Zone::Battlefield);
    let library_card = CardBuilder::new(CardId::from_raw(93_003), "Draw Probe")
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_card(&library_card, alice, Zone::Library);

    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(tracker_id)
            .expect("Trove Tracker permanent should exist"),
        &game,
    );
    let lookback_source_snapshots = game.trigger_source_lookback_snapshots();
    let dies_event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            tracker_id,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_sba(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(lookback_source_snapshots);
    queue_triggers_from_event(&mut game, &mut trigger_queue, dies_event, false);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Trove Tracker should trigger when it dies"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("trigger should be put on stack");
    resolve_stack_entry(&mut game).expect("draw trigger should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        1,
        "Trove Tracker death trigger should draw exactly one card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn trove_tracker_only_triggers_on_battlefield_to_graveyard_moves() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let tracker = CardDefinitionBuilder::new(CardId::from_raw(93_002), "Trove Tracker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("When this creature dies, draw a card.\nEncore {5}{U}{U}")
        .expect("Trove Tracker text should parse");
    let tracker_id = game.create_object_from_definition(&tracker, alice, Zone::Hand);

    let snapshot = ObjectSnapshot::from_object(
        game.object(tracker_id)
            .expect("Trove Tracker card should exist"),
        &game,
    );
    let non_dies_event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            tracker_id,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, non_dies_event, false);

    assert!(
        trigger_queue.entries.is_empty(),
        "moving Trove Tracker from hand to graveyard should not count as dying"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn gloomwidows_feast_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(154_394), "Gloomwidow's Feast")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target creature with flying. If that creature was blue or black, create a 1/2 green Spider creature token with reach.",
        )
        .expect("Gloomwidow's Feast should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_gloomwidows_feast_target(
    game: &mut GameState,
    controller: PlayerId,
    name: &str,
    colors: crate::color::ColorSet,
) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .color_indicator(colors)
        .flying()
        .build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_gloomwidows_feast_at(
    game: &mut GameState,
    controller: PlayerId,
    target: ObjectId,
) {
    let def = gloomwidows_feast_definition();
    let spell_id = game.create_object_from_definition(&def, controller, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, controller).with_targets(vec![Target::Object(target)]),
    );
    resolve_stack_entry(game).expect("Gloomwidow's Feast should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn gloomwidow_spider_tokens_controlled_by(
    game: &GameState,
    player: PlayerId,
) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id).is_some_and(|object| {
                game.controller_of(object) == player
                    && object.kind == ObjectKind::Token
                    && object.name == "Spider"
                    && object.card_types.contains(&CardType::Creature)
                    && object.subtypes.contains(&Subtype::Spider)
                    && game.current_power(id) == Some(1)
                    && game.current_toughness(id) == Some(2)
                    && game.current_colors(id) == Some(crate::color::ColorSet::GREEN)
                    && game.object_has_static_ability_id(
                        id,
                        crate::static_abilities::StaticAbilityId::Reach,
                    )
            })
        })
        .collect()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gloomwidows_feast_targets_only_creatures_with_flying() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let def = gloomwidows_feast_definition();
    let effects = def
        .spell_effect
        .as_ref()
        .expect("Gloomwidow's Feast should have spell effects");
    let flying_creature = create_gloomwidows_feast_target(
        &mut game,
        bob,
        "Gloomwidow Legal Flying Target",
        crate::color::ColorSet::BLUE,
    );
    let nonflying_creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Gloomwidow Nonflying Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .color_indicator(crate::color::ColorSet::BLUE)
            .build(),
        bob,
        Zone::Battlefield,
    );
    let flying_artifact = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Gloomwidow Flying Artifact")
            .card_types(vec![CardType::Artifact])
            .flying()
            .build(),
        bob,
        Zone::Battlefield,
    );

    let requirements = extract_target_requirements(&game, effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "Gloomwidow's Feast should require exactly one target"
    );
    let legal_targets = &requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(flying_creature)),
        "flying creatures should be legal Gloomwidow's Feast targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(nonflying_creature)),
        "nonflying creatures should not be legal Gloomwidow's Feast targets, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(flying_artifact)),
        "noncreature permanents with flying should not be legal Gloomwidow's Feast targets, got {legal_targets:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gloomwidows_feast_destroys_blue_flying_target_and_creates_spider() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = create_gloomwidows_feast_target(
        &mut game,
        bob,
        "Gloomwidow Blue Target",
        crate::color::ColorSet::BLUE,
    );

    resolve_gloomwidows_feast_at(&mut game, alice, target);

    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Gloomwidow Blue Target"),
        1,
        "Gloomwidow's Feast should destroy its flying creature target"
    );
    assert_eq!(
        gloomwidow_spider_tokens_controlled_by(&game, alice).len(),
        1,
        "Gloomwidow's Feast should create the Spider when the destroyed target was blue"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gloomwidows_feast_destroys_black_flying_target_and_creates_spider() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = create_gloomwidows_feast_target(
        &mut game,
        bob,
        "Gloomwidow Black Target",
        crate::color::ColorSet::BLACK,
    );

    resolve_gloomwidows_feast_at(&mut game, alice, target);

    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Gloomwidow Black Target"),
        1,
        "Gloomwidow's Feast should destroy its black flying creature target"
    );
    assert_eq!(
        gloomwidow_spider_tokens_controlled_by(&game, alice).len(),
        1,
        "Gloomwidow's Feast should create the Spider when the destroyed target was black"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gloomwidows_feast_destroys_green_flying_target_without_creating_spider() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = create_gloomwidows_feast_target(
        &mut game,
        bob,
        "Gloomwidow Green Target",
        crate::color::ColorSet::GREEN,
    );

    resolve_gloomwidows_feast_at(&mut game, alice, target);

    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Gloomwidow Green Target"),
        1,
        "Gloomwidow's Feast should still destroy a non-blue, non-black flying target"
    );
    assert!(
        gloomwidow_spider_tokens_controlled_by(&game, alice).is_empty(),
        "Gloomwidow's Feast should not create a Spider when the destroyed target was neither blue nor black"
    );
}
