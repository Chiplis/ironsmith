//! Card definition for Everflowing Chalice.

use crate::ability::Ability;
use crate::cards::CardDefinition;
use crate::cards::builders::CardDefinitionBuilder;
use crate::cost::{OptionalCost, TotalCost};
use crate::effect::{Effect, Value};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::triggers::Trigger;
use crate::types::CardType;

/// Creates the Everflowing Chalice card definition.
///
/// Everflowing Chalice {0}
/// Artifact
/// Multikicker {2}
/// Everflowing Chalice enters the battlefield with a charge counter on it
/// for each time it was kicked.
/// {T}: Add {C} for each charge counter on Everflowing Chalice.
pub fn everflowing_chalice() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Everflowing Chalice")
        .mana_cost(ManaCost::new()) // {0}
        .card_types(vec![CardType::Artifact])
        // Multikicker {2}
        .optional_cost(OptionalCost::multikicker(TotalCost::mana(
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]),
        )))
        // ETB trigger: Put a charge counter for each time it was kicked
        .with_trigger(
            Trigger::this_enters_battlefield(),
            vec![Effect::put_counters_on_source(
                CounterType::Charge,
                Value::KickCount,
            )],
        )
        // Mana ability: {T}: Add {C} for each charge counter
        .with_ability(Ability::mana_with_effects(
            TotalCost::free(),
            vec![Effect::add_colorless_mana(Value::CountersOnSource(
                CounterType::Charge,
            ))],
        ))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn test_everflowing_chalice() {
        let card = everflowing_chalice();
        assert_eq!(card.card.name, "Everflowing Chalice");
        assert_eq!(card.card.mana_cost.as_ref().unwrap().mana_value(), 0);
        assert!(card.card.card_types.contains(&CardType::Artifact));

        // Should have multikicker
        assert_eq!(card.optional_costs.len(), 1);
        assert_eq!(card.optional_costs[0].label, "Multikicker");
        assert!(card.optional_costs[0].repeatable);

        // Should have 2 abilities: ETB trigger and mana ability
        assert_eq!(card.abilities.len(), 2);

        // First ability should be an ETB trigger (now using Trigger struct)
        assert!(matches!(
            &card.abilities[0].kind,
            AbilityKind::Triggered(t) if t.trigger.display().contains("enters")
        ));

        // Second ability should be a mana ability
        assert!(matches!(&card.abilities[1].kind, AbilityKind::Mana(_)));
    }
}
