//! Lightning Bolt card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Lightning Bolt - {R}
/// Instant
/// Lightning Bolt deals 3 damage to any target.
pub fn lightning_bolt() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Lightning Bolt")
        .parse_text(
            "Mana cost: {R}\n\
             Type: Instant\n\
             Lightning Bolt deals 3 damage to any target.",
        )
        .expect("Lightning Bolt text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_lightning_bolt() {
        let def = lightning_bolt();
        assert_eq!(def.name(), "Lightning Bolt");
        assert!(def.is_spell());
        assert_eq!(def.card.mana_value(), 1);
        assert!(def.spell_effect.is_some());
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests Lightning Bolt dealing lethal damage to a creature.
    ///
    /// Lightning Bolt: {R} instant - deal 3 damage to any target
    /// Targeting Grizzly Bears (2/2) should kill it.
    ///
    /// Target order for AnyTarget: 0=Player1, 1=Player2, 2+=creatures
    #[test]
    fn test_replay_lightning_bolt_kills_creature() {
        let game = run_replay_test(
            vec![
                "1", // Cast Lightning Bolt
                "2", // Target Grizzly Bears (index 2: after Player1, Player2)
                "0", // Tap Mountain for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Lightning Bolt"])
                .p1_battlefield(vec!["Mountain"])
                .p2_battlefield(vec!["Grizzly Bears"]),
        );

        let bob = PlayerId::from_index(1);

        // Grizzly Bears should be dead (3 damage to 2 toughness creature)
        assert!(
            !game.battlefield_has("Grizzly Bears"),
            "Grizzly Bears should have died from 3 damage"
        );

        // Grizzly Bears should be in Bob's graveyard
        let bob_player = game.player(bob).unwrap();
        let bears_in_gy = bob_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Grizzly Bears")
                .unwrap_or(false)
        });
        assert!(bears_in_gy, "Grizzly Bears should be in Bob's graveyard");
    }

    /// Tests Lightning Bolt dealing non-lethal damage to a creature.
    ///
    /// Targeting Giant Spider (2/4) should damage but not kill it.
    ///
    /// Target order for AnyTarget: 0=Player1, 1=Player2, 2+=creatures
    #[test]
    fn test_replay_lightning_bolt_damages_creature() {
        let game = run_replay_test(
            vec![
                "1", // Cast Lightning Bolt
                "2", // Target Giant Spider (index 2: after Player1, Player2)
                "0", // Tap Mountain for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Lightning Bolt"])
                .p1_battlefield(vec!["Mountain"])
                .p2_battlefield(vec!["Giant Spider"]),
        );

        // Giant Spider should still be on battlefield (3 damage to 4 toughness)
        assert!(
            game.battlefield_has("Giant Spider"),
            "Giant Spider should survive 3 damage (4 toughness)"
        );

        // Find the Spider ID and check its damage
        let bob = PlayerId::from_index(1);
        let spider_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Giant Spider" && obj.controller == bob)
                .unwrap_or(false)
        });

        if let Some(spider_id) = spider_id {
            assert_eq!(
                game.damage_on(spider_id),
                3,
                "Giant Spider should have 3 damage marked on it"
            );
        } else {
            panic!("Could not find Giant Spider on battlefield");
        }
    }
}
