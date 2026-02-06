//! Amulet of Vigor card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Amulet of Vigor
/// {1}
/// Artifact
/// Whenever a permanent enters the battlefield tapped and under your control, untap it.
pub fn amulet_of_vigor() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Amulet of Vigor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a permanent enters the battlefield tapped and under your control, untap it.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::effect::EffectOutcome;
    use crate::events::zones::EnterBattlefieldEvent;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::triggers::{TriggerEvent, check_triggers};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_land(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn execute_amulet_trigger(
        game: &mut GameState,
        controller: PlayerId,
        source: ObjectId,
        event: TriggerEvent,
    ) -> Vec<EffectOutcome> {
        let def = amulet_of_vigor();
        let triggered = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .expect("Amulet should have a triggered ability");
        let AbilityKind::Triggered(triggered) = &triggered.kind else {
            unreachable!("Expected triggered ability");
        };

        let mut ctx =
            ExecutionContext::new_default(source, controller).with_triggering_event(event);

        let mut outcomes = Vec::with_capacity(triggered.effects.len());
        for effect in &triggered.effects {
            outcomes.push(effect.0.execute(game, &mut ctx).unwrap());
        }
        outcomes
    }

    // ========================================
    // Basic Properties Tests
    // ========================================

    #[test]
    fn test_amulet_of_vigor_basic_properties() {
        let def = amulet_of_vigor();
        assert_eq!(def.name(), "Amulet of Vigor");
        assert!(def.card.is_artifact());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_amulet_of_vigor_is_colorless() {
        let def = amulet_of_vigor();
        assert_eq!(def.card.colors().count(), 0);
    }

    #[test]
    fn test_amulet_of_vigor_has_triggered_ability() {
        let def = amulet_of_vigor();
        assert_eq!(def.abilities.len(), 1);
        assert!(matches!(&def.abilities[0].kind, AbilityKind::Triggered(_)));
    }

    #[test]
    fn test_amulet_of_vigor_ability_functions_on_battlefield() {
        let def = amulet_of_vigor();
        let ability = &def.abilities[0];
        assert!(ability.functions_in(&Zone::Battlefield));
        assert!(!ability.functions_in(&Zone::Graveyard));
    }

    // ========================================
    // Trigger Detection Tests
    // ========================================

    #[test]
    fn test_triggers_when_permanent_enters_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Amulet of Vigor on the battlefield
        let def = amulet_of_vigor();
        let amulet_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a land that will enter tapped
        let land_id = create_land(&mut game, "Tapped Land", alice);

        // Create event where permanent enters tapped
        let event = TriggerEvent::new(EnterBattlefieldEvent::tapped(land_id, Zone::Hand));

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Amulet of Vigor should trigger when permanent enters tapped"
        );
        assert_eq!(triggered[0].source, amulet_id);
    }

    #[test]
    fn test_does_not_trigger_when_permanent_enters_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Amulet of Vigor on the battlefield
        let def = amulet_of_vigor();
        let _amulet_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a land that will enter untapped
        let land_id = create_land(&mut game, "Untapped Land", alice);

        // Create event where permanent enters untapped (default)
        let event = TriggerEvent::new(EnterBattlefieldEvent::new(land_id, Zone::Hand));

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            0,
            "Amulet of Vigor should NOT trigger when permanent enters untapped"
        );
    }

    #[test]
    fn test_does_not_trigger_for_opponent_permanent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Alice controls Amulet of Vigor
        let def = amulet_of_vigor();
        let _amulet_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Bob's land enters tapped
        let land_id = create_land(&mut game, "Bob's Land", bob);
        let event = TriggerEvent::new(EnterBattlefieldEvent::tapped(land_id, Zone::Hand));

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            0,
            "Amulet of Vigor should NOT trigger for opponent's permanents"
        );
    }

    #[test]
    fn test_multiple_amulets_trigger_separately() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two Amulets of Vigor
        let def = amulet_of_vigor();
        let amulet1_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let amulet2_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create a land that enters tapped
        let land_id = create_land(&mut game, "Tapped Land", alice);
        let event = TriggerEvent::new(EnterBattlefieldEvent::tapped(land_id, Zone::Hand));

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            2,
            "Both Amulets should trigger when permanent enters tapped"
        );

        let sources: Vec<ObjectId> = triggered.iter().map(|t| t.source).collect();
        assert!(sources.contains(&amulet1_id));
        assert!(sources.contains(&amulet2_id));
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_effect_untaps_triggering_permanent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Create a land and tap it
        let land_id = create_land(&mut game, "Tapped Land", alice);
        game.tap(land_id);
        assert!(game.is_tapped(land_id));

        // Create execution context with the triggering event
        let triggering_event =
            TriggerEvent::new(EnterBattlefieldEvent::tapped(land_id, Zone::Hand));
        let _outcomes = execute_amulet_trigger(&mut game, alice, source, triggering_event);

        assert!(
            !game.is_tapped(land_id),
            "Land should be untapped by effect"
        );
    }

    #[test]
    fn test_effect_does_nothing_when_permanent_gone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Create a land, then move it to graveyard (simulating it being destroyed)
        let land_id = create_land(&mut game, "Temporary Land", alice);
        game.move_object(land_id, Zone::Graveyard);

        // Create execution context with the triggering event
        let triggering_event =
            TriggerEvent::new(EnterBattlefieldEvent::tapped(land_id, Zone::Hand));
        let _outcomes = execute_amulet_trigger(&mut game, alice, source, triggering_event);

        assert!(
            !game.battlefield.contains(&land_id),
            "Land should still be off the battlefield"
        );
    }

    #[test]
    fn test_effect_does_nothing_without_triggering_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let def = amulet_of_vigor();
        let triggered = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .expect("Amulet should have a triggered ability");
        let AbilityKind::Triggered(triggered) = &triggered.kind else {
            unreachable!("Expected triggered ability");
        };

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = triggered.effects[0].0.execute(&mut game, &mut ctx);

        assert!(result.is_err(), "Missing triggering event should error");
    }

    // ========================================
    // On Battlefield Tests
    // ========================================

    #[test]
    fn test_amulet_of_vigor_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = amulet_of_vigor();
        let amulet_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(game.battlefield.contains(&amulet_id));

        let obj = game.object(amulet_id).unwrap();
        assert!(obj.has_card_type(CardType::Artifact));
        assert_eq!(obj.abilities.len(), 1);
    }

    #[test]
    fn test_oracle_text() {
        let def = amulet_of_vigor();
        assert!(
            def.card
                .oracle_text
                .contains("enters the battlefield tapped")
        );
        assert!(def.card.oracle_text.contains("untap it"));
    }

    // ========================================
    // Replay Integration Tests
    // ========================================

    /// Tests that Amulet of Vigor untaps a shock land that enters tapped.
    ///
    /// Scenario: Alice has Amulet of Vigor on the battlefield and Godless Shrine in hand.
    /// She plays Godless Shrine and chooses NOT to pay 2 life, so it enters tapped.
    /// Amulet of Vigor should trigger and untap it.
    #[test]
    fn test_replay_amulet_untaps_shock_land() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Alice's main phase - play Godless Shrine
                "1", // Play Godless Shrine (PlayLand action)
                "",  // Decline to pay 2 life (MayChoice = false), land enters tapped
                     // Amulet of Vigor triggers, goes on stack, auto-resolves
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Godless Shrine"])
                .p1_battlefield(vec!["Amulet of Vigor"]),
        );

        let alice = PlayerId::from_index(0);

        // Godless Shrine should be on battlefield
        assert!(
            game.battlefield_has("Godless Shrine"),
            "Godless Shrine should be on battlefield"
        );

        // Godless Shrine should be UNTAPPED (Amulet of Vigor untapped it)
        let shrine = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|o| (id, o)))
            .find(|(_, obj)| obj.name == "Godless Shrine" && obj.controller == alice);

        assert!(
            shrine.is_some(),
            "Godless Shrine should exist on battlefield"
        );
        if let Some((id, _)) = shrine {
            assert!(
                !game.is_tapped(id),
                "Godless Shrine should be untapped by Amulet of Vigor"
            );
        }

        // Alice should still be at 20 life (didn't pay life)
        assert_eq!(
            game.life_total(alice),
            20,
            "Alice should be at 20 life (didn't pay life for shock land)"
        );
    }
}
