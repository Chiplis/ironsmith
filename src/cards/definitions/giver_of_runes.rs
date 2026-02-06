//! Giver of Runes card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Giver of Runes - {W}
/// Creature — Kor Cleric
/// 1/2
/// {T}: Another target creature you control gains protection from colorless or from the color of your choice until end of turn.
pub fn giver_of_runes() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Giver of Runes")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Kor, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(1, 2))
        .parse_text("{T}: Another target creature you control gains protection from colorless or from the color of your choice until end of turn.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;
    use crate::ability::{AbilityKind, ActivationTiming};
    use crate::card::CardBuilder;
    use crate::decision::AutoPassDecisionMaker;
    use crate::effect::EffectResult;
    use crate::executor::execute_effect;
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
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

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_giver_of_runes_basic_properties() {
        let def = giver_of_runes();
        assert_eq!(def.name(), "Giver of Runes");
        assert!(def.card.is_creature());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_giver_of_runes_subtypes() {
        let def = giver_of_runes();
        assert!(def.card.subtypes.contains(&Subtype::Kor));
        assert!(def.card.subtypes.contains(&Subtype::Cleric));
    }

    #[test]
    fn test_giver_of_runes_power_toughness() {
        let def = giver_of_runes();
        assert_eq!(def.card.power_toughness, Some(PowerToughness::fixed(1, 2)));
    }

    #[test]
    fn test_giver_of_runes_is_white() {
        let def = giver_of_runes();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_giver_of_runes_has_one_ability() {
        let def = giver_of_runes();
        assert_eq!(def.abilities.len(), 1);
    }

    // ========================================
    // Activated Ability Structure Tests
    // ========================================

    #[test]
    fn test_giver_of_runes_has_tap_ability() {
        let def = giver_of_runes();

        let tap_ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)));
        assert!(tap_ability.is_some(), "Should have an activated ability");

        if let AbilityKind::Activated(activated) = &tap_ability.unwrap().kind {
            // Verify cost is tap (via cost_effects)
            assert!(activated.has_tap_cost(), "Should have tap cost");

            // Verify no mana cost
            assert!(
                activated.mana_cost.mana_cost().is_none(),
                "Should have no mana cost"
            );

            // Verify instant speed (AnyTime)
            assert_eq!(
                activated.timing,
                ActivationTiming::AnyTime,
                "Should be instant speed (AnyTime)"
            );

            // Verify targets another creature you control
            assert_eq!(activated.choices.len(), 1);
            let filter = match &activated.choices[0] {
                ChooseSpec::Target(inner) => match inner.as_ref() {
                    ChooseSpec::Object(filter) => filter,
                    _ => panic!("Target should be an object"),
                },
                ChooseSpec::Object(filter) => filter,
                _ => panic!("Target should be an object"),
            };
            assert!(filter.card_types.contains(&CardType::Creature));
            assert!(matches!(
                filter.controller,
                Some(crate::target::PlayerFilter::You)
            ));
            assert!(filter.other, "Should target 'another' creature (not self)");
        }
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_giver_of_runes_effect_grants_protection() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create creatures
        let _giver =
            game.create_object_from_definition(&giver_of_runes(), alice, Zone::Battlefield);
        let soldier = create_creature(&mut game, "Soldier", alice);

        let def = giver_of_runes();
        let ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)))
            .expect("Should have activated ability");
        let effect = if let AbilityKind::Activated(act) = &ability.kind {
            act.effects
                .first()
                .expect("Activated ability should have an effect")
        } else {
            panic!("Expected activated ability");
        };

        let source_id = game.new_object_id();
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        ctx.targets = vec![ResolvedTarget::Object(soldier)];

        let result = execute_effect(&mut game, effect, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Resolved);

        // The soldier should now have protection
        // (Without a decision maker, it defaults to colorless)
        let chars = game
            .calculated_characteristics(soldier)
            .expect("Should calculate characteristics");

        // Check that the soldier has protection
        let has_protection = chars.static_abilities.iter().any(|a| a.has_protection());
        assert!(has_protection, "Soldier should have protection");
    }

    #[test]
    fn test_giver_of_runes_effect_fails_without_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let _giver =
            game.create_object_from_definition(&giver_of_runes(), alice, Zone::Battlefield);

        let def = giver_of_runes();
        let ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Activated(_)))
            .expect("Should have activated ability");
        let effect = if let AbilityKind::Activated(act) = &ability.kind {
            act.effects
                .first()
                .expect("Activated ability should have an effect")
        } else {
            panic!("Expected activated ability");
        };

        let source_id = game.new_object_id();
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        ctx.targets = vec![]; // No target
        let result = execute_effect(&mut game, effect, &mut ctx);

        assert!(result.is_err(), "Should fail without a target");
    }

    #[test]
    fn test_giver_of_runes_targets_another_creature() {
        let def = giver_of_runes();

        if let Some(ability) = def.abilities.first() {
            if let AbilityKind::Activated(activated) = &ability.kind {
                if let ChooseSpec::Object(filter) = &activated.choices[0] {
                    // The filter should have the "other" flag set
                    assert!(
                        filter.other,
                        "Giver of Runes should only target 'another' creature, not itself"
                    );
                }
            }
        }
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_giver_of_runes_oracle_text() {
        let def = giver_of_runes();
        assert!(def.card.oracle_text.contains("{T}"));
        assert!(
            def.card
                .oracle_text
                .contains("Another target creature you control")
        );
        assert!(def.card.oracle_text.contains("protection"));
        assert!(def.card.oracle_text.contains("colorless"));
        assert!(def.card.oracle_text.contains("color of your choice"));
        assert!(def.card.oracle_text.contains("until end of turn"));
    }

    // ========================================
    // Not a Mana Ability Test
    // ========================================

    #[test]
    fn test_giver_of_runes_ability_not_mana_ability() {
        let def = giver_of_runes();
        for ability in &def.abilities {
            assert!(
                !ability.is_mana_ability(),
                "Protection ability should not be a mana ability"
            );
        }
    }

    // ========================================
    // Functional Zone Tests
    // ========================================

    #[test]
    fn test_giver_of_runes_ability_functional_on_battlefield() {
        let def = giver_of_runes();
        for ability in &def.abilities {
            assert!(
                ability.functional_zones.contains(&Zone::Battlefield),
                "Ability should be functional on battlefield"
            );
        }
    }

    /// Tests casting Giver of Runes.
    ///
    /// Giver of Runes: {W} creature 1/2
    /// {T}: Another target creature you control gains protection from colorless or color until end of turn.
    #[test]
    fn test_replay_giver_of_runes_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Giver of Runes
                "0", // Tap Plains for mana
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Giver of Runes"])
                .p1_battlefield(vec!["Plains"]),
        );

        let alice = PlayerId::from_index(0);

        // Giver of Runes should be on the battlefield
        assert!(
            game.battlefield_has("Giver of Runes"),
            "Giver of Runes should be on battlefield after casting"
        );

        // Verify P/T (notably 1/2, not 1/1)
        let creature_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Giver of Runes" && obj.controller == alice)
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
                Some(2),
                "Should have 2 toughness"
            );
        } else {
            panic!("Could not find Giver of Runes on battlefield");
        }
    }
}
