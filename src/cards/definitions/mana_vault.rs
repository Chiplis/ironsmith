//! Mana Vault card definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;

/// Mana Vault - {1}
/// Artifact
/// Mana Vault doesn't untap during your untap step.
/// At the beginning of your upkeep, you may pay {4}. If you do, untap Mana Vault.
/// At the beginning of your draw step, if Mana Vault is tapped, it deals 1 damage to you.
/// {T}: Add {C}{C}{C}.
///
/// NOTE: The upkeep and draw step triggered abilities require infrastructure
/// that is not fully implemented. For now, we implement the core functionality:
/// - Static ability: Doesn't untap during your untap step
/// - Mana ability: {T}: Add {C}{C}{C}
pub fn mana_vault() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mana Vault")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Mana Vault doesn't untap during your untap step.\n\
            At the beginning of your upkeep, you may pay {4}. If you do, untap Mana Vault.\n\
            At the beginning of your draw step, if Mana Vault is tapped, it deals 1 damage to you.\n\
            {T}: Add {C}{C}{C}."
        )
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Zone;
    use crate::ability::AbilityKind;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_mana_vault_basic_properties() {
        let def = mana_vault();
        assert_eq!(def.name(), "Mana Vault");
        assert!(def.card.card_types.contains(&CardType::Artifact));
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_mana_vault_has_two_abilities() {
        let def = mana_vault();
        // Static ability + mana ability
        assert_eq!(def.abilities.len(), 2);
    }

    // ========================================
    // Static Ability Tests
    // ========================================

    #[test]
    fn test_mana_vault_does_not_untap() {
        use crate::static_abilities::StaticAbilityId;

        let def = mana_vault();
        let ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Static(_)))
            .expect("Expected static ability");

        if let AbilityKind::Static(static_ab) = &ability.kind {
            assert_eq!(static_ab.id(), StaticAbilityId::DoesntUntap);
        } else {
            panic!("Expected static ability");
        }
    }

    // ========================================
    // Mana Ability Tests
    // ========================================

    #[test]
    fn test_mana_vault_produces_three_colorless() {
        let def = mana_vault();
        let ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Mana(_)))
            .expect("Expected mana ability");

        assert!(ability.is_mana_ability());
        if let AbilityKind::Mana(mana_ability) = &ability.kind {
            assert_eq!(
                mana_ability.mana,
                vec![
                    ManaSymbol::Colorless,
                    ManaSymbol::Colorless,
                    ManaSymbol::Colorless
                ]
            );
            assert!(mana_ability.has_tap_cost());
        } else {
            panic!("Expected mana ability");
        }
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_mana_vault_on_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let def = mana_vault();
        let vault_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        assert!(game.battlefield.contains(&vault_id));

        let obj = game.object(vault_id).unwrap();
        assert_eq!(obj.abilities.len(), 2);
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests casting Mana Vault and tapping for mana.
    #[test]
    fn test_replay_mana_vault_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Tap Sol Ring for mana (need mana first before we can cast)
                "1", // Cast Mana Vault (now we have mana to cast it)
                     // Mana Vault resolves and enters battlefield (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Mana Vault"])
                .p1_battlefield(vec!["Sol Ring"]),
        );

        assert!(
            game.battlefield_has("Mana Vault"),
            "Mana Vault should be on battlefield after casting"
        );
    }

    /// Tests Mana Vault's mana ability.
    #[test]
    fn test_replay_mana_vault_tap_for_mana() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Tap Mana Vault for colorless mana (mana ability is index 1 after static)
                "",  // Pass priority
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Mana Vault"]),
        );

        let alice = PlayerId::from_index(0);
        let player = game.player(alice).unwrap();
        assert_eq!(
            player.mana_pool.colorless, 3,
            "Should have 3 colorless mana from Mana Vault"
        );
    }
}
