//! Merciless Executioner card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Merciless Executioner - {2}{B}
/// Creature — Orc Warrior
/// 3/1
/// When Merciless Executioner enters the battlefield, each player sacrifices a creature.
pub fn merciless_executioner() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Merciless Executioner")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Orc, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text(
            "When Merciless Executioner enters the battlefield, each player sacrifices a creature.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::color::Color;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_merciless_executioner_basic_properties() {
        let def = merciless_executioner();
        assert_eq!(def.name(), "Merciless Executioner");
        assert!(def.is_creature());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_merciless_executioner_is_black() {
        let def = merciless_executioner();
        assert!(def.card.colors().contains(Color::Black));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_merciless_executioner_types() {
        let def = merciless_executioner();
        assert!(def.card.has_subtype(Subtype::Orc));
        assert!(def.card.has_subtype(Subtype::Warrior));
    }

    #[test]
    fn test_merciless_executioner_power_toughness() {
        let def = merciless_executioner();
        let pt = def.card.power_toughness.as_ref().unwrap();
        assert_eq!(pt.power.base_value(), 3);
        assert_eq!(pt.toughness.base_value(), 1);
    }

    #[test]
    fn test_merciless_executioner_has_etb() {
        let def = merciless_executioner();
        assert_eq!(def.abilities.len(), 1);

        if let AbilityKind::Triggered(triggered) = &def.abilities[0].kind {
            assert!(
                triggered.trigger.display().contains("enters"),
                "Should trigger on entering battlefield"
            );
            // Effect is now a declarative ForPlayersEffect composition
            assert_eq!(triggered.effects.len(), 1);
            let debug_str = format!("{:?}", &triggered.effects[0]);
            assert!(debug_str.contains("ForPlayersEffect"));
        } else {
            panic!("Expected triggered ability");
        }
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_merciless_executioner_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = merciless_executioner();
        let executioner_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(game.battlefield.contains(&executioner_id));

        let obj = game.object(executioner_id).unwrap();
        assert_eq!(obj.abilities.len(), 1);
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests casting Merciless Executioner with its ETB sacrifice effect.
    #[test]
    fn test_replay_merciless_executioner_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Merciless Executioner
                "0", // Tap Swamp 1
                "0", // Tap Swamp 2
                "0", // Tap Swamp 3
                // ETB trigger goes on stack, both players must sacrifice
                "0", // Alice chooses to sacrifice Grizzly Bears
                "0", // Bob chooses to sacrifice Llanowar Elves
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Merciless Executioner"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp", "Grizzly Bears"])
                .p2_battlefield(vec!["Llanowar Elves"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Merciless Executioner should be on battlefield
        assert!(
            game.battlefield_has("Merciless Executioner"),
            "Merciless Executioner should be on battlefield"
        );

        // Grizzly Bears should be in Alice's graveyard
        let alice_player = game.player(alice).unwrap();
        let bears_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(bears_in_gy, "Grizzly Bears should be in Alice's graveyard");

        // Llanowar Elves should be in Bob's graveyard
        let bob_player = game.player(bob).unwrap();
        let elves_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Llanowar Elves")
                .unwrap_or(false)
        });
        assert!(elves_in_gy, "Llanowar Elves should be in Bob's graveyard");
    }

    /// Tests Merciless Executioner when there are no other creatures - it must sacrifice itself.
    #[test]
    fn test_replay_merciless_executioner_alone() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Merciless Executioner
                "0", // Tap Swamp 1
                "0", // Tap Swamp 2
                "0", // Tap Swamp 3
                // ETB trigger: Alice must sacrifice (only Merciless Executioner available)
                "0", // Alice sacrifices Merciless Executioner
                     // Bob has no creatures, so skips sacrifice
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Merciless Executioner"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp"]),
        );

        let alice = PlayerId::from_index(0);

        // Merciless Executioner should be in graveyard (sacrificed itself)
        let alice_player = game.player(alice).unwrap();
        let executioner_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Merciless Executioner")
                .unwrap_or(false)
        });
        assert!(
            executioner_in_gy,
            "Merciless Executioner should have sacrificed itself"
        );

        // Should not be on battlefield
        assert!(
            !game.battlefield_has("Merciless Executioner"),
            "Merciless Executioner should not be on battlefield"
        );
    }
}
