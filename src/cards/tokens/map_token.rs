//! Map token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates a Map token.
///
/// A Map is an artifact token with:
/// "{1}, {T}, Sacrifice this token: Target creature you control explores.
///  Activate only as a sorcery."
pub fn map_token_definition() -> CardDefinition {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let explore_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![
                Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]])),
                Cost::tap(),
                Cost::sacrifice_self(),
            ]),
            effects: vec![Effect::explore(target.clone())],
            choices: vec![target],
            timing: ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "{1}, {T}, Sacrifice this token: Target creature you control explores. Activate only as a sorcery.".to_string(),
        ),
    };

    CardDefinitionBuilder::new(CardId::new(), "Map")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Map])
        .with_ability(explore_ability)
        .build()
}

#[cfg(test)]
mod tests {
    use super::map_token_definition;
    use crate::ability::AbilityKind;
    use crate::types::{CardType, Subtype};

    #[test]
    fn map_token_has_expected_artifact_explore_ability() {
        let map = map_token_definition();
        assert!(map.card.is_token);
        assert!(map.card.card_types.contains(&CardType::Artifact));
        assert!(map.card.subtypes.contains(&Subtype::Map));
        assert_eq!(map.abilities.len(), 1);
        match &map.abilities[0].kind {
            AbilityKind::Activated(activated) => {
                assert_eq!(activated.choices.len(), 1);
                assert_eq!(activated.effects.len(), 1);
                assert_eq!(
                    activated.timing,
                    crate::ability::ActivationTiming::SorcerySpeed
                );
            }
            other => panic!("expected activated ability, got {other:?}"),
        }
    }
}
