//! Braids, Arisen Nightmare card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;

/// Braids, Arisen Nightmare - Legendary Creature — Nightmare
/// {1}{B}{B}
/// 3/3
/// At the beginning of your end step, you may sacrifice an artifact, creature,
/// enchantment, land, or planeswalker. If you do, each opponent may sacrifice
/// a permanent that shares a card type with it. For each opponent who doesn't,
/// that player loses 2 life and you draw a card.
pub fn braids_arisen_nightmare() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Braids, Arisen Nightmare")
        .parse_text(
            "Mana cost: {1}{B}{B}\n\
             Type: Legendary Creature — Nightmare\n\
             Power/Toughness: 3/3\n\
             At the beginning of your end step, you may sacrifice an artifact, creature, \
             enchantment, land, or planeswalker. If you do, each opponent may sacrifice \
             a permanent that shares a card type with it. For each opponent who doesn't, \
             that player loses 2 life and you draw a card.",
        )
        .expect("Braids, Arisen Nightmare text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::color::Color;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::types::{Subtype, Supertype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_braids_basic_properties() {
        let def = braids_arisen_nightmare();
        assert_eq!(def.name(), "Braids, Arisen Nightmare");
        assert!(def.is_creature());
        assert!(!def.card.is_land());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_braids_is_legendary() {
        let def = braids_arisen_nightmare();
        assert!(def.card.has_supertype(Supertype::Legendary));
    }

    #[test]
    fn test_braids_is_nightmare() {
        let def = braids_arisen_nightmare();
        assert!(def.card.has_subtype(Subtype::Nightmare));
    }

    #[test]
    fn test_braids_power_toughness() {
        use crate::card::PtValue;
        let def = braids_arisen_nightmare();
        let pt = def.card.power_toughness.as_ref().unwrap();
        assert_eq!(pt.power, PtValue::Fixed(3));
        assert_eq!(pt.toughness, PtValue::Fixed(3));
    }

    #[test]
    fn test_braids_is_black() {
        let def = braids_arisen_nightmare();
        assert!(def.card.colors().contains(Color::Black));
        assert!(!def.card.colors().contains(Color::White));
    }

    #[test]
    fn test_braids_mana_cost() {
        let def = braids_arisen_nightmare();
        assert_eq!(def.card.mana_value(), 3);
        // {1}{B}{B} = 1 generic + 2 black
    }

    #[test]
    fn test_braids_has_one_ability() {
        let def = braids_arisen_nightmare();
        assert_eq!(def.abilities.len(), 1);
    }

    // ========================================
    // Triggered Ability Structure Tests
    // ========================================

    #[test]
    fn test_ability_is_triggered() {
        let def = braids_arisen_nightmare();
        let ability = &def.abilities[0];
        assert!(matches!(ability.kind, AbilityKind::Triggered(_)));
    }

    #[test]
    fn test_trigger_is_beginning_of_end_step() {
        let def = braids_arisen_nightmare();
        let ability = &def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            // Now using Trigger struct - check display contains end step
            assert!(
                triggered.trigger.display().contains("end step"),
                "Should trigger on end step"
            );
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_trigger_has_composed_effects() {
        let def = braids_arisen_nightmare();
        let ability = &def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            // Now uses composed effects: with_id(may(...)) and if_then(...)
            assert!(
                triggered.effects.len() >= 2,
                "Should have the top-level may/if-then effect structure, got {:?}",
                triggered.effects
            );

            let first_structural_effect = triggered
                .effects
                .iter()
                .find(|effect| !format!("{:?}", effect).contains("TagTriggeringObjectEffect"))
                .expect("expected non-prelude structural effect");

            // First structural effect should be WithIdEffect wrapping MayEffect
            let debug_str_0 = format!("{:?}", first_structural_effect);
            assert!(
                debug_str_0.contains("WithIdEffect"),
                "First structural effect should be WithIdEffect"
            );
            assert!(
                debug_str_0.contains("MayEffect"),
                "First structural effect should contain MayEffect"
            );

            let second_structural_effect = triggered
                .effects
                .iter()
                .skip_while(|effect| !format!("{:?}", effect).contains("WithIdEffect"))
                .skip(1)
                .find(|effect| !format!("{:?}", effect).contains("TagTriggeringObjectEffect"))
                .expect("expected if-effect after the may wrapper");

            // Next structural effect should be IfEffect
            let debug_str_1 = format!("{:?}", second_structural_effect);
            assert!(
                debug_str_1.contains("IfEffect"),
                "Next structural effect should be IfEffect"
            );
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_trigger_has_no_targets() {
        let def = braids_arisen_nightmare();
        let ability = &def.abilities[0];
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            // Braids doesn't target - the effects determine which permanents
            assert!(triggered.choices.is_empty());
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_ability_functions_on_battlefield() {
        let def = braids_arisen_nightmare();
        let ability = &def.abilities[0];
        assert!(ability.functions_in(&Zone::Battlefield));
        assert!(!ability.functions_in(&Zone::Graveyard));
    }

    // ========================================
    // On Battlefield Tests
    // ========================================

    #[test]
    fn test_braids_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Braids on the battlefield
        let def = braids_arisen_nightmare();
        let braids_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&braids_id));

        // Verify the object has the ability
        let obj = game.object(braids_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);

        // Verify it's the triggered ability
        assert!(matches!(obj.abilities[0].kind, AbilityKind::Triggered(_)));
    }

    #[test]
    fn test_braids_creature_stats() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Braids on the battlefield
        let def = braids_arisen_nightmare();
        let braids_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let obj = game.object(braids_id).unwrap();
        assert!(obj.is_creature());
        assert!(obj.has_subtype(Subtype::Nightmare));
    }

    // ========================================
    // Trigger Detection Tests
    // ========================================

    #[test]
    fn test_triggers_at_your_end_step() {
        use crate::events::phase::BeginningOfEndStepEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Braids for Alice
        let def = braids_arisen_nightmare();
        let _braids_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate Alice's end step
        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Braids should trigger at owner's end step"
        );
    }

    #[test]
    fn test_does_not_trigger_at_opponent_end_step() {
        use crate::events::phase::BeginningOfEndStepEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Braids for Alice
        let def = braids_arisen_nightmare();
        let _braids_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate Bob's end step
        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            0,
            "Braids should NOT trigger at opponent's end step"
        );
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_braids_doesnt_trigger_from_graveyard() {
        use crate::events::phase::BeginningOfEndStepEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Braids and move it to the graveyard
        let def = braids_arisen_nightmare();
        let braids_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _new_id = game
            .move_object_by_effect(braids_id, Zone::Graveyard)
            .unwrap();

        // Simulate Alice's end step
        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );

        let triggered = check_triggers(&game, &event);
        assert_eq!(triggered.len(), 0, "Braids in graveyard should not trigger");
    }

    #[test]
    fn test_multiple_braids_trigger_separately() {
        use crate::events::phase::BeginningOfEndStepEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create two Braids (normally impossible due to legendary rule, but for testing)
        let def = braids_arisen_nightmare();
        let _braids1_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let _braids2_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Simulate Alice's end step
        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            2,
            "Both Braids should trigger (ignoring legendary rule for this test)"
        );
    }

    #[test]
    fn test_opponent_braids_does_not_affect_your_end_step() {
        use crate::events::phase::BeginningOfEndStepEvent;
        use crate::triggers::{TriggerEvent, check_triggers};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Braids for Bob
        let def = braids_arisen_nightmare();
        let _braids_id = game.create_object_from_definition(&def, bob, Zone::Battlefield);

        // Simulate Alice's end step
        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );

        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            0,
            "Bob's Braids should not trigger at Alice's end step"
        );

        // Bob's end step should trigger
        let event = TriggerEvent::new_with_provenance(
            BeginningOfEndStepEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered.len(),
            1,
            "Bob's Braids should trigger at Bob's end step"
        );
    }

    #[test]
    fn test_braids_oracle_like_text_is_emittable() {
        let def = braids_arisen_nightmare();
        let rendered = crate::compiled_text::canonical_compiled_lines(&def).join(" ");
        let lower = rendered.to_ascii_lowercase();

        assert!(
            lower.contains("at the beginning of your end step"),
            "missing trigger lead-in: {rendered}"
        );
        assert!(
            lower.contains("loses 2 life") && lower.contains("draw a card"),
            "missing life-loss/draw payload: {rendered}"
        );
    }

    #[test]
    fn test_braids_raw_effects_match_if_you_do_structure() {
        let def = braids_arisen_nightmare();
        let ability = &def.abilities[0];

        let AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("Expected triggered ability");
        };

        let debug = format!("{:#?}", triggered.effects);
        assert!(
            debug.contains("MayEffect") && debug.contains("SacrificeEffect"),
            "expected optional sacrifice branch, got {debug}"
        );
        assert!(
            debug.contains("IfEffect")
                && debug.contains("DidNotHappen")
                && debug.contains("ForPlayersEffect"),
            "expected if-you-do + opponent loop structure, got {debug}"
        );
        assert!(
            debug.contains("LoseLifeEffect") && debug.contains("DrawCardsEffect"),
            "expected lose-life and draw effects in fallback branch, got {debug}"
        );
    }
}
