//! Darksteel Colossus card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Darksteel Colossus - {11}
/// Artifact Creature — Golem (11/11)
/// Trample, indestructible
/// If Darksteel Colossus would be put into a graveyard from anywhere,
/// reveal it and shuffle it into its owner's library instead.
pub fn darksteel_colossus() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Darksteel Colossus")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(11)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(11, 11))
        .parse_text(
            "Trample, indestructible\nIf Darksteel Colossus would be put into a graveyard from anywhere, reveal it and shuffle it into its owner's library instead.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_darksteel_colossus() {
        let def = darksteel_colossus();
        assert_eq!(def.name(), "Darksteel Colossus");
        assert_eq!(def.card.mana_value(), 11);

        let has_trample = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_trample()
            } else {
                false
            }
        });
        let has_indestructible = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_indestructible()
            } else {
                false
            }
        });

        assert!(has_trample);
        assert!(has_indestructible);
    }

    #[test]
    fn test_replay_darksteel_colossus_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Darksteel Colossus
                "0", "0", "0", "0", "0", "0", "0", "0", "0", "0",
                "0", // Tap 11 lands (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Darksteel Colossus"])
                .p1_battlefield(vec![
                    "Forest", "Forest", "Forest", "Forest", "Forest", "Forest", "Forest", "Forest",
                    "Forest", "Forest", "Forest",
                ]),
        );

        // Darksteel Colossus should be on the battlefield
        assert!(
            game.battlefield_has("Darksteel Colossus"),
            "Darksteel Colossus should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let colossus_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Darksteel Colossus" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(colossus_id) = colossus_id {
            assert_eq!(
                game.calculated_power(colossus_id),
                Some(11),
                "Should have 11 power"
            );
            assert_eq!(
                game.calculated_toughness(colossus_id),
                Some(11),
                "Should have 11 toughness"
            );
        } else {
            panic!("Could not find Darksteel Colossus on battlefield");
        }
    }
}
