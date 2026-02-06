//! Mirran Crusader card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Mirran Crusader - {1}{W}{W}
/// Creature — Human Knight (2/2)
/// Double strike, protection from black and from green
pub fn mirran_crusader() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mirran Crusader")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Knight])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Double strike, protection from black and from green")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_mirran_crusader() {
        let def = mirran_crusader();
        assert_eq!(def.name(), "Mirran Crusader");
        let static_count = def
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Static(_)))
            .count();
        assert_eq!(static_count, 3);
    }

    /// Tests casting Mirran Crusader (creature with double strike and protection).
    ///
    /// Mirran Crusader: {1}{W}{W} creature 2/2
    /// Double strike, protection from black and from green
    #[test]
    fn test_replay_mirran_crusader_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Mirran Crusader
                "0", // Tap first Plains
                "0", // Tap second Plains
                "0", // Tap third Plains (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Mirran Crusader"])
                .p1_battlefield(vec!["Plains", "Plains", "Plains"]),
        );

        // Mirran Crusader should be on the battlefield
        assert!(
            game.battlefield_has("Mirran Crusader"),
            "Mirran Crusader should be on battlefield after casting"
        );
    }
}
