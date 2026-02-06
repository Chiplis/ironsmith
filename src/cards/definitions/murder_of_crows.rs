//! Murder of Crows card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Murder of Crows - {3}{U}{U}
/// Creature — Bird (4/4)
/// Flying
/// Whenever another creature dies, you may draw a card. If you do, discard a card.
pub fn murder_of_crows() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Murder of Crows")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bird])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Flying\nWhenever another creature dies, you may draw a card. If you do, discard a card.",
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
    fn test_murder_of_crows() {
        let def = murder_of_crows();
        assert_eq!(def.name(), "Murder of Crows");
        assert!(def.is_creature());
        assert_eq!(def.abilities.len(), 2); // Flying + trigger
    }

    #[test]
    fn test_murder_of_crows_trigger_structure() {
        let def = murder_of_crows();

        // Find the triggered ability
        let trigger = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Triggered(_)));
        assert!(trigger.is_some());

        if let AbilityKind::Triggered(t) = &trigger.unwrap().kind {
            // Should have 2 effects: WithId(May(draw)) and If(discard)
            assert_eq!(t.effects.len(), 2);

            // First effect: WithIdEffect(MayEffect(draw))
            let debug_str_0 = format!("{:?}", &t.effects[0]);
            assert!(
                debug_str_0.contains("WithIdEffect"),
                "First effect should contain WithIdEffect"
            );
            assert!(
                debug_str_0.contains("MayEffect"),
                "First effect should contain MayEffect"
            );

            // Second effect: IfEffect(discard)
            let debug_str_1 = format!("{:?}", &t.effects[1]);
            assert!(
                debug_str_1.contains("IfEffect"),
                "Second effect should contain IfEffect"
            );
        }
    }

    #[test]
    fn test_replay_murder_of_crows_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Murder of Crows
                "0", // Tap Island 1
                "0", // Tap Island 2
                "0", // Tap Island 3
                "0", // Tap Island 4
                "0", // Tap Island 5 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Murder of Crows"])
                .p1_battlefield(vec!["Island", "Island", "Island", "Island", "Island"]),
        );

        // Murder of Crows should be on the battlefield
        assert!(
            game.battlefield_has("Murder of Crows"),
            "Murder of Crows should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let crows_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Murder of Crows" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(crows_id) = crows_id {
            assert_eq!(
                game.calculated_power(crows_id),
                Some(4),
                "Should have 4 power"
            );
            assert_eq!(
                game.calculated_toughness(crows_id),
                Some(4),
                "Should have 4 toughness"
            );
        } else {
            panic!("Could not find Murder of Crows on battlefield");
        }
    }
}
