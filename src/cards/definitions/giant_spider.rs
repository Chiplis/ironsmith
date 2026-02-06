//! Giant Spider card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Giant Spider - {3}{G}
/// Creature — Spider (2/4)
/// Reach
pub fn giant_spider() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Giant Spider")
        .parse_text(
            "Mana cost: {3}{G}\n\
             Type: Creature — Spider\n\
             Power/Toughness: 2/4\n\
             Reach",
        )
        .expect("Giant Spider text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_giant_spider() {
        let def = giant_spider();
        assert_eq!(def.name(), "Giant Spider");
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_reach()
            } else {
                false
            }
        }));
    }

    /// Tests casting Giant Spider (creature with reach).
    ///
    /// Giant Spider: {3}{G} creature 2/4
    /// Reach
    #[test]
    fn test_replay_giant_spider_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Giant Spider
                "0", // Tap Forest 1
                "0", // Tap Forest 2
                "0", // Tap Forest 3
                "0", // Tap Forest 4 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Giant Spider"])
                .p1_battlefield(vec!["Forest", "Forest", "Forest", "Forest"]),
        );

        // Giant Spider should be on the battlefield
        assert!(
            game.battlefield_has("Giant Spider"),
            "Giant Spider should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let spider_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Giant Spider" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(spider_id) = spider_id {
            assert_eq!(
                game.calculated_power(spider_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(spider_id),
                Some(4),
                "Should have 4 toughness"
            );
        } else {
            panic!("Could not find Giant Spider on battlefield");
        }
    }
}
