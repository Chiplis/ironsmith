//! Grizzly Bears card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Grizzly Bears - {1}{G}
/// Creature — Bear (2/2)
pub fn grizzly_bears() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Grizzly Bears")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_grizzly_bears() {
        let def = grizzly_bears();
        assert_eq!(def.name(), "Grizzly Bears");
        assert!(def.is_creature());
        assert!(def.abilities.is_empty());
    }

    #[test]
    fn test_replay_grizzly_bears_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Grizzly Bears
                "0", // Tap Forest 1
                "0", // Tap Forest 2 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Grizzly Bears"])
                .p1_battlefield(vec!["Forest", "Forest"]),
        );

        // Grizzly Bears should be on the battlefield
        assert!(
            game.battlefield_has("Grizzly Bears"),
            "Grizzly Bears should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let bears_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Grizzly Bears" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(bears_id) = bears_id {
            assert_eq!(
                game.calculated_power(bears_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(bears_id),
                Some(2),
                "Should have 2 toughness"
            );
        } else {
            panic!("Could not find Grizzly Bears on battlefield");
        }
    }
}
