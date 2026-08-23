use super::*;

const COPIED_STACK_OBJECT_TAG: &str = "__copied_stack_object__";

fn is_graveyard_domain(filter: &ObjectFilter) -> bool {
    if filter.zone == Some(Zone::Graveyard) {
        return true;
    }
    filter.zone.is_none()
        && !filter.any_of.is_empty()
        && filter.any_of.iter().all(is_graveyard_domain)
}

fn exact_face_up_graveyard_move_to_exile(
    moved: &crate::effects::MoveToZoneEffect,
) -> Option<&ChooseSpec> {
    if moved.zone != Zone::Exile {
        return None;
    }
    // `to_top` has no rules meaning outside a library. Generic zone lowering
    // historically retained it on face-up exile moves, so normalize only this
    // irrelevant bit before proving the otherwise canonical move.
    let mut semantic = moved.clone();
    semantic.to_top = false;
    (semantic == crate::effects::MoveToZoneEffect::to_exile(moved.target.clone()))
        .then_some(&moved.target)
}

fn describe_graveyard_exile(inner: &Effect) -> String {
    let text = describe_effect(inner);
    if text.contains("put there from") {
        return text
            .replace(
                " from your graveyard that was put there",
                " in your graveyard that was put there",
            )
            .replace(
                " from a graveyard that was put there",
                " in a graveyard that was put there",
            );
    }
    text.replace(" in your graveyard", " from your graveyard")
        .replace(" in a graveyard", " from a graveyard")
}

fn tagged_effect_view(effect: &Effect) -> Option<(&crate::tag::TagKey, &Effect)> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_effect_view(&with_id.effect);
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    Some((
        &tagged.tag,
        structural_unwrap_render_wrappers(&tagged.effect),
    ))
}

fn exact_graveyard_exile(effect: &Effect) -> Option<(&crate::tag::TagKey, &Effect)> {
    let (tag, inner) = tagged_effect_view(effect)?;
    if !crate::cards::is_sentence_helper_tag(tag.as_str(), "exiled") {
        return None;
    }
    let spec = if let Some(exile) = inner.downcast_ref::<crate::effects::ExileEffect>() {
        if exile.face_down {
            return None;
        }
        &exile.spec
    } else {
        let moved = inner.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        exact_face_up_graveyard_move_to_exile(moved)?
    };
    let ChooseSpec::Object(filter) = spec.base() else {
        return None;
    };
    is_graveyard_domain(filter).then_some((tag, inner))
}

fn exact_coordinated_graveyard_move_and_copy(
    effect: &Effect,
) -> Option<(&crate::tag::TagKey, &Effect)> {
    let sequence = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
        return None;
    }
    let [move_effect, copy_effect] = sequence.effects.as_slice() else {
        return None;
    };

    let (exiled_tag, move_inner) = tagged_effect_view(move_effect)?;
    if !crate::cards::is_sentence_helper_tag(exiled_tag.as_str(), "exiled") {
        return None;
    }
    // The executable card-copy lowering has used both generic zone movement
    // and the dedicated exile action over time.  They are equivalent here,
    // but only after proving an ordinary, face-up graveyard exile.  Keep the
    // original effect for rendering so target/count surfaces such as
    // `up to one target` are not reconstructed.
    let moved_target =
        if let Some(moved) = move_inner.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            exact_face_up_graveyard_move_to_exile(moved)?
        } else {
            let exile = move_inner.downcast_ref::<crate::effects::ExileEffect>()?;
            (!exile.face_down).then_some(&exile.spec)?
        };
    let ChooseSpec::Object(filter) = moved_target.base() else {
        return None;
    };
    if !is_graveyard_domain(filter) {
        return None;
    }

    let (copy_tag, copy_inner) = tagged_effect_view(copy_effect)?;
    let copy = copy_inner.downcast_ref::<crate::effects::CopySpellEffect>()?;
    if copy_tag.as_str() != COPIED_STACK_OBJECT_TAG
        || !matches!(copy.target.unhinted(), ChooseSpec::Tagged(tag) if tag == exiled_tag)
        || copy.target_reference_kind.is_some()
        || !copy.target_reference_pronoun
        || copy.count.unhinted() != &crate::effect::Value::Fixed(1)
        || copy.count_surface.is_some()
        || copy.copier != PlayerFilter::You
        || !copy.removed_supertypes.is_empty()
        || copy.has_characteristic_modifiers()
    {
        return None;
    }
    Some((exiled_tag, move_inner))
}

fn exact_optional_cast_copy<'a>(
    effect: &'a Effect,
    exiled_tag: &crate::tag::TagKey,
) -> Option<&'a crate::effects::CastTaggedEffect> {
    let may =
        structural_unwrap_render_wrappers(effect).downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You))
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if &cast.tag != exiled_tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || !cast.as_copy
        || cast.cost_reduction.is_some()
    {
        return None;
    }
    Some(cast)
}

fn append_standard_copy_cast_reminder(text: &mut String, cast: &crate::effects::CastTaggedEffect) {
    if !cast.copy_cast_reminder_surface {
        return;
    }
    text.push(' ');
    text.push_str(STANDARD_REMINDER_OPEN_SENTINEL);
    text.push_str("You still pay its costs. A copy of a permanent spell becomes a token.");
    text.push_str(STANDARD_REMINDER_CLOSE_SENTINEL);
}

/// Renders the executable two-effect representation of copying a card in a
/// graveyard/exile transition and optionally casting that copy. The exile tag
/// and the cast-copy tag must be identical, so this cannot join an unrelated
/// exile with an independent copy permission.
pub(in crate::compiled_text) fn render_graveyard_exile_copy_cast_pair(
    exile_effect: &Effect,
    may_effect: &Effect,
) -> Option<String> {
    let (exiled_tag, exile_inner) = exact_graveyard_exile(exile_effect)
        .or_else(|| exact_coordinated_graveyard_move_and_copy(exile_effect))?;
    let cast = exact_optional_cast_copy(may_effect, exiled_tag)?;
    let mut may_cast = "You may cast the copy".to_string();
    if cast.without_paying_mana_cost {
        may_cast.push_str(" without paying its mana cost");
    }
    let exile_text = describe_graveyard_exile(exile_inner);
    let exile_text = exile_text.trim_end_matches('.');
    let mut rendered = match cast.copy_instruction_surface {
        Some(ironsmith_core::effect::CopyInstructionSurface::SeparateIt) => {
            format!("{exile_text}. Copy it. {may_cast}")
        }
        Some(ironsmith_core::effect::CopyInstructionSurface::SeparateThatCard) => {
            format!("{exile_text}. Copy that card. {may_cast}")
        }
        Some(
            ironsmith_core::effect::CopyInstructionSurface::SeparateItThen
            | ironsmith_core::effect::CopyInstructionSurface::SeparateItThenPermanentCopyReminder,
        ) => {
            let may_cast = may_cast.strip_prefix("You ").unwrap_or(may_cast.as_str());
            format!("{exile_text}. Copy it, then you {may_cast}")
        }
        None => format!("{exile_text} and copy it. {may_cast}"),
    };
    append_standard_copy_cast_reminder(&mut rendered, cast);
    if cast.copy_instruction_surface
        == Some(ironsmith_core::effect::CopyInstructionSurface::SeparateItThenPermanentCopyReminder)
    {
        rendered.push_str(". ");
        rendered.push_str(STANDARD_REMINDER_OPEN_SENTINEL);
        rendered.push_str("A copy of a permanent spell becomes a token.");
        rendered.push_str(STANDARD_REMINDER_CLOSE_SENTINEL);
    }
    Some(rendered)
}

pub(in crate::compiled_text) fn render_conditional_graveyard_exile_copy_cast_pair(
    exile_effect: &Effect,
    conditional_effect: &Effect,
) -> Option<String> {
    let exile_id = exile_effect
        .downcast_ref::<crate::effects::WithIdEffect>()?
        .id;
    let (exiled_tag, exile_inner) = exact_graveyard_exile(exile_effect)?;
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != exile_id
        || conditional.predicate != crate::effect::EffectPredicate::Happened
        || !conditional.else_.is_empty()
        || conditional.prior_result_replacement_surface
    {
        return None;
    }
    let [may_cast] = conditional.then.as_slice() else {
        return None;
    };
    let cast = exact_optional_cast_copy(may_cast, exiled_tag)?;
    let mut cast_text = "You may cast the copy".to_string();
    if cast.without_paying_mana_cost {
        cast_text.push_str(" without paying its mana cost");
    }
    let mut rendered = format!(
        "{}. If you do, copy it. {cast_text}",
        describe_graveyard_exile(exile_inner).trim_end_matches('.')
    );
    append_standard_copy_cast_reminder(&mut rendered, cast);
    Some(rendered)
}

fn is_implicit_trigger_provenance(effect: &Effect) -> bool {
    structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        .is_some()
}

pub(super) fn describe_graveyard_exile_copy_cast(effects: &[Effect]) -> Option<String> {
    let effects = effects
        .iter()
        .skip_while(|effect| is_implicit_trigger_provenance(effect))
        .collect::<Vec<_>>();
    let [exile_effect, may_effect] = effects.as_slice() else {
        return None;
    };
    render_graveyard_exile_copy_cast_pair(exile_effect, may_effect)
        .or_else(|| render_conditional_graveyard_exile_copy_cast_pair(exile_effect, may_effect))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graveyard_copy_cast(exiled_tag: &str, cast_tag: &str) -> Vec<Effect> {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.card_types = vec![CardType::Instant, CardType::Sorcery];
        let exile = Effect::new(crate::effects::ExileEffect::target(ChooseSpec::Object(
            filter,
        )))
        .tag(exiled_tag);
        let cast = Effect::new(
            crate::effects::CastTaggedEffect::new(cast_tag, PlayerFilter::You)
                .without_paying_mana_cost()
                .as_copy(),
        );
        let may = Effect::new(crate::effects::MayEffect::new(vec![cast]));
        vec![exile, may]
    }

    #[test]
    fn graveyard_copy_cast_preserves_only_typed_standard_reminder_surface() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let [exile, may] = graveyard_copy_cast(tag, tag).try_into().unwrap();
        let may = may
            .downcast_ref::<crate::effects::MayEffect>()
            .expect("fixture has an optional cast");
        let mut cast = may.effects[0]
            .downcast_ref::<crate::effects::CastTaggedEffect>()
            .expect("fixture has a typed tagged cast")
            .clone();
        cast.copy_cast_reminder_surface = true;
        let may = Effect::new(crate::effects::MayEffect::new(vec![Effect::new(cast)]));
        let expected = format!(
            "Exile target instant or sorcery card from a graveyard and copy it. You may cast the copy without paying its mana cost {STANDARD_REMINDER_OPEN_SENTINEL}You still pay its costs. A copy of a permanent spell becomes a token.{STANDARD_REMINDER_CLOSE_SENTINEL}"
        );

        assert_eq!(
            render_graveyard_exile_copy_cast_pair(&exile, &may).as_deref(),
            Some(expected.as_str())
        );
    }

    #[test]
    fn graveyard_copy_cast_keeps_typed_authored_instruction_boundaries() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        for (surface, expected) in [
            (
                ironsmith_core::effect::CopyInstructionSurface::SeparateIt,
                "Exile target instant or sorcery card from a graveyard. Copy it. You may cast the copy without paying its mana cost",
            ),
            (
                ironsmith_core::effect::CopyInstructionSurface::SeparateThatCard,
                "Exile target instant or sorcery card from a graveyard. Copy that card. You may cast the copy without paying its mana cost",
            ),
            (
                ironsmith_core::effect::CopyInstructionSurface::SeparateItThen,
                "Exile target instant or sorcery card from a graveyard. Copy it, then you may cast the copy without paying its mana cost",
            ),
        ] {
            let [exile, may] = graveyard_copy_cast(tag, tag).try_into().unwrap();
            let may = may
                .downcast_ref::<crate::effects::MayEffect>()
                .expect("fixture has an optional cast");
            let mut cast = may.effects[0]
                .downcast_ref::<crate::effects::CastTaggedEffect>()
                .expect("fixture has a typed tagged cast")
                .clone();
            cast.copy_instruction_surface = Some(surface);
            let may = Effect::new(crate::effects::MayEffect::new(vec![Effect::new(cast)]));
            assert_eq!(
                render_graveyard_exile_copy_cast_pair(&exile, &may).as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn graveyard_exile_copy_cast_renderer_requires_shared_provenance_tag() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let effects = graveyard_copy_cast(tag, tag);
        assert_eq!(
            describe_graveyard_exile_copy_cast(&effects).as_deref(),
            Some(
                "Exile target instant or sorcery card from a graveyard and copy it. You may cast the copy without paying its mana cost"
            )
        );

        let mismatched = graveyard_copy_cast(tag, "__sentence_helper_exiled_l0_s41_e80");
        assert_eq!(describe_graveyard_exile_copy_cast(&mismatched), None);
    }

    #[test]
    fn standalone_graveyard_move_to_exile_uses_the_same_typed_copy_cast_surface() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.card_types = vec![CardType::Instant, CardType::Sorcery];
        let moved = Effect::new(crate::effects::MoveToZoneEffect::to_exile(
            ChooseSpec::target(ChooseSpec::Object(filter)),
        ))
        .tag(tag);
        let may = graveyard_copy_cast(tag, tag)
            .pop()
            .expect("fixture includes the copied-card cast");

        assert_eq!(
            render_graveyard_exile_copy_cast_pair(&moved, &may).as_deref(),
            Some(
                "Exile target instant or sorcery card from a graveyard and copy it. You may cast the copy without paying its mana cost"
            )
        );

        let wrong_zone = Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::default().in_zone(Zone::Graveyard),
            )),
            Zone::Hand,
            false,
        ))
        .tag(tag);
        assert!(render_graveyard_exile_copy_cast_pair(&wrong_zone, &may).is_none());
    }

    #[test]
    fn graveyard_union_and_nonlibrary_top_bit_keep_copy_cast_provenance() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let mut union = ObjectFilter::default();
        union.any_of = vec![
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .with_type(CardType::Creature),
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .with_ability_marker("freerunning"),
        ];
        let moved = Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::target(ChooseSpec::Object(union)),
            Zone::Exile,
            true,
        ))
        .tag(tag);
        let may = graveyard_copy_cast(tag, tag)
            .pop()
            .expect("fixture includes the copied-card cast");

        assert_eq!(
            render_graveyard_exile_copy_cast_pair(&moved, &may).as_deref(),
            Some(
                "Exile target creature card or card with freerunning from a graveyard and copy it. You may cast the copy without paying its mana cost"
            )
        );

        let mut mixed = ObjectFilter::default();
        mixed.any_of = vec![
            ObjectFilter::default().in_zone(Zone::Graveyard),
            ObjectFilter::default().in_zone(Zone::Hand),
        ];
        let unrelated = Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::target(ChooseSpec::Object(mixed)),
            Zone::Exile,
            true,
        ))
        .tag(tag);
        assert_eq!(
            render_graveyard_exile_copy_cast_pair(&unrelated, &may),
            None,
            "a mixed-zone union must not acquire graveyard copy-cast semantics"
        );
    }

    #[test]
    fn conditional_graveyard_exile_copy_cast_requires_the_exile_result_id() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let [exile, may_cast] = graveyard_copy_cast(tag, tag).try_into().unwrap();
        let exile = Effect::with_id(7, exile);
        let conditional = Effect::new(crate::effects::IfEffect::if_then(
            crate::effect::EffectId(7),
            crate::effect::EffectPredicate::Happened,
            vec![may_cast],
        ));
        assert_eq!(
            describe_graveyard_exile_copy_cast(&[exile.clone(), conditional]).as_deref(),
            Some(
                "Exile target instant or sorcery card from a graveyard. If you do, copy it. You may cast the copy without paying its mana cost"
            )
        );

        let mut unrelated_pair = graveyard_copy_cast(tag, tag);
        let unrelated_may_cast = unrelated_pair
            .pop()
            .expect("graveyard copy/cast fixture includes a may-cast effect");
        let wrong_condition = Effect::new(crate::effects::IfEffect::if_then(
            crate::effect::EffectId(8),
            crate::effect::EffectPredicate::Happened,
            vec![unrelated_may_cast],
        ));
        assert_eq!(
            describe_graveyard_exile_copy_cast(&[exile, wrong_condition]),
            None
        );
    }

    fn coordinated_move_copy(exiled_tag: &str, copy_target_tag: &str) -> Effect {
        let moved = Effect::new(crate::effects::MoveToZoneEffect::to_exile(
            ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::default().in_zone(Zone::Graveyard),
            )),
        ))
        .tag(exiled_tag);
        let copied = Effect::with_id(
            7,
            Effect::new(
                crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from(
                    copy_target_tag,
                )))
                .with_target_reference_pronoun(true),
            ),
        )
        .tag(COPIED_STACK_OBJECT_TAG);
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            moved, copied,
        ]))
    }

    fn coordinated_exile_copy(exiled_tag: &str, copy_target_tag: &str) -> Effect {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.subtypes.push(crate::types::Subtype::Rat);
        filter.supertypes.push(crate::types::Supertype::Legendary);
        filter.set_explicit_card_noun(true);
        let exiled = Effect::new(crate::effects::ExileEffect::with_spec(
            ChooseSpec::target(ChooseSpec::Object(filter))
                .with_count(crate::effect::ChoiceCount::up_to(1)),
        ))
        .tag(exiled_tag);
        let copied = Effect::with_id(
            7,
            Effect::new(
                crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(TagKey::from(
                    copy_target_tag,
                )))
                .with_target_reference_pronoun(true),
            ),
        )
        .tag(COPIED_STACK_OBJECT_TAG);
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            exiled, copied,
        ]))
    }

    #[test]
    fn coordinated_move_copy_requires_shared_exile_provenance() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let may = graveyard_copy_cast(tag, tag)
            .pop()
            .expect("fixture includes optional cast");
        assert_eq!(
            render_graveyard_exile_copy_cast_pair(&coordinated_move_copy(tag, tag), &may)
                .as_deref(),
            Some(
                "Exile target card from a graveyard and copy it. You may cast the copy without paying its mana cost"
            )
        );

        assert!(
            render_graveyard_exile_copy_cast_pair(
                &coordinated_move_copy(tag, "__sentence_helper_exiled_l0_s41_e80"),
                &may,
            )
            .is_none()
        );
    }

    #[test]
    fn coordinated_exile_action_copy_keeps_card_copy_cast_surface_and_reminder() {
        let tag = "__sentence_helper_exiled_l0_s0_e40";
        let cast = crate::effects::CastTaggedEffect::new(tag, PlayerFilter::You)
            .as_copy()
            .with_copy_cast_reminder_surface();
        let may = Effect::new(crate::effects::MayEffect::new(vec![Effect::new(cast)]));
        let expected = format!(
            "Exile up to one target legendary or Rat card from a graveyard and copy it. You may cast the copy {STANDARD_REMINDER_OPEN_SENTINEL}You still pay its costs. A copy of a permanent spell becomes a token.{STANDARD_REMINDER_CLOSE_SENTINEL}"
        );

        assert_eq!(
            render_graveyard_exile_copy_cast_pair(&coordinated_exile_copy(tag, tag), &may)
                .as_deref(),
            Some(expected.as_str())
        );

        assert!(
            render_graveyard_exile_copy_cast_pair(
                &coordinated_exile_copy(tag, "__sentence_helper_exiled_l0_s41_e80"),
                &may,
            )
            .is_none(),
            "the stack-copy bridge must still share the exact exiled-card tag"
        );
    }
}
