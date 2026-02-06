//! Card definition for Frogmite.

use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Creates the Frogmite card definition.
///
/// Frogmite {4}
/// Artifact Creature — Frog
/// 2/2
/// Affinity for artifacts (This spell costs {1} less to cast for each artifact you control.)
pub fn frogmite() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Frogmite")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Frog])
        .power_toughness(PowerToughness::fixed(2, 2))
        .affinity_for_artifacts()
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbilityId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_frogmite() {
        let card = frogmite();
        assert_eq!(card.card.name, "Frogmite");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 4);
        assert!(card.card.card_types.contains(&CardType::Artifact));
        assert!(card.card.card_types.contains(&CardType::Creature));
        assert!(card.card.subtypes.contains(&Subtype::Frog));
        let pt = card.card.power_toughness.as_ref().unwrap();
        assert!(matches!(pt.power, crate::card::PtValue::Fixed(2)));
        assert!(matches!(pt.toughness, crate::card::PtValue::Fixed(2)));

        // Check for Affinity for artifacts ability
        let has_affinity = card.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::AffinityForArtifacts
            } else {
                false
            }
        });
        assert!(has_affinity, "Frogmite should have affinity for artifacts");
    }

    #[test]
    fn test_replay_frogmite_casting_with_affinity() {
        let game = run_replay_test(
            vec![
                "1", // Cast Frogmite (costs {2} with two artifacts on battlefield)
                "0", // Tap Forest
                "0", // Tap Forest (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Frogmite"])
                .p1_battlefield(vec!["Forest", "Forest", "Sol Ring", "Sol Ring"]),
        );

        // Frogmite should be on the battlefield
        assert!(
            game.battlefield_has("Frogmite"),
            "Frogmite should be on battlefield after casting with affinity"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let frogmite_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Frogmite" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(frogmite_id) = frogmite_id {
            assert_eq!(
                game.calculated_power(frogmite_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(frogmite_id),
                Some(2),
                "Should have 2 toughness"
            );
        } else {
            panic!("Could not find Frogmite on battlefield");
        }
    }

    /// Tests casting Frogmite for free with 4+ artifacts.
    #[test]
    fn test_replay_frogmite_casting_free() {
        let game = run_replay_test(
            vec![
                "1", // Cast Frogmite (costs {0} with four artifacts on battlefield)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Frogmite"])
                .p1_battlefield(vec!["Sol Ring", "Sol Ring", "Sol Ring", "Sol Ring"]),
        );

        // Frogmite should be on the battlefield
        assert!(
            game.battlefield_has("Frogmite"),
            "Frogmite should be on battlefield after casting for free"
        );
    }
}
