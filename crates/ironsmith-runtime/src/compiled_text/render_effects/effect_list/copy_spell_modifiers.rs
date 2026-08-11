use super::*;

/// Rejoin a spell-copy action with the typed characteristic change applied to
/// the produced copy.
///
/// Lowering keeps the copy and its modification as separate executable
/// effects so the created stack object can be addressed by tag. The shared
/// tag, permanent duration, and explicit type-retention surface prove that
/// the second effect is the authored copy exception rather than an unrelated
/// later type-changing instruction.
pub(super) fn describe_copy_spell_with_characteristic_modifiers(
    effects: &[Effect],
) -> Option<String> {
    let (copy_effect, modification_effect, retarget_effect) = match effects {
        [copy_effect, modification_effect] => (copy_effect, modification_effect, None),
        [copy_effect, modification_effect, retarget_effect] => {
            (copy_effect, modification_effect, Some(retarget_effect))
        }
        _ => return None,
    };

    let tagged_copy = copy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if tagged_copy.tag.as_str() != "__copied_stack_object__" {
        return None;
    }
    let copy = copy_spell_from_effect(copy_effect)?;
    if copy.count != Value::Fixed(1)
        || !copy.removed_supertypes.is_empty()
        || copy.has_characteristic_modifiers()
    {
        return None;
    }

    let apply = structural_unwrap_render_wrappers(modification_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::Forever
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !matches!(
            apply.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag == &tagged_copy.tag
        )
    {
        return None;
    }

    let modifications = apply
        .modification
        .iter()
        .chain(apply.additional_modifications.iter())
        .collect::<Vec<_>>();
    let exception = match modifications.as_slice() {
        [crate::continuous::Modification::AddCardTypes(card_types)]
            if !card_types.is_empty()
                && apply.type_retention_surface
                    == Some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes) =>
        {
            let copied_types = card_types
                .iter()
                .map(|card_type| card_type.name())
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "except the copy is {} in addition to its other types",
                with_indefinite_article(&copied_types)
            )
        }
        [crate::continuous::Modification::SetColors(colors)]
            if !colors.is_empty() && apply.type_retention_surface.is_none() =>
        {
            format!(
                "except that the copy is {}",
                describe_token_color_words(*colors, false)
            )
        }
        [
            crate::continuous::Modification::AddSubtypes(subtypes),
            crate::continuous::Modification::SetPowerToughness {
                power: Value::Fixed(power),
                toughness: Value::Fixed(toughness),
                sublayer: crate::continuous::PtSublayer::Setting,
            },
        ] if !subtypes.is_empty()
            && apply.type_retention_surface
                == Some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes) =>
        {
            let subtypes = subtypes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "except the copy is a {power}/{toughness} {subtypes} in addition to its other types"
            )
        }
        _ => return None,
    };

    let copy_text = describe_effect(copy_effect);
    let mut text = format!("{}, {exception}", copy_text.trim_end_matches('.'));

    if let Some(retarget_effect) = retarget_effect {
        fn with_id(effect: &Effect) -> Option<&crate::effects::WithIdEffect> {
            if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                return Some(with_id);
            }
            let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
            with_id(&tagged.effect)
        }

        let copy_with_id = with_id(copy_effect)?;
        let retarget = structural_unwrap_render_wrappers(retarget_effect)
            .downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
        if retarget.from_effect != copy_with_id.id
            || !retarget.may
            || !matches!(retarget.chooser, None | Some(PlayerFilter::You))
        {
            return None;
        }
        let targets = if retarget.single_target_surface {
            "a new target"
        } else {
            "new targets"
        };
        text.push_str(&format!(". You may choose {targets} for the copy"));
    }

    Some(text)
}
