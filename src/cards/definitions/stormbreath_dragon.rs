//! Stormbreath Dragon card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Stormbreath Dragon - {3}{R}{R}
/// Creature — Dragon (4/4)
/// Flying, haste, protection from white
/// {5}{R}{R}: Monstrosity 3.
/// When Stormbreath Dragon becomes monstrous, it deals damage to each opponent
/// equal to the number of cards in that player's hand.
pub fn stormbreath_dragon() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Stormbreath Dragon")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Flying, haste, protection from white\n{5}{R}{R}: Monstrosity 3.\nWhen Stormbreath Dragon becomes monstrous, it deals damage to each opponent equal to the number of cards in that player's hand.",
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
    fn test_stormbreath_dragon() {
        let def = stormbreath_dragon();
        assert_eq!(def.name(), "Stormbreath Dragon");
        // 3 static (flying, haste, protection) + 1 activated (monstrosity) + 1 triggered (becomes monstrous)
        assert_eq!(def.abilities.len(), 5);

        let has_flying = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_flying()
            } else {
                false
            }
        });
        let has_haste = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_haste()
            } else {
                false
            }
        });
        let has_activated = def
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Activated(_)));
        let has_triggered = def
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Triggered(_)));

        assert!(has_flying);
        assert!(has_haste);
        assert!(has_activated);
        assert!(has_triggered);
    }

    /// Tests casting Stormbreath Dragon (expensive creature with haste).
    ///
    /// Stormbreath Dragon: {3}{R}{R} creature 4/4
    /// Flying, haste, protection from white
    #[test]
    fn test_replay_stormbreath_dragon_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Stormbreath Dragon
                "0", // Tap Mountain 1
                "0", // Tap Mountain 2
                "0", // Tap Mountain 3
                "0", // Tap Mountain 4
                "0", // Tap Mountain 5 (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Stormbreath Dragon"])
                .p1_battlefield(vec![
                    "Mountain", "Mountain", "Mountain", "Mountain", "Mountain",
                ]),
        );

        // Stormbreath Dragon should be on the battlefield
        assert!(
            game.battlefield_has("Stormbreath Dragon"),
            "Stormbreath Dragon should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let dragon_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Stormbreath Dragon" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(dragon_id) = dragon_id {
            assert_eq!(
                game.calculated_power(dragon_id),
                Some(4),
                "Stormbreath Dragon should have 4 power"
            );
            assert_eq!(
                game.calculated_toughness(dragon_id),
                Some(4),
                "Stormbreath Dragon should have 4 toughness"
            );
        } else {
            panic!("Could not find Stormbreath Dragon on battlefield");
        }
    }
}
