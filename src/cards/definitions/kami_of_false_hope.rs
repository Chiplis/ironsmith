//! Kami of False Hope card definition.

use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Kami of False Hope - {W}
/// Creature — Spirit
/// 1/1
/// Sacrifice Kami of False Hope: Prevent all combat damage that would be dealt this turn.
pub fn kami_of_false_hope() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Kami of False Hope")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("Sacrifice Kami of False Hope: Prevent all combat damage that would be dealt this turn.")
        .expect("Card text should be supported")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Effect;
    use crate::Until;
    use crate::Zone;
    use crate::ability::AbilityKind;
    use crate::ability::ActivationTiming;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;

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

    fn create_kami_of_false_hope(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let def = kami_of_false_hope();
        game.create_object_from_definition(&def, owner, Zone::Battlefield)
    }

    // ========================================
    // Basic Property Tests
    // ========================================

    #[test]
    fn test_kami_of_false_hope_basic_properties() {
        let def = kami_of_false_hope();
        assert_eq!(def.name(), "Kami of False Hope");
        assert!(def.card.is_creature());
        assert_eq!(def.card.mana_value(), 1);
    }

    #[test]
    fn test_kami_of_false_hope_subtypes() {
        let def = kami_of_false_hope();
        assert!(def.card.subtypes.contains(&Subtype::Spirit));
    }

    #[test]
    fn test_kami_of_false_hope_power_toughness() {
        let def = kami_of_false_hope();
        assert_eq!(def.card.power_toughness, Some(PowerToughness::fixed(1, 1)));
    }

    #[test]
    fn test_kami_of_false_hope_is_white() {
        let def = kami_of_false_hope();
        assert!(def.card.colors().contains(Color::White));
        assert_eq!(def.card.colors().count(), 1);
    }

    #[test]
    fn test_kami_of_false_hope_has_one_ability() {
        let def = kami_of_false_hope();
        assert_eq!(def.abilities.len(), 1);
    }

    // ========================================
    // Activated Ability Structure Tests
    // ========================================

    #[test]
    fn test_kami_of_false_hope_has_sacrifice_ability() {
        let def = kami_of_false_hope();

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
    fn test_kami_of_false_hope_effect_creates_prevention_shield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create the Kami
        let kami = create_kami_of_false_hope(&mut game, alice);

        // Execute the effect
        let mut ctx = ExecutionContext::new_default(kami, alice);
        let effect = Effect::prevent_all_combat_damage(Until::EndOfTurn);
        let _result = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Verify a prevention shield was added
        assert_eq!(game.prevention_effects.shields().len(), 1);

        // Verify the shield prevents combat damage
        let shield = &game.prevention_effects.shields()[0];
        assert!(
            shield.damage_filter.combat_only,
            "Shield should only prevent combat damage"
        );
        assert!(
            shield.amount_remaining.is_none(),
            "Shield should prevent unlimited damage"
        );
    }

    #[test]
    fn test_kami_of_false_hope_prevents_combat_damage_to_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create and activate the Kami
        let kami = create_kami_of_false_hope(&mut game, alice);
        let mut ctx = ExecutionContext::new_default(kami, alice);
        let effect = Effect::prevent_all_combat_damage(Until::EndOfTurn);
        let _ = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Try to apply combat damage to Bob
        use crate::color::ColorSet;
        let remaining = game.prevention_effects.apply_prevention_to_player(
            bob,
            5,                       // 5 combat damage
            true,                    // is combat
            ObjectId::from_raw(999), // source
            &ColorSet::COLORLESS,
            &vec![CardType::Creature],
            true, // can be prevented
        );

        // All combat damage should be prevented
        assert_eq!(remaining, 0, "All combat damage should be prevented");
    }

    #[test]
    fn test_kami_of_false_hope_does_not_prevent_noncombat_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create and activate the Kami
        let kami = create_kami_of_false_hope(&mut game, alice);
        let mut ctx = ExecutionContext::new_default(kami, alice);
        let effect = Effect::prevent_all_combat_damage(Until::EndOfTurn);
        let _ = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Try to apply noncombat damage to Bob
        use crate::color::ColorSet;
        let remaining = game.prevention_effects.apply_prevention_to_player(
            bob,
            5,                       // 5 noncombat damage
            false,                   // NOT combat
            ObjectId::from_raw(999), // source
            &ColorSet::RED,
            &vec![CardType::Instant],
            true, // can be prevented
        );

        // Noncombat damage should NOT be prevented
        assert_eq!(remaining, 5, "Noncombat damage should NOT be prevented");
    }

    #[test]
    fn test_kami_of_false_hope_prevents_combat_damage_to_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create a creature for Bob
        let creature = create_creature(&mut game, "Knight", bob);

        // Create and activate the Kami
        let kami = create_kami_of_false_hope(&mut game, alice);
        let mut ctx = ExecutionContext::new_default(kami, alice);
        let effect = Effect::prevent_all_combat_damage(Until::EndOfTurn);
        let _ = effect.0.execute(&mut game, &mut ctx).unwrap();

        // Try to apply combat damage to the creature
        use crate::color::ColorSet;
        let remaining = game.prevention_effects.apply_prevention_to_permanent(
            creature,
            bob,
            4,                       // 4 combat damage
            true,                    // is combat
            ObjectId::from_raw(999), // source
            &ColorSet::COLORLESS,
            &vec![CardType::Creature],
            true, // can be prevented
        );

        // All combat damage should be prevented
        assert_eq!(
            remaining, 0,
            "All combat damage to creatures should be prevented"
        );
    }

    // ========================================
    // Oracle Text Tests
    // ========================================

    #[test]
    fn test_kami_of_false_hope_oracle_text() {
        let def = kami_of_false_hope();
        assert!(def.card.oracle_text.contains("Sacrifice"));
        assert!(def.card.oracle_text.contains("Prevent all combat damage"));
        assert!(def.card.oracle_text.contains("this turn"));
    }

    // ========================================
    // Not a Mana Ability Test
    // ========================================

    #[test]
    fn test_kami_of_false_hope_ability_not_mana_ability() {
        let def = kami_of_false_hope();
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
    fn test_kami_of_false_hope_ability_functional_on_battlefield() {
        let def = kami_of_false_hope();
        for ability in &def.abilities {
            assert!(
                ability.functional_zones.contains(&Zone::Battlefield),
                "Ability should be functional on battlefield"
            );
        }
    }

    /// Tests casting Kami of False Hope.
    ///
    /// Kami of False Hope: {W} creature 1/1 (Spirit)
    /// Sacrifice: Prevent all combat damage that would be dealt this turn.
    #[test]
    fn test_replay_kami_of_false_hope_casting() {
        use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

        let game = run_replay_test(
            vec![
                "1", // Cast Kami of False Hope
                "0", // Tap Plains for mana
            ],
            ReplayTestConfig::new()
                .p1_hand(vec!["Kami of False Hope"])
                .p1_battlefield(vec!["Plains"]),
        );

        let alice = PlayerId::from_index(0);

        // Kami of False Hope should be on the battlefield
        assert!(
            game.battlefield_has("Kami of False Hope"),
            "Kami of False Hope should be on battlefield after casting"
        );

        // Verify P/T
        let creature_id = game.battlefield.iter().copied().find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Kami of False Hope" && obj.controller == alice)
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
            panic!("Could not find Kami of False Hope on battlefield");
        }
    }
}
