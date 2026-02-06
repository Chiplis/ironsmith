//! Fireball card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Fireball - {X}{R}
/// Sorcery
/// This spell costs {1} more to cast for each target beyond the first.
/// Fireball deals X damage divided evenly, rounded down, among any number of
/// target creatures and/or players.
///
/// For simplicity in this implementation, we'll treat it as dealing X damage
/// to a single target (player only for the test case).
pub fn fireball() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Fireball")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "This spell costs {1} more to cast for each target beyond the first.\nFireball deals X damage divided evenly, rounded down, among any number of target creatures and/or players.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fireball() {
        let def = fireball();
        assert_eq!(def.name(), "Fireball");
        assert!(def.is_spell());
        // Mana value of X{R} is 1 (X counts as 0 except on stack)
        assert_eq!(def.card.mana_value(), 1);
        assert!(def.spell_effect.is_some());
    }

    // =========================================================================
    // Replay Tests
    // =========================================================================

    #[test]
    fn test_replay_fireball_deals_damage_to_player() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        // Cast Fireball with X=3 targeting opponent
        // Cost is {X}{R} = {3}{R} = 4 total mana
        // NOTE: Player targets are ordered: [Alice (0), Bob (1)]
        let game = run_replay_test(
            vec![
                "1", // Cast Fireball
                "3", // Choose X=3
                "1", // Target Bob (index 1, opponent)
                "0", // Tap Mountain for {R}
                "0", // Tap Mountain for mana (for X=1)
                "0", // Tap Mountain for mana (for X=2)
                "0", // Tap Mountain for mana (for X=3) (spell resolves)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Fireball"])
                .p1_battlefield(vec!["Mountain", "Mountain", "Mountain", "Mountain"]),
        );

        let bob = PlayerId::from_index(1);

        // Bob should have taken 3 damage (20 - 3 = 17 life)
        assert_eq!(
            game.life_total(bob),
            17,
            "Bob should have 17 life after Fireball for 3"
        );
    }

    #[test]
    fn test_replay_fireball_x_equals_1() {
        use crate::ids::PlayerId;
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        // Cast Fireball with X=1 targeting opponent (costs {1}{R} = 2 mana)
        // NOTE: Player targets are ordered: [Alice (0), Bob (1)]
        let game = run_replay_test(
            vec![
                "1", // Cast Fireball
                "1", // Choose X=1
                "1", // Target Bob (index 1, opponent)
                "0", // Tap Mountain for {R}
                "0", // Tap Mountain for generic mana (spell resolves)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Fireball"])
                .p1_battlefield(vec!["Mountain", "Mountain"]),
        );

        let bob = PlayerId::from_index(1);

        // Bob should have taken 1 damage (20 - 1 = 19 life)
        assert_eq!(
            game.life_total(bob),
            19,
            "Bob should have 19 life after Fireball for 1"
        );
    }
}
