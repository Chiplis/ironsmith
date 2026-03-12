//! Ashnod's Altar card definition.
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Ashnod's Altar - {3}
/// Artifact
/// Sacrifice a creature: Add {C}{C}.
pub fn ashnods_altar() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Ashnod's Altar")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text("Sacrifice a creature: Add {C}{C}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::GameScript;

    #[test]
    fn test_ashnods_altar_basic_properties() {
        let def = ashnods_altar();
        assert_eq!(def.name(), "Ashnod's Altar");
        assert_eq!(def.card.mana_value(), 3);
        assert!(def.card.card_types.contains(&CardType::Artifact));
        assert!(!def.card.card_types.contains(&CardType::Creature));
    }

    #[test]
    fn test_ashnods_altar_has_mana_ability() {
        let def = ashnods_altar();
        assert!(def.abilities.iter().any(|a| a.is_mana_ability()));
    }

    #[test]
    fn test_ashnods_altar_produces_colorless_mana() {
        let def = ashnods_altar();

        let mana_ability = def
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        if let AbilityKind::Activated(act_ab) = &mana_ability.kind {
            // Should produce 2 colorless mana
            let mana = act_ab
                .mana_output
                .as_ref()
                .expect("Should have mana_output");
            assert_eq!(mana.len(), 2);
            assert_eq!(mana[0], ManaSymbol::Colorless);
            assert_eq!(mana[1], ManaSymbol::Colorless);
        } else {
            panic!("Expected activated mana ability");
        }
    }

    #[test]
    fn test_ashnods_altar_requires_creature_sacrifice() {
        let def = ashnods_altar();

        let mana_ability = def
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        if let AbilityKind::Activated(act_ab) = &mana_ability.kind {
            // Should not require tap
            assert!(!act_ab.has_tap_cost(), "Should not require tapping");

            // Sacrifice is modeled as a non-mana cost component so "dies" triggers fire
            assert!(
                !act_ab.mana_cost.costs().is_empty(),
                "Should have non-mana costs for sacrifice"
            );
            // Should have 2 non-mana cost components: choose + sacrifice
            assert_eq!(
                act_ab.mana_cost.costs().len(),
                2,
                "Should have choose + sacrifice effects"
            );

            let debug_str = format!("{:?}", &act_ab.mana_cost.costs());
            assert!(
                debug_str.contains("ChooseObjectsEffect"),
                "non-mana costs should contain choose objects"
            );
            assert!(
                debug_str.contains("SacrificeEffect"),
                "non-mana costs should contain sacrifice"
            );
        } else {
            panic!("Expected activated mana ability");
        }
    }

    #[test]
    fn test_ashnods_altar_is_not_a_creature() {
        let def = ashnods_altar();
        assert!(!def.is_creature());
        assert!(def.is_permanent());
    }

    #[test]
    fn test_ashnods_altar_no_tap_required() {
        let def = ashnods_altar();

        let mana_ability = def
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        if let AbilityKind::Activated(act_ab) = &mana_ability.kind {
            // Ashnod's Altar doesn't need to tap - you can sacrifice multiple creatures
            assert!(!act_ab.has_tap_cost(), "Altar should not require tap");
        } else {
            panic!("Expected activated mana ability");
        }
    }

    // Integration tests using GameScript

    #[test]
    fn test_ashnods_altar_in_hand() {
        // Test that we can set up a game with Ashnod's Altar in hand
        let result = GameScript::new()
            .player("Alice", &["Ashnod's Altar"])
            .player("Bob", &[])
            .run();

        assert!(result.is_ok(), "Game setup should succeed");
        let game = result.unwrap();

        // Verify altar is in Alice's hand
        let alice = PlayerId::from_index(0);
        let alice_player = game.player(alice).expect("Alice should exist");
        let altar_in_hand = alice_player
            .hand
            .iter()
            .any(|&id| game.object(id).is_some_and(|o| o.name == "Ashnod's Altar"));
        assert!(altar_in_hand, "Ashnod's Altar should be in Alice's hand");
    }

    #[test]
    fn test_ashnods_altar_with_creature_in_hand() {
        // Test that we can have both altar and a creature in hand
        let result = GameScript::new()
            .player("Alice", &["Ashnod's Altar", "Grizzly Bears"])
            .player("Bob", &[])
            .run();

        assert!(result.is_ok(), "Game setup should succeed");
        let game = result.unwrap();

        // Verify both are in Alice's hand
        let alice = PlayerId::from_index(0);
        let alice_player = game.player(alice).expect("Alice should exist");

        let altar_in_hand = alice_player
            .hand
            .iter()
            .any(|&id| game.object(id).is_some_and(|o| o.name == "Ashnod's Altar"));
        assert!(altar_in_hand, "Ashnod's Altar should be in hand");

        let creature_in_hand = alice_player
            .hand
            .iter()
            .any(|&id| game.object(id).is_some_and(|o| o.name == "Grizzly Bears"));
        assert!(creature_in_hand, "Grizzly Bears should be in hand");
    }

    #[test]
    fn test_ashnods_altar_mana_ability_is_instant_speed() {
        // Mana abilities don't use the stack and can be activated any time
        // This test verifies the ability structure is correct for a mana ability
        let def = ashnods_altar();

        let mana_ability = def
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        // Mana abilities should function on the battlefield
        assert!(mana_ability.functions_in(&crate::zone::Zone::Battlefield));

        // It's a mana ability (activated with mana_output)
        assert!(mana_ability.is_mana_ability());
    }

    #[test]
    fn test_ashnods_altar_can_sacrifice_any_creature_type() {
        // Verify the filter accepts any creature (not restricted to specific types)
        let def = ashnods_altar();

        let mana_ability = def
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        if let AbilityKind::Activated(act_ab) = &mana_ability.kind {
            // The sacrifice filter is now in the non-mana costs via ChooseObjectsEffect
            let debug_str = format!("{:?}", &act_ab.mana_cost.costs());
            // Should include creature filter
            assert!(
                debug_str.contains("Creature"),
                "Should filter for creatures"
            );
            // Should include "you_control" (YouControl)
            assert!(
                debug_str.contains("YouControl") || debug_str.contains("You"),
                "Should require you control"
            );
        } else {
            panic!("Expected activated mana ability");
        }
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_ashnods_altar_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Tap Sol Ring for mana (2 colorless)
                "2", // Tap Island for mana (1 blue, but used as colorless for generic)
                "1", // Cast Ashnod's Altar
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Ashnod's Altar"])
                .p1_battlefield(vec!["Sol Ring", "Island"]),
        );

        assert!(
            game.battlefield_has("Ashnod's Altar"),
            "Ashnod's Altar should be on battlefield after casting"
        );
    }
}
