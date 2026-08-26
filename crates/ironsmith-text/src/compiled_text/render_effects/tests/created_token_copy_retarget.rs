use super::*;

fn soldier_token() -> crate::cards::CardDefinition {
    crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Soldier")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .color_indicator(crate::color::ColorSet::RED)
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .with_ability(Ability::static_ability(
            crate::static_abilities::StaticAbility::haste(),
        ))
        .build()
}

fn create_copy_retarget_effects(
    fixed_target: ChooseSpec,
    retargeted_copy_tag: TagKey,
) -> [Effect; 3] {
    let created = TagKey::from("__created_token__");
    let copied = TagKey::from("__copied_stack_object__");
    let create = Effect::new(crate::effects::CreateTokenEffect::one(soldier_token())).tag(created);
    let copy = Effect::with_id(
        0,
        Effect::new(
            crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from("triggering")))
                .with_target_reference_kind(crate::filter::StackObjectKind::Spell),
        ),
    )
    .tag(copied);
    let retarget = Effect::new(
        crate::effects::RetargetStackObjectEffect::new(ChooseSpec::Tagged(retargeted_copy_tag))
            .with_mode(crate::effects::RetargetMode::OneToFixed(fixed_target)),
    );
    [create, copy, retarget]
}

#[test]
fn exact_created_token_and_copied_spell_tags_render_the_linked_bundle() {
    let effects = create_copy_retarget_effects(
        ChooseSpec::Tagged(TagKey::from("__created_token__")),
        TagKey::from("__copied_stack_object__"),
    );
    let refs = effects.iter().collect::<Vec<_>>();

    assert_eq!(
        describe_create_token_then_copy_retarget_to_created_token(&refs),
        Some(
            "Create a 1/1 red Soldier creature token with haste, then copy that spell. The copy targets that token"
                .to_string()
        )
    );
}

#[test]
fn linked_bundle_rejects_the_wrong_created_token_or_copied_spell_tag() {
    let wrong_created = create_copy_retarget_effects(
        ChooseSpec::Tagged(TagKey::from("__different_token__")),
        TagKey::from("__copied_stack_object__"),
    );
    assert_eq!(
        describe_create_token_then_copy_retarget_to_created_token(
            &wrong_created.iter().collect::<Vec<_>>()
        ),
        None,
        "the fixed target must be the exact created-token result set"
    );

    let wrong_copy = create_copy_retarget_effects(
        ChooseSpec::Tagged(TagKey::from("__created_token__")),
        TagKey::from("__different_copy__"),
    );
    assert_eq!(
        describe_create_token_then_copy_retarget_to_created_token(
            &wrong_copy.iter().collect::<Vec<_>>()
        ),
        None,
        "the retargeted stack object must be the exact copied-spell result"
    );
}

#[test]
fn resolved_token_filter_keeps_the_exact_created_result_reference() {
    let created = TagKey::from("__created_token__");
    let resolved_filter = ObjectFilter::tagged(created.clone()).token();
    let effects = create_copy_retarget_effects(
        ChooseSpec::Object(resolved_filter),
        TagKey::from("__copied_stack_object__"),
    );
    assert_eq!(
        describe_create_token_then_copy_retarget_to_created_token(
            &effects.iter().collect::<Vec<_>>()
        ),
        Some(
            "Create a 1/1 red Soldier creature token with haste, then copy that spell. The copy targets that token"
                .to_string()
        )
    );

    let broader_filter = ObjectFilter::tagged(created)
        .token()
        .with_type(CardType::Creature);
    let broader = create_copy_retarget_effects(
        ChooseSpec::Object(broader_filter),
        TagKey::from("__copied_stack_object__"),
    );
    assert_eq!(
        describe_create_token_then_copy_retarget_to_created_token(
            &broader.iter().collect::<Vec<_>>()
        ),
        None,
        "an additional object predicate must not be treated as the exact created result"
    );
}
