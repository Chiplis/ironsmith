//! Charismatic Conqueror card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Charismatic Conqueror
/// {1}{W}
/// Creature — Vampire Soldier
/// 2/2
/// Vigilance
/// Whenever an artifact or creature an opponent controls enters the battlefield untapped,
/// they may tap that permanent. If they don't, you create a 1/1 white Vampire creature
/// token with lifelink.
pub fn charismatic_conqueror() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Charismatic Conqueror")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Vampire, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Vigilance\n\
             Whenever an artifact or creature an opponent controls enters the battlefield \
             untapped, they may tap that permanent. If they don't, you create a 1/1 white \
             Vampire creature token with lifelink.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::{CardBuilder, PowerToughness as CardPT};
    use crate::color::Color;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    /// Create a test creature on the battlefield.
    fn create_test_creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(CardPT::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    // ========================================
    // Basic Properties Tests
    // ========================================

    #[test]
    fn test_charismatic_conqueror_basic_properties() {
        let def = charismatic_conqueror();
        assert_eq!(def.name(), "Charismatic Conqueror");
        assert!(def.is_creature());
        assert_eq!(def.card.mana_value(), 2);
    }

    #[test]
    fn test_charismatic_conqueror_is_white() {
        let def = charismatic_conqueror();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_charismatic_conqueror_subtypes() {
        let def = charismatic_conqueror();
        assert!(def.card.has_subtype(Subtype::Vampire));
        assert!(def.card.has_subtype(Subtype::Soldier));
    }

    #[test]
    fn test_charismatic_conqueror_power_toughness() {
        let def = charismatic_conqueror();
        let pt = def.card.power_toughness.as_ref().unwrap();
        use crate::card::PtValue;
        assert_eq!(pt.power, PtValue::Fixed(2));
        assert_eq!(pt.toughness, PtValue::Fixed(2));
    }

    #[test]
    fn test_charismatic_conqueror_has_vigilance() {
        let def = charismatic_conqueror();
        let has_vigilance = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_vigilance()
            } else {
                false
            }
        });
        assert!(has_vigilance, "Should have vigilance");
    }

    #[test]
    fn test_charismatic_conqueror_has_triggered_ability() {
        let def = charismatic_conqueror();
        let has_trigger = def
            .abilities
            .iter()
            .any(|a| matches!(&a.kind, AbilityKind::Triggered(_)));
        assert!(has_trigger, "Should have triggered ability");
    }

    // ========================================
    // On Battlefield Tests
    // ========================================

    #[test]
    fn test_charismatic_conqueror_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = charismatic_conqueror();
        let conqueror_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(game.battlefield.contains(&conqueror_id));

        let obj = game.object(conqueror_id).unwrap();
        assert!(obj.is_creature());
        assert!(obj.has_subtype(Subtype::Vampire));
        assert!(obj.has_subtype(Subtype::Soldier));
    }

    // ========================================
    // Rules Interaction Tests
    // ========================================

    #[test]
    fn test_oracle_text_mentions_may_tap() {
        let def = charismatic_conqueror();
        assert!(def.card.oracle_text.contains("may tap"));
    }

    #[test]
    fn test_oracle_text_mentions_artifact_or_creature() {
        let def = charismatic_conqueror();
        assert!(def.card.oracle_text.contains("artifact or creature"));
    }

    #[test]
    fn test_oracle_text_mentions_opponent_controls() {
        let def = charismatic_conqueror();
        assert!(def.card.oracle_text.contains("opponent controls"));
    }

    #[test]
    fn test_oracle_text_mentions_untapped() {
        let def = charismatic_conqueror();
        assert!(def.card.oracle_text.contains("untapped"));
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_trigger_detection_for_opponent_creature() {
        use crate::events::zones::EnterBattlefieldEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice controls Charismatic Conqueror
        let def = charismatic_conqueror();
        let _conqueror_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Bob's creature enters the battlefield
        let creature_id = create_test_creature(&mut game, bob, "Bob's Bear");

        // Check triggers for the ETB event
        let event = TriggerEvent::new(EnterBattlefieldEvent::new(creature_id, Zone::Hand));

        let triggers = check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1, "Should trigger once for Bob's creature");
        assert_eq!(
            triggers[0].source_name, "Charismatic Conqueror",
            "Trigger should be from Charismatic Conqueror"
        );
    }

    #[test]
    fn test_no_trigger_for_own_creature() {
        use crate::events::zones::EnterBattlefieldEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Alice controls Charismatic Conqueror
        let def = charismatic_conqueror();
        let _conqueror_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Alice's own creature enters the battlefield
        let creature_id = create_test_creature(&mut game, alice, "Alice's Bear");

        // Check triggers for the ETB event
        let event = TriggerEvent::new(EnterBattlefieldEvent::new(creature_id, Zone::Hand));

        let triggers = check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            0,
            "Should not trigger for controller's own creatures"
        );
    }

    // ========================================
    // Replay Integration Tests
    // ========================================

    /// Test: Opponent's creature enters, opponent declines to tap → Token is created.
    ///
    /// Setup: Alice controls Charismatic Conqueror on battlefield.
    /// Bob has Snapcaster Mage (flash) in hand with 2 Islands and a Lightning Bolt in graveyard.
    /// Bob casts Snapcaster on Alice's turn, declines to tap it, Alice gets a Vampire token.
    #[test]
    fn test_replay_charismatic_conqueror_opponent_declines_tap() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Alice's main phase - Alice has mana abilities from Plains, so must pass
                "", // Alice passes priority
                // Bob gets priority - can cast flash creature
                "1", // Bob casts Snapcaster Mage (CastSpell action)
                "0", // Mana payment (tap Islands)
                "",  // Alice passes priority (Snapcaster on stack)
                // Bob auto-passes, Snapcaster resolves, enters battlefield
                // Two triggers go on stack (APNAP order):
                // - Alice's Charismatic Conqueror trigger (bottom)
                // - Bob's Snapcaster ETB trigger (top)
                "0", // Target Lightning Bolt for Snapcaster's ETB
                "",  // Alice passes priority (triggers on stack)
                // Bob auto-passes, Snapcaster ETB resolves (grants flashback)
                "", // Alice passes priority (CC trigger still on stack)
                // Now Charismatic Conqueror trigger resolves
                "", // Bob DECLINES to tap Snapcaster (MayChoice = false)
                    // Token is created for Alice
            ],
            ReplayTestConfig::new()
                .p1_battlefield(vec!["Charismatic Conqueror", "Plains"])
                .p2_hand(vec!["Snapcaster Mage"])
                .p2_battlefield(vec!["Island", "Island"])
                .p2_graveyard(vec!["Lightning Bolt"]),
        );

        let alice = PlayerId::from_index(0);

        // Alice should have a Vampire token on the battlefield
        let has_vampire = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| obj.name == "Vampire" && obj.controller == alice);

        assert!(
            has_vampire,
            "Alice should have a Vampire token when opponent declines to tap"
        );

        // Snapcaster Mage should be on battlefield and UNTAPPED (opponent declined to tap)
        let snapcaster_untapped = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|o| (id, o)))
            .find(|(_, obj)| obj.name == "Snapcaster Mage")
            .map(|(id, _)| !game.is_tapped(id))
            .unwrap_or(false);

        assert!(
            snapcaster_untapped,
            "Snapcaster Mage should be untapped (opponent declined)"
        );
    }

    /// Test: Opponent's creature enters, opponent chooses to tap → No token created.
    ///
    /// Same setup as above, but Bob chooses to tap Snapcaster Mage to prevent token.
    #[test]
    fn test_replay_charismatic_conqueror_opponent_taps() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Alice's main phase - Alice has mana abilities, must pass
                "", // Alice passes priority
                // Bob gets priority - can cast flash creature
                "1", // Bob casts Snapcaster Mage (CastSpell action)
                "0", // Mana payment (tap Islands)
                "",  // Alice passes priority (Snapcaster on stack)
                // Bob auto-passes, Snapcaster resolves, enters battlefield
                // Two triggers go on stack (APNAP order):
                // - Alice's Charismatic Conqueror trigger (bottom)
                // - Bob's Snapcaster ETB trigger (top)
                "0", // Target Lightning Bolt for Snapcaster's ETB
                "",  // Alice passes priority (triggers on stack)
                // Bob auto-passes, Snapcaster ETB resolves (grants flashback)
                // Now Charismatic Conqueror trigger resolves
                "y", // Bob ACCEPTS tapping Snapcaster (MayChoice = true)
                // Snapcaster gets tapped, no token created
                "", // Final priority pass after trigger resolution
            ],
            ReplayTestConfig::new()
                .p1_battlefield(vec!["Charismatic Conqueror", "Plains"])
                .p2_hand(vec!["Snapcaster Mage"])
                .p2_battlefield(vec!["Island", "Island"])
                .p2_graveyard(vec!["Lightning Bolt"]),
        );

        let alice = PlayerId::from_index(0);

        // Alice should NOT have a Vampire token
        let has_vampire = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| obj.name == "Vampire" && obj.controller == alice);

        assert!(
            !has_vampire,
            "Alice should NOT have a Vampire token when opponent taps"
        );

        // Snapcaster Mage should be on battlefield and TAPPED
        let snapcaster_tapped = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|o| (id, o)))
            .find(|(_, obj)| obj.name == "Snapcaster Mage")
            .map(|(id, _)| game.is_tapped(id))
            .unwrap_or(false);

        assert!(
            snapcaster_tapped,
            "Snapcaster Mage should be tapped (opponent chose to tap)"
        );
    }
}
