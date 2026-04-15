//! Shattered Sanctum card definition.

use super::CardDefinitionBuilder;
use crate::cards::CardDefinition;
use crate::ids::CardId;
use crate::types::CardType;

/// Shattered Sanctum
/// Land
/// Shattered Sanctum enters the battlefield tapped unless you control two or more other lands.
/// {T}: Add {W} or {B}.
pub fn shattered_sanctum() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Shattered Sanctum")
        .card_types(vec![CardType::Land])
        .parse_text("Shattered Sanctum enters the battlefield tapped unless you control two or more other lands.\n{T}: Add {W} or {B}.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::effects::AddManaOfAnyColorEffect;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_shattered_sanctum_basic_properties() {
        let def = shattered_sanctum();
        assert_eq!(def.name(), "Shattered Sanctum");
        assert!(def.card.is_land());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_shattered_sanctum_is_not_basic() {
        let def = shattered_sanctum();
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[test]
    fn test_shattered_sanctum_has_two_abilities() {
        let def = shattered_sanctum();
        assert_eq!(def.abilities.len(), 2);
    }

    // ========================================
    // Mana Ability Tests
    // ========================================

    #[test]
    fn test_first_ability_offers_white_and_black() {
        let def = shattered_sanctum();
        let mana_abilities: Vec<_> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(mana_ability) if mana_ability.is_mana_ability() => {
                    Some(mana_ability)
                }
                _ => None,
            })
            .collect();
        assert_eq!(mana_abilities.len(), 1);

        let add_any = mana_abilities[0]
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddManaOfAnyColorEffect>())
            .expect("Should use restricted color-choice mana effect");
        let colors = add_any
            .available_colors
            .as_ref()
            .expect("Should expose restricted colors");
        assert_eq!(colors.len(), 2);
        assert!(colors.contains(&crate::color::Color::White));
        assert!(colors.contains(&crate::color::Color::Black));
        assert!(mana_abilities[0].has_tap_cost());
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_shattered_sanctum_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = shattered_sanctum();
        let sanctum_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(game.battlefield.contains(&sanctum_id));

        let obj = game.object(sanctum_id).unwrap();
        assert_eq!(obj.abilities.len(), 2);
    }

    #[test]
    fn test_shattered_sanctum_oracle_text() {
        let def = shattered_sanctum();
        assert!(def.card.oracle_text.contains("two or more other lands"));
        assert!(
            def.card
                .oracle_text
                .contains("enters the battlefield tapped")
        );
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests Shattered Sanctum tapping for white mana.
    #[test]
    fn test_replay_shattered_sanctum_tap_for_white() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Activate Shattered Sanctum's mana ability
                "W", // Choose white
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Shattered Sanctum"]),
        );

        let alice = PlayerId::from_index(0);
        let player = game.player(alice).unwrap();
        assert_eq!(player.mana_pool.white, 1, "Should have 1 white mana");
    }

    /// Tests Shattered Sanctum tapping for black mana.
    #[test]
    fn test_replay_shattered_sanctum_tap_for_black() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Activate Shattered Sanctum's mana ability
                "B", // Choose black
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Shattered Sanctum"]),
        );

        let alice = PlayerId::from_index(0);
        let player = game.player(alice).unwrap();
        assert_eq!(player.mana_pool.black, 1, "Should have 1 black mana");
    }
}
