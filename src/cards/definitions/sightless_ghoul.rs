//! Sightless Ghoul card definition.

use crate::ability::Ability;
use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::static_abilities::StaticAbility;
use crate::types::{CardType, Subtype};

/// Sightless Ghoul - {3}{B}
/// Creature — Zombie Soldier (2/2)
/// Sightless Ghoul can't block.
/// Undying (When this creature dies, if it had no +1/+1 counters on it,
/// return it to the battlefield under its owner's control with a +1/+1 counter on it.)
pub fn sightless_ghoul() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Sightless Ghoul")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(2, 2))
        // Can't block
        .with_ability(
            Ability::static_ability(StaticAbility::cant_block())
                .with_text("Sightless Ghoul can't block."),
        )
        // Undying
        .undying()
        .parse_text("Sightless Ghoul can't block.\nUndying")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn test_sightless_ghoul() {
        let def = sightless_ghoul();
        assert_eq!(def.name(), "Sightless Ghoul");

        // Should be creature
        assert!(def.card.card_types.contains(&CardType::Creature));

        // Should be Zombie Soldier
        assert!(def.card.subtypes.contains(&Subtype::Zombie));
        assert!(def.card.subtypes.contains(&Subtype::Soldier));

        // Should have 2/2 P/T
        assert_eq!(
            def.card
                .power_toughness
                .as_ref()
                .unwrap()
                .power
                .base_value(),
            2
        );
        assert_eq!(
            def.card
                .power_toughness
                .as_ref()
                .unwrap()
                .toughness
                .base_value(),
            2
        );

        // Should have can't block
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::CantBlock
            } else {
                false
            }
        }));

        // Should have undying (triggered ability, now using Trigger struct)
        assert!(def.abilities.iter().any(|a| matches!(
            &a.kind,
            AbilityKind::Triggered(t) if t.trigger.display() == "Undying"
        )));
    }

    /// Tests casting Sightless Ghoul (creature with undying and can't block).
    ///
    /// Sightless Ghoul: {3}{B} creature 2/2
    /// Can't block. Undying.
    #[test]
    fn test_replay_sightless_ghoul_casting() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Sightless Ghoul
                "0", // Tap Swamp 1
                "0", // Tap Swamp 2
                "0", // Tap Swamp 3
                "0", // Tap Swamp 4 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Sightless Ghoul"])
                .p1_battlefield(vec!["Swamp", "Swamp", "Swamp", "Swamp"]),
        );

        // Sightless Ghoul should be on the battlefield
        assert!(
            game.battlefield_has("Sightless Ghoul"),
            "Sightless Ghoul should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let ghoul_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Sightless Ghoul" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(ghoul_id) = ghoul_id {
            assert_eq!(
                game.calculated_power(ghoul_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(ghoul_id),
                Some(2),
                "Should have 2 toughness"
            );
        } else {
            panic!("Could not find Sightless Ghoul on battlefield");
        }
    }
}
