//! Butcher Ghoul card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Butcher Ghoul - {1}{B}
/// Creature — Zombie (1/1)
/// Undying (When this creature dies, if it had no +1/+1 counters on it,
/// return it to the battlefield under its owner's control with a +1/+1 counter on it.)
pub fn butcher_ghoul() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Butcher Ghoul")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("Undying")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_butcher_ghoul() {
        let def = butcher_ghoul();
        assert_eq!(def.name(), "Butcher Ghoul");

        // Should be creature
        assert!(def.card.card_types.contains(&CardType::Creature));

        // Should be Zombie
        assert!(def.card.subtypes.contains(&Subtype::Zombie));

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

        // Should have undying (triggered ability)
        // Verify undying trigger exists (now using Trigger struct)
        assert!(def.abilities.iter().any(|a| matches!(
            &a.kind,
            AbilityKind::Triggered(t) if t.trigger.display() == "Undying"
        )));
    }

    #[test]
    fn test_replay_butcher_ghoul_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Butcher Ghoul
                "0", // Tap Swamp 1
                "0", // Tap Swamp 2 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Butcher Ghoul"])
                .p1_battlefield(vec!["Swamp", "Swamp"]),
        );

        // Butcher Ghoul should be on the battlefield
        assert!(
            game.battlefield_has("Butcher Ghoul"),
            "Butcher Ghoul should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let ghoul_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Butcher Ghoul" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(ghoul_id) = ghoul_id {
            assert_eq!(
                game.calculated_power(ghoul_id),
                Some(1),
                "Should have 1 power"
            );
            assert_eq!(
                game.calculated_toughness(ghoul_id),
                Some(1),
                "Should have 1 toughness"
            );
        } else {
            panic!("Could not find Butcher Ghoul on battlefield");
        }
    }
}
