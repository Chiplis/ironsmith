//! Treasure token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates a Treasure token.
/// A Treasure is an artifact token with "{T}, Sacrifice this artifact: Add one mana of any color."
pub fn treasure_token_definition() -> CardDefinition {
    let mana_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: crate::ability::merge_cost_effects(
                TotalCost::free(),
                vec![Effect::tap_source(), Effect::sacrifice_source()],
            ),
            effects: vec![Effect::add_mana_of_any_color(1)],
            choices: vec![],
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            mana_output: Some(vec![]),
            activation_condition: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{T}, Sacrifice this artifact: Add one mana of any color.".to_string()),
    };

    CardDefinitionBuilder::new(CardId::new(), "Treasure")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Treasure])
        .with_ability(mana_ability)
        .build()
}
