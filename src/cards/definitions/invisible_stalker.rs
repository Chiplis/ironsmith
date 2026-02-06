//! Invisible Stalker card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;

/// Invisible Stalker - {1}{U}
/// Creature — Human Rogue (1/1)
/// Hexproof
/// Invisible Stalker can't be blocked.
pub fn invisible_stalker() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Invisible Stalker")
        .parse_text(
            "Mana cost: {1}{U}\n\
             Type: Creature — Human Rogue\n\
             Power/Toughness: 1/1\n\
             Hexproof\n\
             Invisible Stalker can't be blocked.",
        )
        .expect("Invisible Stalker text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_invisible_stalker() {
        let def = invisible_stalker();
        assert_eq!(def.name(), "Invisible Stalker");
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_hexproof()
            } else {
                false
            }
        }));
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.is_unblockable()
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_replay_invisible_stalker_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Invisible Stalker
                "0", // Tap first Island
                "0", // Tap second Island
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Invisible Stalker"])
                .p1_battlefield(vec!["Island", "Island"]),
        );

        let alice = PlayerId::from_index(0);

        assert!(
            game.battlefield_has("Invisible Stalker"),
            "Invisible Stalker should be on battlefield after casting"
        );

        let creature_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Invisible Stalker" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(creature_id) = creature_id {
            assert_eq!(
                game.calculated_power(creature_id),
                Some(1),
                "Should have 1 power"
            );
            assert_eq!(
                game.calculated_toughness(creature_id),
                Some(1),
                "Should have 1 toughness"
            );
        } else {
            panic!("Could not find Invisible Stalker on battlefield");
        }
    }
}
