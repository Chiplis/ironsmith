//! Mox Diamond card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::ManaCost;
use crate::types::CardType;

/// Creates the Mox Diamond card definition.
///
/// Mox Diamond {0}
/// Artifact
/// If Mox Diamond would enter the battlefield, you may discard a land card instead.
/// If you do, put Mox Diamond onto the battlefield. If you don't, put it into its
/// owner's graveyard.
/// {T}: Add one mana of any color.
pub fn mox_diamond() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mox Diamond")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Artifact])
        .parse_text("If Mox Diamond would enter the battlefield, you may discard a land card instead. If you do, put Mox Diamond onto the battlefield. If you don't, put it into its owner's graveyard.\n{T}: Add one mana of any color.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::effects::EffectExecutor;
    use crate::effects::mana::AddManaOfAnyColorEffect;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_land_in_hand(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), "Mountain")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_mox_diamond_basic_properties() {
        let def = mox_diamond();

        // Check name
        assert_eq!(def.name(), "Mox Diamond");

        // Check it's an artifact
        assert!(def.card.is_artifact());
        assert!(def.card.card_types.contains(&CardType::Artifact));

        // Check mana cost is {0}
        assert_eq!(def.card.mana_value(), 0);

        // Check it's colorless
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_mox_diamond_has_two_abilities() {
        let def = mox_diamond();

        // Should have 2 abilities: static (ETB replacement) and mana
        assert_eq!(def.abilities.len(), 2);
    }

    #[test]
    fn test_mox_diamond_has_static_ability() {
        use crate::static_abilities::StaticAbilityId;
        let def = mox_diamond();

        // First ability should be a static ability for the ETB replacement
        assert!(matches!(def.abilities[0].kind, AbilityKind::Static(_)));

        if let AbilityKind::Static(static_ability) = &def.abilities[0].kind {
            assert!(
                static_ability.id() == StaticAbilityId::DiscardOrRedirectReplacement,
                "Should be a DiscardOrRedirectReplacement ability"
            );
        }
    }

    #[test]
    fn test_mox_diamond_has_mana_ability() {
        let def = mox_diamond();

        // Second ability should be a mana ability
        assert!(def.abilities[1].is_mana_ability());

        if let AbilityKind::Mana(mana_ability) = &def.abilities[1].kind {
            // Should have tap cost
            assert!(mana_ability.has_tap_cost());
            // Should have effects (add mana of any color)
            assert!(mana_ability.effects.is_some());
        }
    }

    // =========================================================================
    // Mana Production Tests
    // =========================================================================

    #[test]
    fn test_mox_diamond_mana_ability_produces_any_color() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Mox Diamond on battlefield (assuming ETB replacement was satisfied)
        let def = mox_diamond();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Execute the mana ability effect
        let mut ctx = ExecutionContext::new_default(mox_id, alice);
        let effect = AddManaOfAnyColorEffect::you(1);
        let result = EffectExecutor::execute(&effect, &mut game, &mut ctx).unwrap();

        // Should produce 1 mana (defaults to green without decision maker)
        assert_eq!(result.result, crate::effect::EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn test_mox_diamond_can_tap_for_mana_immediately() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Mox Diamond on battlefield
        let def = mox_diamond();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(mox_id).unwrap();

        // Artifacts (non-creatures) are not affected by summoning sickness
        assert!(!obj.is_creature(), "Mox Diamond is not a creature");

        // Verify it has the mana ability
        let mana_ability = obj.abilities.iter().find(|a| a.is_mana_ability());
        assert!(mana_ability.is_some(), "Should have a mana ability");
    }

    // =========================================================================
    // ETB Replacement Effect Tests
    // (These test the expected behavior - actual implementation in game loop)
    // =========================================================================

    #[test]
    fn test_mox_diamond_has_etb_replacement_marker() {
        let def = mox_diamond();

        // The card should have a static ability with a self-replacement effect
        use crate::static_abilities::StaticAbilityId;
        let has_discard_replacement = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::DiscardOrRedirectReplacement
            } else {
                false
            }
        });
        assert!(
            has_discard_replacement,
            "Should have DiscardOrRedirectReplacement static ability"
        );
    }

    #[test]
    fn test_mox_diamond_oracle_text() {
        let def = mox_diamond();

        assert!(def.card.oracle_text.contains("discard a land card"));
        assert!(def.card.oracle_text.contains("graveyard"));
        assert!(def.card.oracle_text.contains("any color"));
    }

    // =========================================================================
    // Full ETB Replacement Effect Integration Tests
    // =========================================================================

    use crate::decision::{DecisionMaker, LegalAction};
    use crate::decisions::context::{PriorityContext, SelectObjectsContext};
    use crate::ids::ObjectId;

    /// A decision maker that tracks decisions and can be configured for Mox Diamond tests.
    struct MoxDiamondTestDecisionMaker {
        /// If true, discard the first available land for Mox Diamond.
        /// If false, decline (which means Mox Diamond goes to graveyard).
        discard_land: bool,
        /// Record of decisions made.
        decisions_made: Vec<String>,
    }

    impl MoxDiamondTestDecisionMaker {
        fn new(discard_land: bool) -> Self {
            Self {
                discard_land,
                decisions_made: Vec::new(),
            }
        }
    }

    impl DecisionMaker for MoxDiamondTestDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.decisions_made
                .push(format!("ChooseCardToDiscard: {}", ctx.description));
            if self.discard_land && !ctx.candidates.is_empty() {
                // Discard the first legal land
                vec![ctx.candidates[0].id]
            } else {
                // Decline to discard
                vec![]
            }
        }

        fn decide_priority(&mut self, _game: &GameState, _ctx: &PriorityContext) -> LegalAction {
            LegalAction::PassPriority
        }
    }

    #[test]
    fn test_mox_diamond_enters_when_land_discarded() {
        use crate::game_loop::resolve_stack_entry_with;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a land in Alice's hand
        let _land_id = create_land_in_hand(&mut game, alice);

        // Create Mox Diamond on the stack (simulating it being cast)
        let def = mox_diamond();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Put it on the stack properly
        game.push_to_stack(crate::game_state::StackEntry {
            object_id: mox_id,
            controller: alice,
            targets: vec![],
            x_value: None,
            is_ability: false,
            ability_effects: None,
            optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
            defending_player: None,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_name: None,
            intervening_if: None,
            triggering_event: None,
            chosen_modes: None,
        });

        // Resolve with a decision maker that discards a land
        let mut dm = MoxDiamondTestDecisionMaker::new(true);
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert!(result.is_ok(), "Resolution should succeed");

        // Verify Mox Diamond is on the battlefield (new ID after zone change)
        let mox_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(
            mox_on_battlefield,
            "Mox Diamond should be on the battlefield"
        );

        // Verify the land was discarded to graveyard
        let land_in_graveyard = game.player(alice).unwrap().graveyard.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.has_card_type(CardType::Land))
                .unwrap_or(false)
        });
        assert!(land_in_graveyard, "Land should be in graveyard");

        // Verify the decision was made
        assert!(
            !dm.decisions_made.is_empty(),
            "A decision should have been made"
        );
    }

    #[test]
    fn test_mox_diamond_goes_to_graveyard_when_no_land_discarded() {
        use crate::game_loop::resolve_stack_entry_with;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a land in Alice's hand
        let land_id = create_land_in_hand(&mut game, alice);

        // Create Mox Diamond on the stack
        let def = mox_diamond();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Put it on the stack
        game.push_to_stack(crate::game_state::StackEntry {
            object_id: mox_id,
            controller: alice,
            targets: vec![],
            x_value: None,
            is_ability: false,
            ability_effects: None,
            optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
            defending_player: None,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_name: None,
            intervening_if: None,
            triggering_event: None,
            chosen_modes: None,
        });

        // Resolve with a decision maker that declines to discard
        let mut dm = MoxDiamondTestDecisionMaker::new(false);
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert!(result.is_ok(), "Resolution should succeed");

        // Verify Mox Diamond is in the graveyard
        let mox_in_graveyard = game.player(alice).unwrap().graveyard.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(mox_in_graveyard, "Mox Diamond should be in graveyard");

        // Verify Mox Diamond is NOT on the battlefield
        let mox_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(
            !mox_on_battlefield,
            "Mox Diamond should NOT be on the battlefield"
        );

        // Verify the land is still in hand
        assert!(
            game.player(alice).unwrap().hand.contains(&land_id),
            "Land should still be in hand"
        );
    }

    #[test]
    fn test_mox_diamond_goes_to_graveyard_when_no_lands_in_hand() {
        use crate::game_loop::resolve_stack_entry_with;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Don't create any lands in hand - Alice's hand is empty

        // Create Mox Diamond on the stack
        let def = mox_diamond();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Put it on the stack
        game.push_to_stack(crate::game_state::StackEntry {
            object_id: mox_id,
            controller: alice,
            targets: vec![],
            x_value: None,
            is_ability: false,
            ability_effects: None,
            optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
            defending_player: None,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_name: None,
            intervening_if: None,
            triggering_event: None,
            chosen_modes: None,
        });

        // Resolve with a decision maker that would discard a land (but there are none)
        let mut dm = MoxDiamondTestDecisionMaker::new(true);
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert!(result.is_ok(), "Resolution should succeed");

        // Verify Mox Diamond is in the graveyard (because no lands to discard)
        let mox_in_graveyard = game.player(alice).unwrap().graveyard.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(
            mox_in_graveyard,
            "Mox Diamond should be in graveyard when no lands available"
        );

        // No decision should have been made (since there were no legal cards)
        assert!(
            dm.decisions_made.is_empty(),
            "No decision should be made when no lands available"
        );
    }

    #[test]
    fn test_mox_diamond_can_tap_after_entering_with_land_discard() {
        use crate::game_loop::resolve_stack_entry_with;

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a land in Alice's hand
        let _land_id = create_land_in_hand(&mut game, alice);

        // Create Mox Diamond on the stack
        let def = mox_diamond();
        let mox_id = game.create_object_from_definition(&def, alice, Zone::Stack);

        // Put it on the stack
        game.push_to_stack(crate::game_state::StackEntry {
            object_id: mox_id,
            controller: alice,
            targets: vec![],
            x_value: None,
            is_ability: false,
            ability_effects: None,
            optional_costs_paid: crate::cost::OptionalCostsPaid::default(),
            defending_player: None,
            casting_method: crate::alternative_cast::CastingMethod::Normal,
            saga_final_chapter_source: None,
            source_stable_id: None,
            source_name: None,
            intervening_if: None,
            triggering_event: None,
            chosen_modes: None,
        });

        // Resolve with a decision maker that discards a land
        let mut dm = MoxDiamondTestDecisionMaker::new(true);
        let _ = resolve_stack_entry_with(&mut game, &mut dm);

        // Find the Mox Diamond on the battlefield
        let mox_on_bf = game
            .battlefield
            .iter()
            .find(|&&id| {
                game.object(id)
                    .map(|obj| obj.name == "Mox Diamond")
                    .unwrap_or(false)
            })
            .copied();

        assert!(mox_on_bf.is_some(), "Mox Diamond should be on battlefield");
        let mox_id = mox_on_bf.unwrap();

        // Verify it's not tapped
        assert!(!game.is_tapped(mox_id), "Mox Diamond should start untapped");

        // Activate the mana ability
        let mut ctx = ExecutionContext::new_default(mox_id, alice);
        let effect = AddManaOfAnyColorEffect::you(1);
        let result = EffectExecutor::execute(&effect, &mut game, &mut ctx);
        assert!(result.is_ok(), "Mana ability should succeed");

        // Verify mana was added
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
    }

    // =========================================================================
    // Mox Comparison Tests
    // =========================================================================

    #[test]
    fn test_mox_diamond_is_zero_cost() {
        let def = mox_diamond();
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_mox_diamond_is_permanent() {
        let def = mox_diamond();
        assert!(def.is_permanent());
    }

    /// Test that the ETB replacement prompt works through the priority loop
    #[test]
    fn test_mox_diamond_via_priority_loop() {
        use crate::game_loop::run_priority_loop_with;
        use crate::triggers::TriggerQueue;

        struct TestDM {
            decisions: Vec<String>,
        }

        impl DecisionMaker for TestDM {
            fn decide_priority(&mut self, game: &GameState, ctx: &PriorityContext) -> LegalAction {
                // Find Mox Diamond cast action
                for (i, action) in ctx.legal_actions.iter().enumerate() {
                    if let LegalAction::CastSpell { spell_id, .. } = action {
                        if let Some(obj) = game.object(*spell_id) {
                            if obj.name == "Mox Diamond" {
                                self.decisions
                                    .push(format!("Casting Mox Diamond (action {})", i));
                                return action.clone();
                            }
                        }
                    }
                }
                // Otherwise pass priority
                LegalAction::PassPriority
            }

            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &SelectObjectsContext,
            ) -> Vec<ObjectId> {
                self.decisions.push(format!(
                    "ChooseCardToDiscard: {}, cards: {:?}",
                    ctx.description,
                    ctx.candidates.len()
                ));
                if !ctx.candidates.is_empty() {
                    vec![ctx.candidates[0].id]
                } else {
                    vec![]
                }
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a land in Alice's hand
        let _land_id = create_land_in_hand(&mut game, alice);

        // Create Mox Diamond in Alice's hand
        let mox_def = mox_diamond();
        let _mox_id = game.create_object_from_definition(&mox_def, alice, Zone::Hand);

        // Set up the turn so Alice has priority in main phase
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = crate::Phase::FirstMain;
        game.turn.step = None;

        println!("=== Initial State ===");
        println!("Active player: {:?}", game.turn.active_player);
        println!("Priority player: {:?}", game.turn.priority_player);
        println!("Phase: {:?}", game.turn.phase);
        println!(
            "Hand size: {}",
            game.player(alice).map(|p| p.hand.len()).unwrap_or(0)
        );

        let mut trigger_queue = TriggerQueue::default();
        let mut dm = TestDM {
            decisions: Vec::new(),
        };

        // Run the priority loop - this should cast Mox Diamond and prompt for discard
        let result = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm);

        // Print decisions for debugging
        println!("=== Results ===");
        println!("Loop result: {:?}", result);
        println!("Decisions made: {:?}", dm.decisions);

        // Check that ChooseCardToDiscard was one of the decisions
        let had_discard_prompt = dm
            .decisions
            .iter()
            .any(|d| d.contains("ChooseCardToDiscard"));
        assert!(
            had_discard_prompt,
            "Should have been prompted to choose a card to discard. Decisions: {:?}",
            dm.decisions
        );

        // Check that Mox Diamond ended up on the battlefield
        let mox_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(
            mox_on_battlefield,
            "Mox Diamond should be on the battlefield after discarding a land"
        );
    }
}
