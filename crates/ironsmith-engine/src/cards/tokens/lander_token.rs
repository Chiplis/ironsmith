//! Lander token definition.

use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::ids::CardId;
use crate::types::{CardType, Subtype};

/// Creates a Lander token.
///
/// A Lander is an artifact token with:
/// "{2}, {T}, Sacrifice this token: Search your library for a basic land card,
/// put it onto the battlefield tapped, then shuffle."
pub fn lander_token_definition() -> CardDefinition {
    use crate::ability::{Ability, ActivationTiming};
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::effect::Effect;
    use crate::effects::{ChooseObjectsEffect, PutOntoBattlefieldEffect, SequenceEffect};
    use crate::filter::{ObjectFilter, PlayerFilter};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::tag::TagKey;
    use crate::target::ChooseSpec;
    use crate::types::Supertype;
    use crate::zone::Zone;

    let found = TagKey::from("lander_found");
    let search = ChooseObjectsEffect::new(
        ObjectFilter::land()
            .in_zone(Zone::Library)
            .with_supertype(Supertype::Basic)
            .owned_by(PlayerFilter::You),
        1,
        PlayerFilter::You,
        found.clone(),
    )
    .in_zone(Zone::Library)
    .as_search()
    .with_description("basic land card");
    let ability = Ability::activated_with_timing(
        TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::tap(),
            Cost::sacrifice_self(),
        ]),
        vec![Effect::new(SequenceEffect::new(vec![
            Effect::new(search),
            Effect::new(PutOntoBattlefieldEffect::new(
                ChooseSpec::Tagged(found),
                true,
                PlayerFilter::You,
            )),
            Effect::shuffle_library(),
        ]))],
        ActivationTiming::AnyTime,
    );
    CardDefinitionBuilder::new(CardId::new(), "Lander Token")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Lander])
        .with_ability(ability)
        .build()
}

#[cfg(test)]
mod tests {
    use super::lander_token_definition;
    use crate::ability::AbilityKind;
    use crate::types::{CardType, Subtype};

    #[test]
    fn lander_token_has_expected_basic_land_search_ability() {
        let lander = lander_token_definition();
        assert!(lander.card.is_token);
        assert!(lander.card.card_types.contains(&CardType::Artifact));
        assert!(lander.card.subtypes.contains(&Subtype::Lander));
        assert_eq!(lander.abilities.len(), 1);
        let AbilityKind::Activated(ability) = &lander.abilities[0].kind else {
            panic!("expected activated ability");
        };
        let effects = ability.effects.flattened_default_effects();
        let sequence = effects[0].downcast_ref::<crate::effects::SequenceEffect>().unwrap();
        let [search, put, shuffle] = sequence.effects.as_slice() else { panic!("search, put, shuffle"); };
        let search = search.downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap();
        assert!(search.is_search);
        assert_eq!(search.count.max, Some(1));
        assert_eq!(search.chooser, crate::filter::PlayerFilter::You);
        assert_eq!(search.zone, Some(crate::zone::Zone::Library));
        assert_eq!(search.filter.card_types, vec![CardType::Land]);
        assert_eq!(search.filter.supertypes, vec![crate::types::Supertype::Basic]);
        let put = put.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>().unwrap();
        assert_eq!(put.target, crate::target::ChooseSpec::Tagged(search.tag.clone()));
        assert!(put.tapped);
        assert_eq!(put.controller, crate::filter::PlayerFilter::You);
        assert!(shuffle.downcast_ref::<crate::effects::ShuffleLibraryEffect>().is_some());
    }
}
