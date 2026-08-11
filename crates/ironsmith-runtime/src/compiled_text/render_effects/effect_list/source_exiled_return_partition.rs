use super::*;

/// Compact a counted return from the source-linked exile set followed by the
/// exact complement moving to each card owner's library.
pub(in crate::compiled_text) fn describe_source_exiled_return_partition(
    effects: &[Effect],
) -> Option<String> {
    let effects = match effects {
        [trigger_tag_effect, sequence_effect]
            if trigger_tag_effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some_and(|tag| tag.tag.as_str() == "triggering") =>
        {
            let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
            if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
                || sequence.result_label.is_some()
            {
                return None;
            }
            sequence.effects.as_slice()
        }
        effects => effects,
    };
    let [return_effect, remainder_effect] = effects else {
        return None;
    };
    let returned_tag = wrapped_effect_tag(return_effect)?;
    let returned = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::WithCount(inner, count) = returned.target.unhinted() else {
        return None;
    };
    let expected_filter = ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile);
    let source_exiled_set = match inner.base() {
        // Lowering simplifies the exact tagged-object filter to a direct tag
        // choice. Keep accepting the unsimplified representation as well so
        // this renderer follows semantics rather than an optimizer detail.
        ChooseSpec::Tagged(tag) => tag.as_str() == crate::tag::SOURCE_EXILED_TAG,
        ChooseSpec::Object(filter) => filter == &expected_filter,
        _ => false,
    };
    let surface = returned.exiled_with_source_surface.as_ref()?;
    if !source_exiled_set
        || count.min == 0
        || count.max != Some(count.min)
        || count.dynamic_x
        || returned.zone != Zone::Battlefield
        || returned.to_top
        || returned.library_order.is_some()
        || returned.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Return
        || returned.battlefield_controller != crate::effects::BattlefieldController::Owner
        || !returned.controller_surface_explicit
        || returned.enters_tapped
        || returned.enters_attacking
        || returned.enters_face_down
        || returned.enters_transformed
        || !returned.enters_with_counters.is_empty()
        || !matches!(
            surface.source,
            ironsmith_core::ExiledWithSourceReferenceSurface::Source(_)
        )
    {
        return None;
    }

    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if remainder.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
        || remainder.controller_at_last_blocked_by.is_some()
    {
        return None;
    }
    let [conditional_effect] = remainder.effects.as_slice() else {
        return None;
    };
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, membership) =
        &conditional.condition
    else {
        return None;
    };
    let expected_membership = ObjectFilter {
        tagged_constraints: vec![crate::target::TaggedObjectConstraint {
            tag: returned_tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::SameStableId,
        }],
        ..Default::default()
    };
    if condition_tag.as_str() != "__it__"
        || membership != &expected_membership
        || !conditional.if_true.is_empty()
        || conditional.if_false.len() != 1
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
    {
        return None;
    }
    let bottom = structural_unwrap_render_wrappers(&conditional.if_false[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let expected_bottom =
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Library, false);
    let expected_bottom_with_surface = expected_bottom
        .clone()
        .with_remainder_surface(ironsmith_core::LibraryRemainderSurface::Rest);
    if bottom != &expected_bottom && bottom != &expected_bottom_with_surface {
        return None;
    }

    Some(format!(
        "{} and put the rest on the bottom of their owners' libraries",
        describe_effect(return_effect).trim().trim_end_matches('.')
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(returned_tag: &str, condition_tag: &str, membership_tag: &str) -> Vec<Effect> {
        let source_surface = ironsmith_core::ExiledWithSourceMoveSurface {
            verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Return,
            subject: ironsmith_core::ExiledWithSourceSubjectSurface::Custom(
                "two cards".to_string(),
            ),
            source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(
                crate::target::SourceReferenceSurface::ThisPermanentType("this Saga".to_string()),
            ),
            destination: ironsmith_core::ExiledWithSourceDestinationSurface::TheirOwners,
        };
        let return_effect = Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(TagKey::from(crate::tag::SOURCE_EXILED_TAG))
                    .with_count(ChoiceCount::exactly(2)),
                Zone::Battlefield,
                false,
            )
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
            .with_exiled_with_source_surface(source_surface)
            .under_owner_control(),
        )
        .tag_all(returned_tag);
        let membership = ObjectFilter {
            tagged_constraints: vec![crate::target::TaggedObjectConstraint {
                tag: TagKey::from(membership_tag),
                relation: crate::filter::TaggedOpbjectRelation::SameStableId,
            }],
            ..Default::default()
        };
        let bottom = Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Iterated,
            Zone::Library,
            false,
        ));
        let remainder = Effect::for_each_tagged(
            crate::tag::SOURCE_EXILED_TAG,
            vec![Effect::conditional(
                crate::effect::Condition::TaggedObjectMatches(
                    TagKey::from(condition_tag),
                    membership,
                ),
                Vec::new(),
                vec![bottom],
            )],
        );
        vec![return_effect, remainder]
    }

    #[test]
    fn exact_source_exiled_partition_renders_as_one_instruction() {
        let effects = program("returned_0", "__it__", "returned_0");
        assert_eq!(
            describe_source_exiled_return_partition(&effects).as_deref(),
            Some(
                "Return two cards exiled with this Saga to the battlefield under their owners' control and put the rest on the bottom of their owners' libraries"
            )
        );

        let changed_correlation = program("returned_0", "__it__", "different_set");
        assert!(describe_source_exiled_return_partition(&changed_correlation).is_none());

        let self_comparison = program("returned_0", "__it__", "__it__");
        assert!(
            describe_source_exiled_return_partition(&self_comparison).is_none(),
            "a candidate compared with itself does not prove the returned-set complement"
        );

        let wrapped = vec![
            Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering")),
            Effect::new(crate::effects::SequenceEffect::coordinated(effects)),
        ];
        assert_eq!(
            describe_source_exiled_return_partition(&wrapped).as_deref(),
            Some(
                "Return two cards exiled with this Saga to the battlefield under their owners' control and put the rest on the bottom of their owners' libraries"
            )
        );

        let wrong_prefix = vec![
            Effect::new(crate::effects::TagTriggeringObjectEffect::new("other")),
            wrapped[1].clone(),
        ];
        assert!(describe_source_exiled_return_partition(&wrong_prefix).is_none());
    }
}
