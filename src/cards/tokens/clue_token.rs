//! Clue token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates a Clue token.
/// A Clue is an artifact token with "{2}, Sacrifice this artifact: Draw a card."
pub fn clue_token_definition() -> CardDefinition {
    let draw_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: crate::ability::merge_cost_effects(
                TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]])),
                vec![Effect::sacrifice_source()],
            ),
            effects: vec![Effect::draw(1)],
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some("{2}, Sacrifice this artifact: Draw a card.".to_string()),
    };

    CardDefinitionBuilder::new(CardId::new(), "Clue")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Clue])
        .with_ability(draw_ability)
        .build()
}
