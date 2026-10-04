use super::*;
use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};

/// This presentation follows four typed operations and their exact identities,
/// never a card name or saved Oracle string.
pub(in crate::compiled_text) fn describe_reciprocal_group_power_damage(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen
        || sequence.result_label.is_some()
    {
        return None;
    }
    let [declaration, capture, first, second] = sequence.effects.as_slice() else {
        return None;
    };
    let source_tag = super::helpers_00::wrapped_effect_tag(declaration)?;
    let declaration = structural_unwrap_render_wrappers(declaration)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if declaration.explicit_declaration
        || declaration.chooser.is_some()
        || !declaration.target.is_target()
        || declaration.target.count() != crate::effect::ChoiceCount::exactly(1)
    {
        return None;
    }
    let capture = structural_unwrap_render_wrappers(capture)
        .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    if capture.zone != Some(Zone::Battlefield)
        || !capture.additional_zones.is_empty()
        || !capture.source_tags.is_empty()
    {
        return None;
    }
    let mut expected = ObjectFilter::creature();
    expected.zone = Some(Zone::Battlefield);
    expected.controller = Some(PlayerFilter::AliasedControllerOf(
        crate::target::ObjectRef::Tagged(source_tag.clone()),
    ));
    expected.tagged_constraints.push(TaggedObjectConstraint {
        tag: source_tag.clone(),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    if capture.filter != expected {
        return None;
    }
    let first = structural_unwrap_render_wrappers(first)
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(first.source.base(),ChooseSpec::Tagged(tag) if tag==source_tag) {
        return None;
    }
    let first = structural_unwrap_render_wrappers(&first.effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if first.amount.unhinted() != &Value::SourcePower
        || first.source_is_combat
        || first.unpreventable
        || first.excess_to_controller.is_some()
    {
        return None;
    }
    let mut members = ObjectFilter::default();
    members.zone = Some(Zone::Battlefield);
    members.tagged_constraints.push(TaggedObjectConstraint {
        tag: capture.tag.clone(),
        relation: TaggedOpbjectRelation::SameObjectId,
    });
    members.set_plural_object_noun_surface(true);
    members.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));
    if !matches!(first.target.base(),ChooseSpec::All(filter) if *filter==members) {
        return None;
    }
    let second = structural_unwrap_render_wrappers(second)
        .downcast_ref::<crate::effects::DealDamageBySourcesEffect>()?;
    if second.source_binding != ironsmith_core::DamageSourceSetBinding::CapturedIncarnations
        || second.recipient_binding != ironsmith_core::DamageRecipientSetBinding::SharedSet
        || second.unpreventable
        || second.amount.unhinted() != &Value::SourcePower
        || !matches!(second.sources.as_slice(),[source] if matches!(source.base(),ChooseSpec::Tagged(tag) if *tag==capture.tag))
    {
        return None;
    }
    let mut original = ObjectFilter::default();
    original.zone = Some(Zone::Battlefield);
    original.tagged_constraints.push(TaggedObjectConstraint {
        tag: source_tag.clone(),
        relation: TaggedOpbjectRelation::SameObjectId,
    });
    original.set_singular_pronoun_reference_surface(true);
    if !matches!(second.target.base(),ChooseSpec::Object(filter) if *filter==original) {
        return None;
    }
    Some(format!(
        "{} deals damage equal to its power to each other creature that player controls, then each of those creatures deals damage equal to its power to that creature",
        capitalize_first(&describe_choose_spec(&declaration.target))
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn program() -> crate::effects::SequenceEffect {
        let source = crate::TagKey::from("original");
        let others = crate::TagKey::from("others");
        let declare = Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_creature(),
        ))
        .tag(source.clone());
        let mut filter = ObjectFilter::creature();
        filter.zone = Some(Zone::Battlefield);
        filter.controller = Some(PlayerFilter::AliasedControllerOf(
            crate::target::ObjectRef::Tagged(source.clone()),
        ));
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: source.clone(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
        let capture = Effect::new(
            crate::effects::TagMatchingObjectsEffect::new(filter, others.clone())
                .in_zone(Zone::Battlefield),
        );
        let mut members = ObjectFilter::default();
        members.zone = Some(Zone::Battlefield);
        members.tagged_constraints.push(TaggedObjectConstraint {
            tag: others.clone(),
            relation: TaggedOpbjectRelation::SameObjectId,
        });
        members.set_plural_object_noun_surface(true);
        members.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));
        let first = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::Tagged(source.clone()),
            Effect::deal_damage(Value::SourcePower, ChooseSpec::All(members)),
        ));
        let mut original = ObjectFilter::default();
        original.zone = Some(Zone::Battlefield);
        original.tagged_constraints.push(TaggedObjectConstraint {
            tag: source,
            relation: TaggedOpbjectRelation::SameObjectId,
        });
        original.set_singular_pronoun_reference_surface(true);
        let second = Effect::new(
            crate::effects::DealDamageBySourcesEffect::new(
                vec![ChooseSpec::Tagged(others)],
                Value::SourcePower,
                ChooseSpec::Object(original),
            )
            .with_source_binding(ironsmith_core::DamageSourceSetBinding::CapturedIncarnations),
        );
        crate::effects::SequenceEffect::comma_then(vec![declare, capture, first, second])
    }
    #[test]
    fn reciprocal_rendering_requires_the_complete_typed_identity_and_lki_contract() {
        let sequence = program();
        let rendered = describe_reciprocal_group_power_damage(&sequence).unwrap();
        assert!(rendered.contains("then each of those creatures"));
        assert!(!rendered.contains("tagged"));
        let mut changed = sequence.clone();
        let mut second = changed.effects[3]
            .downcast_ref::<crate::effects::DealDamageBySourcesEffect>()
            .unwrap()
            .clone();
        second.source_binding = ironsmith_core::DamageSourceSetBinding::LiveMembers;
        changed.effects[3] = Effect::new(second);
        assert!(describe_reciprocal_group_power_damage(&changed).is_none());
        let mut changed = sequence.clone();
        let mut capture = changed.effects[1]
            .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
            .unwrap()
            .clone();
        capture.filter.controller = Some(PlayerFilter::You);
        changed.effects[1] = Effect::new(capture);
        assert!(describe_reciprocal_group_power_damage(&changed).is_none());
        let mut changed = sequence;
        let mut second = changed.effects[3]
            .downcast_ref::<crate::effects::DealDamageBySourcesEffect>()
            .unwrap()
            .clone();
        second.amount = Value::Fixed(3);
        changed.effects[3] = Effect::new(second);
        assert!(describe_reciprocal_group_power_damage(&changed).is_none());
    }
}
