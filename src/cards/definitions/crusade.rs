//! Card definition for Crusade.

use crate::ability::Ability;
use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::color::ColorSet;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;
use crate::types::CardType;

/// Creates the Crusade card definition.
///
/// Crusade {W}{W}
/// Enchantment
/// White creatures get +1/+1.
///
/// Crusade applies in Layer 7c as a P/T modification effect.
/// Note: The original Crusade affects ALL white creatures, not just your own.
pub fn crusade() -> CardDefinition {
    // Create a filter for all white creatures (not just yours)
    let white_creatures = ObjectFilter::creature().with_colors(ColorSet::WHITE);

    CardDefinitionBuilder::new(CardId::new(), "Crusade")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::anthem(
            white_creatures,
            1,
            1,
        )))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_crusade() {
        let card = crusade();
        assert_eq!(card.card.name, "Crusade");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 2); // WW = 2
        assert!(card.card.card_types.contains(&CardType::Enchantment));

        // Should have one ability: Anthem
        assert_eq!(card.abilities.len(), 1);

        let ability = &card.abilities[0];
        if let AbilityKind::Static(s) = &ability.kind {
            assert_eq!(s.id(), crate::static_abilities::StaticAbilityId::Anthem);
        } else {
            panic!("Expected Anthem static ability");
        }
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Crusade and verifying the anthem effect on white creatures.
    ///
    /// Crusade: {W}{W} enchantment
    /// White creatures get +1/+1.
    ///
    /// Scenario: Cast Crusade with Savannah Lions on battlefield.
    /// Savannah Lions is 2/1 white creature, should become 3/2.
    #[test]
    fn test_replay_crusade_anthem_effect() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Crusade
                "0", // Tap Plains 1 for {W}
                "0", // Tap Plains 2 for {W} (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Crusade"])
                .p1_battlefield(vec!["Plains", "Plains", "Savannah Lions"]),
        );

        let alice = PlayerId::from_index(0);

        // Crusade should be on the battlefield
        assert!(
            game.battlefield_has("Crusade"),
            "Crusade should be on battlefield after casting"
        );

        // Find Savannah Lions and verify it's now 3/2 (base 2/1 + 1/1 from Crusade)
        let lions_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Savannah Lions" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(lions_id) = lions_id {
            assert_eq!(
                game.calculated_power(lions_id),
                Some(3),
                "Savannah Lions should have 3 power (2 + 1)"
            );
            assert_eq!(
                game.calculated_toughness(lions_id),
                Some(2),
                "Savannah Lions should have 2 toughness (1 + 1)"
            );
        } else {
            panic!("Could not find Savannah Lions on battlefield");
        }
    }

    /// Tests that Crusade does not affect non-white creatures.
    ///
    /// Scenario: Cast Crusade with both white and non-white creatures.
    /// Only white creatures should get the buff.
    #[test]
    fn test_replay_crusade_only_affects_white_creatures() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Crusade
                "0", // Tap Plains 1 for {W}
                "0", // Tap Plains 2 for {W} (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Crusade"])
                .p1_battlefield(vec!["Plains", "Plains", "Savannah Lions", "Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);

        // Crusade should be on the battlefield
        assert!(
            game.battlefield_has("Crusade"),
            "Crusade should be on battlefield after casting"
        );

        // Savannah Lions (white) should be buffed to 3/2
        let lions_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Savannah Lions" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(lions_id) = lions_id {
            assert_eq!(
                game.calculated_power(lions_id),
                Some(3),
                "Savannah Lions should have 3 power"
            );
            assert_eq!(
                game.calculated_toughness(lions_id),
                Some(2),
                "Savannah Lions should have 2 toughness"
            );
        } else {
            panic!("Could not find Savannah Lions on battlefield");
        }

        // Grizzly Bears (green, not white) should remain 2/2
        let bears_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Grizzly Bears" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(bears_id) = bears_id {
            assert_eq!(
                game.calculated_power(bears_id),
                Some(2),
                "Grizzly Bears should still have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(bears_id),
                Some(2),
                "Grizzly Bears should still have 2 toughness"
            );
        } else {
            panic!("Could not find Grizzly Bears on battlefield");
        }
    }
}
