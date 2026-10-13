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
        match &lander.abilities[0].kind {
            AbilityKind::Activated(_) => {}
            other => panic!("expected activated ability, got {other:?}"),
        }
        let lines = crate::runtime_display::unprocessed_compiled_lines(&lander);
        let joined = lines.join("\n");
        assert!(
            joined.contains("Search your library for a basic land card, put it onto the battlefield tapped, then shuffle"),
            "expected search-to-battlefield-tapped text, got {joined}"
        );
    }
}
