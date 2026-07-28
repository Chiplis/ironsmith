use super::*;

/// Rejoin a spell-copy action with the typed characteristic change applied to
/// the produced copy.
///
/// Lowering keeps the copy and its modification as separate executable
/// effects so the created stack object can be addressed by tag. The shared
/// tag, permanent duration, and explicit type-retention surface prove that
/// the second effect is the authored copy exception rather than an unrelated
/// later type-changing instruction.
pub(super) fn describe_copy_spell_with_added_card_types(effects: &[Effect]) -> Option<String> {
    let [copy_effect, modification_effect] = effects else {
        return None;
    };

    let tagged_copy = copy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if tagged_copy.tag.as_str() != "__copied_stack_object__" {
        return None;
    }
    let copy = copy_spell_from_effect(copy_effect)?;
    if copy.count != Value::Fixed(1) || !copy.removed_supertypes.is_empty() {
        return None;
    }

    let apply = structural_unwrap_render_wrappers(modification_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let Some(crate::continuous::Modification::AddCardTypes(card_types)) = &apply.modification
    else {
        return None;
    };
    if card_types.is_empty()
        || apply.until != Until::Forever
        || apply.condition.is_some()
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || apply.type_retention_surface
            != Some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes)
        || !matches!(
            apply.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag == &tagged_copy.tag
        )
    {
        return None;
    }

    let copied_types = card_types
        .iter()
        .map(|card_type| card_type.name())
        .collect::<Vec<_>>()
        .join(" ");
    let copy_text = describe_effect(copy_effect);
    Some(format!(
        "{}, except the copy is {} in addition to its other types",
        copy_text.trim_end_matches('.'),
        with_indefinite_article(&copied_types)
    ))
}
