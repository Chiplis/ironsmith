//! Shard token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates a Shard token.
///
/// A Shard is an enchantment token with:
/// "{2}, Sacrifice this token: Scry 1, then draw a card."
pub fn shard_token_definition() -> CardDefinition {
    let scry_and_draw_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![
                Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]])),
                Cost::sacrifice_self(),
            ]),
            effects: vec![Effect::scry(1), Effect::draw(1)],
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{2}, Sacrifice this token: Scry 1, then draw a card.".to_string()),
    };

    CardDefinitionBuilder::new(CardId::new(), "Shard")
        .token()
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Shard])
        .with_ability(scry_and_draw_ability)
        .build()
}

#[cfg(test)]
mod tests {
    use super::shard_token_definition;
    use crate::ability::AbilityKind;
    use crate::types::{CardType, Subtype};

    #[test]
    fn shard_token_has_expected_scry_draw_ability() {
        let shard = shard_token_definition();
        assert!(shard.card.is_token);
        assert!(shard.card.card_types.contains(&CardType::Enchantment));
        assert!(shard.card.subtypes.contains(&Subtype::Shard));
        assert_eq!(shard.abilities.len(), 1);
        match &shard.abilities[0].kind {
            AbilityKind::Activated(activated) => {
                assert_eq!(activated.effects.len(), 2);
                assert_eq!(activated.choices.len(), 0);
                assert_eq!(activated.timing, crate::ability::ActivationTiming::AnyTime);
            }
            other => panic!("expected activated ability, got {other:?}"),
        }
    }
}
