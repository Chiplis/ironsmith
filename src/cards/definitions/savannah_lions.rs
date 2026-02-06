//! Savannah Lions card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Savannah Lions - {W}
/// Creature — Cat (2/1)
/// (Vanilla, but classic efficient creature)
pub fn savannah_lions() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Savannah Lions")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Cat])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_savannah_lions() {
        let def = savannah_lions();
        assert_eq!(def.name(), "Savannah Lions");
        assert!(def.is_creature());
        assert!(def.abilities.is_empty());
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Savannah Lions (simple 1-mana creature).
    ///
    /// Savannah Lions: {W} creature 2/1
    #[test]
    fn test_replay_savannah_lions_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Savannah Lions
                "0", // Tap Plains for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Savannah Lions"])
                .p1_battlefield(vec!["Plains"]),
        );

        // Savannah Lions should be on the battlefield
        assert!(
            game.battlefield_has("Savannah Lions"),
            "Savannah Lions should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let lions_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Savannah Lions" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(lions_id) = lions_id {
            assert_eq!(
                game.calculated_power(lions_id),
                Some(2),
                "Savannah Lions should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(lions_id),
                Some(1),
                "Savannah Lions should have 1 toughness"
            );
        } else {
            panic!("Could not find Savannah Lions on battlefield");
        }
    }
}
