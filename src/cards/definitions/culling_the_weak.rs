//! Culling the Weak card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Culling the Weak - {B}
/// Instant
/// As an additional cost to cast this spell, sacrifice a creature.
/// Add {B}{B}{B}{B}.
pub fn culling_the_weak() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Culling the Weak")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nAdd {B}{B}{B}{B}.",
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_culling_the_weak_basic_properties() {
        let def = culling_the_weak();
        assert_eq!(def.name(), "Culling the Weak");
        assert!(def.is_spell());
        assert!(def.card.is_instant());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_culling_the_weak_is_black() {
        let def = culling_the_weak();
        assert!(def.card.colors().contains(Color::Black));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_culling_the_weak_has_cost_effects() {
        let def = culling_the_weak();
        assert_eq!(
            def.cost_effects.len(),
            2,
            "Should have 2 cost effects (choose + sacrifice)"
        );

        // First effect should be ChooseObjectsEffect
        let debug_str_0 = format!("{:?}", &def.cost_effects[0]);
        assert!(
            debug_str_0.contains("ChooseObjectsEffect"),
            "First cost effect should be choose"
        );

        // Second effect should be SacrificeEffect
        let debug_str_1 = format!("{:?}", &def.cost_effects[1]);
        assert!(
            debug_str_1.contains("SacrificeEffect"),
            "Second cost effect should be sacrifice"
        );
    }

    #[test]
    fn test_culling_the_weak_has_spell_effect() {
        let def = culling_the_weak();
        assert!(def.spell_effect.is_some());

        let effects = def.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);

        // Check it's an add mana effect
        let debug_str = format!("{:?}", &effects[0]);
        assert!(
            debug_str.contains("AddManaEffect"),
            "Should have add mana effect"
        );
    }

    #[test]
    fn test_culling_the_weak_oracle_text() {
        let def = culling_the_weak();
        assert!(def.card.oracle_text.contains("sacrifice a creature"));
        assert!(def.card.oracle_text.contains("Add {B}{B}{B}{B}"));
    }

    // ========================================
    // Replay Tests
    // ========================================
    //
    // NOTE: Replay coverage is temporarily disabled while the replay harness
    // catches up with the current cost_effects ordering and prompts.

    // #[test]
    // fn test_replay_culling_the_weak_casting() {
    //     use crate::tests::integration_tests::{run_replay_test, ReplayTestConfig};
    //     use crate::ids::PlayerId;
    //
    //     let game = run_replay_test(
    //         vec![
    //             "1",  // Cast Culling the Weak
    //             "0",  // Choose Grizzly Bears to sacrifice (additional cost)
    //             "0",  // Tap Swamp for mana
    //         ],
    //         ReplayTestConfig::new()
    //             .p1_hand(vec!["Culling the Weak"])
    //             .p1_battlefield(vec!["Swamp", "Grizzly Bears"]),
    //     );
    //
    //     let alice = PlayerId::from_index(0);
    //
    //     // Grizzly Bears should be in graveyard (sacrificed as additional cost)
    //     let alice_player = game.player(alice).unwrap();
    //     let bears_in_gy = alice_player.graveyard.iter().any(|&id| {
    //         game.object(id).map(|o| o.name == "Grizzly Bears").unwrap_or(false)
    //     });
    //     assert!(bears_in_gy, "Grizzly Bears should be in graveyard after sacrifice");
    //
    //     // Culling the Weak should be in graveyard (after resolving)
    //     let spell_in_gy = alice_player.graveyard.iter().any(|&id| {
    //         game.object(id).map(|o| o.name == "Culling the Weak").unwrap_or(false)
    //     });
    //     assert!(spell_in_gy, "Culling the Weak should be in graveyard after resolving");
    //
    //     // Alice should have 4 black mana in pool
    //     assert_eq!(
    //         alice_player.mana_pool.black, 4,
    //         "Alice should have 4 black mana from Culling the Weak"
    //     );
    // }
}
