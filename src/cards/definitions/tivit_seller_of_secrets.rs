//! Tivit, Seller of Secrets card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Tivit, Seller of Secrets - Legendary Creature — Sphinx Rogue
/// {3}{W}{U}{B}
/// 6/6
/// Flying, ward {3}
/// Council's dilemma — Whenever Tivit enters the battlefield or deals combat
/// damage to a player, starting with you, each player votes for evidence or
/// bribery. For each evidence vote, investigate. For each bribery vote, create
/// a Treasure token. You may vote an additional time.
pub fn tivit_seller_of_secrets() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Tivit, Seller of Secrets")
        .parse_text(
            "Mana cost: {3}{W}{U}{B}\n\
             Type: Legendary Creature — Sphinx Rogue\n\
             Power/Toughness: 6/6\n\
             Flying, ward {3}\n\
             Council's dilemma — Whenever Tivit enters the battlefield or deals combat \
             damage to a player, starting with you, each player votes for evidence or \
             bribery. For each evidence vote, investigate. For each bribery vote, create \
             a Treasure token. You may vote an additional time.",
        )
        .expect("Tivit text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::cards::tokens::{clue_token_definition, treasure_token_definition};
    use crate::color::Color;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::types::{CardType, Subtype, Supertype};
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
    // Treasure Token Tests
    // ========================================

    #[test]
    fn test_treasure_token_properties() {
        let token = treasure_token_definition();
        assert_eq!(token.name(), "Treasure");
        assert!(token.card.has_card_type(CardType::Artifact));
        assert!(token.card.has_subtype(Subtype::Treasure));
        assert!(token.card.is_token);
    }

    #[test]
    fn test_treasure_token_has_mana_ability() {
        let token = treasure_token_definition();
        assert_eq!(token.abilities.len(), 1);
        assert!(matches!(token.abilities[0].kind, AbilityKind::Mana(_)));
    }

    // ========================================
    // Clue Token Tests
    // ========================================

    #[test]
    fn test_clue_token_properties() {
        let token = clue_token_definition();
        assert_eq!(token.name(), "Clue");
        assert!(token.card.has_card_type(CardType::Artifact));
        assert!(token.card.has_subtype(Subtype::Clue));
        assert!(token.card.is_token);
    }

    #[test]
    fn test_clue_token_has_activated_ability() {
        let token = clue_token_definition();
        assert_eq!(token.abilities.len(), 1);
        assert!(matches!(token.abilities[0].kind, AbilityKind::Activated(_)));
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_tivit_basic_properties() {
        let def = tivit_seller_of_secrets();
        assert_eq!(def.name(), "Tivit, Seller of Secrets");
        assert!(def.is_creature());
        assert!(!def.card.is_land());
        assert_eq!(def.card.mana_value(), 6);
    }

    #[test]
    fn test_tivit_is_legendary() {
        let def = tivit_seller_of_secrets();
        assert!(def.card.has_supertype(Supertype::Legendary));
    }

    #[test]
    fn test_tivit_is_sphinx_rogue() {
        let def = tivit_seller_of_secrets();
        assert!(def.card.has_subtype(Subtype::Sphinx));
        assert!(def.card.has_subtype(Subtype::Rogue));
    }

    #[test]
    fn test_tivit_power_toughness() {
        use crate::card::PtValue;
        let def = tivit_seller_of_secrets();
        let pt = def.card.power_toughness.as_ref().unwrap();
        assert_eq!(pt.power, PtValue::Fixed(6));
        assert_eq!(pt.toughness, PtValue::Fixed(6));
    }

    #[test]
    fn test_tivit_is_esper_colors() {
        let def = tivit_seller_of_secrets();
        let colors = def.card.colors();
        assert!(colors.contains(Color::White));
        assert!(colors.contains(Color::Blue));
        assert!(colors.contains(Color::Black));
        assert!(!colors.contains(Color::Red));
        assert!(!colors.contains(Color::Green));
    }

    #[test]
    fn test_tivit_has_flying() {
        use crate::static_abilities::StaticAbilityId;
        let def = tivit_seller_of_secrets();
        // Flying is a static ability
        let has_flying = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(sa) = &a.kind {
                sa.id() == StaticAbilityId::Flying
            } else {
                false
            }
        });
        assert!(has_flying, "Tivit should have flying");
    }

    #[test]
    fn test_tivit_has_ward() {
        use crate::static_abilities::StaticAbilityId;
        let def = tivit_seller_of_secrets();
        // Ward is a static ability
        let has_ward = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(sa) = &a.kind {
                sa.id() == StaticAbilityId::Ward
            } else {
                false
            }
        });
        assert!(has_ward, "Tivit should have ward");
    }

    // ========================================
    // Triggered Ability Structure Tests
    // ========================================

    #[test]
    fn test_tivit_has_three_abilities() {
        // Flying, Ward, and the triggered ability
        let def = tivit_seller_of_secrets();
        assert_eq!(def.abilities.len(), 3);
    }

    #[test]
    fn test_tivit_has_triggered_ability() {
        let def = tivit_seller_of_secrets();
        let has_triggered = def
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(has_triggered, "Tivit should have a triggered ability");
    }

    #[test]
    fn test_trigger_is_etb_or_combat_damage() {
        let def = tivit_seller_of_secrets();
        let triggered = def.abilities.iter().find_map(|a| {
            if let AbilityKind::Triggered(t) = &a.kind {
                Some(t)
            } else {
                None
            }
        });
        let triggered = triggered.expect("Should have triggered ability");
        let display = triggered.trigger.display();

        // Should contain both trigger conditions
        assert!(
            display.contains("enters the battlefield"),
            "Should trigger on ETB"
        );
        assert!(
            display.contains("deals combat damage"),
            "Should trigger on combat damage"
        );
    }

    #[test]
    fn test_trigger_has_vote_effect() {
        let def = tivit_seller_of_secrets();
        let triggered = def.abilities.iter().find_map(|a| {
            if let AbilityKind::Triggered(t) = &a.kind {
                Some(t)
            } else {
                None
            }
        });
        let triggered = triggered.expect("Should have triggered ability");

        // Should have exactly one effect (the vote)
        assert_eq!(triggered.effects.len(), 1);

        // Check it's a vote effect
        let debug_str = format!("{:?}", &triggered.effects[0]);
        assert!(
            debug_str.contains("VoteEffect"),
            "Effect should be VoteEffect"
        );
    }

    // ========================================
    // Trigger Detection Tests
    // ========================================

    #[test]
    fn test_triggers_on_etb() {
        use crate::events::zones::ZoneChangeEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tivit for Alice
        let def = tivit_seller_of_secrets();
        let tivit_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate Tivit entering the battlefield
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            tivit_id,
            Zone::Hand,
            Zone::Battlefield,
            None,
        ));

        let triggered = check_triggers(&game, &event);
        assert_eq!(triggered.len(), 1, "Tivit should trigger on ETB");
    }

    #[test]
    fn test_triggers_on_combat_damage_to_player() {
        use crate::events::DamageEvent;
        use crate::game_event::DamageTarget;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Tivit for Alice
        let def = tivit_seller_of_secrets();
        let tivit_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate Tivit dealing combat damage to Bob
        let event = TriggerEvent::new(DamageEvent::new(
            tivit_id,
            DamageTarget::Player(bob),
            6,
            true, // is_combat
        ));

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Tivit should trigger on combat damage to player"
        );
    }

    #[test]
    fn test_does_not_trigger_on_non_combat_damage() {
        use crate::events::DamageEvent;
        use crate::game_event::DamageTarget;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Tivit for Alice
        let def = tivit_seller_of_secrets();
        let tivit_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate non-combat damage from Tivit to Bob
        let event = TriggerEvent::new(DamageEvent::new(
            tivit_id,
            DamageTarget::Player(bob),
            6,
            false, // not combat
        ));

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            0,
            "Tivit should NOT trigger on non-combat damage"
        );
    }

    #[test]
    fn test_does_not_trigger_on_combat_damage_to_creature() {
        use crate::events::DamageEvent;
        use crate::game_event::DamageTarget;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Tivit for Alice
        let def = tivit_seller_of_secrets();
        let tivit_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a creature for Bob
        let creature_id = game.new_object_id();

        // Simulate combat damage to a creature (not a player)
        let event = TriggerEvent::new(DamageEvent::new(
            tivit_id,
            DamageTarget::Object(creature_id),
            6,
            true, // is_combat
        ));

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            0,
            "Tivit should NOT trigger on damage to a creature"
        );
    }

    // ========================================
    // On Battlefield Tests
    // ========================================

    #[test]
    fn test_tivit_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = tivit_seller_of_secrets();
        let tivit_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&tivit_id));

        // Verify the object has the abilities
        let obj = game.object(tivit_id).unwrap();
        assert_eq!(obj.abilities.len(), 3);
    }

    #[test]
    fn test_tivit_creature_stats() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = tivit_seller_of_secrets();
        let tivit_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(tivit_id).unwrap();
        assert!(obj.is_creature());
        assert!(obj.has_subtype(Subtype::Sphinx));
        assert!(obj.has_subtype(Subtype::Rogue));
    }

    // ========================================
    // Multiplayer Tests
    // ========================================

    #[test]
    fn test_tivit_triggers_in_multiplayer() {
        use crate::events::zones::ZoneChangeEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_multiplayer_game();
        let alice = PlayerId::from_index(0);

        // Create Tivit for Alice
        let def = tivit_seller_of_secrets();
        let tivit_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate Tivit entering the battlefield
        let event = TriggerEvent::new(ZoneChangeEvent::new(
            tivit_id,
            Zone::Hand,
            Zone::Battlefield,
            None,
        ));

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Tivit should trigger in multiplayer game"
        );
    }

    // ========================================
    // Replay Integration Tests
    // ========================================

    /// Tests Tivit's ETB council's dilemma vote creates the correct mix of tokens.
    ///
    /// Scenario: Alice casts Tivit, chooses to vote an additional time.
    /// Votes: Alice -> evidence, bribery; Bob -> bribery.
    /// Expected: Alice creates 1 Clue and 2 Treasure tokens.
    #[test]
    fn test_replay_tivit_etb_creates_tokens() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Tivit
                "0", // Pay {W}
                "0", // Pay {U}
                "0", // Pay {B}
                "0", // Pay {1}
                "0", // Pay {1} (final {1} auto-paid if only one option remains)
                // ETB trigger resolves: council's dilemma voting
                "1", // Alice chooses to vote an additional time
                "0", // Alice vote 1: evidence
                "1", // Alice vote 2: bribery
                "1", // Bob vote: bribery
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Tivit, Seller of Secrets"])
                .p1_battlefield(vec![
                    "Plains", "Island", "Swamp", "Plains", "Island", "Swamp",
                ]),
        );

        let alice = PlayerId::from_index(0);

        // Tivit should be on the battlefield
        assert!(
            game.battlefield_has("Tivit, Seller of Secrets"),
            "Tivit should be on battlefield after casting"
        );

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
            clue_count, 1,
            "Alice should have 1 Clue token from 1 evidence vote"
        );

        let treasure_count = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|o| o.name == "Treasure" && o.controller == alice)
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(
            treasure_count, 2,
            "Alice should have 2 Treasure tokens from 2 bribery votes"
        );
    }

    /// Tests Marneus Calgar triggers once per token created when Tivit creates tokens.
    ///
    /// Scenario (3 players): Player 1 controls Tivit + Marneus and votes an additional time.
    /// Votes: Player 1 -> bribery, bribery; Player 2 -> evidence; Player 3 -> evidence.
    /// Expected: Player 1 creates 2 Clue and 2 Treasure tokens; Marneus draws three times
    /// (two separate investigates + one Treasure batch).
    #[test]
    fn test_replay_tivit_etb_with_marneus_draws_on_tokens() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Tivit
                "0", // Pay {W}
                "0", // Pay {U}
                "0", // Pay {B}
                "0", // Pay {1}
                "0", // Pay {1} (final {1} auto-paid if only one option remains)
                // ETB trigger resolves: council's dilemma voting
                "1", // Tivit controller votes an additional time
                "1", // Player 1 vote 1: bribery
                "1", // Player 1 vote 2: bribery
                "0", // Player 2 vote: evidence
                "0", // Player 3 vote: evidence
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Tivit, Seller of Secrets"])
                .p1_battlefield(vec![
                    "Marneus Calgar",
                    "Plains",
                    "Island",
                    "Swamp",
                    "Plains",
                    "Island",
                    "Swamp",
                ])
                .p1_deck(vec!["Plains", "Plains", "Plains"])
                .p3_hand(vec![]),
        );

        let controller = PlayerId::from_index(0);

        // Marneus should be on the battlefield
        assert!(
            game.battlefield_has("Marneus Calgar"),
            "Marneus should be on battlefield"
        );

        // Verify tokens were created under Tivit/Marneus's controller
        let clue_count = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|o| o.name == "Clue" && o.controller == controller)
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(
            clue_count, 2,
            "Player 1 should have 2 Clue tokens from the evidence votes"
        );

        let treasure_count = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|o| o.name == "Treasure" && o.controller == controller)
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(
            treasure_count, 2,
            "Player 1 should have 2 Treasure tokens from 2 bribery votes"
        );

        // Marneus should draw once per event (2 Clues separately + Treasure batch = 3 draws).
        let marneus_hand_size = game.player(controller).map(|p| p.hand.len()).unwrap_or(0);
        assert_eq!(
            marneus_hand_size, 3,
            "Marneus should draw three times from the Clue and Treasure tokens"
        );
    }
}
