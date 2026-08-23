//! Blood token definition.

use crate::ability::{Ability, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

/// Creates a Blood token with its intrinsic rummage ability.
pub fn blood_token_definition() -> CardDefinition {
    let ability = Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
            Cost::tap(),
            Cost::discard(1, None),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::draw(1)],
        ActivationTiming::AnyTime,
    );

    CardDefinitionBuilder::new(CardId::new(), "Blood")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Blood])
        .with_ability(ability)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;

    #[test]
    fn blood_token_keeps_its_complete_cost_and_draw_program() {
        let token = blood_token_definition();
        let activated = token
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("Blood should have its intrinsic activated ability");
        let costs = activated.mana_cost.costs();
        assert_eq!(costs.len(), 4);
        assert_eq!(
            costs[0].mana_cost_ref(),
            Some(&ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        );
        assert!(costs[1].requires_tap());
        assert!(costs[2].is_discard());
        assert!(costs[3].is_sacrifice_self());
        assert!(format!("{:#?}", activated.effects).contains("DrawCardsEffect"));
    }
}
