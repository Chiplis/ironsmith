//! Zodiac Rooster card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Zodiac Rooster - {1}{G}
/// Creature — Bird (2/1)
/// Horsemanship
/// (This creature can't be blocked except by creatures with horsemanship.)
pub fn zodiac_rooster() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Zodiac Rooster")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bird])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text("Horsemanship")
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
    fn test_zodiac_rooster() {
        let def = zodiac_rooster();
        assert_eq!(def.name(), "Zodiac Rooster");
        assert!(def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.id() == StaticAbilityId::Horsemanship
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_replay_zodiac_rooster_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Zodiac Rooster
                "0", // Tap first Forest
                "0", // Tap second Forest
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Zodiac Rooster"])
                .p1_battlefield(vec!["Forest", "Forest"]),
        );

        let alice = PlayerId::from_index(0);

        assert!(
            game.battlefield_has("Zodiac Rooster"),
            "Zodiac Rooster should be on battlefield after casting"
        );

        let creature_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Zodiac Rooster" && obj.controller == alice)
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
                Some(1),
                "Should have 1 toughness"
            );
        } else {
            panic!("Could not find Zodiac Rooster on battlefield");
        }
    }
}
