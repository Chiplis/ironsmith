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
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_convoke_taps_creatures_on_cast() {
    // When casting with Convoke, the creatures used should be tapped
    use crate::cards::definitions::stoke_the_flames;
    use crate::color::ColorSet;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Create 4 red creatures (enough to pay the entire cost with Convoke)
    let mut creature_ids = Vec::new();
    for i in 0..4 {
        let creature = CardBuilder::new(CardId::new(), format!("Red Creature {}", i))
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.remove_summoning_sickness(id);
        creature_ids.push(id);
    }

    // Put Stoke the Flames in hand
    let stoke_def = stoke_the_flames();
    let stoke_id = game.create_object_from_definition(&stoke_def, alice, Zone::Hand);

    // Give alice no mana - should still be able to cast with 4 creatures
    // (2 pay generic, 2 pay red)

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    let cast_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                ..
            } if *spell_id == stoke_id
        )
    });

    assert!(
        cast_action.is_some(),
        "Should be able to cast Stoke the Flames with 4 creatures to convoke (paying all costs)"
    );

    // Cast the spell - since it requires targeting, we need to handle that
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());

    // Apply the cast action - this returns the ChooseTargets decision
    let response = PriorityResponse::PriorityAction(cast_action.unwrap().clone());
    let result =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response).unwrap();

    // The spell requires targets, so we should get a ChooseTargets decision
    if let GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_)) =
        result
    {
        // Choose bob as target - this finalizes the cast and taps creatures
        let target_response = PriorityResponse::Targets(vec![Target::Player(bob)]);
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &target_response)
            .unwrap();
    } else {
        panic!("Expected ChooseTargets decision, got {:?}", result);
    }

    // Now the spell should be on the stack and creatures should be tapped
    // Check how many creatures are tapped
    let tapped_count = creature_ids
        .iter()
        .filter(|&&id| game.is_tapped(id))
        .count();

    assert!(
        tapped_count >= 2,
        "At least 2 creatures should be tapped for Convoke (tapped: {})",
        tapped_count
    );
}

#[test]
pub(super) fn test_convoke_colored_creatures_pay_colored_mana() {
    // Red creatures should be used to pay {R} pips first
    use crate::color::ColorSet;
    use crate::decision::{calculate_convoke_cost, get_convoke_creatures};
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create 2 red creatures and 2 colorless creatures
    let red1 = CardBuilder::new(CardId::from_raw(800), "Red Creature 1")
        .card_types(vec![CardType::Creature])
        .color_indicator(ColorSet::RED)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let red2 = CardBuilder::new(CardId::from_raw(801), "Red Creature 2")
        .card_types(vec![CardType::Creature])
        .color_indicator(ColorSet::RED)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let colorless1 = CardBuilder::new(CardId::from_raw(802), "Colorless Creature 1")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let colorless2 = CardBuilder::new(CardId::from_raw(803), "Colorless Creature 2")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let red1_id = game.create_object_from_card(&red1, alice, Zone::Battlefield);
    let red2_id = game.create_object_from_card(&red2, alice, Zone::Battlefield);
    let colorless1_id = game.create_object_from_card(&colorless1, alice, Zone::Battlefield);
    let colorless2_id = game.create_object_from_card(&colorless2, alice, Zone::Battlefield);

    // Mark them as not summoning sick
    for id in [red1_id, red2_id, colorless1_id, colorless2_id] {
        game.remove_summoning_sickness(id);
    }

    // Get convoke creatures
    let convoke_creatures = get_convoke_creatures(&game, alice);
    assert_eq!(
        convoke_creatures.len(),
        4,
        "Should have 4 creatures available for convoke"
    );

    // Calculate convoke cost for Stoke the Flames: {2}{R}{R}
    let cost = ManaCost::from_pips(vec![
        vec![ManaSymbol::Generic(2)],
        vec![ManaSymbol::Red],
        vec![ManaSymbol::Red],
    ]);

    let (creatures_to_tap, remaining_cost) = calculate_convoke_cost(&game, alice, &cost);

    // Should tap all 4 creatures: 2 red for the {R}{R}, 2 colorless for the {2}
    assert_eq!(
        creatures_to_tap.len(),
        4,
        "Should tap 4 creatures to pay the entire cost"
    );

    // Remaining cost should be empty (mana value 0)
    assert_eq!(
        remaining_cost.mana_value(),
        0,
        "Remaining cost should be 0 after tapping 4 creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_convoke_summoning_sick_creatures_can_be_tapped() {
    // Convoke taps creatures to pay a spell's cost, so summoning sickness does not matter.
    use crate::cards::definitions::stoke_the_flames;
    use crate::decision::{compute_legal_actions, get_convoke_creatures};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Create 4 summoning sick creatures so Stoke the Flames can be fully convoked.
    let creature1 = CardBuilder::new(CardId::from_raw(800), "Fresh Creature 1")
        .card_types(vec![CardType::Creature])
        .color_indicator(crate::color::ColorSet::RED)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature2 = CardBuilder::new(CardId::from_raw(801), "Fresh Creature 2")
        .card_types(vec![CardType::Creature])
        .color_indicator(crate::color::ColorSet::RED)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature3 = CardBuilder::new(CardId::from_raw(802), "Fresh Creature 3")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature4 = CardBuilder::new(CardId::from_raw(803), "Fresh Creature 4")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let id1 = game.create_object_from_card(&creature1, alice, Zone::Battlefield);
    let id2 = game.create_object_from_card(&creature2, alice, Zone::Battlefield);
    let id3 = game.create_object_from_card(&creature3, alice, Zone::Battlefield);
    let id4 = game.create_object_from_card(&creature4, alice, Zone::Battlefield);

    // Simulate all four creatures entering this turn.
    game.set_summoning_sick(id1);
    game.set_summoning_sick(id2);
    game.set_summoning_sick(id3);
    game.set_summoning_sick(id4);

    let stoke_def = stoke_the_flames();
    let stoke_id = game.create_object_from_definition(&stoke_def, alice, Zone::Hand);

    let convoke_creatures = get_convoke_creatures(&game, alice);
    assert_eq!(
        convoke_creatures.len(),
        4,
        "All untapped creatures should be available for convoke, even while summoning sick"
    );

    let actions = compute_legal_actions(&game, alice);
    let cast_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                ..
            } if *spell_id == stoke_id
        )
    });
    assert!(
        cast_action.is_some(),
        "Should be able to cast Stoke the Flames by convoking summoning-sick creatures"
    );

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let response = PriorityResponse::PriorityAction(cast_action.expect("cast action").clone());
    let result =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response).unwrap();

    if let GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_)) =
        result
    {
        let target_response = PriorityResponse::Targets(vec![Target::Player(bob)]);
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &target_response)
            .unwrap();
    } else {
        panic!("Expected ChooseTargets decision, got {:?}", result);
    }

    let tapped_count = [id1, id2, id3, id4]
        .iter()
        .filter(|&&id| game.is_tapped(id))
        .count();
    assert!(
        tapped_count >= 4,
        "All four creatures should be tapped when fully convoking Stoke the Flames"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_improvise_reduces_mana_cost_with_artifacts() {
    // Reverse Engineer costs {3}{U}{U} with Improvise
    // With 3 untapped artifacts, it should cost just {U}{U}
    use crate::cards::definitions::reverse_engineer;
    use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Create 3 untapped artifacts on battlefield
    for i in 0..3 {
        let artifact = CardBuilder::new(CardId::new(), format!("Artifact {}", i))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    }

    // Put Reverse Engineer in hand
    let re_def = reverse_engineer();
    let re_id = game.create_object_from_definition(&re_def, alice, Zone::Hand);

    // Give alice {U}{U} mana (3 artifacts pay the {3})
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    // Verify the effective cost is just {U}{U} (mana value 2)
    let re_obj = game.object(re_id).unwrap();
    let base_cost = re_obj.mana_cost.as_ref().unwrap();
    let effective_cost = calculate_effective_mana_cost(&game, alice, re_obj, base_cost);
    assert_eq!(
        effective_cost.mana_value(),
        2,
        "Effective cost should be 2 (just UU) with 3 artifacts to improvise"
    );

    // Compute legal actions - Reverse Engineer should be castable
    let actions = compute_legal_actions(&game, alice);

    let can_cast_re = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                ..
            } if *spell_id == re_id
        )
    });

    assert!(
        can_cast_re,
        "Should be able to cast Reverse Engineer with 3 artifacts to improvise and 2 blue mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_improvise_taps_artifacts_on_cast() {
    // When casting with Improvise, the artifacts used should be tapped
    use crate::cards::definitions::reverse_engineer;
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Create 3 untapped artifacts
    let mut artifact_ids = Vec::new();
    for i in 0..3 {
        let artifact = CardBuilder::new(CardId::new(), format!("Artifact {}", i))
            .card_types(vec![CardType::Artifact])
            .build();
        let id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        artifact_ids.push(id);
    }

    // Put Reverse Engineer in hand
    let re_def = reverse_engineer();
    let re_id = game.create_object_from_definition(&re_def, alice, Zone::Hand);

    // Give alice {U}{U} mana (3 artifacts pay the {3})
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    let cast_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                ..
            } if *spell_id == re_id
        )
    });

    assert!(
        cast_action.is_some(),
        "Should be able to cast Reverse Engineer"
    );

    // Cast the spell (no targets needed for draw spell)
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());

    let response = PriorityResponse::PriorityAction(cast_action.unwrap().clone());
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &response).unwrap();

    // Now the spell should be on the stack and artifacts should be tapped
    let tapped_count = artifact_ids
        .iter()
        .filter(|&&id| game.is_tapped(id))
        .count();

    assert_eq!(
        tapped_count, 3,
        "All 3 artifacts should be tapped for Improvise"
    );
}

#[test]
pub(super) fn test_improvise_only_pays_generic_mana() {
    // Improvise cannot pay for colored mana pips
    use crate::decision::{calculate_improvise_cost, get_improvise_artifacts};
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create 5 untapped artifacts (more than enough)
    for i in 0..5 {
        let artifact = CardBuilder::new(CardId::new(), format!("Artifact {}", i))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    }

    // Verify artifacts are available
    let artifacts = get_improvise_artifacts(&game, alice);
    assert_eq!(artifacts.len(), 5, "Should have 5 artifacts available");

    // Calculate improvise cost for {3}{U}{U} - should only reduce the {3}
    let cost = ManaCost::from_pips(vec![
        vec![ManaSymbol::Generic(3)],
        vec![ManaSymbol::Blue],
        vec![ManaSymbol::Blue],
    ]);

    let (artifacts_to_tap, remaining_cost) = calculate_improvise_cost(&game, alice, &cost);

    // Should tap 3 artifacts to pay the {3}
    assert_eq!(
        artifacts_to_tap.len(),
        3,
        "Should tap 3 artifacts to pay the generic mana"
    );

    // Remaining cost should be {U}{U} (mana value 2)
    assert_eq!(
        remaining_cost.mana_value(),
        2,
        "Remaining cost should be 2 (UU) - Improvise doesn't pay colored"
    );
}

#[test]
pub(super) fn test_improvise_already_tapped_artifacts_cannot_be_used() {
    // Tapped artifacts cannot be used for Improvise
    use crate::decision::get_improvise_artifacts;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create 3 artifacts - 2 tapped, 1 untapped
    for i in 0..3 {
        let artifact = CardBuilder::new(CardId::new(), format!("Artifact {}", i))
            .card_types(vec![CardType::Artifact])
            .build();
        let id = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        if i < 2 {
            game.tap(id);
        }
    }

    // Get improvise artifacts
    let artifacts = get_improvise_artifacts(&game, alice);

    // Should only get the 1 untapped artifact
    assert_eq!(
        artifacts.len(),
        1,
        "Only 1 artifact should be available (tapped artifacts can't improvise)"
    );
}

// =========================================================================
// Search Library Tests (The Birth of Meletis)
// =========================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_search_library_finds_matching_card() {
    use crate::cards::definitions::{basic_plains, the_birth_of_meletis};
    use crate::decision::DecisionMaker;
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;

    // Decision maker that always selects the first matching card
    struct SelectFirstDecisionMaker;
    impl DecisionMaker for SelectFirstDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            // Select the first legal candidate
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .take(1)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Add basic Plains to library
    let plains_def = basic_plains();
    let _plains_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

    // Also add some non-Plains cards to make the search interesting
    for i in 0..3 {
        let card = CardBuilder::new(CardId::new(), format!("Random Card {}", i))
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }

    let initial_hand_size = game.player(alice).unwrap().hand.len();
    let initial_library_size = game.player(alice).unwrap().library.len();

    // Create a dummy source object for the context
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    // Execute the search effect directly
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

    let search_effect = Effect::search_library(
        crate::target::ObjectFilter::default()
            .with_supertype(crate::types::Supertype::Basic)
            .with_subtype(crate::types::Subtype::Plains),
        Zone::Hand,
        crate::target::PlayerFilter::You,
        true,
    );

    let result = execute_effect(&mut game, &search_effect, &mut ctx);
    assert!(result.is_ok(), "Search should succeed");

    // Verify Plains moved to hand
    let final_hand_size = game.player(alice).unwrap().hand.len();
    assert_eq!(
        final_hand_size,
        initial_hand_size + 1,
        "Should have one more card in hand"
    );

    // Verify library has one fewer card
    let final_library_size = game.player(alice).unwrap().library.len();
    assert_eq!(
        final_library_size,
        initial_library_size - 1,
        "Should have one fewer card in library"
    );

    // Verify the card in hand is a Plains
    let hand = &game.player(alice).unwrap().hand;
    let plains_in_hand = hand.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Plains" && o.subtypes.contains(&crate::types::Subtype::Plains))
            .unwrap_or(false)
    });
    assert!(plains_in_hand, "Plains should be in hand after search");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_search_library_no_matching_cards() {
    use crate::cards::definitions::the_birth_of_meletis;
    use crate::decision::DecisionMaker;
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;

    // Decision maker for search (shouldn't be called if no matches)
    struct NoMatchDecisionMaker;
    impl DecisionMaker for NoMatchDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            // Should have no matching cards
            assert!(ctx.candidates.is_empty(), "Should have no matching cards");
            vec![]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Add only non-Plains cards to library (no basic Plains)
    for i in 0..3 {
        let card = CardBuilder::new(CardId::new(), format!("Non-Plains Card {}", i))
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }

    let initial_hand_size = game.player(alice).unwrap().hand.len();

    // Create source
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    // Execute the search effect
    let mut dm = NoMatchDecisionMaker;
    let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

    let search_effect = Effect::search_library(
        crate::target::ObjectFilter::default()
            .with_supertype(crate::types::Supertype::Basic)
            .with_subtype(crate::types::Subtype::Plains),
        Zone::Hand,
        crate::target::PlayerFilter::You,
        true,
    );

    let result = execute_effect(&mut game, &search_effect, &mut ctx);
    assert!(result.is_ok(), "Search should complete without error");

    // Result should indicate nothing was found
    if let Ok(outcome) = result
        && let crate::effect::OutcomeValue::Count(n) = outcome.value
    {
        assert_eq!(n, 0, "Should find 0 cards when no Plains in library");
    }

    // Hand size should be unchanged
    let final_hand_size = game.player(alice).unwrap().hand.len();
    assert_eq!(
        final_hand_size, initial_hand_size,
        "Hand size should be unchanged when no matching cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_search_library_fail_to_find() {
    use crate::cards::definitions::{basic_plains, the_birth_of_meletis};
    use crate::decision::DecisionMaker;
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;

    // Decision maker that always chooses to "fail to find" even with matching cards
    struct FailToFindDecisionMaker;
    impl DecisionMaker for FailToFindDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            // Verify there ARE matching cards, but we choose not to find them
            assert!(
                !ctx.candidates.is_empty(),
                "Should have matching cards available"
            );
            // Return empty to "fail to find"
            vec![]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Add basic Plains to library
    let plains_def = basic_plains();
    let _plains_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

    let initial_hand_size = game.player(alice).unwrap().hand.len();
    let initial_library_size = game.player(alice).unwrap().library.len();

    // Create source
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    // Execute the search effect with fail-to-find decision maker
    let mut dm = FailToFindDecisionMaker;
    let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

    let search_effect = Effect::search_library(
        crate::target::ObjectFilter::default()
            .with_supertype(crate::types::Supertype::Basic)
            .with_subtype(crate::types::Subtype::Plains),
        Zone::Hand,
        crate::target::PlayerFilter::You,
        true,
    );

    let result = execute_effect(&mut game, &search_effect, &mut ctx);
    assert!(result.is_ok(), "Search should complete without error");

    // Result should indicate nothing was found (player chose to fail)
    if let Ok(outcome) = result
        && let crate::effect::OutcomeValue::Count(n) = outcome.value
    {
        assert_eq!(
            n, 0,
            "Should report 0 cards found when player fails to find"
        );
    }

    // Hand size should be unchanged (player declined to take the Plains)
    let final_hand_size = game.player(alice).unwrap().hand.len();
    assert_eq!(
        final_hand_size, initial_hand_size,
        "Hand size should be unchanged when player fails to find"
    );

    // Library size should also be unchanged (no card moved)
    let final_library_size = game.player(alice).unwrap().library.len();
    assert_eq!(
        final_library_size, initial_library_size,
        "Library size should be unchanged when player fails to find"
    );

    // Plains should still be in library
    let library = &game.player(alice).unwrap().library;
    let plains_in_library = library
        .iter()
        .any(|&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false));
    assert!(
        plains_in_library,
        "Plains should still be in library after fail to find"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_search_library_for_card_cannot_fail_to_find() {
    use crate::cards::definitions::the_birth_of_meletis;
    use crate::decision::DecisionMaker;
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;

    struct FailToFindDecisionMaker;
    impl DecisionMaker for FailToFindDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert!(
                !ctx.allow_partial_completion,
                "quantity-only library searches should not allow failing to find"
            );
            Vec::new()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let card = CardBuilder::new(CardId::new(), "Tutor Target")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_card(&card, alice, Zone::Library);

    let source = the_birth_of_meletis();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let mut dm = FailToFindDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut dm);

    let search_effect = Effect::search_library(
        crate::target::ObjectFilter::default(),
        Zone::Hand,
        crate::target::PlayerFilter::You,
        false,
    );

    let result = execute_effect(&mut game, &search_effect, &mut ctx);
    assert!(result.is_ok(), "Search should succeed");

    let hand = &game.player(alice).expect("alice").hand;
    assert!(
        hand.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "Tutor Target")),
        "search for a card should put the searched card into hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_search_library_selects_specific_card() {
    use crate::cards::definitions::{basic_island, basic_plains, the_birth_of_meletis};
    use crate::decision::DecisionMaker;
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;

    // Decision maker that selects the second matching card (if available)
    struct SelectSecondDecisionMaker;
    impl DecisionMaker for SelectSecondDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            // Select second card if available, otherwise first
            let legal_ids: Vec<ObjectId> = ctx
                .candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .collect();
            if legal_ids.len() > 1 {
                vec![legal_ids[1]]
            } else if let Some(&id) = legal_ids.first() {
                vec![id]
            } else {
                vec![]
            }
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Add multiple basic Plains to library
    let plains_def = basic_plains();
    let plains1_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);
    let plains2_id = game.create_object_from_definition(&plains_def, alice, Zone::Library);

    // Add a non-matching card between them
    let island_def = basic_island();
    let _island_id = game.create_object_from_definition(&island_def, alice, Zone::Library);

    // Create source
    let saga_def = the_birth_of_meletis();
    let saga_id = game.create_object_from_definition(&saga_def, alice, Zone::Battlefield);

    // Execute the search effect
    let mut dm = SelectSecondDecisionMaker;
    let mut ctx = ExecutionContext::new_default(saga_id, alice).with_decision_maker(&mut dm);

    let search_effect = Effect::search_library(
        crate::target::ObjectFilter::default()
            .with_supertype(crate::types::Supertype::Basic)
            .with_subtype(crate::types::Subtype::Plains),
        Zone::Hand,
        crate::target::PlayerFilter::You,
        true,
    );

    let result = execute_effect(&mut game, &search_effect, &mut ctx);
    assert!(result.is_ok(), "Search should succeed");

    // Verify exactly one Plains moved to hand
    let hand = &game.player(alice).unwrap().hand;
    let plains_count_in_hand = hand
        .iter()
        .filter(|&&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false))
        .count();
    assert_eq!(
        plains_count_in_hand, 1,
        "Exactly one Plains should be in hand"
    );

    // Verify one Plains remains in library
    let library = &game.player(alice).unwrap().library;
    let plains_count_in_library = library
        .iter()
        .filter(|&&id| game.object(id).map(|o| o.name == "Plains").unwrap_or(false))
        .count();
    assert_eq!(
        plains_count_in_library, 1,
        "One Plains should remain in library"
    );

    // Check that one of the specific Plains IDs moved
    // (Note: IDs change on zone change, so we check by name)
    let moved_to_hand = !game.player(alice).unwrap().library.contains(&plains1_id)
        || !game.player(alice).unwrap().library.contains(&plains2_id);
    assert!(moved_to_hand, "One of the Plains should have moved to hand");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_evolving_door_finds_two_color_creature_and_respects_may_cast_decline() {
    use crate::ability::AbilityKind;
    use crate::decision::DecisionMaker;
    use crate::game_loop::resolve_stack_entry_with_dm_and_triggers;

    #[derive(Default)]
    struct DeclineMayDecisionMaker;

    impl DecisionMaker for DeclineMayDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let legal: Vec<ObjectId> = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect();
            let required = ctx.min.max(1);
            let count = ctx.max.unwrap_or(required).min(legal.len()).max(required);
            legal.into_iter().take(count).collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let evolving_door = CardDefinitionBuilder::new(CardId::new(), "Evolving Door Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}, {T}, Sacrifice a creature: Count the colors of the sacrificed creature, then search your library for a creature card that's exactly that many colors plus one. Exile that card, then shuffle. You may cast the exiled card. Activate only as a sorcery.",
        )
        .expect("evolving door probe should parse");
    let door_id = game.create_object_from_definition(&evolving_door, alice, Zone::Battlefield);
    let one_color_fodder = CardBuilder::new(CardId::new(), "One Color Fodder")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .color_indicator(crate::color::ColorSet::GREEN)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let _one_color_id = game.create_object_from_card(&one_color_fodder, alice, Zone::Library);

    let two_color_prize = CardBuilder::new(CardId::new(), "Two Color Prize")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Blue],
        ]))
        .color_indicator(crate::color::ColorSet::GREEN.union(crate::color::ColorSet::BLUE))
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let _two_color_id = game.create_object_from_card(&two_color_prize, alice, Zone::Library);

    let sacrifice_fodder = CardBuilder::new(CardId::new(), "Sacrificial Fodder")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .color_indicator(crate::color::ColorSet::GREEN)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let fodder_id = game.create_object_from_card(&sacrifice_fodder, alice, Zone::Battlefield);

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let ability_index = game
        .object(door_id)
        .expect("evolving door should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("evolving door should have an activated ability");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = DeclineMayDecisionMaker;

    let activate = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
        source: door_id,
        ability_index,
    });
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &activate,
        &mut dm,
    )
    .expect("activation should start");

    let progress = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(cost_ctx),
        ) => {
            let sacrifice_cost_index = cost_ctx
                .options
                .iter()
                .find(|option| {
                    option
                        .description
                        .to_ascii_lowercase()
                        .contains("sacrifice")
                })
                .map(|option| option.index)
                .expect("expected a sacrifice cost option for Evolving Door");

            apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::NextCostChoice(sacrifice_cost_index),
                &mut dm,
            )
            .expect("choosing the sacrifice cost should continue Evolving Door activation")
        }
        other => panic!(
            "expected Evolving Door to ask for a cost choice first, got {:?}",
            other
        ),
    };

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {}
        other => panic!(
            "expected Evolving Door to ask for a sacrificed creature after choosing the cost, got {:?}",
            other
        ),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::SacrificeTarget(fodder_id),
        &mut dm,
    )
    .expect("should sacrifice a creature to activate Evolving Door");

    assert_eq!(
        game.stack.len(),
        1,
        "Evolving Door's activated ability should be on the stack after costs are paid"
    );

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Evolving Door should resolve");

    assert!(
        game.exile.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "Two Color Prize")),
        "Evolving Door should exile the two-color creature when the sacrificed creature has one color"
    );
    assert!(
        !game.exile.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "One Color Fodder")),
        "Evolving Door should not be able to choose the one-color creature from the library"
    );
    assert!(
        !game.battlefield.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "Two Color Prize")),
        "declining the may-cast choice should leave the exiled creature in exile"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_silverglade_elemental_may_search_puts_forest_onto_battlefield() {
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::basic_forest;
    use crate::decision::DecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, ObjectId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    struct ChooseForestDecisionMaker;
    impl DecisionMaker for ChooseForestDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .find(|candidate| {
                    game.object(candidate.id)
                        .map(|obj| obj.name == "Forest")
                        .unwrap_or(false)
                })
                .map(|candidate| vec![candidate.id])
                .unwrap_or_default()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Build Silverglade from parser text to exercise the exact parse/compile path.
    let silverglade = CardDefinitionBuilder::new(CardId::new(), "Silverglade Elemental")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Green],
            ]))
            .power_toughness(PowerToughness::fixed(3, 4))
            .parse_text(
                "When this creature enters, you may search your library for a Forest card, put that card onto the battlefield, then shuffle.",
            )
            .expect("silverglade text should parse");

    let silverglade_id = game.create_object_from_definition(&silverglade, alice, Zone::Battlefield);

    let filler = CardBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&filler, alice, Zone::Library);
    let forest = basic_forest();
    let forest_library_id = game.create_object_from_definition(&forest, alice, Zone::Library);

    let triggered = silverglade
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("silverglade should have ETB trigger");
    assert!(
        !triggered.effects.is_empty(),
        "silverglade trigger should have effects"
    );
    let rendered_effect = format!("{:?}", triggered.effects[0]);
    assert!(
        rendered_effect.contains("MayEffect"),
        "search clause should preserve explicit may choice: {rendered_effect}"
    );

    let battlefield_before = game.battlefield.len();
    let library_before = game.player(alice).map(|p| p.library.len()).unwrap_or(0);

    let mut dm = ChooseForestDecisionMaker;
    let mut ctx = ExecutionContext::new_default(silverglade_id, alice).with_decision_maker(&mut dm);
    let outcome =
        execute_effect(&mut game, &triggered.effects[0], &mut ctx).expect("effect resolves");

    assert!(
        !matches!(outcome.value, crate::effect::OutcomeValue::Count(0)),
        "search should select and move a Forest"
    );
    assert_eq!(
        game.battlefield.len(),
        battlefield_before + 1,
        "forest should be added to battlefield"
    );
    assert_eq!(
        game.player(alice).map(|p| p.library.len()).unwrap_or(0),
        library_before - 1,
        "library should have one fewer card after moving forest"
    );
    assert!(
        game.object(forest_library_id).is_none(),
        "moved card should become a new object id"
    );
    let forest_on_battlefield = game.battlefield.iter().any(|&id| {
        game.object(id)
            .map(|obj| obj.name == "Forest" && obj.owner == alice)
            .unwrap_or(false)
    });
    assert!(forest_on_battlefield, "forest should be on battlefield");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_oreskos_explorer_searches_for_players_with_more_lands_than_you() {
    use crate::ability::AbilityKind;
    use crate::card::PowerToughness;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::basic_plains;
    use crate::decision::DecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    struct ChoosePlainsDecisionMaker;

    impl DecisionMaker for ChoosePlainsDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .filter(|candidate| {
                    game.object(candidate.id)
                        .is_some_and(|obj| obj.name == "Plains")
                })
                .map(|candidate| candidate.id)
                .take(2)
                .collect()
        }
    }

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    game.create_object_from_definition(&basic_plains(), alice, Zone::Battlefield);
    game.create_object_from_definition(&basic_plains(), bob, Zone::Battlefield);
    game.create_object_from_definition(&basic_plains(), bob, Zone::Battlefield);
    game.create_object_from_definition(&basic_plains(), charlie, Zone::Battlefield);
    game.create_object_from_definition(&basic_plains(), charlie, Zone::Battlefield);
    game.create_object_from_definition(&basic_plains(), charlie, Zone::Battlefield);

    let oreskos = CardDefinitionBuilder::new(CardId::new(), "Oreskos Explorer")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "When this creature enters, search your library for up to X Plains cards, where X is the number of players who control more lands than you. Reveal those cards, put them into your hand, then shuffle.",
        )
        .expect("Oreskos Explorer text should parse");

    let source_id = game.create_object_from_definition(&oreskos, alice, Zone::Battlefield);
    let triggered_effects = {
        let triggered = game
            .object(source_id)
            .expect("Oreskos Explorer should exist on the battlefield")
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered)
                    if triggered.trigger.display().contains("enters") =>
                {
                    Some(triggered)
                }
                _ => None,
            })
            .expect("Oreskos Explorer should have an ETB trigger");
        assert!(
            !triggered.effects.is_empty(),
            "Oreskos Explorer trigger should have effects"
        );
        triggered.effects.clone()
    };

    game.create_object_from_definition(&basic_plains(), alice, Zone::Library);
    game.create_object_from_definition(&basic_plains(), alice, Zone::Library);
    game.create_object_from_definition(&basic_plains(), alice, Zone::Library);
    let library_before = game.player(alice).expect("alice exists").library.len();
    let hand_before = game.player(alice).expect("alice exists").hand.len();

    let mut dm = ChoosePlainsDecisionMaker;
    let etb_event = TriggerEvent::new_with_provenance(
        EnterBattlefieldEvent::new(source_id, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_decision_maker(&mut dm)
        .with_triggering_event(etb_event);
    for effect in &triggered_effects {
        execute_effect(&mut game, effect, &mut ctx).expect("effect resolves");
    }

    let alice_state = game.player(alice).expect("alice exists");
    let found_plains = alice_state.hand.len() - hand_before;
    assert_eq!(
        found_plains, 2,
        "Oreskos Explorer should put two Plains into hand"
    );
    assert_eq!(
        alice_state.library.len(),
        library_before - 2,
        "Oreskos Explorer should remove two cards from the library"
    );

    let plains_in_hand = alice_state
        .hand
        .iter()
        .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Plains"))
        .count();
    assert_eq!(plains_in_hand, 2, "Oreskos Explorer should find two Plains");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_cream_of_the_crop_etb_trigger_uses_may_and_source_power_rearrange() {
    use crate::ability::AbilityKind;
    use crate::card::PowerToughness;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let cream = CardDefinitionBuilder::new(CardId::new(), "Cream of the Crop")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a creature you control enters, you may look at the top X cards of your library, where X is that creature's power. If you do, put one of those cards on top of your library and the rest on the bottom of your library in any order.")
        .expect("Cream of the Crop text should parse");
    let grizzly = CardDefinitionBuilder::new(CardId::new(), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("")
        .expect("vanilla creature should parse");

    let cream_id = game.create_object_from_definition(&cream, alice, Zone::Battlefield);
    let _creature_id = game.create_object_from_definition(&grizzly, alice, Zone::Battlefield);

    let trigger = game
        .object(cream_id)
        .expect("Cream of the Crop should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cream of the Crop should have an enters trigger");

    let trigger_debug = format!("{:#?}", trigger).to_ascii_lowercase();
    assert!(
        trigger_debug.contains("controller: some(\n                    you,")
            && trigger_debug.contains("card_types: [\n                    creature,"),
        "Cream trigger should only watch your creatures entering, got {trigger_debug}"
    );
    assert!(
        trigger_debug.contains("mayeffect"),
        "Cream trigger should preserve the may branch, got {trigger_debug}"
    );
    assert!(
        trigger_debug.contains("lookattopcards") && trigger_debug.contains("powerof"),
        "Cream trigger should scale looked card count from the entering creature's power, got {trigger_debug}"
    );
    assert!(
        trigger_debug.contains("chooseobjectseffect")
            && trigger_debug.contains("min: 1")
            && trigger_debug
                .contains("max: some(\n                                                1,")
            && trigger_debug.contains("foreachtaggedeffect")
            && trigger_debug.contains("to_top: true")
            && trigger_debug.contains("puttaggedremainderonlibrarybottomeffect"),
        "Cream trigger should rearrange looked cards by choosing exactly one for top, got {trigger_debug}"
    );
}

pub(super) fn doubling_chant_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Doubling Chant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "For each creature you control, you may search your library for a creature card with the same name as that creature. Put those cards onto the battlefield, then shuffle.",
        )
        .expect("Doubling Chant should parse")
}

pub(super) fn count_battlefield_permanents_named(
    game: &GameState,
    controller: PlayerId,
    name: &str,
) -> usize {
    game.battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == controller && obj.name == name)
        })
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_doubling_chant_resolves_search_for_each_creature_you_control() {
    use crate::cards::definitions::{grizzly_bears, llanowar_elves};
    use std::collections::VecDeque;

    struct DoublingChantDecisionMaker {
        may_choices: VecDeque<bool>,
    }

    impl DecisionMaker for DoublingChantDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.may_choices
                .pop_front()
                .expect("expected one may decision per creature iteration")
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    game.create_object_from_definition(&llanowar_elves(), alice, Zone::Battlefield);

    let bears_library_id =
        game.create_object_from_definition(&grizzly_bears(), alice, Zone::Library);
    let elf_library_id =
        game.create_object_from_definition(&llanowar_elves(), alice, Zone::Library);

    let battlefield_bears_before =
        count_battlefield_permanents_named(&game, alice, "Grizzly Bears");
    let battlefield_elves_before =
        count_battlefield_permanents_named(&game, alice, "Llanowar Elves");
    let library_before = game.player(alice).expect("alice exists").library.len();

    let doubling_chant = doubling_chant_definition();
    let spell_id = game.create_object_from_definition(&doubling_chant, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    let mut dm = DoublingChantDecisionMaker {
        may_choices: VecDeque::from(vec![true, true]),
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Doubling Chant should resolve");

    assert_eq!(
        count_battlefield_permanents_named(&game, alice, "Grizzly Bears"),
        battlefield_bears_before + 1,
        "Doubling Chant should put a Grizzly Bears match onto the battlefield"
    );
    assert_eq!(
        count_battlefield_permanents_named(&game, alice, "Llanowar Elves"),
        battlefield_elves_before + 1,
        "Doubling Chant should put a Llanowar Elves match onto the battlefield"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        library_before - 2,
        "each accepted search should remove its matching creature from the library"
    );
    assert!(
        game.object(bears_library_id).is_none(),
        "the searched Grizzly Bears should become a new battlefield object"
    );
    assert!(
        game.object(elf_library_id).is_none(),
        "the searched Llanowar Elves should become a new battlefield object"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_doubling_chant_declined_iteration_leaves_matching_card_in_library() {
    use crate::cards::definitions::{grizzly_bears, llanowar_elves};
    use std::collections::VecDeque;

    struct DoublingChantDecisionMaker {
        may_choices: VecDeque<bool>,
    }

    impl DecisionMaker for DoublingChantDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.may_choices
                .pop_front()
                .expect("expected one may decision per creature iteration")
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Battlefield creation order drives the per-creature iteration order here:
    // Grizzly Bears resolves first, then Llanowar Elves.
    game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    game.create_object_from_definition(&llanowar_elves(), alice, Zone::Battlefield);

    let bears_library_id =
        game.create_object_from_definition(&grizzly_bears(), alice, Zone::Library);
    let elf_library_id =
        game.create_object_from_definition(&llanowar_elves(), alice, Zone::Library);

    let battlefield_bears_before =
        count_battlefield_permanents_named(&game, alice, "Grizzly Bears");
    let battlefield_elves_before =
        count_battlefield_permanents_named(&game, alice, "Llanowar Elves");
    let library_before = game.player(alice).expect("alice exists").library.len();

    let doubling_chant = doubling_chant_definition();
    let spell_id = game.create_object_from_definition(&doubling_chant, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    let mut dm = DoublingChantDecisionMaker {
        may_choices: VecDeque::from(vec![true, false]),
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Doubling Chant should resolve");

    assert_eq!(
        count_battlefield_permanents_named(&game, alice, "Grizzly Bears"),
        battlefield_bears_before + 1,
        "accepting the first iteration should put the matching Grizzly Bears onto the battlefield"
    );
    assert_eq!(
        count_battlefield_permanents_named(&game, alice, "Llanowar Elves"),
        battlefield_elves_before,
        "declining the second iteration should leave Llanowar Elves unchanged on the battlefield"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        library_before - 1,
        "declining one iteration should leave its matching library card in place"
    );
    assert!(
        game.object(bears_library_id).is_none(),
        "the accepted Grizzly Bears search should move that card out of the library"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .library
            .contains(&elf_library_id),
        "the declined Llanowar Elves search should leave that card in the library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_doubling_chant_same_name_search_prompts_are_user_facing() {
    use crate::cards::definitions::ornithopter;
    use std::collections::VecDeque;

    struct DoublingChantPromptDecisionMaker {
        may_choices: VecDeque<bool>,
        boolean_prompts: Vec<String>,
        object_prompts: Vec<String>,
        object_candidate_names: Vec<String>,
        object_candidate_ids: Vec<ObjectId>,
    }

    impl DecisionMaker for DoublingChantPromptDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.boolean_prompts.push(ctx.description.clone());
            self.may_choices
                .pop_front()
                .expect("expected one may decision for the Doubling Chant iteration")
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.object_prompts.push(ctx.description.clone());
            self.object_candidate_names = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.name.to_string())
                .collect();
            self.object_candidate_ids = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect();
            self.object_candidate_ids.iter().copied().take(1).collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let battlefield_ornithopter =
        game.create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);
    let library_ornithopter_a =
        game.create_object_from_definition(&ornithopter(), alice, Zone::Library);
    let library_ornithopter_b =
        game.create_object_from_definition(&ornithopter(), alice, Zone::Library);

    let doubling_chant = doubling_chant_definition();
    let spell_id = game.create_object_from_definition(&doubling_chant, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    let mut dm = DoublingChantPromptDecisionMaker {
        may_choices: VecDeque::from(vec![true]),
        boolean_prompts: Vec::new(),
        object_prompts: Vec::new(),
        object_candidate_names: Vec::new(),
        object_candidate_ids: Vec::new(),
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Doubling Chant should resolve");

    let may_prompt = dm
        .boolean_prompts
        .first()
        .expect("Doubling Chant should prompt for the optional same-name search")
        .to_ascii_lowercase();
    assert!(
        may_prompt.contains("search your library for")
            && may_prompt.contains("creature card")
            && may_prompt.contains("same name as ornithopter"),
        "expected a user-facing Doubling Chant may prompt, got {:?}",
        dm.boolean_prompts
    );
    assert!(
        !may_prompt.contains("tags it as 'searched'"),
        "Doubling Chant may prompt should not expose internal search tags: {:?}",
        dm.boolean_prompts
    );

    let object_prompt = dm
        .object_prompts
        .first()
        .expect("accepting the may prompt should produce a library-choice prompt")
        .to_ascii_lowercase();
    assert!(
        object_prompt.contains("search your library for")
            && object_prompt.contains("creature card")
            && object_prompt.contains("same name as ornithopter"),
        "expected a user-facing Doubling Chant search prompt, got {:?}",
        dm.object_prompts
    );
    assert_eq!(
        dm.object_candidate_names,
        vec!["Ornithopter".to_string(), "Ornithopter".to_string()],
        "Doubling Chant should present the matching library Ornithopters as candidates"
    );
    assert!(
        !dm.object_candidate_ids.contains(&battlefield_ornithopter),
        "the battlefield Ornithopter should not appear in the library search candidates"
    );
    assert!(
        dm.object_candidate_ids.contains(&library_ornithopter_a)
            && dm.object_candidate_ids.contains(&library_ornithopter_b),
        "the search candidates should point at the library Ornithopter objects"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_sundering_eruption_lets_target_controller_search_after_land_dies() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::{basic_forest, basic_mountain};
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::ids::ObjectId;

    struct AcceptAndChooseFirstDecisionMaker;
    impl DecisionMaker for AcceptAndChooseFirstDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let sundering_eruption = CardDefinitionBuilder::new(CardId::new(), "Sundering Eruption")
            .parse_text(
                "Mana cost: {2}{R}\n\
                 Type: Sorcery\n\
                 Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield tapped, then shuffle. Creatures without flying can't block this turn.",
            )
            .expect("Sundering Eruption text should parse");
    let spell_effects = sundering_eruption
        .spell_effect
        .as_ref()
        .expect("Sundering Eruption should have spell effects");

    let source_id = game.create_object_from_definition(&sundering_eruption, alice, Zone::Hand);
    let target_land_id =
        game.create_object_from_definition(&basic_forest(), bob, Zone::Battlefield);
    let library_basic_id =
        game.create_object_from_definition(&basic_mountain(), bob, Zone::Library);
    let bob_library_before = game.player(bob).expect("bob exists").library.len();

    let mut dm = AcceptAndChooseFirstDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_decision_maker(&mut dm)
        .with_targets(vec![ResolvedTarget::Object(target_land_id)]);
    ctx.snapshot_targets(&game);

    for effect in spell_effects {
        execute_effect(&mut game, effect, &mut ctx).expect("spell effect should resolve");
    }

    let bob_battlefield_has_mountain = game.battlefield.iter().any(|&id| {
        game.object(id)
            .map(|obj| {
                obj.name == "Mountain" && game.controller_of(obj) == bob && game.is_tapped(id)
            })
            .unwrap_or(false)
    });
    assert!(
        bob_battlefield_has_mountain,
        "Bob should put a tapped basic land onto the battlefield"
    );
    let bob_graveyard_has_forest = game.player(bob).is_some_and(|player| {
        player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Forest" && obj.owner == bob)
                .unwrap_or(false)
        })
    });
    assert!(
        bob_graveyard_has_forest,
        "the destroyed target land should be in Bob's graveyard"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        bob_library_before - 1,
        "Bob should have searched a basic land out of the library"
    );
    assert!(
        game.object(library_basic_id).is_none(),
        "the searched basic land should become a new battlefield object"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_boseiju_channel_lets_destroyed_permanent_controller_search_for_land() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::{basic_forest, command_tower};
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::ids::CardId;
    use crate::ids::ObjectId;

    struct AcceptAndChooseFirstDecisionMaker;
    impl DecisionMaker for AcceptAndChooseFirstDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let boseiju = CardDefinitionBuilder::new(CardId::new(), "Boseiju, Who Endures")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}: Add {G}.\n\
             Channel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That permanent's controller may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle.\n\
             This ability costs {1} less to activate for each legendary creature you control.",
        )
        .expect("Boseiju text should parse");

    let source_id = game.create_object_from_definition(&boseiju, alice, Zone::Hand);
    let target_land_id =
        game.create_object_from_definition(&command_tower(), bob, Zone::Battlefield);
    let library_basic_id = game.create_object_from_definition(&basic_forest(), bob, Zone::Library);
    let bob_library_before = game.player(bob).expect("bob exists").library.len();

    let activated = game
        .object(source_id)
        .expect("Boseiju object should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if ability.functions_in(&Zone::Hand) => {
                Some(activated.clone())
            }
            _ => None,
        })
        .expect("Boseiju should have a hand-zone channel ability");

    let mut dm = AcceptAndChooseFirstDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_decision_maker(&mut dm)
        .with_targets(vec![ResolvedTarget::Object(target_land_id)]);
    ctx.snapshot_targets(&game);

    for effect in &activated.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("channel effect should resolve");
    }

    let bob_battlefield_has_forest = game.battlefield.iter().any(|&id| {
        game.object(id)
            .map(|obj| obj.name == "Forest" && game.controller_of(obj) == bob)
            .unwrap_or(false)
    });
    assert!(
        bob_battlefield_has_forest,
        "Boseiju should let the destroyed permanent's controller find a basic land"
    );
    assert!(
        game.player(bob)
            .is_some_and(|player| player.graveyard.iter().any(|&id| {
                game.object(id)
                    .map(|obj| obj.name == "Command Tower" && obj.owner == bob)
                    .unwrap_or(false)
            })),
        "the destroyed nonbasic land should be in Bob's graveyard"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        bob_library_before - 1,
        "Boseiju should remove the searched land from Bob's library"
    );
    assert!(
        game.object(library_basic_id).is_none(),
        "the searched land should become a new battlefield object"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_boseiju_channel_activation_flow_preserves_land_search_after_destroying_target() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::{basic_forest, command_tower};
    use crate::decision::{GameProgress, LegalAction};
    use crate::ids::CardId;

    struct AcceptAndChooseFirstDecisionMaker;
    impl DecisionMaker for AcceptAndChooseFirstDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let boseiju = CardDefinitionBuilder::new(CardId::new(), "Boseiju, Who Endures")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}: Add {G}.\n\
             Channel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That permanent's controller may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle.\n\
             This ability costs {1} less to activate for each legendary creature you control.",
        )
        .expect("Boseiju text should parse");

    let source_id = game.create_object_from_definition(&boseiju, alice, Zone::Hand);
    let ability_index = game
        .object(source_id)
        .expect("Boseiju should exist in hand")
        .abilities
        .iter()
        .position(|ability| {
            matches!(
                ability.kind,
                AbilityKind::Activated(_) if ability.functions_in(&Zone::Hand)
            )
        })
        .expect("Boseiju should expose its channel ability from hand");
    let target_land_id =
        game.create_object_from_definition(&command_tower(), bob, Zone::Battlefield);
    let library_basic_id = game.create_object_from_definition(&basic_forest(), bob, Zone::Library);
    let bob_library_before = game.player(bob).expect("bob exists").library.len();

    if let Some(player) = game.player_mut(alice) {
        player.mana_pool.add(ManaSymbol::Green, 2);
    }

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AcceptAndChooseFirstDecisionMaker;

    let activate = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
        source: source_id,
        ability_index,
    });
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &activate,
        &mut dm,
    )
    .expect("Boseiju activation should start");

    match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_)) => {}
        other => panic!("expected Boseiju to choose its target first, got {other:?}"),
    }

    let choose_target = PriorityResponse::Targets(vec![Target::Object(target_land_id)]);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_target,
        &mut dm,
    )
    .expect("Boseiju should accept its target");

    let next_cost_ctx = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected Boseiju to ask for its next cost, got {other:?}"),
    };

    let discard_cost_index = next_cost_ctx
        .options
        .iter()
        .find(|opt| opt.description.to_ascii_lowercase().contains("discard"))
        .map(|opt| opt.index)
        .expect("expected Boseiju to offer its discard cost");

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::NextCostChoice(discard_cost_index),
        &mut dm,
    )
    .expect("Boseiju should let us pay discard first");

    match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {
            apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::CardCostChoice(source_id),
                &mut dm,
            )
            .expect("discarding Boseiju should finish the activation flow");
        }
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Priority(_)) => {
        }
        other => panic!(
            "expected Boseiju activation to either auto-pay discard or ask for it, got {other:?}"
        ),
    }

    assert_eq!(
        game.stack.len(),
        1,
        "Boseiju's channel ability should be on the stack after paying costs"
    );

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Boseiju channel ability should resolve");

    let bob_battlefield_has_forest = game.battlefield.iter().any(|&id| {
        game.object(id)
            .map(|obj| obj.name == "Forest" && game.controller_of(obj) == bob)
            .unwrap_or(false)
    });
    assert!(
        bob_battlefield_has_forest,
        "Boseiju should still let the destroyed permanent's controller search during real activation"
    );
    assert!(
        game.player(bob)
            .is_some_and(|player| player.graveyard.iter().any(|&id| {
                game.object(id)
                    .map(|obj| obj.name == "Command Tower" && obj.owner == bob)
                    .unwrap_or(false)
            })),
        "the destroyed nonbasic land should end up in Bob's graveyard"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        bob_library_before - 1,
        "the searched land should leave Bob's library after channel resolves"
    );
    assert!(
        game.object(library_basic_id).is_none(),
        "the searched land should enter as a new battlefield object"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_boseiju_channel_assigns_multiplayer_search_prompt_to_destroyed_controller() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::{basic_forest, ornithopter};
    use crate::decision::{GameProgress, LegalAction};
    use crate::ids::CardId;

    struct CaptureSearchPromptDecisionMaker {
        search_prompt_player: Option<PlayerId>,
        saw_forest_candidate: bool,
    }

    impl DecisionMaker for CaptureSearchPromptDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx
                .candidates
                .iter()
                .any(|candidate| candidate.name == "Forest")
            {
                self.search_prompt_player = Some(ctx.player);
                self.saw_forest_candidate = ctx
                    .candidates
                    .iter()
                    .any(|candidate| candidate.legal && candidate.name == "Forest");
            }

            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let boseiju = CardDefinitionBuilder::new(CardId::new(), "Boseiju, Who Endures")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}: Add {G}.\n\
             Channel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That permanent's controller may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle.\n\
             This ability costs {1} less to activate for each legendary creature you control.",
        )
        .expect("Boseiju text should parse");

    let source_id = game.create_object_from_definition(&boseiju, alice, Zone::Hand);
    let ability_index = game
        .object(source_id)
        .expect("Boseiju should exist in hand")
        .abilities
        .iter()
        .position(|ability| {
            matches!(
                ability.kind,
                AbilityKind::Activated(_) if ability.functions_in(&Zone::Hand)
            )
        })
        .expect("Boseiju should expose its channel ability from hand");
    let target_artifact_id =
        game.create_object_from_definition(&ornithopter(), bob, Zone::Battlefield);
    let bob_library_before = game.player(bob).expect("bob exists").library.len();
    game.create_object_from_definition(&basic_forest(), bob, Zone::Library);

    if let Some(player) = game.player_mut(alice) {
        player.mana_pool.add(ManaSymbol::Green, 2);
    }

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = CaptureSearchPromptDecisionMaker {
        search_prompt_player: None,
        saw_forest_candidate: false,
    };

    let activate = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
        source: source_id,
        ability_index,
    });
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &activate,
        &mut dm,
    )
    .expect("Boseiju activation should start");

    match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_)) => {}
        other => panic!("expected Boseiju to choose its target first, got {other:?}"),
    }

    let choose_target = PriorityResponse::Targets(vec![Target::Object(target_artifact_id)]);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_target,
        &mut dm,
    )
    .expect("Boseiju should accept its multiplayer target");

    let next_cost_ctx = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected Boseiju to ask for its next cost, got {other:?}"),
    };

    let discard_cost_index = next_cost_ctx
        .options
        .iter()
        .find(|opt| opt.description.to_ascii_lowercase().contains("discard"))
        .map(|opt| opt.index)
        .expect("expected Boseiju to offer its discard cost");

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::NextCostChoice(discard_cost_index),
        &mut dm,
    )
    .expect("Boseiju should let us pay discard first");

    match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {
            apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::CardCostChoice(source_id),
                &mut dm,
            )
            .expect("discarding Boseiju should finish the activation flow");
        }
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Priority(_)) => {
        }
        other => panic!(
            "expected Boseiju activation to either auto-pay discard or ask for it, got {other:?}"
        ),
    }

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Boseiju channel ability should resolve in multiplayer");

    assert_eq!(
        dm.search_prompt_player,
        Some(bob),
        "Boseiju should hand the search prompt to the destroyed permanent's controller in multiplayer"
    );
    assert!(
        dm.saw_forest_candidate,
        "the destroyed permanent's controller should be offered their legal basic land search"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        bob_library_before,
        "Bob should end up with the searched Forest removed from library after resolving the prompt"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_the_one_ring_prevents_combat_damage_until_your_next_turn() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::CardId;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let one_ring = CardDefinitionBuilder::new(CardId::new(), "The One Ring")
        .card_types(vec![CardType::Artifact])
        .parse_text("When The One Ring enters the battlefield, if you cast it, you gain protection from everything until your next turn.")
        .expect("The One Ring trigger text should parse");
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = AutoPassDecisionMaker;

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let ring_id = game.create_object_from_definition(&one_ring, alice, Zone::Stack);
    let (ring_stable_id, ring_name) = game
        .object(ring_id)
        .map(|object| (object.stable_id, object.name.to_string()))
        .expect("The One Ring spell should exist");
    game.push_to_stack(StackEntry::new(ring_id, alice).with_source_info(ring_stable_id, ring_name));

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("The One Ring should resolve");
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "The One Ring should queue its enters trigger after resolving from the stack"
    );

    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("The One Ring trigger should be put on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("The One Ring trigger should resolve");

    assert!(
        game.effect_store.prevention_effects.shields().iter().any(|shield| {
            matches!(shield.protected, crate::prevention::PreventionTarget::Player(player) if player == alice)
        }),
        "The One Ring should create a prevention shield for its controller"
    );

    game.turn.turn_number += 1;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);

    let attacker_id = create_creature(&mut game, "Ring Breaker", bob, 4, 4);
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(alice),
    });
    combat.blockers.insert(attacker_id, Vec::new());

    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(events.len(), 1, "combat damage should still be assigned");
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        20,
        "The One Ring should prevent combat damage dealt before Alice's next turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_the_stasis_coffin_activation_grants_protection_and_exiles_itself() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::compute_legal_actions;
    use crate::ids::CardId;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let coffin = CardDefinitionBuilder::new(CardId::new(), "The Stasis Coffin")
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .parse_text(
            "{2}, {T}, Exile The Stasis Coffin: You gain protection from everything until your next turn.",
        )
        .expect("The Stasis Coffin text should parse");
    let coffin_id = game.create_object_from_definition(&coffin, alice, Zone::Battlefield);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let ability_index = game
        .object(coffin_id)
        .expect("The Stasis Coffin should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("The Stasis Coffin should have an activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == coffin_id && *idx == ability_index
            )
        })
        .expect("The Stasis Coffin activation should be legal with mana available");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("activating The Stasis Coffin should succeed");

    let progress = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(cost_ctx),
        ) => {
            let exile_cost_index = cost_ctx
                .options
                .iter()
                .find(|option| option.description.to_ascii_lowercase().contains("exile"))
                .map(|option| option.index)
                .expect("expected an exile cost option for The Stasis Coffin");

            apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::NextCostChoice(exile_cost_index),
                &mut dm,
            )
            .expect("choosing the exile cost should continue The Stasis Coffin activation")
        }
        other => panic!(
            "expected The Stasis Coffin to ask for a cost choice first, got {:?}",
            other
        ),
    };

    assert!(
        matches!(
            progress,
            crate::decision::GameProgress::Continue
                | crate::decision::GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Priority(_)
                )
        ),
        "expected The Stasis Coffin activation to proceed to the priority window after choosing the exile cost, got {:?}",
        progress
    );

    let coffin_exiled = game
        .exile
        .iter()
        .filter_map(|&id| game.object(id))
        .any(|obj| obj.name == "The Stasis Coffin" && game.controller_of(obj) == alice);
    assert!(
        coffin_exiled,
        "The Stasis Coffin should be in exile after its activation cost is paid"
    );

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("The Stasis Coffin ability should resolve");

    assert!(
        !game.can_target_player(alice),
        "The Stasis Coffin should make its controller untargetable until their next turn"
    );
    assert!(
        game.effect_store.prevention_effects.shields().iter().any(|shield| {
            matches!(shield.protected, crate::prevention::PreventionTarget::Player(player) if player == alice)
        }),
        "The Stasis Coffin should create a prevention shield for its controller"
    );

    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);

    let attacker_id = create_creature(&mut game, "Stasis Breaker", bob, 4, 4);
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker_id,
        target: AttackTarget::Player(alice),
    });
    combat.blockers.insert(attacker_id, Vec::new());

    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(events.len(), 1, "combat damage should still be assigned");
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        20,
        "The Stasis Coffin should prevent combat damage before Alice's next turn"
    );

    game.next_turn();
    game.refresh_continuous_state();

    assert!(
        game.can_target_player(alice),
        "The Stasis Coffin protection should expire on Alice's next turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn heroism_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(30_267), "Heroism")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Sacrifice a white creature: For each attacking red creature, prevent all combat \
             damage that would be dealt by that creature this turn unless its controller pays {2}{R}.",
        )
        .expect("Heroism should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn colored_test_creature(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    color: crate::color::ColorSet,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .color_indicator(color)
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn activate_heroism(
    game: &mut GameState,
    controller: PlayerId,
    heroism_id: ObjectId,
    sacrifice_id: ObjectId,
    dm: &mut impl DecisionMaker,
) {
    use crate::decision::{LegalAction, compute_legal_actions};

    let ability_index = game
        .object(heroism_id)
        .expect("Heroism should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Heroism should have an activated ability");
    let activate_action = compute_legal_actions(game, controller)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == heroism_id && *idx == ability_index
            )
        })
        .expect("Heroism activation should be legal with a white creature to sacrifice");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        dm,
    )
    .expect("Heroism activation should start");

    while let crate::decision::GameProgress::NeedsDecisionCtx(decision) = progress {
        progress = match decision {
            crate::decisions::context::DecisionContext::SelectOptions(ctx) => {
                let choice = ctx
                    .options
                    .iter()
                    .find(|option| {
                        option.legal
                            && option
                                .description
                                .to_ascii_lowercase()
                                .contains("sacrifice")
                    })
                    .or_else(|| ctx.options.iter().find(|option| option.legal))
                    .map(|option| option.index)
                    .expect("Heroism should offer a legal sacrifice-cost choice");
                apply_priority_response_with_dm(
                    game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(choice),
                    dm,
                )
                .expect("Heroism sacrifice-cost choice should continue activation")
            }
            crate::decisions::context::DecisionContext::SelectObjects(_) => {
                apply_priority_response_with_dm(
                    game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::SacrificeTarget(sacrifice_id),
                    dm,
                )
                .expect("Heroism should accept the selected white creature sacrifice")
            }
            crate::decisions::context::DecisionContext::Priority(_) => break,
            other => panic!("unexpected decision while activating Heroism: {:?}", other),
        };

        if game.stack.len() == 1 {
            break;
        }
    }

    assert_eq!(
        game.stack.len(),
        1,
        "Heroism ability should be on the stack"
    );
    assert!(
        game.object(sacrifice_id)
            .map(|object| object.zone != Zone::Battlefield)
            .unwrap_or(true),
        "Heroism should sacrifice the chosen white creature as an activation cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Debug, Default)]
pub(super) struct PayUnlessDecisionMaker;

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for PayUnlessDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_heroism_prevents_unpaid_attacking_red_creature_damage_only() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let heroism_id =
        game.create_object_from_definition(&heroism_definition(), alice, Zone::Battlefield);
    let sacrifice_id = colored_test_creature(
        &mut game,
        "Heroism Cost Bearer",
        alice,
        crate::color::ColorSet::WHITE,
        1,
        1,
    );
    let red_attacker = colored_test_creature(
        &mut game,
        "Red Attacker",
        bob,
        crate::color::ColorSet::RED,
        3,
        3,
    );
    let colorless_attacker = create_creature(&mut game, "Colorless Attacker", bob, 2, 2);

    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareBlockers);

    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: red_attacker,
        target: AttackTarget::Player(alice),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: colorless_attacker,
        target: AttackTarget::Player(alice),
    });
    combat.blockers.insert(red_attacker, Vec::new());
    combat.blockers.insert(colorless_attacker, Vec::new());
    game.combat = Some(combat.clone());

    let mut dm = AutoPassDecisionMaker;
    activate_heroism(&mut game, alice, heroism_id, sacrifice_id, &mut dm);
    resolve_stack_entry_with(&mut game, &mut dm).expect("Heroism ability should resolve");

    game.turn.step = Some(crate::game_state::Step::CombatDamage);
    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(
        events.len(),
        2,
        "both attackers should assign combat damage"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        18,
        "Heroism should prevent unpaid red attacker damage while leaving nonred attacker damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_heroism_controller_payment_allows_red_attacker_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let heroism_id =
        game.create_object_from_definition(&heroism_definition(), alice, Zone::Battlefield);
    let sacrifice_id = colored_test_creature(
        &mut game,
        "Heroism Cost Bearer",
        alice,
        crate::color::ColorSet::WHITE,
        1,
        1,
    );
    let red_attacker = colored_test_creature(
        &mut game,
        "Paying Red Attacker",
        bob,
        crate::color::ColorSet::RED,
        3,
        3,
    );
    game.player_mut(bob)
        .expect("Bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    game.player_mut(bob)
        .expect("Bob exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareBlockers);

    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: red_attacker,
        target: AttackTarget::Player(alice),
    });
    combat.blockers.insert(red_attacker, Vec::new());
    game.combat = Some(combat.clone());

    let mut dm = PayUnlessDecisionMaker;
    activate_heroism(&mut game, alice, heroism_id, sacrifice_id, &mut dm);
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Heroism ability should resolve after attacker controller pays");

    game.turn.step = Some(crate::game_state::Step::CombatDamage);
    let events = execute_combat_damage_step(&mut game, &combat, false);
    assert_eq!(
        events.len(),
        1,
        "the red attacker should assign combat damage"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        17,
        "paying {{2}}{{R}} should stop Heroism from preventing that red attacker's damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_elsewhere_flask_activation_changes_land_type_until_cleanup() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let elsewhere_flask = CardDefinitionBuilder::new(CardId::new(), "Elsewhere Flask")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .parse_text(
            "When this artifact enters, draw a card.\nSacrifice this artifact: Choose a basic land type. Each land you control becomes that type until end of turn.",
        )
        .expect("Elsewhere Flask should parse");
    let flask_id = game.create_object_from_definition(&elsewhere_flask, alice, Zone::Battlefield);

    let forest_id = game.create_object_from_definition(
        &crate::cards::definitions::basic_forest(),
        alice,
        Zone::Battlefield,
    );
    let island_id = game.create_object_from_definition(
        &crate::cards::definitions::basic_island(),
        alice,
        Zone::Battlefield,
    );

    let ability_index = game
        .object(flask_id)
        .expect("Elsewhere Flask should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Elsewhere Flask should have an activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == flask_id && *idx == ability_index
            )
        })
        .expect("Elsewhere Flask activation should be legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("activating Elsewhere Flask should succeed");

    let progress = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(cost_ctx),
        ) => apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::NextCostChoice(cost_ctx.options[0].index),
            &mut dm,
        )
        .expect("choosing Elsewhere Flask cost option should continue activation"),
        other => other,
    };
    assert!(
        matches!(
            progress,
            crate::decision::GameProgress::Continue
                | crate::decision::GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Priority(_)
                )
        ),
        "Elsewhere Flask activation should proceed after any cost choices, got {progress:?}"
    );

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Elsewhere Flask ability should resolve");

    assert!(
        !game.battlefield.contains(&flask_id),
        "Elsewhere Flask should be sacrificed to pay the activation cost"
    );

    let forest_subtypes = game.calculated_subtypes(forest_id);
    assert!(
        !forest_subtypes.contains(&Subtype::Forest),
        "Elsewhere Flask should change at least one controlled land's basic subtype until end of turn; got {forest_subtypes:?}"
    );

    execute_cleanup_step(&mut game);

    let forest_after_cleanup = game.calculated_subtypes(forest_id);
    let island_after_cleanup = game.calculated_subtypes(island_id);
    assert!(
        forest_after_cleanup.contains(&Subtype::Forest),
        "Forest should regain its original subtype after until-end-of-turn expires"
    );
    assert!(
        island_after_cleanup.contains(&Subtype::Island),
        "Island should regain its original subtype after until-end-of-turn expires"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_cephalid_inkshrouder_grants_shroud_and_unblockable_after_discard() {
    use crate::PriorityResponse;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::compute_legal_actions;
    use crate::game_loop::{
        PriorityLoopState, apply_priority_response_with_dm,
        resolve_stack_entry_with_dm_and_triggers,
    };
    use crate::ids::CardId;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let inkshrouder = CardDefinitionBuilder::new(CardId::new(), "Cephalid Inkshrouder")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Octopus])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text("Discard a card: This creature gains shroud until end of turn and can't be blocked this turn.")
        .expect("Cephalid Inkshrouder text should parse");
    let inkshrouder_id = game.create_object_from_definition(&inkshrouder, alice, Zone::Battlefield);
    game.remove_summoning_sickness(inkshrouder_id);

    let discard_fodder = CardBuilder::new(CardId::new(), "Discard Fodder")
        .card_types(vec![CardType::Artifact])
        .build();
    let discard_id = game.create_object_from_card(&discard_fodder, alice, Zone::Hand);

    let blocker_id = create_creature(&mut game, "Training Blocker", bob, 2, 2);

    let ability_index = game
        .object(inkshrouder_id)
        .expect("Cephalid Inkshrouder should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Cephalid Inkshrouder should have an activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == inkshrouder_id && *idx == ability_index
            )
        })
        .expect("Cephalid Inkshrouder activation should be legal");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Cephalid Inkshrouder activation should start");

    let progress = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::CardCostChoice(discard_id),
            &mut dm,
        )
        .expect("discarding a card should finish paying Cephalid Inkshrouder's cost"),
        other => panic!(
            "expected Cephalid Inkshrouder to prompt for a discard, got {:?}",
            other
        ),
    };

    assert!(
        matches!(
            progress,
            crate::decision::GameProgress::Continue
                | crate::decision::GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Priority(_)
                )
        ),
        "expected Cephalid Inkshrouder activation to resolve or return priority, got {:?}",
        progress
    );

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Cephalid Inkshrouder ability should resolve");

    assert!(
        game.is_untargetable(inkshrouder_id),
        "Cephalid Inkshrouder should gain shroud after the discard cost resolves"
    );
    assert!(
        !game.can_be_blocked(inkshrouder_id),
        "Cephalid Inkshrouder should be unblockable after the discard cost resolves"
    );
    assert!(
        !crate::rules::combat::can_block(
            game.object(inkshrouder_id).expect("inkshrouder exists"),
            game.object(blocker_id).expect("blocker exists"),
            &game,
        ),
        "the blocker should not be able to block Cephalid Inkshrouder once the ability resolves"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .any(|&id| {
                game.object(id)
                    .is_some_and(|object| object.name == "Discard Fodder" && object.owner == alice)
            }),
        "the discarded card should end up in Alice's graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sleep_with_the_fishes_creates_unblockable_fish_token() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::effects::CreateTokenEffect;
    use crate::ids::CardId;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let sleep = CardDefinitionBuilder::new(CardId::new(), "Sleep with the Fishes")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Aura])
        .parse_text(
            "Enchant creature\nWhen this Aura enters, tap enchanted creature and you create a 1/1 blue Fish creature token with \"This token can't be blocked.\"\nEnchanted creature doesn't untap during its controller's untap step.",
        )
        .expect("Sleep with the Fishes should parse");

    fn find_create_token(effect: &crate::effect::Effect) -> Option<CreateTokenEffect> {
        if let Some(create) = effect.downcast_ref::<CreateTokenEffect>() {
            return Some(create.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_create_token(child);
            }
        });
        found
    }

    let create_effect = sleep
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .all_effects()
                .into_iter()
                .find_map(find_create_token),
            _ => None,
        })
        .expect("Sleep with the Fishes trigger should create a token");

    let fish_id =
        game.create_object_from_definition(&create_effect.token, alice, Zone::Battlefield);
    let blocker_id = create_creature(&mut game, "Would-be Blocker", bob, 2, 2);
    game.refresh_continuous_state();

    assert!(
        !game.can_be_blocked(fish_id),
        "Fish token from Sleep with the Fishes should be unblockable"
    );
    assert!(
        !crate::rules::combat::can_block(
            game.object(fish_id).expect("fish token should exist"),
            game.object(blocker_id).expect("blocker should exist"),
            &game,
        ),
        "opponent creature should not be able to block Sleep with the Fishes Fish token"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gilded_light_shroud_blocks_all_player_targeting_until_end_of_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let gilded_light = CardDefinitionBuilder::new(CardId::from_raw(46_396), "Gilded Light")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "You gain shroud until end of turn. (You can't be the target of spells or abilities.)\nCycling {2} ({2}, Discard this card: Draw a card.)",
        )
        .expect("Gilded Light should parse");
    let spell_id = game.create_object_from_definition(&gilded_light, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, alice).with_source_info(
            game.object(spell_id)
                .expect("Gilded Light spell should exist")
                .stable_id,
            "Gilded Light".to_string(),
        ),
    );

    let alice_source = create_creature(&mut game, "Friendly Source", alice, 2, 2);
    let bob_source = create_creature(&mut game, "Opposing Source", bob, 2, 2);
    assert!(game.can_target_player_from_source(alice, alice_source));
    assert!(game.can_target_player_from_source(alice, bob_source));

    resolve_stack_entry(&mut game).expect("Gilded Light should resolve");

    assert!(
        !game.can_target_player(alice),
        "Gilded Light should make its controller untargetable"
    );
    assert!(
        !game.can_target_player_from_source(alice, alice_source),
        "shroud should stop the controller's own sources from targeting them"
    );
    assert!(
        !game.can_target_player_from_source(alice, bob_source),
        "shroud should stop opposing sources from targeting the controller"
    );
    assert!(
        game.can_target_player_from_source(bob, bob_source),
        "Gilded Light should not grant shroud to other players"
    );

    execute_cleanup_step(&mut game);
    game.refresh_continuous_state();

    assert!(
        game.can_target_player_from_source(alice, alice_source)
            && game.can_target_player_from_source(alice, bob_source),
        "Gilded Light's shroud should expire at end of turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn absolute_virtue_blocks_opponent_controlled_sources_from_targeting_you() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::CardId;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let absolute_virtue = CardDefinitionBuilder::new(CardId::new(), "Absolute Virtue")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text("You have protection from each of your opponents.")
        .expect("Absolute Virtue should parse");
    game.create_object_from_definition(&absolute_virtue, alice, Zone::Battlefield);

    let bob_source = create_creature(&mut game, "Opposing Source", bob, 2, 2);
    let alice_source = create_creature(&mut game, "Friendly Source", alice, 2, 2);

    game.refresh_continuous_state();

    assert!(
        !game.can_target_player_from_source(alice, bob_source),
        "opponent-controlled source should not be able to target Absolute Virtue's controller"
    );
    assert!(
        game.can_target_player_from_source(alice, alice_source),
        "controller's own source should still be able to target them"
    );
    assert!(
        game.can_target_player_from_source(bob, bob_source),
        "Absolute Virtue should not prevent targeting opponents"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn giant_solifuge_keywords_apply_to_targeting_haste_and_trample() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::static_abilities::StaticAbilityId;
    use crate::targeting::{TargetingInvalidReason, TargetingResult, can_target_object};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let solifuge = CardDefinitionBuilder::new(CardId::new(), "Giant Solifuge")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red, ManaSymbol::Green],
            vec![ManaSymbol::Red, ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Insect])
        .power_toughness(PowerToughness::fixed(4, 1))
        .parse_text("Trample; haste; shroud")
        .expect("Giant Solifuge text should parse");
    let solifuge_id = game.create_object_from_definition(&solifuge, alice, Zone::Battlefield);

    assert!(
        game.current_has_static_ability_id(solifuge_id, StaticAbilityId::Trample)
            && game.current_has_static_ability_id(solifuge_id, StaticAbilityId::Haste)
            && game.current_has_static_ability_id(solifuge_id, StaticAbilityId::Shroud),
        "Giant Solifuge should expose trample, haste, and shroud as static abilities"
    );

    game.set_summoning_sick(solifuge_id);
    assert!(
        crate::rules::combat::can_attack(
            game.object(solifuge_id).expect("Giant Solifuge exists"),
            &game,
        ),
        "haste should allow Giant Solifuge to attack despite summoning sickness"
    );

    let source = CardBuilder::new(CardId::new(), "Targeting Probe")
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source, bob, Zone::Stack);
    assert!(
        matches!(
            can_target_object(&game, solifuge_id, source_id, bob),
            TargetingResult::Invalid(TargetingInvalidReason::HasShroud)
        ),
        "shroud should make Giant Solifuge an illegal target"
    );

    let blocker_id = create_creature(&mut game, "Training Blocker", bob, 2, 2);
    let excess = crate::rules::damage::calculate_trample_excess(
        game.object(solifuge_id).expect("Giant Solifuge exists"),
        &[game.object(blocker_id).expect("blocker exists")],
        4,
        &game,
    );
    assert_eq!(
        excess, 2,
        "trample should leave two excess damage over a 2-toughness blocker"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn gaeas_revenge_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(193_669), "Gaea's Revenge")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental])
        .power_toughness(PowerToughness::fixed(8, 5))
        .parse_text(
            "This spell can't be countered.\n\
             Haste\n\
             This creature can't be the target of nongreen spells or abilities from nongreen sources.",
        )
        .expect("Gaea's Revenge should parse strictly for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn bartel_runeaxe_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(1_648), "Bartel Runeaxe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Green],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Giant, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(6, 5))
        .parse_text("Vigilance\nBartel Runeaxe can't be the target of Aura spells.")
        .expect("Bartel Runeaxe should parse strictly for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn colored_instant_definition(
    name: &str,
    colors: crate::color::ColorSet,
) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .color_indicator(colors)
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn colored_creature_definition(
    name: &str,
    colors: crate::color::ColorSet,
) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .color_indicator(colors)
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gaeas_revenge_strict_parser_and_compiled_text_regression() {
    let def = gaeas_revenge_definition();
    let rendered = crate::runtime_display::unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("This spell can't be countered."),
        "Gaea's Revenge should render its uncounterable spell restriction, got {rendered}"
    );
    assert!(
        rendered.contains("Haste"),
        "Gaea's Revenge should render haste, got {rendered}"
    );
    assert!(
        rendered.contains(
            "This creature can't be the target of nongreen spells or abilities from nongreen sources."
        ),
        "Gaea's Revenge should render its nongreen source targeting restriction, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gaeas_revenge_static_abilities_apply_to_countering_haste_and_targeting() {
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::targeting::{
        TargetingInvalidReason, TargetingResult, can_target_object, compute_legal_targets,
    };

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let revenge = gaeas_revenge_definition();

    let revenge_spell = game.create_object_from_definition(&revenge, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(revenge_spell, alice));
    game.update_cant_effects();
    assert!(
        !game.can_be_countered(revenge_spell),
        "Gaea's Revenge should be uncounterable while it is a spell on the stack"
    );

    let revenge_id = game.create_object_from_definition(&revenge, alice, Zone::Battlefield);
    game.set_summoning_sick(revenge_id);
    game.refresh_continuous_state();
    assert!(
        crate::rules::combat::can_attack(
            game.object(revenge_id).expect("Gaea's Revenge exists"),
            &game,
        ),
        "haste should let Gaea's Revenge attack despite summoning sickness"
    );

    let red_spell =
        colored_instant_definition("Nongreen Targeting Spell", crate::color::ColorSet::RED);
    let red_spell_id = game.create_object_from_definition(&red_spell, bob, Zone::Stack);
    assert!(
        matches!(
            can_target_object(&game, revenge_id, red_spell_id, bob),
            TargetingResult::Invalid(TargetingInvalidReason::CantBeTargeted)
        ),
        "an opponent's nongreen spell should not be able to target Gaea's Revenge"
    );

    let friendly_red_source = colored_creature_definition(
        "Friendly Nongreen Ability Source",
        crate::color::ColorSet::RED,
    );
    let friendly_red_source_id =
        game.create_object_from_definition(&friendly_red_source, alice, Zone::Battlefield);
    assert!(
        matches!(
            can_target_object(&game, revenge_id, friendly_red_source_id, alice),
            TargetingResult::Invalid(TargetingInvalidReason::CantBeTargeted)
        ),
        "Gaea's Revenge should also reject its controller's nongreen ability sources"
    );

    let green_spell =
        colored_instant_definition("Green Targeting Spell", crate::color::ColorSet::GREEN);
    let green_spell_id = game.create_object_from_definition(&green_spell, bob, Zone::Stack);
    assert!(
        can_target_object(&game, revenge_id, green_spell_id, bob).is_legal(),
        "a green spell should be able to target Gaea's Revenge"
    );

    let legal_targets_from_red = compute_legal_targets(
        &game,
        &ChooseSpec::Target(Box::new(ChooseSpec::Object(ObjectFilter::creature()))),
        bob,
        Some(red_spell_id),
    );
    assert!(
        !legal_targets_from_red.contains(&Target::Object(revenge_id)),
        "Gaea's Revenge should not appear in legal target lists for nongreen sources"
    );

    let legal_targets_from_green = compute_legal_targets(
        &game,
        &ChooseSpec::Target(Box::new(ChooseSpec::Object(ObjectFilter::creature()))),
        bob,
        Some(green_spell_id),
    );
    assert!(
        legal_targets_from_green.contains(&Target::Object(revenge_id)),
        "Gaea's Revenge should appear in legal target lists for green sources"
    );
}
