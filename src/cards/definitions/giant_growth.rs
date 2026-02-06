//! Giant Growth card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::effect::{Effect, Until};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ChooseSpec;
use crate::types::CardType;

/// Giant Growth - {G}
/// Instant
/// Target creature gets +3/+3 until end of turn.
pub fn giant_growth() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Giant Growth")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::pump(
            3,
            3,
            ChooseSpec::creature(),
            Until::EndOfTurn,
        )])
        .oracle_text("Target creature gets +3/+3 until end of turn.")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_giant_growth() {
        let def = giant_growth();
        assert_eq!(def.name(), "Giant Growth");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 1);
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Giant Growth targeting a creature and pumping it.
    ///
    /// Giant Growth: {G} instant - Target creature gets +3/+3 until end of turn.
    /// This test verifies:
    /// 1. Casting the instant spell
    /// 2. Targeting a creature
    /// 3. The P/T modification being applied
    #[test]
    fn test_replay_giant_growth_pump() {
        let game = run_replay_test(
            vec![
                "1", // Cast Giant Growth (index 1, after PassPriority at 0)
                "0", // Target Grizzly Bears
                "0", // Tap Forest for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Giant Growth"])
                .p1_battlefield(vec!["Forest", "Grizzly Bears"]),
        );

        // Grizzly Bears should still be on battlefield
        assert!(
            game.battlefield_has("Grizzly Bears"),
            "Grizzly Bears should be on battlefield"
        );

        // Find Grizzly Bears ID and verify its P/T including continuous effects
        let alice = PlayerId::from_index(0);
        let bears_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Grizzly Bears" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(bears_id) = bears_id {
            // Base 2/2 + Giant Growth +3/+3 = 5/5
            // Use calculated_power/toughness which applies continuous effects
            assert_eq!(
                game.calculated_power(bears_id),
                Some(5),
                "Bears should have 5 power after Giant Growth"
            );
            assert_eq!(
                game.calculated_toughness(bears_id),
                Some(5),
                "Bears should have 5 toughness after Giant Growth"
            );
        } else {
            panic!("Could not find Grizzly Bears on battlefield");
        }
    }
}
