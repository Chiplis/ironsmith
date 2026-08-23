//! Mutagen token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates a Mutagen token.
///
/// A Mutagen is a colorless artifact token with:
/// "{1}, {T}, Sacrifice this token: Put a +1/+1 counter on target creature.
/// Activate only as a sorcery."
pub fn mutagen_token_definition() -> CardDefinition {
    let ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![
                Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])),
                Cost::tap(),
                Cost::sacrifice_self(),
            ]),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::put_counters(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    ChooseSpec::target(ChooseSpec::creature()),
                ),
            ]),
            choices: vec![],
            timing: ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };

    CardDefinitionBuilder::new(CardId::new(), "Mutagen")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Mutagen])
        .with_ability(ability)
        .build()
}
