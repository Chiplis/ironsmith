//! Vault of Champions card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::CardType;

/// Vault of Champions
/// Land
/// This land enters tapped unless you have two or more opponents.
/// {T}: Add {W} or {B}.
pub fn vault_of_champions() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Vault of Champions")
        .card_types(vec![CardType::Land])
        // ETB tapped condition handled elsewhere (in etb_replacement.rs for multiplayer check)
        // For now, implement the mana ability
        // Mana ability: {T}: Add {W} or {B}.
        .parse_text(
            "This land enters tapped unless you have two or more opponents.\n{T}: Add {W} or {B}.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::mana::ManaSymbol;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_vault_of_champions_basic_properties() {
        let def = vault_of_champions();
        assert_eq!(def.name(), "Vault of Champions");
        assert!(def.card.is_land());
        assert!(!def.card.is_creature());
        assert_eq!(def.card.mana_value(), 0);
    }

    #[test]
    fn test_vault_of_champions_is_not_basic() {
        let def = vault_of_champions();
        assert!(!def.card.has_supertype(crate::types::Supertype::Basic));
    }

    #[test]
    fn test_vault_of_champions_has_two_abilities() {
        let def = vault_of_champions();
        assert_eq!(def.abilities.len(), 2);
    }

    // ========================================
    // Mana Ability Tests
    // ========================================

    #[test]
    fn test_first_ability_produces_white_mana() {
        let def = vault_of_champions();
        let ability = &def.abilities[0];

        assert!(ability.is_mana_ability());
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::White]);
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_second_ability_produces_black_mana() {
        let def = vault_of_champions();
        let ability = &def.abilities[1];

        assert!(ability.is_mana_ability());
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(mana_ability.mana, vec![ManaSymbol::Black]);
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    #[test]
    fn test_both_abilities_are_mana_abilities() {
        let def = vault_of_champions();

        for ability in &def.abilities {
            assert!(ability.is_mana_ability());
        }
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_vault_of_champions_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = vault_of_champions();
        let vault_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Verify it's on the battlefield
        assert!(game.battlefield.contains(&vault_id));

        let obj = game.object(vault_id).unwrap();
        assert_eq!(obj.abilities.len(), 2);
    }

    #[test]
    fn test_vault_of_champions_oracle_text() {
        let def = vault_of_champions();
        assert!(def.card.oracle_text.contains("enters tapped"));
        assert!(def.card.oracle_text.contains("two or more opponents"));
        assert!(def.card.oracle_text.contains("Add {W} or {B}"));
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests Vault of Champions tapping for white mana.
    #[test]
    fn test_replay_vault_of_champions_white() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Tap Vault of Champions for white mana (first mana ability)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Vault of Champions"]),
        );

        let alice = PlayerId::from_index(0);

        // Player should have 1 white mana in pool
        let player = game.player(alice).unwrap();
        assert_eq!(
            player.mana_pool.white, 1,
            "Should have 1 white mana from Vault of Champions"
        );
    }

    /// Tests Vault of Champions tapping for black mana.
    #[test]
    fn test_replay_vault_of_champions_black() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "2", // Tap Vault of Champions for black mana (second mana ability)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Vault of Champions"]),
        );

        let alice = PlayerId::from_index(0);

        // Player should have 1 black mana in pool
        let player = game.player(alice).unwrap();
        assert_eq!(
            player.mana_pool.black, 1,
            "Should have 1 black mana from Vault of Champions"
        );
    }
}
