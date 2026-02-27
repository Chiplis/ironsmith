//! Gold token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates a Gold token.
///
/// A Gold is an artifact token with:
/// "Sacrifice this token: Add one mana of any color."
pub fn gold_token_definition() -> CardDefinition {
    let mana_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: crate::ability::merge_cost_effects(
                TotalCost::free(),
                vec![Effect::sacrifice_source()],
            ),
            effects: vec![Effect::add_mana_of_any_color(1)],
            choices: vec![],
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            mana_output: Some(vec![]),
            activation_condition: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("Sacrifice this token: Add one mana of any color.".to_string()),
    };

    CardDefinitionBuilder::new(CardId::new(), "Gold")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Gold])
        .with_ability(mana_ability)
        .build()
}

#[cfg(test)]
mod tests {
    use super::gold_token_definition;
    use crate::ability::AbilityKind;
    use crate::types::{CardType, Subtype};

    #[test]
    fn gold_token_has_expected_mana_ability() {
        let gold = gold_token_definition();
        assert!(gold.card.is_token);
        assert!(gold.card.card_types.contains(&CardType::Artifact));
        assert!(gold.card.subtypes.contains(&Subtype::Gold));
        assert_eq!(gold.abilities.len(), 1);
        match &gold.abilities[0].kind {
            AbilityKind::Activated(activated) => {
                assert_eq!(activated.effects.len(), 1);
                assert_eq!(activated.choices.len(), 0);
            }
            other => panic!("expected activated mana ability, got {other:?}"),
        }
    }
}
