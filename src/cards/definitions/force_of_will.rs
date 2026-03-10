//! Card definition for Force of Will.

use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::color::ColorSet;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ChooseSpec;
use crate::types::CardType;

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
                crate::costs::Cost::life(1),
                crate::costs::Cost::exile_from_hand(1, Some(ColorSet::BLUE)),
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

        // Alternative cost should have no mana cost and two non-mana costs:
        // 1. Pay 1 life
        // 2. Exile a blue card from your hand
        if let AlternativeCastingMethod::Composed { total_cost, .. } = alt {
            let mana_cost = total_cost.mana_cost();
            let costs = alt.non_mana_costs();
            assert!(
                mana_cost.is_none(),
                "Alternative cost should have no mana cost"
            );
            assert_eq!(
                costs.len(),
                2,
                "Should have 2 non-mana costs: pay life, exile from hand"
            );

            assert_eq!(costs[0].life_amount(), Some(1), "First cost should be pay 1 life");

            let debug_str_1 = format!("{:?}", &costs[1]);
            assert!(
                debug_str_1.contains("CostEffect"),
                "Second cost should be exile-from-hand"
            );
        } else {
            panic!("Expected Composed variant");
        }

        // Should have spell effect
        assert!(card.spell_effect.is_some());
        let effects = card.spell_effect.as_ref().unwrap();
        assert_eq!(effects.len(), 1);
        let debug_str = format!("{:?}", &effects[0]);
        assert!(debug_str.contains("CounterEffect"));
    }
}
