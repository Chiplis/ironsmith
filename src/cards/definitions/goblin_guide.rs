//! Goblin Guide card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Goblin Guide - {R}
/// Creature — Goblin Scout (2/2)
/// Haste
/// Whenever Goblin Guide attacks, defending player reveals the top card of their library.
/// If it's a land card, that player puts it into their hand.
pub fn goblin_guide() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Goblin Guide")
        .parse_text(
            "Mana cost: {R}\n\
             Type: Creature — Goblin Scout\n\
             Power/Toughness: 2/2\n\
             Haste\n\
             Whenever Goblin Guide attacks, defending player reveals the top card of their library. If it's a land card, that player puts it into their hand.",
        )
        .expect("Goblin Guide text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::events::EventKind;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_goblin_guide_basic_properties() {
        let def = goblin_guide();
        assert_eq!(def.name(), "Goblin Guide");
        assert!(def.card.is_creature());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_goblin_guide_has_haste() {
        let def = goblin_guide();
        let has_haste = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_haste()
            } else {
                false
            }
        });
        assert!(has_haste, "Goblin Guide should have haste");
    }

    #[test]
    fn test_goblin_guide_subtypes() {
        let def = goblin_guide();
        assert!(def.card.has_subtype(Subtype::Goblin));
        assert!(def.card.has_subtype(Subtype::Scout));
    }

    #[test]
    fn test_goblin_guide_power_toughness() {
        let def = goblin_guide();
        let pt = def.card.power_toughness.as_ref().expect("Should have P/T");
        use crate::card::PtValue;
        assert_eq!(pt.power, PtValue::Fixed(2));
        assert_eq!(pt.toughness, PtValue::Fixed(2));
    }

    // ========================================
    // Attack Trigger Tests
    // ========================================

    #[test]
    fn test_goblin_guide_has_attack_trigger() {
        let def = goblin_guide();
        // Now using Trigger struct - check display contains attacks
        let attack_trigger = def.abilities.iter().find(|a| {
            if let AbilityKind::Triggered(t) = &a.kind {
                t.trigger.display().contains("attacks")
            } else {
                false
            }
        });
        assert!(
            attack_trigger.is_some(),
            "Should have 'whenever attacks' trigger"
        );
    }

    #[test]
    fn test_attack_trigger_uses_defending_player_filter() {
        let def = goblin_guide();

        // Find the attack trigger (now using Trigger struct)
        let trigger = def.abilities.iter().find_map(|a| {
            if let AbilityKind::Triggered(t) = &a.kind {
                if t.trigger.display().contains("attacks") {
                    Some(t)
                } else {
                    None
                }
            } else {
                None
            }
        });

        let trigger = trigger.expect("Should have attack trigger");

        // The trigger should have two effects (reveal + conditional move)
        assert_eq!(trigger.effects.len(), 2);

        // Verify one of the effects uses PlayerFilter::Defending
        let uses_defending = trigger
            .effects
            .iter()
            .any(|effect| format!("{:?}", effect).contains("Defending"));
        assert!(
            uses_defending,
            "Expected defending player filter in trigger effects"
        );
    }

    #[test]
    fn test_attack_trigger_fires_on_attack_event() {
        use crate::events::combat::CreatureAttackedEvent;
        use crate::triggers::{AttackEventTarget, TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Goblin Guide on battlefield
        let def = goblin_guide();
        let guide_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate the attack event
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            guide_id,
            AttackEventTarget::Player(bob),
        ));

        // Check if triggers fire
        let triggers = check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            1,
            "Attacking with Goblin Guide should trigger its ability"
        );

        // Verify it's the Goblin Guide's trigger
        assert_eq!(triggers[0].source_name, "Goblin Guide");
    }

    #[test]
    fn test_trigger_does_not_fire_for_other_creature_attacks() {
        use crate::card::{CardBuilder, PowerToughness as CardPT};
        use crate::events::combat::CreatureAttackedEvent;
        use crate::ids::CardId;
        use crate::triggers::{AttackEventTarget, TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Goblin Guide on battlefield
        let def = goblin_guide();
        let _guide_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create another creature
        let bear = CardBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(CardPT::fixed(2, 2))
            .build();
        let bear_id = game.create_object_from_card(&bear, alice, Zone::Battlefield);

        // Simulate the attack event for the OTHER creature
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            bear_id,
            AttackEventTarget::Player(bob),
        ));

        // Check if triggers fire - should NOT trigger Goblin Guide
        let triggers = check_triggers(&game, &event);
        assert_eq!(
            triggers.len(),
            0,
            "Goblin Guide's trigger should not fire for other creatures attacking"
        );
    }

    // ========================================
    // Defending Player Resolution Tests
    // ========================================

    #[test]
    fn test_defending_player_available_in_execution_context() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Goblin Guide on battlefield
        let def = goblin_guide();
        let guide_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Create execution context with defending player set
        // This simulates how the game loop sets up the context when processing the attack trigger
        let ctx = ExecutionContext::new_default(guide_id, alice).with_defending_player(bob);

        assert_eq!(
            ctx.defending_player,
            Some(bob),
            "ExecutionContext should have defending player set"
        );
    }

    #[test]
    fn test_trigger_event_contains_attack_target() {
        use crate::events::combat::CreatureAttackedEvent;
        use crate::triggers::{AttackEventTarget, TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Goblin Guide on battlefield
        let def = goblin_guide();
        let guide_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate the attack event targeting Bob
        let event = TriggerEvent::new(CreatureAttackedEvent::new(
            guide_id,
            AttackEventTarget::Player(bob),
        ));

        // Check the trigger
        let triggers = check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);

        // Verify the triggering event contains the target
        assert_eq!(
            triggers[0].triggering_event.kind(),
            EventKind::CreatureAttacked
        );
        if let Some(attacked) = triggers[0]
            .triggering_event
            .downcast::<CreatureAttackedEvent>()
        {
            match attacked.target {
                AttackEventTarget::Player(player_id) => {
                    assert_eq!(
                        player_id, bob,
                        "Target should be Bob (the defending player)"
                    );
                }
                AttackEventTarget::Planeswalker(_) => {
                    panic!("Expected player target, got planeswalker");
                }
            }
        } else {
            panic!("Expected CreatureAttacked event");
        }
    }

    // ========================================
    // Integration Notes
    // ========================================
    //
    // The defending player resolution works correctly because:
    // 1. When Goblin Guide attacks, GameEvent::CreatureAttacked is generated
    //    with the target (AttackEventTarget::Player(defender_id))
    // 2. When the trigger is put on the stack, game_loop.rs extracts the
    //    defending player from the event and calls with_defending_player()
    //    on the StackEntry
    // 3. When the stack entry resolves, the ExecutionContext has
    //    defending_player set, allowing PlayerFilter::Defending to resolve
    //
    // See game_loop.rs:3377-3380 for the code that extracts defending player
    // from CreatureAttacked events.

    // ========================================
    // Replay Tests
    // ========================================

    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    /// Tests casting Goblin Guide (creature with haste).
    ///
    /// Goblin Guide: {R} creature 2/2
    /// Haste
    #[test]
    fn test_replay_goblin_guide_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Goblin Guide
                "0", // Tap Mountain for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Goblin Guide"])
                .p1_battlefield(vec!["Mountain"]),
        );

        // Goblin Guide should be on the battlefield
        assert!(
            game.battlefield_has("Goblin Guide"),
            "Goblin Guide should be on battlefield after casting"
        );
    }
}
