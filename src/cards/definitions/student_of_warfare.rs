//! Student of Warfare card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Student of Warfare - {W}
/// Creature — Human Knight (1/1)
/// Level up {W} ({W}: Put a level counter on this. Level up only as a sorcery.)
/// LEVEL 2-6 (3/3, First strike)
/// LEVEL 7+ (4/4, Double strike)
pub fn student_of_warfare() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Student of Warfare")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Knight])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Level up {W}\n\
             LEVEL 2-6\n\
             3/3\n\
             First strike\n\
             LEVEL 7+\n\
             4/4\n\
             Double strike",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ability::LevelAbility;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_student_of_warfare() {
        let def = student_of_warfare();
        assert_eq!(def.name(), "Student of Warfare");
        assert!(def.is_creature());

        // Check base P/T
        assert_eq!(def.card.power_toughness, Some(PowerToughness::fixed(1, 1)));
    }

    #[test]
    fn test_level_up_ability() {
        let def = student_of_warfare();

        // Should have an activated ability for level-up
        let has_level_up = def
            .abilities
            .iter()
            .any(|a| matches!(&a.kind, AbilityKind::Activated(_)));
        assert!(has_level_up, "Should have level-up activated ability");
    }

    #[test]
    fn test_level_abilities() {
        use crate::static_abilities::StaticAbility;

        let def = student_of_warfare();

        // Should have level abilities static ability
        let level_ability = def.abilities.iter().find_map(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.level_abilities()
            } else {
                None
            }
        });
        assert!(level_ability.is_some(), "Should have level abilities");

        let levels = level_ability.unwrap();
        assert_eq!(levels.len(), 2);

        // First tier: levels 2-6
        assert_eq!(levels[0].min_level, 2);
        assert_eq!(levels[0].max_level, Some(6));
        assert_eq!(levels[0].power_toughness, Some((3, 3)));
        assert!(levels[0].abilities.contains(&StaticAbility::first_strike()));

        // Second tier: level 7+
        assert_eq!(levels[1].min_level, 7);
        assert_eq!(levels[1].max_level, None);
        assert_eq!(levels[1].power_toughness, Some((4, 4)));
        assert!(
            levels[1]
                .abilities
                .contains(&StaticAbility::double_strike())
        );
    }

    #[test]
    fn test_level_ability_applies_at_level() {
        // Test the helper method
        let tier1 = LevelAbility::new(2, Some(6));
        assert!(!tier1.applies_at_level(0));
        assert!(!tier1.applies_at_level(1));
        assert!(tier1.applies_at_level(2));
        assert!(tier1.applies_at_level(4));
        assert!(tier1.applies_at_level(6));
        assert!(!tier1.applies_at_level(7));

        let tier2 = LevelAbility::new(7, None);
        assert!(!tier2.applies_at_level(6));
        assert!(tier2.applies_at_level(7));
        assert!(tier2.applies_at_level(100));
    }

    #[test]
    fn test_replay_student_of_warfare_casting() {
        let game = run_replay_test(
            vec![
                "1", // Cast Student of Warfare
                "0", // Tap Plains (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Student of Warfare"])
                .p1_battlefield(vec!["Plains"]),
        );

        // Student of Warfare should be on the battlefield
        assert!(
            game.battlefield_has("Student of Warfare"),
            "Student of Warfare should be on battlefield after casting"
        );

        // Verify P/T at level 0
        let alice = PlayerId::from_index(0);
        let student_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Student of Warfare" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(student_id) = student_id {
            assert_eq!(
                game.calculated_power(student_id),
                Some(1),
                "Should have 1 power at level 0"
            );
            assert_eq!(
                game.calculated_toughness(student_id),
                Some(1),
                "Should have 1 toughness at level 0"
            );
        } else {
            panic!("Could not find Student of Warfare on battlefield");
        }
    }
}
