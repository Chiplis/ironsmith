//! Powerstone token definition.

use crate::ability::{
    Ability, AbilityKind, ActivatedAbility, ActivationTiming, ManaPaymentPredicate,
    ManaPaymentPurpose, ManaUsageRestriction,
};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates a Powerstone token whose mana cannot cast nonartifact spells.
pub fn powerstone_token_definition() -> CardDefinition {
    let restriction = ManaUsageRestriction::PaymentTransaction {
        restriction: Some(ManaPaymentPredicate::Not(Box::new(
            ManaPaymentPredicate::All(vec![
                ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
                ManaPaymentPredicate::SourceMatches(
                    ObjectFilter::default().without_type(CardType::Artifact),
                ),
            ]),
        ))),
        on_spend: Vec::new(),
    };
    let ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![Cost::tap()]),
            effects: vec![Effect::add_mana(vec![ManaSymbol::Colorless])].into(),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: Some(vec![ManaSymbol::Colorless]),
            activation_condition: None,
            mana_usage_restrictions: vec![restriction],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    CardDefinitionBuilder::new(CardId::new(), "Powerstone")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Powerstone])
        .with_ability(ability)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powerstone_token_keeps_colorless_mana_and_nonartifact_spell_restriction() {
        let token = powerstone_token_definition();
        let activated = token
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("Powerstone should have its intrinsic mana ability");
        let costs = activated.mana_cost.costs();
        assert_eq!(costs.len(), 1);
        assert!(costs[0].requires_tap());
        assert_eq!(activated.mana_output, Some(vec![ManaSymbol::Colorless]));
        assert_eq!(activated.mana_usage_restrictions.len(), 1);
        assert!(matches!(
            &activated.mana_usage_restrictions[0],
            ManaUsageRestriction::PaymentTransaction {
                restriction: Some(ManaPaymentPredicate::Not(_)),
                on_spend,
            } if on_spend.is_empty()
        ));
    }
}
