use super::*;

fn ash_runner_create(
    presentation: ironsmith_core::TokenAbilityPresentation,
) -> crate::effects::CreateTokenEffect {
    let token =
        crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ash Runner")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elemental])
            .color_indicator(crate::color::ColorSet::RED)
            .power_toughness(crate::card::PowerToughness::fixed(3, 1))
            .with_abilities(vec![
                Ability::static_ability(crate::static_abilities::StaticAbility::trample()),
                Ability::static_ability(crate::static_abilities::StaticAbility::haste()),
            ])
            .build();
    crate::effects::CreateTokenEffect::one(token)
        .with_ability_presentation(presentation)
        .sacrifice_at_next_end_step()
}

#[test]
fn separate_token_definition_includes_the_linked_end_step_ability() {
    let create = ash_runner_create(ironsmith_core::TokenAbilityPresentation::SeparateSentence);

    assert_eq!(
        describe_effect(&Effect::new(create)),
        "Create a 3/1 red Elemental creature token named Ash Runner. It has trample, haste, and \"At the beginning of the end step, sacrifice this token.\""
    );
}

#[test]
fn inline_token_presentation_does_not_claim_the_post_create_surface() {
    let create = ash_runner_create(ironsmith_core::TokenAbilityPresentation::InlineWith);

    assert_eq!(
        describe_token_definition_with_end_step_sacrifice(&create),
        None
    );
}

#[test]
fn standard_junk_token_uses_its_named_token_surface() {
    let create =
        crate::effects::CreateTokenEffect::one(crate::cards::tokens::junk_token_definition());

    assert_eq!(describe_effect(&Effect::new(create)), "Create a Junk token");
}
