//! Fleshbag Marauder card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Fleshbag Marauder - {2}{B}
/// Creature — Zombie Warrior
/// 3/1
/// When this creature enters, each player sacrifices a creature of their choice.
pub fn fleshbag_marauder() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Fleshbag Marauder")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text("When this creature enters, each player sacrifices a creature of their choice.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::color::Color;

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_fleshbag_marauder_basic_properties() {
        let def = fleshbag_marauder();
        assert_eq!(def.name(), "Fleshbag Marauder");
        assert!(def.is_creature());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_fleshbag_marauder_is_black() {
        let def = fleshbag_marauder();
        assert!(def.card.colors().contains(Color::Black));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_fleshbag_marauder_subtypes() {
        let def = fleshbag_marauder();
        assert!(def.card.has_subtype(Subtype::Zombie));
        assert!(def.card.has_subtype(Subtype::Warrior));
    }

    #[test]
    fn test_fleshbag_marauder_power_toughness() {
        let def = fleshbag_marauder();
        let pt = def.card.power_toughness.unwrap();
        assert_eq!(pt.power.base_value(), 3);
        assert_eq!(pt.toughness.base_value(), 1);
    }

    #[test]
    fn test_fleshbag_marauder_has_etb_trigger() {
        let def = fleshbag_marauder();

        // Should have exactly one ability (the ETB trigger)
        assert_eq!(def.abilities.len(), 1);

        // Check that it's a triggered ability with ETB trigger
        let ability = &def.abilities[0];
        match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                assert!(
                    triggered.trigger.display().contains("enters"),
                    "Should trigger on entering battlefield"
                );

                // Check that the effect is present (now uses declarative ForPlayersEffect composition)
                assert_eq!(triggered.effects.len(), 1);
                let debug_str = format!("{:?}", &triggered.effects[0]);
                assert!(debug_str.contains("ForPlayersEffect"));
            }
            _ => panic!("Expected triggered ability"),
        }
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests casting Fleshbag Marauder with ETB sacrifice trigger.
    ///
    /// Note: The ETB trigger causes each player to sacrifice a creature.
    /// If the Marauder is the only creature Alice controls when the trigger
    /// resolves, she will have to sacrifice it.
    #[test]
    fn test_replay_fleshbag_marauder_casting() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Fleshbag Marauder
                "0", // Tap Swamp 1
                "0", // Tap Swamp 2
                "0", // Tap Swamp 3
                     // ETB trigger resolves automatically via auto-pass
                     // EachPlayerSacrificesEffect runs for each player
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Fleshbag Marauder"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp", "Grizzly Bears"])
                .p2_battlefield(vec!["Llanowar Elves"]),
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Fleshbag Marauder should be on the battlefield (it ETBs first, then trigger resolves)
        assert!(
            game.battlefield_has("Fleshbag Marauder"),
            "Fleshbag Marauder should be on battlefield"
        );

        // Alice's Grizzly Bears should be in graveyard (sacrificed to ETB)
        let alice_player = game.player(alice).unwrap();
        let bears_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(
            bears_in_gy,
            "Grizzly Bears should be in graveyard after sacrifice"
        );

        // Bob's Llanowar Elves should be in graveyard
        let bob_player = game.player(bob).unwrap();
        let elves_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Llanowar Elves")
                .unwrap_or(false)
        });
        assert!(
            elves_in_gy,
            "Llanowar Elves should be in graveyard after sacrifice"
        );
    }
}
