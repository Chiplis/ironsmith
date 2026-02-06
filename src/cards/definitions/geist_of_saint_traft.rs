//! Geist of Saint Traft card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype, Supertype};

/// Geist of Saint Traft - {1}{W}{U}
/// Legendary Creature — Spirit Cleric (2/2)
/// Hexproof
/// Whenever Geist of Saint Traft attacks, create a 4/4 white Angel creature token
/// with flying that's tapped and attacking. Exile that token at end of combat.
pub fn geist_of_saint_traft() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Geist of Saint Traft")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(2, 2))
        .hexproof()
        .parse_text(
            "Hexproof\nWhenever Geist of Saint Traft attacks, create a 4/4 white Angel creature \
             token with flying that's tapped and attacking. Exile that token at end of combat.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

    /// Helper to check if a trigger is an "attacks" trigger
    fn is_attacks_trigger(trigger: &crate::triggers::Trigger) -> bool {
        trigger.display().contains("attacks")
    }

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_geist_of_saint_traft_basic_properties() {
        let def = geist_of_saint_traft();
        assert_eq!(def.name(), "Geist of Saint Traft");
        assert!(def.card.is_legendary());
        assert!(def.card.is_creature());
        assert_eq!(def.card.mana_value(), 3);
    }

    #[test]
    fn test_geist_has_hexproof() {
        let def = geist_of_saint_traft();
        let has_hexproof = def.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_hexproof()
            } else {
                false
            }
        });
        assert!(has_hexproof, "Geist should have hexproof");
    }

    #[test]
    fn test_geist_has_attack_trigger() {
        let def = geist_of_saint_traft();
        // Now using Trigger struct - check display contains attacks
        let attack_trigger = def.abilities.iter().find(|a| {
            if let AbilityKind::Triggered(t) = &a.kind {
                is_attacks_trigger(&t.trigger)
            } else {
                false
            }
        });
        assert!(
            attack_trigger.is_some(),
            "Geist should have 'whenever attacks' trigger"
        );
    }

    #[test]
    fn test_geist_subtypes() {
        let def = geist_of_saint_traft();
        assert!(def.card.has_subtype(Subtype::Spirit));
        assert!(def.card.has_subtype(Subtype::Cleric));
    }

    #[test]
    fn test_geist_power_toughness() {
        let def = geist_of_saint_traft();
        let pt = def.card.power_toughness.as_ref().expect("Should have P/T");
        use crate::card::PtValue;
        assert_eq!(pt.power, PtValue::Fixed(2));
        assert_eq!(pt.toughness, PtValue::Fixed(2));
    }

    // ========================================
    // Token Creation Tests
    // ========================================

    #[test]
    fn test_attack_trigger_creates_token() {
        let def = geist_of_saint_traft();

        // Find the attack trigger (now using Trigger struct)
        let trigger = def.abilities.iter().find_map(|a| {
            if let AbilityKind::Triggered(t) = &a.kind {
                if is_attacks_trigger(&t.trigger) {
                    Some(t)
                } else {
                    None
                }
            } else {
                None
            }
        });

        let trigger = trigger.expect("Should have attack trigger");

        // The trigger should have exactly one effect (create token)
        assert_eq!(trigger.effects.len(), 1);

        // Verify the effect is a create token effect
        let effect_debug = format!("{:?}", &trigger.effects[0]);
        assert!(
            effect_debug.contains("CreateTokenEffect"),
            "Effect should create token: {}",
            effect_debug
        );
    }

    #[test]
    fn test_token_creation_execution() {
        // Test that executing the trigger effect creates a token with proper properties
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Geist on battlefield
        let def = geist_of_saint_traft();
        let geist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Get the attack trigger ability from the Geist
        let geist = game.object(geist_id).unwrap();
        let trigger = geist
            .abilities
            .iter()
            .find_map(|a| {
                if let AbilityKind::Triggered(t) = &a.kind {
                    if is_attacks_trigger(&t.trigger) {
                        Some(t.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .expect("Geist should have attack trigger");

        // Execute the trigger's effect
        let mut ctx = ExecutionContext::new_default(geist_id, alice);
        let result = trigger.effects[0].0.execute(&mut game, &mut ctx);

        assert!(result.is_ok(), "Token creation should succeed");

        // Verify an Angel token was created
        let angel_ids: Vec<_> = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|obj| obj.name == "Angel" && obj.controller == alice)
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        assert_eq!(angel_ids.len(), 1, "Should have created 1 Angel token");
        let angel_id = angel_ids[0];

        let angel = game.object(angel_id).unwrap();
        assert!(angel.is_creature());
        assert!(angel.has_subtype(Subtype::Angel));
        assert_eq!(angel.power(), Some(4));
        assert_eq!(angel.toughness(), Some(4));

        // Verify the token is tapped (effect applies tapped())
        assert!(game.is_tapped(angel_id), "Angel should enter tapped");

        // Verify a delayed trigger was registered for end of combat exile
        // (exile_at_end_of_combat() on the effect creates a delayed trigger)
        assert_eq!(
            game.delayed_triggers.len(),
            1,
            "Should have 1 delayed trigger for exile at EOC"
        );
        let delayed = &game.delayed_triggers[0];
        assert!(delayed.trigger.display().contains("end of combat"));
        assert!(delayed.one_shot);
    }

    #[test]
    fn test_angel_token_has_flying() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create Geist on battlefield
        let def = geist_of_saint_traft();
        let geist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Get and execute the attack trigger
        let geist = game.object(geist_id).unwrap();
        let trigger = geist
            .abilities
            .iter()
            .find_map(|a| {
                if let AbilityKind::Triggered(t) = &a.kind {
                    if is_attacks_trigger(&t.trigger) {
                        Some(t.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .expect("Should have trigger");

        let mut ctx = ExecutionContext::new_default(geist_id, alice);
        trigger.effects[0].0.execute(&mut game, &mut ctx).unwrap();

        // Find the Angel token and verify it has Flying
        let angel = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .find(|obj| obj.name == "Angel")
            .expect("Should have Angel token");

        let has_flying = angel.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_flying()
            } else {
                false
            }
        });
        assert!(has_flying, "Angel token should have Flying");
    }

    #[test]
    fn test_angel_enters_attacking() {
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create Geist on battlefield
        let def = geist_of_saint_traft();
        let geist_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        // Set up combat with Geist attacking Bob
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: geist_id,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        // Get and execute the attack trigger
        let geist = game.object(geist_id).unwrap();
        let trigger = geist
            .abilities
            .iter()
            .find_map(|a| {
                if let AbilityKind::Triggered(t) = &a.kind {
                    if is_attacks_trigger(&t.trigger) {
                        Some(t.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .expect("Should have trigger");

        let mut ctx = ExecutionContext::new_default(geist_id, alice);
        trigger.effects[0].0.execute(&mut game, &mut ctx).unwrap();

        // Find the Angel token ID
        let angel_id = game
            .battlefield
            .iter()
            .find(|&&id| game.object(id).is_some_and(|obj| obj.name == "Angel"))
            .copied()
            .expect("Should have Angel token");

        // Verify the token was added to combat attackers
        let combat = game.combat.as_ref().expect("Combat should still be active");
        assert!(
            combat
                .attackers
                .iter()
                .any(|info| info.creature == angel_id),
            "Angel token should be in combat attackers"
        );
        // Token should be attacking the same target as Geist (Bob)
        let angel_attacker = combat
            .attackers
            .iter()
            .find(|info| info.creature == angel_id)
            .expect("Angel should be attacking");
        assert_eq!(
            angel_attacker.target,
            AttackTarget::Player(bob),
            "Angel should attack the same player as Geist"
        );
    }

    /// Tests casting Geist of Saint Traft (legendary creature with hexproof).
    ///
    /// Geist of Saint Traft: {1}{W}{U} legendary creature 2/2
    /// Hexproof
    /// Whenever attacks, creates 4/4 Angel token.
    #[test]
    fn test_replay_geist_of_saint_traft_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        // With the fixed mana allocation algorithm, colored pips are paid first,
        // so 1 Plains + 2 Islands correctly pays {1}{W}{U}:
        // - {W} takes Plains, leaving 2 Islands
        // - {U} takes Island, leaving 1 Island
        // - Generic(1) takes Island, success!
        let game = run_replay_test(
            vec![
                "1", // Cast Geist of Saint Traft
                "0", // Tap land 1 for mana
                "0", // Tap land 2 for mana
                "0", // Tap land 3 for mana (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Geist of Saint Traft"])
                .p1_battlefield(vec!["Plains", "Island", "Island"]),
        );

        // Geist of Saint Traft should be on the battlefield
        assert!(
            game.battlefield_has("Geist of Saint Traft"),
            "Geist of Saint Traft should be on battlefield after casting"
        );

        // Verify P/T
        let alice = PlayerId::from_index(0);
        let geist_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Geist of Saint Traft" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(geist_id) = geist_id {
            assert_eq!(
                game.calculated_power(geist_id),
                Some(2),
                "Should have 2 power"
            );
            assert_eq!(
                game.calculated_toughness(geist_id),
                Some(2),
                "Should have 2 toughness"
            );
        } else {
            panic!("Could not find Geist of Saint Traft on battlefield");
        }
    }
}
