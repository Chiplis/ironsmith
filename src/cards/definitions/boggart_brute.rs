//! Boggart Brute card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Boggart Brute - {2}{R}
/// Creature — Goblin Warrior (3/2)
/// Menace
pub fn boggart_brute() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Boggart Brute")
        .parse_text(
            "Mana cost: {2}{R}\n\
             Type: Creature — Goblin Warrior\n\
             Power/Toughness: 3/2\n\
             Menace",
        )
        .expect("Boggart Brute text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_boggart_brute() {
        let def = boggart_brute();
        assert_eq!(def.name(), "Boggart Brute");
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_menace()
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_replay_boggart_brute_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Boggart Brute
                "0", // Tap Mountain 1
                "0", // Tap Mountain 2
                "0", // Tap Mountain 3 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Boggart Brute"])
                .p1_battlefield(vec!["Mountain", "Mountain", "Mountain"]),
        );

        // Boggart Brute should be on the battlefield
        assert!(
            game.battlefield_has("Boggart Brute"),
            "Boggart Brute should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let brute_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Boggart Brute" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(brute_id) = brute_id {
            assert_eq!(
                game.calculated_power(brute_id),
                Some(3),
                "Should have 3 power"
            );
            assert_eq!(
                game.calculated_toughness(brute_id),
                Some(2),
                "Should have 2 toughness"
            );
        } else {
            panic!("Could not find Boggart Brute on battlefield");
        }
    }
}
