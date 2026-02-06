//! Hex Parasite card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Hex Parasite - {1}
/// Artifact Creature — Phyrexian Insect (1/1)
/// {X}, {B/P}: Remove up to X counters from target permanent.
/// For each counter removed this way, Hex Parasite gets +1/+0 until end of turn.
///
/// Implementation notes:
/// - The ability can remove ANY type of counter (loyalty, +1/+1, lore, charge, etc.)
/// - The +1/+0 bonus is applied via a continuous effect based on counters removed.
/// - "Up to X" is fully implemented: player chooses how many counters (0 to X) to remove.
/// - Counters are removed in alphabetical order by type when multiple types exist.
pub fn hex_parasite() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Hex Parasite")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Phyrexian, Subtype::Insect])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{X}, {B/P}: Remove up to X counters from target permanent. For each counter removed this way, Hex Parasite gets +1/+0 until end of turn.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_hex_parasite() {
        let def = hex_parasite();
        assert_eq!(def.name(), "Hex Parasite");

        // Should be artifact creature
        assert!(def.card.card_types.contains(&CardType::Artifact));
        assert!(def.card.card_types.contains(&CardType::Creature));

        // Should be Phyrexian Insect
        assert!(def.card.subtypes.contains(&Subtype::Phyrexian));
        assert!(def.card.subtypes.contains(&Subtype::Insect));

        // Should have 1/1 P/T
        assert_eq!(
            def.card
                .power_toughness
                .as_ref()
                .unwrap()
                .power
                .base_value(),
            1
        );
        assert_eq!(
            def.card
                .power_toughness
                .as_ref()
                .unwrap()
                .toughness
                .base_value(),
            1
        );

        // Should have an activated ability
        assert!(
            def.abilities
                .iter()
                .any(|a| matches!(a.kind, AbilityKind::Activated(_)))
        );
    }

    #[test]
    fn test_hex_parasite_ability_has_x_in_cost() {
        let def = hex_parasite();

        // Find the activated ability and verify X in cost
        let mut found_x = false;
        for ability in &def.abilities {
            if let AbilityKind::Activated(activated) = &ability.kind {
                for cost in activated.mana_cost.costs() {
                    if let Some(mana_cost) = cost.mana_cost_ref() {
                        assert!(
                            mana_cost.has_x(),
                            "Hex Parasite's ability should have X in mana cost"
                        );
                        found_x = true;
                    }
                }
            }
        }
        assert!(
            found_x,
            "Should have found an activated ability with X in cost"
        );
    }

    #[test]
    fn test_replay_hex_parasite_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Hex Parasite
                "0", // Tap Forest (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Hex Parasite"])
                .p1_battlefield(vec!["Forest"]),
        );

        // Hex Parasite should be on the battlefield
        assert!(
            game.battlefield_has("Hex Parasite"),
            "Hex Parasite should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let parasite_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Hex Parasite" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(parasite_id) = parasite_id {
            assert_eq!(
                game.calculated_power(parasite_id),
                Some(1),
                "Should have 1 power"
            );
            assert_eq!(
                game.calculated_toughness(parasite_id),
                Some(1),
                "Should have 1 toughness"
            );
        } else {
            panic!("Could not find Hex Parasite on battlefield");
        }
    }
}
