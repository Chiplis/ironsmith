use super::*;

fn artifact_copy_program(reference_kind: StackObjectKind) -> Vec<Effect> {
    let copied = TagKey::from("__copied_stack_object__");
    let copy = Effect::with_id(
        0,
        Effect::new(
            crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from("triggering")))
                .with_target_reference_kind(reference_kind),
        ),
    )
    .tag(copied.clone());
    let add_artifact = Effect::new(
        crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(copied),
            crate::continuous::Modification::AddCardTypes(vec![CardType::Artifact]),
            Until::Forever,
        )
        .with_type_retention_surface(Some(
            ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes,
        )),
    );
    vec![copy, add_artifact]
}

fn assert_typed_copy_reference(kind: StackObjectKind, reference: &str) {
    let effects = artifact_copy_program(kind);
    let expected = format!(
        "Copy that {reference}, except the copy is an artifact in addition to its other types"
    );
    let expected_clause = format!(
        "copy that {reference}, except the copy is an artifact in addition to its other types"
    );

    assert_eq!(describe_effect_list(&effects), expected, "{kind:?}");
    assert_eq!(
        describe_effect_clause_list(&effects).as_deref(),
        Some(expected_clause.as_str()),
        "{kind:?}"
    );
}

#[test]
fn copied_spell_reference_uses_spell_noun() {
    assert_typed_copy_reference(StackObjectKind::Spell, "spell");
}

#[test]
fn copied_ability_reference_uses_ability_noun() {
    assert_typed_copy_reference(StackObjectKind::Ability, "ability");
}

#[test]
fn copied_mixed_reference_uses_spell_or_ability_noun() {
    assert_typed_copy_reference(StackObjectKind::SpellOrAbility, "spell or ability");
}

#[test]
fn copied_pronoun_surface_overrides_resolved_trigger_kind() {
    let effect = Effect::new(
        crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from("triggering")))
            .with_target_reference_kind(StackObjectKind::Spell)
            .with_target_reference_pronoun(true),
    );

    assert_eq!(describe_effect(&effect), "Copy it");
}

#[test]
fn copied_spell_color_exception_stays_with_retarget_sentence() {
    let copy_id = crate::effect::EffectId(7);
    let copied = TagKey::from("__copied_stack_object__");
    let copy = Effect::with_id(
        copy_id.0,
        Effect::new(crate::effects::CopySpellEffect::single(ChooseSpec::spell())),
    )
    .tag(copied.clone());
    let set_red = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
        ChooseSpec::Tagged(copied),
        crate::continuous::Modification::SetColors(crate::color::ColorSet::RED),
        Until::Forever,
    ));
    let retarget = Effect::new(crate::effects::ChooseNewTargetsEffect::may(copy_id));

    assert_eq!(
        describe_effect_list(&[copy, set_red, retarget]),
        "Copy target spell, except that the copy is red. You may choose new targets for the copy"
    );
}
