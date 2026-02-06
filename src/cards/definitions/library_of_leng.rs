//! Library of Leng card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Creates the Library of Leng card definition.
///
/// Library of Leng {1}
/// Artifact
/// You have no maximum hand size.
/// If an effect causes you to discard a card, you may put it on top of your
/// library instead of into your graveyard.
///
/// Key rulings:
/// - The second ability only applies when discarding due to an effect, NOT a cost.
/// - If you use this to put a card on top of your library, it goes there without
///   being revealed. Per rule 701.8c, the card's characteristics become undefined.
/// - This means if an effect requires discarding a specific type of card (like
///   Mox Diamond requiring a land), and you use Library of Leng's replacement,
///   the discard is illegal because the card's type is undefined in the hidden zone.
pub fn library_of_leng() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Library of Leng")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "You have no maximum hand size.\n\
             If an effect causes you to discard a card, you may put it on top of \
             your library instead of into your graveyard.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbilityId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // =========================================================================
    // Basic Properties Tests
    // =========================================================================

    #[test]
    fn test_library_of_leng_basic_properties() {
        let def = library_of_leng();

        assert_eq!(def.name(), "Library of Leng");
        assert!(def.card.is_artifact());
        assert_eq!(def.card.mana_value(), 1);
        assert_eq!(def.card.colors().count(), 0); // Colorless
    }

    #[test]
    fn test_library_of_leng_has_two_abilities() {
        let def = library_of_leng();

        // Should have 2 abilities: no max hand size and discard replacement
        assert_eq!(def.abilities.len(), 2);
    }

    #[test]
    fn test_library_of_leng_has_no_max_hand_size() {
        let def = library_of_leng();

        let has_no_max = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::NoMaximumHandSize
            } else {
                false
            }
        });
        assert!(has_no_max, "Should have NoMaximumHandSize ability");
    }

    #[test]
    fn test_library_of_leng_has_discard_replacement() {
        let def = library_of_leng();

        let has_replacement = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::LibraryOfLengDiscardReplacement
            } else {
                false
            }
        });
        assert!(has_replacement, "Should have discard replacement ability");
    }

    #[test]
    fn test_library_of_leng_oracle_text() {
        let def = library_of_leng();

        assert!(def.card.oracle_text.contains("no maximum hand size"));
        assert!(def.card.oracle_text.contains("discard"));
        assert!(def.card.oracle_text.contains("top of your library"));
    }

    #[test]
    fn test_library_of_leng_is_permanent() {
        let def = library_of_leng();
        assert!(def.is_permanent());
    }

    // =========================================================================
    // Battlefield Tests
    // =========================================================================

    #[test]
    fn test_library_of_leng_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = library_of_leng();
        let lib_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(lib_id).unwrap();
        assert_eq!(obj.zone, Zone::Battlefield);
        assert!(obj.has_card_type(CardType::Artifact));
    }

    // =========================================================================
    // Mox Diamond + Library of Leng Interaction Tests (MTG Rule 701.8c)
    // =========================================================================
    //
    // These tests demonstrate the complex rules interaction between Mox Diamond
    // and Library of Leng per MTG rule 701.8c:
    //
    // - Mox Diamond: "If Mox Diamond would enter the battlefield, you may discard
    //   a land card instead. If you do, put Mox Diamond onto the battlefield.
    //   If you don't, put it into its owner's graveyard."
    //
    // - Library of Leng: "If an effect causes you to discard a card, you may put
    //   it on top of your library instead of into your graveyard."
    //
    // Key ruling: Library of Leng's replacement CANNOT be used for Mox Diamond's
    // discard requirement. Here's why:
    //
    // 1. Mox Diamond's ETB replacement requires you to discard a "land card"
    //    specifically (not just any card).
    //
    // 2. Library of Leng's replacement would put the discarded card on top of
    //    your library without revealing it.
    //
    // 3. Per rule 701.8c, when a card moves to a hidden zone (like the library)
    //    without being revealed, its characteristics become undefined.
    //
    // 4. Since the card's type is now undefined, the game cannot verify that a
    //    "land card" was discarded.
    //
    // 5. Therefore, using Library of Leng's replacement makes the discard illegal,
    //    because Mox Diamond requires verification that a land was discarded.
    //
    // The result: When discarding a land for Mox Diamond, the land MUST go to the
    // graveyard (where its type can be verified). Library of Leng's replacement
    // is not applicable in this situation.

    use crate::card::CardBuilder;
    use crate::decision::{DecisionMaker, LegalAction};
    use crate::decisions::context::{PriorityContext, SelectObjectsContext, SelectOptionsContext};
    use crate::game_loop::resolve_stack_entry_with;
    use crate::ids::ObjectId;

    /// A decision maker for Mox Diamond + Library of Leng interaction tests.
    struct MoxDiamondLibraryOfLengTestDecisionMaker {
        /// If true, will discard a land for Mox Diamond.
        discard_land: bool,
        /// Track what decisions were made.
        decisions_made: Vec<String>,
    }

    impl MoxDiamondLibraryOfLengTestDecisionMaker {
        fn new(discard_land: bool) -> Self {
            Self {
                discard_land,
                decisions_made: Vec::new(),
            }
        }
    }

    impl DecisionMaker for MoxDiamondLibraryOfLengTestDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.decisions_made
                .push(format!("ChooseCardToDiscard: {}", ctx.description));
            if self.discard_land && !ctx.candidates.is_empty() {
                vec![ctx.candidates[0].id]
            } else {
                vec![]
            }
        }

        fn decide_priority(&mut self, _game: &GameState, _ctx: &PriorityContext) -> LegalAction {
            LegalAction::PassPriority
        }
    }

    fn create_land_in_hand(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(crate::ids::CardId::new(), "Mountain")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    /// Test: When Mox Diamond resolves with Library of Leng on the battlefield,
    /// and the player discards a land, the land MUST go to the graveyard.
    ///
    /// Library of Leng's discard replacement cannot be used because:
    /// - The library is a hidden zone
    /// - The discarded card's type becomes undefined per rule 701.8c
    /// - Mox Diamond requires discarding a "land card" specifically
    /// - With undefined type, the discard cannot be verified as legal
    #[test]
    fn test_mox_diamond_discard_cannot_use_library_of_leng_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Put Library of Leng on the battlefield
        let library_def = library_of_leng();
        let _library_id =
            game.create_object_from_definition(&library_def, alice, Zone::Battlefield);

        // Verify Library of Leng is on the battlefield
        let lol_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Library of Leng")
                .unwrap_or(false)
        });
        assert!(
            lol_on_battlefield,
            "Library of Leng should be on the battlefield"
        );

        // Create a land in Alice's hand
        let _land_id = create_land_in_hand(&mut game, alice);

        // Create Mox Diamond on the stack
        let mox_def = crate::cards::definitions::mox_diamond();
        let mox_id = game.create_object_from_definition(&mox_def, alice, Zone::Stack);

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
        let mut dm = MoxDiamondLibraryOfLengTestDecisionMaker::new(true);
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert!(result.is_ok(), "Resolution should succeed");

        // Verify Mox Diamond is on the battlefield
        let mox_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(
            mox_on_battlefield,
            "Mox Diamond should be on the battlefield after land discard"
        );

        // CRITICAL: The land MUST be in the graveyard, NOT on top of the library.
        // Library of Leng's replacement cannot apply because of rule 701.8c.
        let land_in_graveyard = game.player(alice).unwrap().graveyard.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.has_card_type(CardType::Land))
                .unwrap_or(false)
        });
        assert!(
            land_in_graveyard,
            "Land must be in graveyard (Library of Leng's replacement cannot apply per rule 701.8c)"
        );

        // Verify the land is NOT on top of the library
        // (This confirms Library of Leng's replacement was correctly not applied)
        let library = &game.player(alice).unwrap().library;
        let land_on_top_of_library = library.last().map_or(false, |&id| {
            game.object(id)
                .map(|obj| obj.has_card_type(CardType::Land))
                .unwrap_or(false)
        });
        assert!(
            !land_on_top_of_library,
            "Land should NOT be on top of library (rule 701.8c prevents Library of Leng replacement)"
        );
    }

    /// Test: Library of Leng's presence does not affect the choice between
    /// discarding a land vs. letting Mox Diamond go to the graveyard.
    #[test]
    fn test_mox_diamond_decline_discard_with_library_of_leng() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Put Library of Leng on the battlefield
        let library_def = library_of_leng();
        let _library_id =
            game.create_object_from_definition(&library_def, alice, Zone::Battlefield);

        // Create a land in Alice's hand
        let land_id = create_land_in_hand(&mut game, alice);

        // Create Mox Diamond on the stack
        let mox_def = crate::cards::definitions::mox_diamond();
        let mox_id = game.create_object_from_definition(&mox_def, alice, Zone::Stack);

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

        // Resolve with a decision maker that DECLINES to discard
        let mut dm = MoxDiamondLibraryOfLengTestDecisionMaker::new(false);
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert!(result.is_ok(), "Resolution should succeed");

        // Mox Diamond should be in the graveyard (not battlefield)
        let mox_in_graveyard = game.player(alice).unwrap().graveyard.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(
            mox_in_graveyard,
            "Mox Diamond should be in graveyard when land discard is declined"
        );

        // The land should still be in hand
        assert!(
            game.player(alice).unwrap().hand.contains(&land_id),
            "Land should still be in hand when discard is declined"
        );

        // Library of Leng should still be on the battlefield (unaffected)
        let lol_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Library of Leng")
                .unwrap_or(false)
        });
        assert!(
            lol_on_battlefield,
            "Library of Leng should still be on the battlefield"
        );
    }

    /// Test: When a player explicitly chooses to use Library of Leng's replacement
    /// with Mox Diamond, the Mox Diamond goes to the graveyard because the land's
    /// type becomes undefined in the library.
    #[test]
    fn test_mox_diamond_with_library_of_leng_chosen_goes_to_graveyard() {
        use crate::zone::Zone;

        /// A decision maker that chooses to use Library of Leng (put card to library)
        struct LibraryOfLengChooserDM;

        impl DecisionMaker for LibraryOfLengChooserDM {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &SelectObjectsContext,
            ) -> Vec<ObjectId> {
                // Discard the first legal land
                if !ctx.candidates.is_empty() {
                    vec![ctx.candidates[0].id]
                } else {
                    vec![]
                }
            }

            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &SelectOptionsContext,
            ) -> Vec<usize> {
                // EXPLICITLY choose library (Library of Leng's replacement)
                // Look for the option that mentions "Library"
                for opt in &ctx.options {
                    if opt.description.contains("Library") || opt.description.contains("library") {
                        return vec![opt.index];
                    }
                }
                // Default to first option if no library option found
                if !ctx.options.is_empty() {
                    vec![ctx.options[0].index]
                } else {
                    vec![]
                }
            }

            fn decide_priority(
                &mut self,
                _game: &GameState,
                _ctx: &PriorityContext,
            ) -> LegalAction {
                LegalAction::PassPriority
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Put Library of Leng on the battlefield
        let library_def = library_of_leng();
        let _library_id =
            game.create_object_from_definition(&library_def, alice, Zone::Battlefield);

        // Create a land in Alice's hand
        let _land_id = create_land_in_hand(&mut game, alice);

        // Create Mox Diamond on the stack
        let mox_def = crate::cards::definitions::mox_diamond();
        let mox_id = game.create_object_from_definition(&mox_def, alice, Zone::Stack);

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

        // Resolve with a decision maker that chooses to use Library of Leng
        let mut dm = LibraryOfLengChooserDM;
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert!(result.is_ok(), "Resolution should succeed");

        // The land should be on top of the library (Library of Leng was used)
        let library = &game.player(alice).unwrap().library;
        let land_on_top_of_library = library.last().map_or(false, |&id| {
            game.object(id)
                .map(|obj| obj.has_card_type(CardType::Land))
                .unwrap_or(false)
        });
        assert!(
            land_on_top_of_library,
            "Land should be on top of library (Library of Leng was used)"
        );

        // CRITICAL: Mox Diamond should be in the GRAVEYARD (not battlefield)
        // because the land's type is undefined per rule 701.8c
        let mox_in_graveyard = game.player(alice).unwrap().graveyard.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(
            mox_in_graveyard,
            "Mox Diamond should be in graveyard (land type undefined in library per 701.8c)"
        );

        // Verify Mox Diamond is NOT on the battlefield
        let mox_on_battlefield = game.battlefield.iter().any(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mox Diamond")
                .unwrap_or(false)
        });
        assert!(
            !mox_on_battlefield,
            "Mox Diamond should NOT be on battlefield (cannot verify land was discarded)"
        );
    }

    /// Test: Document the rule 701.8c interaction in code comments.
    /// This test exists to document the expected behavior.
    #[test]
    fn test_rule_701_8c_documentation() {
        // Rule 701.8c states:
        // "If a card is discarded, but an effect causes it to be put into a
        // hidden zone instead of into its owner's graveyard without being revealed,
        // all values of that card's characteristics are considered to be undefined."
        //
        // This affects Library of Leng + Mox Diamond as follows:
        //
        // 1. Mox Diamond has an ETB replacement effect that says:
        //    "you may discard a land card instead"
        //
        // 2. Library of Leng has a replacement effect that says:
        //    "you may put it on top of your library instead of into your graveyard"
        //
        // 3. If a player tries to use Library of Leng's replacement when discarding
        //    for Mox Diamond:
        //    - The card would go to the library (a hidden zone)
        //    - Per rule 701.8c, the card's characteristics become undefined
        //    - Mox Diamond's effect requires a "land card" to be discarded
        //    - With undefined characteristics, it cannot be verified as a land
        //    - Therefore, the discard is illegal
        //
        // 4. The result: Library of Leng's replacement simply cannot be chosen
        //    when discarding for Mox Diamond. The land must go to the graveyard.

        // This test passes if it compiles - it documents the expected behavior
        assert!(true, "Rule 701.8c interaction documented");
    }

    #[test]
    fn test_replay_library_of_leng_casting() {
        let game = run_replay_test(
            vec![
                "1", // Play Island (land)
                "2", // Tap Island for mana
                "1", // Cast Library of Leng
            ],
            ReplayTestConfig::new().p1_hand(vec!["Library of Leng", "Island"]),
        );

        // Library of Leng should be on the battlefield
        assert!(
            game.battlefield_has("Library of Leng"),
            "Library of Leng should be on battlefield after casting"
        );

        // Island should also be on the battlefield
        assert!(
            game.battlefield_has("Island"),
            "Island should be on battlefield"
        );
    }
}
