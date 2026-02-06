//! Valley Floodcaller card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Valley Floodcaller - {2}{U}
/// Creature — Otter Wizard (2/2)
/// Flash
/// You may cast noncreature spells as though they had flash.
/// Whenever you cast a noncreature spell, Birds, Frogs, Otters, and Rats you control
/// get +1/+1 until end of turn. Untap them.
pub fn valley_floodcaller() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Valley Floodcaller")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Otter, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Flash\nYou may cast noncreature spells as though they had flash.\nWhenever you cast a noncreature spell, Birds, Frogs, Otters, and Rats you control get +1/+1 until end of turn. Untap them.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;
    use crate::static_abilities::StaticAbilityId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_valley_floodcaller() {
        let def = valley_floodcaller();
        assert_eq!(def.name(), "Valley Floodcaller");

        // Check mana cost is {2}{U}
        assert_eq!(def.card.mana_cost.as_ref().unwrap().mana_value(), 3);

        // Check it's a creature
        assert!(def.card.card_types.contains(&CardType::Creature));

        // Check creature types
        assert!(def.card.subtypes.contains(&Subtype::Otter));
        assert!(def.card.subtypes.contains(&Subtype::Wizard));

        // Check P/T is 2/2
        let pt = def.card.power_toughness.as_ref().unwrap();
        assert_eq!(pt.power.base_value(), 2);
        assert_eq!(pt.toughness.base_value(), 2);

        // Should have 3 abilities: Flash, Grants(flash-to-noncreature), and the triggered ability
        assert_eq!(def.abilities.len(), 3);

        // Check Flash
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::Flash
            } else {
                false
            }
        }));

        // Check Grants (unified grant ability)
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::Grants
            } else {
                false
            }
        }));

        // Check triggered ability (now using Trigger struct)
        assert!(def.abilities.iter().any(|a| matches!(
            &a.kind,
            AbilityKind::Triggered(t) if t.trigger.display().contains("cast")
        )));
    }

    #[test]
    fn test_replay_valley_floodcaller_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Valley Floodcaller
                "0", // Tap Island 1
                "0", // Tap Island 2
                "0", // Tap Island 3 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Valley Floodcaller"])
                .p1_battlefield(vec!["Island", "Island", "Island"]),
        );

        // Valley Floodcaller should be on the battlefield
        assert!(
            game.battlefield_has("Valley Floodcaller"),
            "Valley Floodcaller should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let floodcaller_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Valley Floodcaller" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(floodcaller_id) = floodcaller_id {
            assert_eq!(
                game.calculated_power(floodcaller_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(floodcaller_id),
                Some(2),
                "Should have 2 toughness"
            );
        } else {
            panic!("Could not find Valley Floodcaller on battlefield");
        }
    }
}
