//! Mother of Runes card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Mother of Runes - {W}
/// Creature — Human Cleric
/// 1/1
/// {T}: Target creature you control gains protection from the color of your choice until end of turn.
pub fn mother_of_runes() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mother of Runes")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{T}: Target creature you control gains protection from the color of your choice until end of turn.",
        )
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
    use crate::executor::ExecutionContext;
    use crate::executor::ResolvedTarget;
    use crate::executor::execute_effect;
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

    fn create_mother_of_runes(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let def = mother_of_runes();
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_mother_of_runes_basic_properties() {
        let def = mother_of_runes();
        assert_eq!(def.name(), "Mother of Runes");
        assert!(def.card.is_creature());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_mother_of_runes_subtypes() {
        let def = mother_of_runes();
        assert!(def.card.subtypes.contains(&Subtype::Human));
        assert!(def.card.subtypes.contains(&Subtype::Cleric));
    }

    #[test]
    fn test_mother_of_runes_power_toughness() {
        let def = mother_of_runes();
        assert_eq!(def.card.power_toughness, Some(PowerToughness::fixed(1, 1)));
    }

    #[test]
    fn test_mother_of_runes_is_white() {
        let def = mother_of_runes();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_mother_of_runes_has_one_ability() {
        let def = mother_of_runes();
        assert_eq!(def.abilities.len(), 1);
    }

    // ========================================
    // Activated Ability Structure Tests
    // ========================================

    #[test]
    fn test_mother_of_runes_has_tap_ability() {
        let def = mother_of_runes();

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

            // Verify targets creature you control
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
        }
    }

    // ========================================
    // Effect Execution Tests
    // ========================================

    #[test]
    fn test_mother_of_runes_effect_grants_protection() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create creatures
        let _mother = create_mother_of_runes(&mut game, alice);
        let soldier = create_creature(&mut game, "Soldier", alice);

        let def = mother_of_runes();
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

        // The soldier should now have protection from some color
        // (Without a decision maker, it defaults to white)
        let chars = game
            .calculated_characteristics(soldier)
            .expect("Should calculate characteristics");

        // Check that the soldier has protection (from white by default)
        let has_protection = chars.static_abilities.iter().any(|a| a.has_protection());
        assert!(
            has_protection,
            "Soldier should have protection from a color"
        );
    }

    #[test]
    fn test_mother_of_runes_effect_fails_without_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let _mother = create_mother_of_runes(&mut game, alice);

        let def = mother_of_runes();
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

        // Execute without a target
        let source_id = game.new_object_id();
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        ctx.targets = vec![]; // No target

        let result = execute_effect(&mut game, effect, &mut ctx);

        assert!(result.is_err(), "Should fail without a target");
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_mother_of_runes_oracle_text() {
        let def = mother_of_runes();
        assert!(def.card.oracle_text.contains("{T}"));
        assert!(def.card.oracle_text.contains("Target creature you control"));
        assert!(def.card.oracle_text.contains("protection"));
        assert!(def.card.oracle_text.contains("color of your choice"));
        assert!(def.card.oracle_text.contains("until end of turn"));
    }

    // ========================================
    // Not a Mana Ability Test
    // ========================================

    #[test]
    fn test_mother_of_runes_ability_not_mana_ability() {
        let def = mother_of_runes();
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
    fn test_mother_of_runes_ability_functional_on_battlefield() {
        let def = mother_of_runes();
        for ability in &def.abilities {
            assert!(
                ability.functional_zones.contains(&Zone::Battlefield),
                "Ability should be functional on battlefield"
            );
        }
    }

    /// Tests casting Mother of Runes.
    ///
    /// Mother of Runes: {W} creature 1/1
    /// {T}: Target creature you control gains protection from the color of your choice until end of turn.
    #[test]
    fn test_replay_mother_of_runes_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Mother of Runes
                "0", // Tap Plains for mana
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Mother of Runes"])
                .p1_battlefield(vec!["Plains"]),
        );

        let alice = PlayerId::from_index(0);

        // Mother of Runes should be on the battlefield
        assert!(
            game.battlefield_has("Mother of Runes"),
            "Mother of Runes should be on battlefield after casting"
        );

        // Verify P/T
        let creature_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Mother of Runes" && obj.controller == alice)
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
            panic!("Could not find Mother of Runes on battlefield");
        }
    }
}
