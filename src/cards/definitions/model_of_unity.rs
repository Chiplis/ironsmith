//! Model of Unity card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::CardType;

/// Model of Unity - Artifact
/// {3}
/// Whenever players finish voting, you and each opponent who voted for a
/// choice you voted for may scry 2.
/// {T}: Add one mana of any color.
pub fn model_of_unity() -> CardDefinition {
    use crate::mana::{ManaCost, ManaSymbol};

    CardDefinitionBuilder::new(CardId::new(), "Model of Unity")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever players finish voting, you and each opponent who voted for a \
             choice you voted for may scry 2.\n\
             {T}: Add one mana of any color.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn setup_multiplayer_game() -> GameState {
        GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        )
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_model_of_unity_basic_properties() {
        let def = model_of_unity();
        assert_eq!(def.name(), "Model of Unity");
        assert!(def.card.has_card_type(CardType::Artifact));
        assert!(!def.is_creature());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_model_of_unity_is_colorless() {
        let def = model_of_unity();
        assert!(def.card.colors().is_empty());
    }

    #[test]
    fn test_model_of_unity_has_two_abilities() {
        let def = model_of_unity();
        // Mana ability + triggered ability
        assert_eq!(def.abilities.len(), 2);
    }

    #[test]
    fn test_model_of_unity_has_mana_ability() {
        let def = model_of_unity();
        let has_mana = def
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Mana(_)));
        assert!(has_mana, "Model of Unity should have a mana ability");
    }

    #[test]
    fn test_model_of_unity_has_triggered_ability() {
        let def = model_of_unity();
        let has_triggered = def
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(
            has_triggered,
            "Model of Unity should have a triggered ability"
        );
    }

    // ========================================
    // Trigger Detection Tests
    // ========================================

    #[test]
    fn test_triggers_on_voting_finished() {
        use crate::events::{KeywordActionEvent, KeywordActionKind, PlayerVote};
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Model of Unity on the battlefield
        let def = model_of_unity();
        let _model_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a voting event
        let vote_source = ObjectId::from_raw(100);
        let votes = vec![
            PlayerVote {
                player: alice,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: bob,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
        ];

        let voting_event = KeywordActionEvent::new(
            KeywordActionKind::Vote,
            alice,
            vote_source,
            votes.len() as u32,
        )
        .with_votes(votes);

        let event = TriggerEvent::new(voting_event);
        let triggered = check_triggers(&game, &event);

        assert_eq!(
            triggered.len(),
            1,
            "Model of Unity should trigger when players finish voting"
        );
    }

    #[test]
    fn test_does_not_trigger_on_etb() {
        use crate::events::EnterBattlefieldEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Model of Unity on the battlefield
        let def = model_of_unity();
        let model_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create an ETB event (Model of Unity itself entering)
        let etb_event = TriggerEvent::new(EnterBattlefieldEvent::new(model_id, Zone::Hand));

        let triggered = check_triggers(&game, &etb_event);
        assert_eq!(
            triggered.len(),
            0,
            "Model of Unity should NOT trigger on ETB"
        );
    }

    // ========================================
    // On Battlefield Tests
    // ========================================

    #[test]
    fn test_model_of_unity_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = model_of_unity();
        let model_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&model_id));

        // Verify the object has the abilities
        let obj = game.object(model_id).unwrap();
        assert_eq!(obj.abilities.len(), 2);
    }

    // ========================================
    // Multiplayer Tests
    // ========================================

    #[test]
    fn test_triggers_in_multiplayer() {
        use crate::events::{KeywordActionEvent, KeywordActionKind, PlayerVote};
        use crate::ids::ObjectId;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_multiplayer_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);

        // Create Model of Unity on the battlefield for Alice
        let def = model_of_unity();
        let _model_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a voting event where Alice and Bob voted together
        let vote_source = ObjectId::from_raw(100);
        let votes = vec![
            PlayerVote {
                player: alice,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: bob,
                option_index: 0,
                option_name: "evidence".to_string(),
            },
            PlayerVote {
                player: charlie,
                option_index: 1,
                option_name: "bribery".to_string(),
            },
        ];

        let voting_event = KeywordActionEvent::new(
            KeywordActionKind::Vote,
            alice,
            vote_source,
            votes.len() as u32,
        )
        .with_votes(votes);

        let event = TriggerEvent::new(voting_event);
        let triggered = check_triggers(&game, &event);

        assert_eq!(
            triggered.len(),
            1,
            "Model of Unity should trigger in multiplayer"
        );
    }

    // ========================================
    // Replay Integration Tests
    // ========================================

    /// Tests that Model of Unity's triggered ability fires when voting completes
    /// and that the "voted_with_you" tags are computed from the ability controller's perspective.
    ///
    /// Setup:
    /// - Player 1 (Alice) controls Tivit, Seller of Secrets
    /// - Player 2 (Bob) controls Model of Unity
    ///
    /// When Tivit deals combat damage:
    /// 1. Tivit's council's dilemma triggers → voting happens
    /// 2. Both players vote (Alice gets 2 votes due to council's dilemma)
    /// 3. PlayersFinishedVotingEvent is emitted
    /// 4. Model of Unity triggers for Bob
    /// 5. Bob may scry 2
    /// 6. If Alice voted with Bob, Alice may scry 2
    #[test]
    fn test_replay_model_of_unity_voting_interaction() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test_full_turn};

        // Based on actual debug output from previous run:
        // [0] Priority = "" pass (upkeep)
        // [1] Priority = "" pass (draw)
        // [2] Priority = "" pass (main)
        // [3] Priority = "" pass (begin combat)
        // [4] Priority = "" pass
        // [5] Attackers = "0" (declare Tivit)
        // [6] Priority = "" pass (declare attackers step)
        // [7] Blockers = "" (Bob doesn't block)
        // [8] Priority = "" pass (after blockers) <-- was "0", should be ""
        // [9] Priority = "" pass <-- was "0", should be ""
        // [10] Boolean = "1" (Alice may vote an additional time)
        // [11] Options = "0" (Alice vote 1: evidence) ✓
        // [12] Options = "0" (Alice vote 2: evidence) <-- was "1", should be "0"
        // [13] Options = "0" (Bob vote: evidence) <-- was "1", should be "0"
        // Then priority passes and triggered abilities
        let game = run_replay_test_full_turn(
            vec![
                "",  // [0] Priority (upkeep) - Alice pass
                "",  // [1] Priority (draw) - Alice pass
                "",  // [2] Priority (main) - Alice pass (go to combat)
                "",  // [3] Priority (begin combat) - Alice pass
                "",  // [4] Priority - Alice pass
                "0", // [5] Attackers: declare Tivit (index 0) as attacker
                "",  // [6] Priority (declare attackers step) - Alice pass
                "",  // [7] Blockers: Bob doesn't block
                "",  // [8] Priority (after blockers) - pass
                "",  // [9] Priority - pass
                // Tivit deals combat damage → council's dilemma triggers
                // Voting: Alice may vote an additional time
                "1", // [10] Boolean: Alice chooses to vote an additional time
                "0", // [11] Options: Alice vote 1 = "evidence"
                "0", // [12] Options: Alice vote 2 = "evidence"
                "0", // [13] Options: Bob vote = "evidence"
                // After votes, Tivit's trigger resolves (creates 3 Clues)
                // Then Model of Unity triggers
                "", // [14] Priority - pass
                "", // [15] Priority - pass
                // May scry prompts for Model of Unity
                "1", // [16] Boolean: Bob may scry 2 (1=yes)
                "1", // [17] Boolean: Alice may scry 2 (1=yes)
                // Scry decisions
                "0", // [18+] Scry choices
                "0", "0", "0",
            ],
            ReplayTestConfig::new()
                .p1_battlefield(vec!["Tivit, Seller of Secrets"])
                .p2_battlefield(vec!["Model of Unity"])
                .p1_deck(vec!["Plains", "Plains", "Plains", "Plains"])
                .p2_deck(vec!["Island", "Island", "Island", "Island"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Verify Bob took 6 damage from Tivit
        assert_eq!(
            game.life_total(bob),
            14,
            "Bob should be at 14 life after Tivit dealt 6 combat damage"
        );

        // Verify Clue tokens were created from the evidence votes
        // Alice voted twice for evidence, Bob voted once = 3 Clue tokens for Alice
        let clue_count = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|o| o.name == "Clue" && o.controller == alice)
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(
            clue_count, 3,
            "Alice should have 3 Clue tokens from 3 evidence votes"
        );
    }
}
