//! Selfless Spirit card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

#[cfg(test)]
use crate::effect::Effect;
#[cfg(test)]
use crate::effect::Until;
#[cfg(test)]
use crate::executor::ExecutionContext;
#[cfg(test)]
use crate::static_abilities::StaticAbility;
#[cfg(test)]
use crate::target::ObjectFilter;

/// Selfless Spirit - {1}{W}
/// Creature — Spirit Cleric
/// 2/1
/// Flying
/// Sacrifice Selfless Spirit: Creatures you control gain indestructible until end of turn.
pub fn selfless_spirit() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Selfless Spirit")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "Flying\n\
             Sacrifice Selfless Spirit: Creatures you control gain indestructible until end of turn.",
        )
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ability::ActivationTiming;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_selfless_spirit(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let def = selfless_spirit();
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_selfless_spirit_basic_properties() {
        let def = selfless_spirit();
        assert_eq!(def.name(), "Selfless Spirit");
        assert!(def.card.is_creature());
        assert_eq!(def.card.mana_value(), 2);
    }

    #[test]
    fn test_selfless_spirit_subtypes() {
        let def = selfless_spirit();
        assert!(def.card.subtypes.contains(&Subtype::Spirit));
        assert!(def.card.subtypes.contains(&Subtype::Cleric));
    }

    #[test]
    fn test_selfless_spirit_power_toughness() {
        let def = selfless_spirit();
        assert_eq!(def.card.power_toughness, Some(PowerToughness::fixed(2, 1)));
    }

    #[test]
    fn test_selfless_spirit_is_white() {
        let def = selfless_spirit();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_selfless_spirit_has_two_abilities() {
        let def = selfless_spirit();
        // Flying + Sacrifice ability
        assert_eq!(def.abilities.len(), 2);
    }

    // ========================================
    // Flying Ability Tests
    // ========================================

    #[test]
    fn test_selfless_spirit_has_flying() {
        let def = selfless_spirit();

        let flying_ability = def.abilities.iter().find(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.has_flying()
            } else {
                false
            }
        });
        assert!(flying_ability.is_some(), "Should have flying");
    }

    // ========================================
    // Activated Ability Structure Tests
    // ========================================

    #[test]
    fn test_selfless_spirit_has_sacrifice_ability() {
        let def = selfless_spirit();

        let sac_ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)));
        assert!(sac_ability.is_some(), "Should have an activated ability");

        if let AbilityKind::Activated(activated) = &sac_ability.unwrap().kind {
            // Verify sacrifice is in cost_effects (not TotalCost) so "dies" triggers fire
            assert!(
                !activated.mana_cost.costs().is_empty(),
                "Should have cost_effects for sacrifice"
            );
            let debug_str = format!("{:?}", &activated.mana_cost.costs()[0]);
            assert!(
                debug_str.contains("SacrificeTargetEffect"),
                "cost_effects should contain sacrifice self"
            );

            // Verify no mana cost
            assert!(
                activated.mana_cost.mana_cost().is_none(),
                "Should have no mana cost"
            );

            // Verify instant speed (AnyTime means can be activated at instant speed)
            assert_eq!(
                activated.timing,
                ActivationTiming::AnyTime,
                "Should be instant speed (AnyTime)"
            );

            // Verify no targets
            assert!(activated.choices.is_empty(), "Should have no targets");
        }
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_selfless_spirit_effect_gives_indestructible() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create some creatures for Alice
        let creature1 = create_creature(&mut game, "Soldier", alice);
        let creature2 = create_creature(&mut game, "Knight", alice);
        let spirit = create_selfless_spirit(&mut game, alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(spirit, alice);
        let effect = Effect::grant_abilities_all(
            ObjectFilter::creature().you_control(),
            vec![StaticAbility::indestructible()],
            Until::EndOfTurn,
        );
        let _result = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Check that creatures have indestructible
        {
            let chars = game
                .calculated_characteristics(creature1)
                .expect("Should calculate characteristics");
            assert!(
                chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Creature 1 should have indestructible"
            );
        }
        {
            let chars = game
                .calculated_characteristics(creature2)
                .expect("Should calculate characteristics");
            assert!(
                chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Creature 2 should have indestructible"
            );
        }
    }

    #[test]
    fn test_selfless_spirit_effect_only_affects_your_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create creatures for Alice
        let alice_creature = create_creature(&mut game, "Soldier", alice);
        let spirit = create_selfless_spirit(&mut game, alice);

        // Create a creature for Bob
        let bob_creature = create_creature(&mut game, "Goblin", bob);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(spirit, alice);
        let effect = Effect::grant_abilities_all(
            ObjectFilter::creature().you_control(),
            vec![StaticAbility::indestructible()],
            Until::EndOfTurn,
        );
        let _ = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Alice's creature should have indestructible
        {
            let chars = game
                .calculated_characteristics(alice_creature)
                .expect("Should calculate characteristics");
            assert!(
                chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Alice's creature should have indestructible"
            );
        }

        // Bob's creature should NOT have indestructible
        {
            let chars = game
                .calculated_characteristics(bob_creature)
                .expect("Should calculate characteristics");
            assert!(
                !chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Bob's creature should NOT have indestructible"
            );
        }
    }

    #[test]
    fn test_selfless_spirit_effect_no_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spirit = create_selfless_spirit(&mut game, alice);

        // Remove spirit from battlefield to simulate sacrifice
        if let Some(obj) = game.object_mut(spirit) {
            obj.zone = Zone::Graveyard;
        }
        game.battlefield.retain(|&id| id != spirit);

        // Execute the effect (after spirit is sacrificed, no creatures left)
        let mut ctx = ExecutionContext::new_default(spirit, alice);
        let effect = Effect::grant_abilities_all(
            ObjectFilter::creature().you_control(),
            vec![StaticAbility::indestructible()],
            Until::EndOfTurn,
        );
        let _result = effect.0.execute(&mut game, &mut ctx).unwrap();

        assert!(game.battlefield.is_empty(), "No creatures should remain");
    }

    #[test]
    fn test_selfless_spirit_creatures_entering_after_do_not_gain_indestructible() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create a creature and the spirit
        let creature_before = create_creature(&mut game, "Soldier", alice);
        let spirit = create_selfless_spirit(&mut game, alice);

        // Execute the effect (simulating sacrifice)
        let mut ctx = ExecutionContext::new_default(spirit, alice);
        let effect = Effect::grant_abilities_all(
            ObjectFilter::creature().you_control(),
            vec![StaticAbility::indestructible()],
            Until::EndOfTurn,
        );
        let _ = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Verify the creature that existed before has indestructible
        {
            let chars = game
                .calculated_characteristics(creature_before)
                .expect("Should calculate characteristics");
            assert!(
                chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Creature that existed when ability resolved should have indestructible"
            );
        }

        // Now create a NEW creature AFTER the ability has resolved
        let creature_after = create_creature(&mut game, "Knight", alice);

        // The new creature should NOT have indestructible
        {
            let chars = game
                .calculated_characteristics(creature_after)
                .expect("Should calculate characteristics");
            assert!(
                !chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Creature entering AFTER ability resolved should NOT have indestructible"
            );
        }

        // Double-check the original creature still has indestructible
        {
            let chars = game
                .calculated_characteristics(creature_before)
                .expect("Should calculate characteristics");
            assert!(
                chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Original creature should still have indestructible"
            );
        }
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_selfless_spirit_oracle_text() {
        let def = selfless_spirit();
        assert!(def.card.oracle_text.contains("Flying"));
        assert!(def.card.oracle_text.contains("Sacrifice"));
        assert!(def.card.oracle_text.contains("indestructible"));
        assert!(def.card.oracle_text.contains("until end of turn"));
    }

    // ========================================
    // Not a Mana Ability Tests
    // ========================================

    #[test]
    fn test_selfless_spirit_abilities_not_mana_abilities() {
        let def = selfless_spirit();
        for ability in &def.abilities {
            if matches!(ability.kind, AbilityKind::Activated(_)) {
                assert!(
                    !ability.is_mana_ability(),
                    "Sacrifice ability should not be a mana ability"
                );
            }
        }
    }

    // ========================================
    // Functional Zone Tests
    // ========================================

    #[test]
    fn test_selfless_spirit_abilities_functional_on_battlefield() {
        let def = selfless_spirit();
        for ability in &def.abilities {
            assert!(
                ability.functional_zones.contains(&Zone::Battlefield),
                "Abilities should be functional on battlefield"
            );
        }
    }

    // ========================================
    // Replay Tests
    // ========================================

    /// Tests casting Selfless Spirit.
    ///
    /// Selfless Spirit: {1}{W} creature 2/1
    /// Flying
    /// Sacrifice Selfless Spirit: Creatures you control gain indestructible until end of turn.
    #[test]
    fn test_replay_selfless_spirit_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Selfless Spirit
                "0", // Tap first Plains
                "0", // Tap second Plains (auto-passes handle resolution)
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Selfless Spirit"])
                .p1_battlefield(vec!["Plains", "Plains"]),
        );

        // Selfless Spirit should be on the battlefield
        assert!(
            game.battlefield_has("Selfless Spirit"),
            "Selfless Spirit should be on battlefield after casting"
        );
    }

    /// Tests Selfless Spirit sacrifice ability granting indestructible.
    #[test]
    fn test_replay_selfless_spirit_sacrifice() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                // Selfless Spirit is on battlefield with Grizzly Bears
                // Actions: 0=pass, 1=activate sacrifice ability
                "1", // Activate Selfless Spirit sacrifice ability (auto-passes handle resolution)
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Selfless Spirit", "Grizzly Bears"]),
        );

        let alice = PlayerId::from_index(0);

        // Selfless Spirit should be in graveyard (sacrificed)
        let alice_player = game.player(alice).unwrap();
        let spirit_in_gy = alice_player.graveyard.iter().any(|&id| {
            game.object(id)
                .map(|o| o.name == "Selfless Spirit")
                .unwrap_or(false)
        });
        assert!(
            spirit_in_gy,
            "Selfless Spirit should be in graveyard after sacrifice"
        );

        // Grizzly Bears should still be on battlefield
        assert!(
            game.battlefield_has("Grizzly Bears"),
            "Grizzly Bears should still be on battlefield"
        );

        // Grizzly Bears should have indestructible (from Selfless Spirit's ability)
        let bears_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Grizzly Bears" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(bears_id) = bears_id {
            let chars = game
                .calculated_characteristics(bears_id)
                .expect("Should calculate characteristics");
            assert!(
                chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Grizzly Bears should have indestructible after Selfless Spirit sacrifice"
            );
        } else {
            panic!("Could not find Grizzly Bears on battlefield");
        }
    }
}
