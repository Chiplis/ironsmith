//! Dauthi Slayer card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Dauthi Slayer - {B}{B}
/// Creature — Dauthi Soldier (2/2)
/// Shadow
/// Dauthi Slayer attacks each combat if able.
pub fn dauthi_slayer() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Dauthi Slayer")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dauthi, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Shadow\nDauthi Slayer attacks each combat if able.")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbilityId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_dauthi_slayer() {
        let def = dauthi_slayer();
        assert_eq!(def.name(), "Dauthi Slayer");
        // Should have shadow
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::Shadow
            } else {
                false
            }
        }));
        // Should have "attacks each combat if able"
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::MustAttack
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_replay_dauthi_slayer_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Dauthi Slayer
                "0", // Tap first Swamp
                "0", // Tap second Swamp
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Dauthi Slayer"])
                .p1_battlefield(vec!["Swamp", "Swamp"]),
        );

        let alice = PlayerId::from_index(0);

        assert!(
            game.battlefield_has("Dauthi Slayer"),
            "Dauthi Slayer should be on battlefield after casting"
        );

        let creature_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Dauthi Slayer" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(creature_id) = creature_id {
            assert_eq!(
                game.calculated_power(creature_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(creature_id),
                Some(2),
                "Should have 2 toughness"
            );
        } else {
            panic!("Could not find Dauthi Slayer on battlefield");
        }
    }
}
