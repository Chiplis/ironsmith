//! Clue token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
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
            keyword: None,
            mana_cost: TotalCost::from_costs(vec![
                Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]])),
                Cost::sacrifice_self(),
            ]),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
            choices: vec![],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    // The predefined Clue supplies no explicit name (CR 111.10f). Its name
    // is therefore its subtype followed by Token (CR 111.4), including when
    // a native keyword or token replacement uses this blueprint directly.
    CardDefinitionBuilder::new(CardId::new(), "Clue Token")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Clue])
        .with_ability(draw_ability)
        .build()
}
