//! Godless Shrine card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::types::{CardType, Subtype};

/// Godless Shrine
/// Land — Plains Swamp
/// ({T}: Add {W} or {B}.)
/// As Godless Shrine enters the battlefield, you may pay 2 life.
/// If you don't, it enters the battlefield tapped.
pub fn godless_shrine() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Godless Shrine")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains, Subtype::Swamp])
        .parse_text("({T}: Add {W} or {B}.)\nAs Godless Shrine enters the battlefield, you may pay 2 life. If you don't, it enters the battlefield tapped.")
        .expect("Card text should be supported")
}

#[cfg(all(test, ironsmith_runtime_parser_tests))]
mod tests {
    use super::*;
    use crate::Zone;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_godless_shrine_basic_properties() {
        let def = godless_shrine();
        assert_eq!(def.name(), "Godless Shrine");
        assert!(def.card.is_land());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_godless_shrine_is_not_basic() {
        let def = godless_shrine();
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_godless_shrine_has_plains_swamp_types() {
        let def = godless_shrine();
        assert!(def.card.has_subtype(Subtype::Plains));
        assert!(def.card.has_subtype(Subtype::Swamp));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_godless_shrine_has_only_its_authored_entry_ability() {
        let def = godless_shrine();
        assert_eq!(def.abilities.len(), 1);
        assert!(!def.abilities.iter().any(|ability| ability.is_mana_ability()));
    }

    // ========================================
    // Mana Ability Tests
    // ========================================

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_first_ability_is_static_pay_life() {
        let def = godless_shrine();
        let ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Static(_)));
        assert!(ability.is_some(), "Should have a static ability");
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_current_types_supply_white_and_black_mana() {
        let mut game = setup_game();
        let id = game.create_object_from_definition(&godless_shrine(), PlayerId::from_index(0), Zone::Battlefield);
        let abilities = game.current_abilities(id).unwrap();
        assert_eq!(abilities.iter().filter(|ability| ability.is_mana_ability()).count(), 2);
        for subtype in [Subtype::Plains, Subtype::Swamp] {
            assert!(abilities.contains(&crate::ability::Ability::basic_land_mana(subtype).unwrap()));
        }
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_godless_shrine_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = godless_shrine();
        let shrine_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(game.battlefield.contains(&shrine_id));

        let obj = game.object(shrine_id).unwrap();
        // Only the authored entry ability is stored; subtype mana is derived.
        assert_eq!(obj.abilities.len(), 1);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_godless_shrine_oracle_text() {
        let def = godless_shrine();
        assert!(
            crate::runtime_display::debug_compiled_lines(&def)
                .join("\n")
                .contains("pay 2 life")
        );
        assert!(
            crate::runtime_display::debug_compiled_lines(&def)
                .join("\n")
                .contains("enters tapped")
        );
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests Godless Shrine tapping for white mana.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_godless_shrine_tap_for_white() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Activate Godless Shrine's mana ability
                "W", // Choose white
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Godless Shrine"]),
        );

        let alice = PlayerId::from_index(0);
        let player = game.player(alice).unwrap();
        assert_eq!(player.mana_pool.white, 1, "Should have 1 white mana");
    }

    /// Tests Godless Shrine tapping for black mana.
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_godless_shrine_tap_for_black() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Activate Godless Shrine's mana ability
                "B", // Choose black
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Godless Shrine"]),
        );

        let alice = PlayerId::from_index(0);
        let player = game.player(alice).unwrap();
        assert_eq!(player.mana_pool.black, 1, "Should have 1 black mana");
    }

    /// Tests playing Godless Shrine from hand and paying 2 life (enters untapped).
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_godless_shrine_play_pay_life() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Godless Shrine from hand
                "1", // Pay 2 life (yes)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_hand(vec!["Godless Shrine"]),
        );

        let alice = PlayerId::from_index(0);

        // Check that the shrine is on the battlefield and untapped
        let shrine = game
            .battlefield
            .iter()
            .find(|&&id| {
                game.object(id)
                    .map(|o| o.name == "Godless Shrine")
                    .unwrap_or(false)
            })
            .copied()
            .expect("Godless Shrine should be on battlefield");
        assert!(
            !game.is_tapped(shrine),
            "Godless Shrine should be untapped after paying life"
        );

        // Check that 2 life was paid
        let player = game.player(alice).unwrap();
        assert_eq!(player.life, 18, "Should have lost 2 life (20 - 2 = 18)");
    }

    /// Tests playing Godless Shrine from hand and declining to pay life (enters tapped).
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_replay_godless_shrine_play_decline_pay() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Play Godless Shrine from hand
                "",  // Decline to pay life (empty = no)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_hand(vec!["Godless Shrine"]),
        );

        let alice = PlayerId::from_index(0);

        // Check that the shrine is on the battlefield and tapped
        let shrine = game
            .battlefield
            .iter()
            .find(|&&id| {
                game.object(id)
                    .map(|o| o.name == "Godless Shrine")
                    .unwrap_or(false)
            })
            .copied()
            .expect("Godless Shrine should be on battlefield");
        assert!(
            game.is_tapped(shrine),
            "Godless Shrine should be tapped after declining to pay life"
        );

        // Check that no life was paid
        let player = game.player(alice).unwrap();
        assert_eq!(player.life, 20, "Should still have 20 life");
    }
}
