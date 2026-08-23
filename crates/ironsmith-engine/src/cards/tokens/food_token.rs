//! Food token definition.

use crate::ability::{Ability, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Creates a Food token with its intrinsic life-gain ability.
pub fn food_token_definition() -> CardDefinition {
    let ability = Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::tap(),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::gain_life(3)],
        ActivationTiming::AnyTime,
    );

    CardDefinitionBuilder::new(CardId::new(), "Food")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Food])
        .with_ability(ability)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn food_token_keeps_its_mana_tap_sacrifice_and_life_gain_program() {
        let token = food_token_definition();
        let activated = token
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("Food should have its intrinsic activated ability");
        let costs = activated.mana_cost.costs();
        assert_eq!(costs.len(), 3);
        assert_eq!(
            costs[0].mana_cost_ref(),
            Some(&ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
        );
        assert!(costs[1].requires_tap());
        assert!(costs[2].is_sacrifice_self());
        assert!(format!("{:#?}", activated.effects).contains("GainLifeEffect"));
    }
}
