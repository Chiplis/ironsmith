//! Selfless Savior card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Selfless Savior - {W}
/// Creature — Dog
/// 1/1
/// Sacrifice Selfless Savior: Another target creature you control gains indestructible until end of turn.
pub fn selfless_savior() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Selfless Savior")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dog])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Sacrifice Selfless Savior: Another target creature you control gains indestructible until end of turn.",
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
    use crate::effect::EffectResult;
    use crate::executor::execute_effect;
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::static_abilities::StaticAbility;
    use crate::target::ChooseSpec;
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

    fn create_selfless_savior(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let def = selfless_savior();
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_selfless_savior_basic_properties() {
        let def = selfless_savior();
        assert_eq!(def.name(), "Selfless Savior");
        assert!(def.card.is_creature());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_selfless_savior_subtypes() {
        let def = selfless_savior();
        assert!(def.card.subtypes.contains(&Subtype::Dog));
    }

    #[test]
    fn test_selfless_savior_power_toughness() {
        let def = selfless_savior();
        assert_eq!(def.card.power_toughness, Some(PowerToughness::fixed(1, 1)));
    }

    #[test]
    fn test_selfless_savior_is_white() {
        let def = selfless_savior();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_selfless_savior_has_one_ability() {
        let def = selfless_savior();
        assert_eq!(def.abilities.len(), 1);
    }

    // ========================================
    // Activated Ability Structure Tests
    // ========================================

    #[test]
    fn test_selfless_savior_has_sacrifice_ability() {
        let def = selfless_savior();

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

            // Verify instant speed
            assert_eq!(
                activated.timing,
                ActivationTiming::AnyTime,
                "Should be instant speed (AnyTime)"
            );

            // Verify targets another creature you control
            assert_eq!(activated.choices.len(), 1);
            println!("{:#?}", activated.choices);
            if let ChooseSpec::Target(inner) = &activated.choices[0] {
                if let ChooseSpec::Object(filter) = inner.as_ref() {
                    assert!(filter.card_types.contains(&CardType::Creature));
                    assert!(matches!(
                        filter.controller,
                        Some(crate::target::PlayerFilter::You)
                    ));
                    assert!(filter.other, "Should target 'another' creature (not self)");
                } else {
                    panic!("Target should be an object");
                }
            } else {
                panic!("Target should be a targeted object");
            }
        }
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_selfless_savior_effect_gives_indestructible() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create creatures
        let _savior = create_selfless_savior(&mut game, alice);
        let soldier = create_creature(&mut game, "Soldier", alice);

        // Execute the effect with the soldier as the target
        let source_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.targets = vec![ResolvedTarget::Object(soldier)];

        let def = selfless_savior();
        let ability = def
            .abilities
            .iter()
            .find_map(|a| {
                if let AbilityKind::Activated(activated) = &a.kind {
                    activated.effects.first()
                } else {
                    None
                }
            })
            .expect("Should have activated ability effect");
        let result = execute_effect(&mut game, ability, &mut ctx).unwrap();

        assert!(
            matches!(
                result.result,
                EffectResult::Resolved | EffectResult::Count(1)
            ),
            "Expected effect to resolve successfully"
        );

        // The soldier should now have indestructible
        let chars = game
            .calculated_characteristics(soldier)
            .expect("Should calculate characteristics");

        assert!(
            chars
                .static_abilities
                .contains(&StaticAbility::indestructible()),
            "Soldier should have indestructible"
        );
    }

    #[test]
    fn test_selfless_savior_effect_fails_without_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let _savior = create_selfless_savior(&mut game, alice);

        // Execute without a target
        let source_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.targets = vec![]; // No target

        let def = selfless_savior();
        let ability = def
            .abilities
            .iter()
            .find_map(|a| {
                if let AbilityKind::Activated(activated) = &a.kind {
                    activated.effects.first()
                } else {
                    None
                }
            })
            .expect("Should have activated ability effect");
        let result = execute_effect(&mut game, ability, &mut ctx);

        assert!(result.is_err(), "Should fail without a target");
    }

    #[test]
    fn test_selfless_savior_only_affects_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create multiple creatures
        let _savior = create_selfless_savior(&mut game, alice);
        let soldier = create_creature(&mut game, "Soldier", alice);
        let knight = create_creature(&mut game, "Knight", alice);

        // Execute the effect targeting only the soldier
        let source_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.targets = vec![ResolvedTarget::Object(soldier)];

        let def = selfless_savior();
        let ability = def
            .abilities
            .iter()
            .find_map(|a| {
                if let AbilityKind::Activated(activated) = &a.kind {
                    activated.effects.first()
                } else {
                    None
                }
            })
            .expect("Should have activated ability effect");
        let _ = execute_effect(&mut game, ability, &mut ctx).unwrap();

        // The soldier should have indestructible
        {
            let chars = game
                .calculated_characteristics(soldier)
                .expect("Should calculate characteristics");
            assert!(
                chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Soldier should have indestructible"
            );
        }

        // The knight should NOT have indestructible
        {
            let chars = game
                .calculated_characteristics(knight)
                .expect("Should calculate characteristics");
            assert!(
                !chars
                    .static_abilities
                    .contains(&StaticAbility::indestructible()),
                "Knight should NOT have indestructible"
            );
        }
    }

    #[test]
    fn test_selfless_savior_targets_another_creature() {
        let def = selfless_savior();

        if let Some(ability) = def.abilities.first() {
            if let AbilityKind::Activated(activated) = &ability.kind {
                if let ChooseSpec::Object(filter) = &activated.choices[0] {
                    // The filter should have the "other" flag set
                    assert!(
                        filter.other,
                        "Selfless Savior should only target 'another' creature, not itself"
                    );
                }
            }
        }
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_selfless_savior_oracle_text() {
        let def = selfless_savior();
        assert!(def.card.oracle_text.contains("Sacrifice"));
        assert!(
            def.card
                .oracle_text
                .contains("Another target creature you control")
        );
        assert!(def.card.oracle_text.contains("indestructible"));
        assert!(def.card.oracle_text.contains("until end of turn"));
    }

    // ========================================
    // Not a Mana Ability Test
    // ========================================

    #[test]
    fn test_selfless_savior_ability_not_mana_ability() {
        let def = selfless_savior();
        for ability in &def.abilities {
            assert!(
                !ability.is_mana_ability(),
                "Sacrifice ability should not be a mana ability"
            );
        }
    }

    // ========================================
    // Functional Zone Tests
    // ========================================

    #[test]
    fn test_selfless_savior_ability_functional_on_battlefield() {
        let def = selfless_savior();
        for ability in &def.abilities {
            assert!(
                ability.functional_zones.contains(&Zone::Battlefield),
                "Ability should be functional on battlefield"
            );
        }
    }

    /// Tests casting Selfless Savior.
    ///
    /// Selfless Savior: {W} creature 1/1 (Dog)
    /// Sacrifice: Another target creature you control gains indestructible until end of turn.
    #[test]
    fn test_replay_selfless_savior_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Selfless Savior
                "0", // Tap Plains for mana
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Selfless Savior"])
                .p1_battlefield(vec!["Plains"]),
        );

        let alice = PlayerId::from_index(0);

        // Selfless Savior should be on the battlefield
        assert!(
            game.battlefield_has("Selfless Savior"),
            "Selfless Savior should be on battlefield after casting"
        );

        // Verify P/T
        let creature_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Selfless Savior" && obj.controller == alice)
                .unwrap_or(false)
        });

        if let Some(creature_id) = creature_id {
            assert_eq!(
                game.calculated_power(creature_id),
                Some(1),
                "Should have 1 power"
            );
            assert_eq!(
                game.calculated_toughness(creature_id),
                Some(1),
                "Should have 1 toughness"
            );
        } else {
            panic!("Could not find Selfless Savior on battlefield");
        }
    }
}
