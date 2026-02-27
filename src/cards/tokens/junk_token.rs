//! Junk token definition.

use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::ids::CardId;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Creates a Junk token.
///
/// A Junk is an artifact token with:
/// "{T}, Sacrifice this token: Exile the top card of your library.
/// You may play that card this turn. Activate only as a sorcery."
pub fn junk_token_definition() -> CardDefinition {
    let exile_tag = crate::tag::TagKey::from("junk_exiled_card");
    let exile_and_play_ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: crate::ability::merge_cost_effects(
                TotalCost::free(),
                vec![Effect::tap_source(), Effect::sacrifice_source()],
            ),
            effects: vec![
                Effect::new(
                    crate::effects::ChooseObjectsEffect::new(
                        ObjectFilter::default()
                            .in_zone(Zone::Library)
                            .owned_by(PlayerFilter::You),
                        1,
                        PlayerFilter::You,
                        exile_tag.clone(),
                    )
                    .in_zone(Zone::Library)
                    .top_only(),
                ),
                Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(
                    exile_tag.clone(),
                ))),
                Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    exile_tag,
                    PlayerFilter::You,
                    crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                )),
            ],
            choices: vec![],
            timing: ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
        }),
        functional_zones: vec![Zone::Battlefield],
        text: Some(
            "{T}, Sacrifice this token: Exile the top card of your library. You may play that card this turn. Activate only as a sorcery.".to_string(),
        ),
    };

    CardDefinitionBuilder::new(CardId::new(), "Junk")
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Junk])
        .with_ability(exile_and_play_ability)
        .build()
}

#[cfg(test)]
mod tests {
    use super::junk_token_definition;
    use crate::ability::AbilityKind;
    use crate::types::{CardType, Subtype};

    #[test]
    fn junk_token_has_expected_impulse_draw_ability() {
        let junk = junk_token_definition();
        assert!(junk.card.is_token);
        assert!(junk.card.card_types.contains(&CardType::Artifact));
        assert!(junk.card.subtypes.contains(&Subtype::Junk));
        assert_eq!(junk.abilities.len(), 1);
        match &junk.abilities[0].kind {
            AbilityKind::Activated(activated) => {
                assert_eq!(activated.effects.len(), 3);
                assert_eq!(activated.choices.len(), 0);
                assert_eq!(activated.timing, crate::ability::ActivationTiming::SorcerySpeed);
            }
            other => panic!("expected activated ability, got {other:?}"),
        }
    }
}
