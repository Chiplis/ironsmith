//! Incubator token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::card::{LinkedFaceLayout, PowerToughness};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

pub fn incubator_token_definitions() -> (CardDefinition, CardDefinition) {
    let front_id = CardId::new();
    let back_id = CardId::new();

    let transform_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![Cost::mana(ManaCost::from_pips(vec![vec![
                ManaSymbol::Generic(2),
            ]]))]),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::transform(
                ChooseSpec::Source,
            )]),
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

    let front = CardDefinitionBuilder::new(front_id, "Incubator")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Incubator])
        .other_face(back_id)
        .other_face_name("Phyrexian Token")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .with_ability(transform_ability)
        .build();

    let back = CardDefinitionBuilder::new(back_id, "Phyrexian Token")
        .token()
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Phyrexian])
        .power_toughness(PowerToughness::fixed(0, 0))
        .other_face(front_id)
        .other_face_name("Incubator")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();

    (front, back)
}

pub fn incubator_token_definition() -> CardDefinition {
    incubator_token_definitions().0
}

#[cfg(test)]
mod tests {
    use super::incubator_token_definitions;
    use crate::ability::AbilityKind;
    use crate::card::LinkedFaceLayout;
    use crate::types::{CardType, Subtype};

    #[test]
    fn incubator_token_has_transform_linked_faces() {
        let (front, back) = incubator_token_definitions();

        assert!(front.card.is_token);
        assert!(front.card.card_types.contains(&CardType::Artifact));
        assert!(front.card.subtypes.contains(&Subtype::Incubator));
        assert_eq!(
            front.card.linked_face_layout,
            LinkedFaceLayout::TransformLike
        );
        assert_eq!(front.card.other_face, Some(back.card.id));
        assert_eq!(front.abilities.len(), 1);
        assert!(matches!(front.abilities[0].kind, AbilityKind::Activated(_)));

        assert!(back.card.is_token);
        assert!(back.card.card_types.contains(&CardType::Artifact));
        assert!(back.card.card_types.contains(&CardType::Creature));
        assert!(back.card.subtypes.contains(&Subtype::Phyrexian));
        assert_eq!(
            back.card.power_toughness,
            Some(crate::card::PowerToughness::fixed(0, 0))
        );
        assert_eq!(
            back.card.linked_face_layout,
            LinkedFaceLayout::TransformLike
        );
        assert_eq!(back.card.other_face, Some(front.card.id));
    }
}
