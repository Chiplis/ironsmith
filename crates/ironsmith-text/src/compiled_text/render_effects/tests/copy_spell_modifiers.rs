use super::*;

fn render_public(text: &str, name: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("typed copy-and-retarget route should compile");
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

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

#[test]
fn copied_spell_fixed_pt_subtype_exception_stays_one_typed_clause() {
    let copied = TagKey::from("__copied_stack_object__");
    let copy = Effect::with_id(
        3,
        Effect::new(
            crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from("triggering")))
                .with_target_reference_kind(StackObjectKind::Spell),
        ),
    )
    .tag(copied.clone());
    let modifier = Effect::new(
        crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(copied),
            crate::continuous::Modification::AddSubtypes(vec![Subtype::Spirit]),
            Until::Forever,
        )
        .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
            power: Value::Fixed(1),
            toughness: Value::Fixed(1),
            sublayer: crate::continuous::PtSublayer::Setting,
        })
        .with_type_retention_surface(Some(
            ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes,
        )),
    );

    assert_eq!(
        describe_effect_list(&[copy.clone(), modifier.clone()]),
        "Copy that spell, except the copy is a 1/1 Spirit in addition to its other types"
    );

    let mut wrong_duration = modifier
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("modifier")
        .clone();
    wrong_duration.until = Until::EndOfTurn;
    assert_ne!(
        describe_effect_list(&[copy, Effect::new(wrong_duration)]),
        "Copy that spell, except the copy is a 1/1 Spirit in addition to its other types"
    );
}

#[test]
fn copiable_fixed_pt_subtype_exception_renders_directly_on_the_copy_effect() {
    let copy = Effect::new(
        crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from("triggering")))
            .with_target_reference_kind(StackObjectKind::Spell)
            .with_added_subtypes(vec![Subtype::Spirit])
            .with_set_base_power_toughness(Some((1, 1))),
    );

    assert_eq!(
        describe_effect(&copy),
        "Copy that spell, except the copy is a 1/1 Spirit in addition to its other types"
    );
}

#[test]
fn chosen_legal_target_copy_program_keeps_its_authored_assignment_surface() {
    let text = "Whenever you cast an instant or sorcery spell that targets only this creature, if you control one or more other creatures that spell could target, choose one of those creatures. Copy that spell. The copy targets the chosen creature.";
    assert_eq!(render_public(text, "Copy Target Probe"), text);
}

#[test]
fn each_other_opponent_copy_loop_keeps_its_correlated_assignment_surface() {
    let text = "{2}, {T}: When you next cast an instant or sorcery spell that targets only a single opponent or a single permanent an opponent controls this turn, for each other opponent, choose that player or a permanent they control, copy that spell, and the copy targets the chosen player or permanent.";
    assert_eq!(render_public(text, "Opponent Copy Loop Probe"), text);
}

#[test]
fn chosen_creature_complement_copy_keeps_the_filtered_recipient_set() {
    let text = "{U}, {T}: Choose target creature you control. Each creature you control other than the chosen creature becomes a copy of that creature until end of turn, except it isn't legendary. Activate only as a sorcery.";
    assert_eq!(render_public(text, "Complement Copy Probe"), text);
}
