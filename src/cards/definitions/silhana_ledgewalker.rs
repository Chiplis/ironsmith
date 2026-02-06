//! Silhana Ledgewalker card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Silhana Ledgewalker - {1}{G}
/// Creature — Elf Rogue (1/1)
/// Hexproof
/// Silhana Ledgewalker can't be blocked except by creatures with flying.
pub fn silhana_ledgewalker() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Silhana Ledgewalker")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Rogue])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Hexproof\nSilhana Ledgewalker can't be blocked except by creatures with flying.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_silhana_ledgewalker() {
        let def = silhana_ledgewalker();
        assert_eq!(def.name(), "Silhana Ledgewalker");
        assert_eq!(def.abilities.len(), 2);
    }

    /// Tests casting Silhana Ledgewalker (hexproof creature with flying restriction evasion).
    ///
    /// Silhana Ledgewalker: {1}{G} creature 1/1
    /// Hexproof
    /// Can't be blocked except by creatures with flying.
    #[test]
    fn test_replay_silhana_ledgewalker_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Silhana Ledgewalker
                "0", // Tap Forest 1
                "0", // Tap Forest 2 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Silhana Ledgewalker"])
                .p1_battlefield(vec!["Forest", "Forest"]),
        );

        // Silhana Ledgewalker should be on the battlefield
        assert!(
            game.battlefield_has("Silhana Ledgewalker"),
            "Silhana Ledgewalker should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let stalker_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Silhana Ledgewalker" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(stalker_id) = stalker_id {
            assert_eq!(
                game.calculated_power(stalker_id),
                Some(1),
                "Should have 1 power"
            );
            assert_eq!(
                game.calculated_toughness(stalker_id),
                Some(1),
                "Should have 1 toughness"
            );
        } else {
            panic!("Could not find Silhana Ledgewalker on battlefield");
        }
    }
}
