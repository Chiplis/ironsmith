//! Llanowar Elves card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Llanowar Elves - {G}
/// Creature — Elf Druid (1/1)
/// {T}: Add {G}.
pub fn llanowar_elves() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Llanowar Elves")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("{T}: Add {G}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};
    use crate::zone::Zone;

    #[test]
    fn test_llanowar_elves_basic_properties() {
        let def = llanowar_elves();
        assert_eq!(def.name(), "Llanowar Elves");
        assert!(def.is_creature());
        assert_eq!(def.card.mana_value(), 1);

        // Should be Elf Druid
        assert!(def.card.subtypes.contains(&Subtype::Elf));
        assert!(def.card.subtypes.contains(&Subtype::Druid));

        // Should be 1/1
        let pt = def.card.power_toughness.as_ref().unwrap();
        assert_eq!(pt.power.base_value(), 1);
        assert_eq!(pt.toughness.base_value(), 1);
    }

    #[test]
    fn test_llanowar_elves_has_mana_ability() {
        let def = llanowar_elves();
        assert!(def.abilities.iter().any(|a| a.is_mana_ability()));
    }

    #[test]
    fn test_llanowar_elves_produces_green_mana() {
        let def = llanowar_elves();

        let mana_ability = def
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        if let AbilityKind::Mana(mana_ab) = &mana_ability.kind {
            // Should produce green mana
            assert_eq!(mana_ab.mana.len(), 1);
            assert_eq!(mana_ab.mana[0], ManaSymbol::Green);
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_llanowar_elves_requires_tap() {
        let def = llanowar_elves();

        let mana_ability = def
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        if let AbilityKind::Mana(mana_ab) = &mana_ability.kind {
            assert!(
                mana_ab.has_tap_cost(),
                "Mana ability should require tapping"
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_llanowar_elves_is_creature_affected_by_summoning_sickness() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        // Create Llanowar Elves on the battlefield
        let def = llanowar_elves();
        let elf_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        let elf = game.object(elf_id).unwrap();

        // Verify it's a creature (and thus subject to summoning sickness rules)
        assert!(elf.is_creature(), "Should be a creature");

        // Creatures without haste can't tap the turn they enter
        // Llanowar Elves doesn't have haste
        let has_haste = def.abilities.iter().any(|a| {
            if let crate::ability::AbilityKind::Static(s) = &a.kind {
                s.has_haste()
            } else {
                false
            }
        });
        assert!(!has_haste, "Llanowar Elves should not have haste");

        // Therefore, summoning sickness will prevent tapping for mana
        // on the turn it enters (this is enforced by the game rules, not the card definition)
    }

    #[test]
    fn test_llanowar_elves_no_mana_cost_on_ability() {
        let def = llanowar_elves();

        let mana_ability = def
            .abilities
            .iter()
            .find(|a| a.is_mana_ability())
            .expect("Should have mana ability");

        if let AbilityKind::Mana(mana_ab) = &mana_ability.kind {
            // Should not require mana payment, just tap
            assert!(
                mana_ab.mana_cost.mana_cost().is_none(),
                "Should not have mana cost"
            );
        } else {
            panic!("Expected mana ability");
        }
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    /// Tests casting Llanowar Elves.
    ///
    /// Llanowar Elves: {G} creature 1/1
    /// {T}: Add {G}.
    #[test]
    fn test_replay_llanowar_elves_casting() {
        let game = run_replay_test(
            vec![
                "1", // Play Forest (index 1, after PassPriority at 0)
                "2", // Tap Forest for mana (mana abilities come after spells)
                "1", // Cast Llanowar Elves (now we have mana)
                "",  // Pass priority
                "",  // Opponent passes (Llanowar Elves resolves)
            ],
            ReplayTestConfig::new().p1_hand(vec!["Forest", "Llanowar Elves"]),
        );

        // Llanowar Elves should be on the battlefield after resolving
        assert!(
            game.battlefield_has("Llanowar Elves"),
            "Llanowar Elves should be on battlefield after casting"
        );

        // Forest should also be on the battlefield (and tapped)
        assert!(
            game.battlefield_has("Forest"),
            "Forest should be on battlefield"
        );
    }

    /// Tests that Llanowar Elves' mana ability works correctly.
    /// Start with Llanowar Elves already on battlefield (no summoning sickness),
    /// tap it for green mana.
    #[test]
    fn test_replay_llanowar_elves_mana_production() {
        let game = run_replay_test(
            vec![
                "1", // Tap Llanowar Elves for mana (index 1, after PassPriority at 0)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Llanowar Elves"]),
        );

        // Llanowar Elves should be on battlefield (and tapped)
        assert!(
            game.battlefield_has("Llanowar Elves"),
            "Llanowar Elves should be on battlefield"
        );

        // Player should have 1 green mana in pool
        let alice = PlayerId::from_index(0);
        let player = game.player(alice).unwrap();
        assert_eq!(
            player.mana_pool.green, 1,
            "Should have 1 green mana from Llanowar Elves"
        );
    }
}
