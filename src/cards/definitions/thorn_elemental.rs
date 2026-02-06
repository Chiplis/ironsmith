//! Thorn Elemental card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Thorn Elemental - {5}{G}{G}
/// Creature — Elemental (7/7)
/// Trample
/// You may have Thorn Elemental assign its combat damage as though it weren't blocked.
pub fn thorn_elemental() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Thorn Elemental")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "Trample\nYou may have Thorn Elemental assign its combat damage as though it weren't blocked.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_thorn_elemental() {
        let def = thorn_elemental();
        assert_eq!(def.name(), "Thorn Elemental");
        assert!(def.is_creature());
    }

    /// Tests casting Thorn Elemental (large creature with trample).
    ///
    /// Thorn Elemental: {5}{G}{G} creature 7/7
    /// Trample
    /// May assign combat damage as though it weren't blocked.
    #[test]
    fn test_replay_thorn_elemental_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Thorn Elemental
                "0", // Tap Forest 1
                "0", // Tap Forest 2
                "0", // Tap Forest 3
                "0", // Tap Forest 4
                "0", // Tap Forest 5
                "0", // Tap Forest 6
                "0", // Tap Forest 7 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Thorn Elemental"])
                .p1_battlefield(vec![
                    "Forest", "Forest", "Forest", "Forest", "Forest", "Forest", "Forest",
                ]),
        );

        // Thorn Elemental should be on the battlefield
        assert!(
            game.battlefield_has("Thorn Elemental"),
            "Thorn Elemental should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let elemental_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Thorn Elemental" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(elemental_id) = elemental_id {
            assert_eq!(
                game.calculated_power(elemental_id),
                Some(7),
                "Should have 7 power"
            );
            assert_eq!(
                game.calculated_toughness(elemental_id),
                Some(7),
                "Should have 7 toughness"
            );
        } else {
            panic!("Could not find Thorn Elemental on battlefield");
        }
    }
}
