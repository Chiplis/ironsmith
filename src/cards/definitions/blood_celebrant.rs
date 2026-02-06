//! Blood Celebrant card definition.

use crate::ability::{Ability, AbilityKind, ManaAbility};
use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::effects::AddManaOfAnyColorEffect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates the Blood Celebrant card definition.
///
/// Blood Celebrant
/// {B}
/// Creature — Human Cleric
/// 1/1
/// {B}, Pay 1 life: Add one mana of any color.
pub fn blood_celebrant() -> CardDefinition {
    // The effect: Add one mana of any color
    let add_mana_effect = Effect::new(AddManaOfAnyColorEffect::you(1));

    let mut def = CardDefinitionBuilder::new(CardId::new(), "Blood Celebrant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("{B}, Pay 1 life: Add one mana of any color.")
        .expect("Card text should be supported");

    def.abilities.push(Ability {
        kind: AbilityKind::Mana(ManaAbility {
            mana_cost: crate::ability::merge_cost_effects(
                TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Black]])),
                vec![Effect::pay_life(1)],
            ),
            mana: vec![],
            effects: Some(vec![add_mana_effect]),
            activation_condition: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{B}, Pay 1 life: Add one mana of any color.".to_string()),
    });

    def
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::ids::PlayerId;

    #[test]
    fn test_blood_celebrant_basic_properties() {
        let def = blood_celebrant();
        assert_eq!(def.name(), "Blood Celebrant");
        assert!(def.card.is_creature());
        assert_eq!(def.card.mana_value(), 1);
        assert!(def.card.has_subtype(Subtype::Human));
        assert!(def.card.has_subtype(Subtype::Cleric));
    }

    #[test]
    fn test_blood_celebrant_power_toughness() {
        let def = blood_celebrant();
        if let Some(pt) = &def.card.power_toughness {
            assert_eq!(pt.power.base_value(), 1);
            assert_eq!(pt.toughness.base_value(), 1);
        } else {
            panic!("Blood Celebrant should have power/toughness");
        }
    }

    #[test]
    fn test_blood_celebrant_has_mana_ability() {
        let def = blood_celebrant();
        assert!(
            !def.abilities.is_empty(),
            "Blood Celebrant should have abilities"
        );

        let has_mana_ability = def
            .abilities
            .iter()
            .any(|a| matches!(a.kind, AbilityKind::Mana(_)));
        assert!(
            has_mana_ability,
            "Blood Celebrant should have a mana ability"
        );
    }

    #[test]
    fn test_blood_celebrant_mana_ability_cost() {
        let def = blood_celebrant();
        let mana_ability = def
            .abilities
            .iter()
            .find(|a| matches!(a.kind, AbilityKind::Mana(_)))
            .expect("Should have mana ability");

        if let AbilityKind::Mana(ma) = &mana_ability.kind {
            // Should not require tap
            assert!(
                !ma.has_tap_cost(),
                "Blood Celebrant's ability should not require tap"
            );

            // Should have mana cost (in TotalCost)
            let has_mana_cost = ma.mana_cost.mana_cost().is_some();
            assert!(
                has_mana_cost,
                "Blood Celebrant's ability should have a mana cost"
            );

            // Should have life cost (in cost_effects)
            assert_eq!(
                ma.life_cost_amount(),
                Some(1),
                "Blood Celebrant's ability should have a life cost of 1"
            );
        }
    }

    // Replay test for Blood Celebrant mana ability
    use crate::tests::integration_tests::{ReplayTestConfig, run_replay_test};

    #[test]
    fn test_replay_blood_celebrant_mana_ability() {
        // Start with Blood Celebrant on battlefield + 4 Swamps
        // Activate the ability 4 times to produce W, U, R, G (all colors except Black)
        //
        // Action sequence per activation:
        // 1. Tap Swamp (mana ability) - adds {B} to pool
        // 2. Activate Blood Celebrant (uses {B} from pool, pays 1 life)
        // 3. Choose color (W, U, R, or G using character input)
        //
        // Action indices:
        // 0 = Pass Priority
        // 1 = Blood Celebrant ability ({B}, Pay 1 life: Add one mana of any color)
        // 2+ = Tap Swamp (indices shift as Swamps get tapped)
        let game = run_replay_test(
            vec![
                "2", "1", "W", // Tap Swamp 1, activate BC, choose White
                "2", "1", "U", // Tap Swamp 2, activate BC, choose Blue
                "2", "1", "R", // Tap Swamp 3, activate BC, choose Red
                "2", "1", "G", // Tap Swamp 4, activate BC, choose Green
            ],
            ReplayTestConfig::new().p1_battlefield(vec![
                "Blood Celebrant",
                "Swamp",
                "Swamp",
                "Swamp",
                "Swamp",
            ]),
        );

        let alice = PlayerId::from_index(0);

        // Player should have paid 4 life (started at 20)
        assert_eq!(game.life_total(alice), 16, "Player should have paid 4 life");

        // Player should have W, U, R, G in mana pool
        let player = game.player(alice).expect("Alice should exist");
        assert!(player.mana_pool.white >= 1, "Should have white mana");
        assert!(player.mana_pool.blue >= 1, "Should have blue mana");
        assert!(player.mana_pool.red >= 1, "Should have red mana");
        assert!(player.mana_pool.green >= 1, "Should have green mana");
    }

    #[test]
    fn test_replay_blood_celebrant_pay_mana_flow() {
        // Test the PayMana flow: select Blood Celebrant first (with empty pool),
        // then pay for it via the PayMana menu.
        //
        // Setup: Blood Celebrant + Swamp + Mountain on battlefield
        // Mountain should NOT appear in PayMana options since it produces {R}, not {B}
        //
        // Action sequence:
        // 1. Select Blood Celebrant ability (action 1) - triggers PayMana decision
        // 2. In PayMana menu, select "Tap Swamp" (index 0 - the only option that helps)
        // 3. Now have {B} in pool, ability executes automatically (mana cost paid)
        // 4. Choose color for the mana production
        let game = run_replay_test(
            vec![
                "1", // Activate Blood Celebrant (with empty mana pool)
                "0", // In PayMana: tap Swamp (only option, Mountain filtered out)
                "W", // Choose White mana
            ],
            ReplayTestConfig::new().p1_battlefield(vec!["Blood Celebrant", "Swamp", "Mountain"]),
        );

        let alice = PlayerId::from_index(0);

        // Player should have paid 1 life
        assert_eq!(game.life_total(alice), 19, "Player should have paid 1 life");

        // Player should have W in mana pool
        let player = game.player(alice).expect("Alice should exist");
        assert!(player.mana_pool.white >= 1, "Should have white mana");
    }
}
