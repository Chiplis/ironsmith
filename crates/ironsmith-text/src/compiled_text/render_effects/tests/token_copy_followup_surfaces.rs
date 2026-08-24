use super::*;

fn copy_of_it(count: i32) -> crate::effects::CreateTokenCopyEffect {
    crate::effects::CreateTokenCopyEffect::new(
        ChooseSpec::Tagged(TagKey::from("triggering")),
        count,
        PlayerFilter::You,
    )
}

#[test]
fn inline_copy_haste_remains_an_exception() {
    let create = copy_of_it(1).haste(true);

    assert_eq!(
        describe_effect(&Effect::new(create)),
        "Create a token that's a copy of it, except it has haste"
    );
}

#[test]
fn singular_copy_followups_keep_sentence_boundaries_and_anaphors() {
    use crate::effects::TokenCopyReferenceSurface as Surface;

    let create = copy_of_it(1)
        .haste(true)
        .haste_followup_reference_surface(Some(Surface::ThatToken))
        .sacrifice_at_next_end_step(true)
        .sacrifice_at_next_end_step_reference_surface(Some(Surface::It));

    assert_eq!(
        describe_effect(&Effect::new(create)),
        "Create a token that's a copy of it. That token gains haste. Sacrifice it at the beginning of the next end step"
    );
}

#[test]
fn plural_copy_followups_keep_sentence_boundaries_and_anaphors() {
    use crate::effects::TokenCopyReferenceSurface as Surface;

    let create = copy_of_it(2)
        .haste(true)
        .haste_followup_reference_surface(Some(Surface::ThoseTokens))
        .exile_at_next_end_step(true)
        .exile_at_next_end_step_reference_surface(Some(Surface::They));

    assert_eq!(
        describe_effect(&Effect::new(create)),
        "Create 2 tokens that are copies of it. Those tokens gain haste. Exile them at the beginning of the next end step"
    );
}

fn created_goblins_with_temporary_haste(
    count: i32,
    reference_surface: Option<&str>,
) -> Vec<Effect> {
    let token =
        crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goblin")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Goblin])
            .color_indicator(crate::color::ColorSet::RED)
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build();
    let created = TagKey::from("created_0");
    let create =
        Effect::new(crate::effects::CreateTokenEffect::you(token, count)).tag(created.clone());
    let mut target = ChooseSpec::Tagged(created);
    if let Some(surface) = reference_surface {
        target = target.with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::ThisPermanentType(surface.to_string()),
        ));
    }
    let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
        target,
        crate::continuous::Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
        Until::EndOfTurn,
    ));
    vec![create, grant]
}

#[test]
fn created_token_temporary_grants_keep_authored_singular_anaphors() {
    assert_eq!(
        describe_create_token_then_grant_same_tag(&created_goblins_with_temporary_haste(
            1,
            Some("it")
        ))
        .as_deref(),
        Some("Create a 1/1 red Goblin creature token. It gains haste until end of turn")
    );
    assert_eq!(
        describe_create_token_then_grant_same_tag(&created_goblins_with_temporary_haste(
            1,
            Some("that token")
        ))
        .as_deref(),
        Some("Create a 1/1 red Goblin creature token. That token gains haste until end of turn")
    );
}

#[test]
fn created_token_temporary_grants_use_plural_token_anaphor() {
    assert_eq!(
        describe_create_token_then_grant_same_tag(&created_goblins_with_temporary_haste(2, None))
            .as_deref(),
        Some(
            "Create two 1/1 red Goblin creature tokens. Those tokens gain haste until end of turn"
        )
    );
}

#[test]
fn explicitly_coordinated_token_grants_remain_one_sentence() {
    assert_eq!(
        describe_coordinated_create_token_then_grant_same_tag(
            &created_goblins_with_temporary_haste(1, Some("it"))
        )
        .as_deref(),
        Some("Create a 1/1 red Goblin creature token, and it gains haste until end of turn")
    );
}
