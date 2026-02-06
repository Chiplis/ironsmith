//! Card definition for Force of Will.

use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::color::ColorSet;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

/// Creates the Force of Will card definition.
///
/// Force of Will {3}{U}{U}
/// Instant
/// You may pay 1 life and exile a blue card from your hand rather than pay
/// this spell's mana cost.
/// Counter target spell.
pub fn force_of_will() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Force of Will")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        // Alternative cost: Pay 1 life and exile a blue card from your hand
        // Uses composable effects: choose a blue card, then exile it
        .alternative_cast(AlternativeCastingMethod::alternative_cost(
            "Force of Will",
            None, // No mana cost for the alternative
            vec![
                Effect::pay_life(1),
                // Choose a blue card from hand (other than the source being cast)
                Effect::choose_objects(
                    ObjectFilter::default()
                        .in_zone(Zone::Hand)
                        .you_control()
                        .with_colors(ColorSet::BLUE)
                        .other(), // Exclude Force of Will itself
                    1,
                    PlayerFilter::You,
                    "exile_cost",
                ),
                // Exile the chosen card
                Effect::exile(ChooseSpec::tagged("exile_cost")),
            ],
        ))
        // Counter target spell (using target_spell() to indicate it's a TARGET)
        .with_spell_effect(vec![Effect::counter(ChooseSpec::target_spell())])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_of_will() {
        let card = force_of_will();
        assert_eq!(card.card.name, "Force of Will");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 5); // 3UU = 5
        assert!(card.card.card_types.contains(&CardType::Instant));

        // Should have alternative casting method
        assert_eq!(card.alternative_casts.len(), 1);
        let alt = &card.alternative_casts[0];
        assert_eq!(alt.name(), "Force of Will");

        // Alternative cost should have no mana cost and three cost effects:
        // 1. Pay 1 life
        // 2. Choose a blue card from hand
        // 3. Exile the chosen card
        if let AlternativeCastingMethod::AlternativeCost {
            mana_cost,
            cost_effects,
            ..
        } = alt
        {
            assert!(
                mana_cost.is_none(),
                "Alternative cost should have no mana cost"
            );
            assert_eq!(
                cost_effects.len(),
                3,
                "Should have 3 cost effects: pay life, choose, exile"
            );

            // First effect: pay 1 life
            assert_eq!(
                cost_effects[0].0.pay_life_amount(),
                Some(1),
                "First effect should be pay 1 life"
            );

            // Second effect: choose objects (blue card from hand)
            let debug_str_1 = format!("{:?}", &cost_effects[1]);
            assert!(
                debug_str_1.contains("ChooseObjectsEffect"),
                "Second effect should be choose objects"
            );

            // Third effect: exile the chosen card
            let debug_str_2 = format!("{:?}", &cost_effects[2]);
            assert!(
                debug_str_2.contains("ExileEffect"),
                "Third effect should be exile"
            );
        } else {
            panic!("Expected AlternativeCost variant");
        }

        // Should have spell effect
        assert!(card.spell_effect.is_some());
        let effects = card.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);
        let debug_str = format!("{:?}", &effects[0]);
        assert!(debug_str.contains("CounterEffect"));
    }
}
