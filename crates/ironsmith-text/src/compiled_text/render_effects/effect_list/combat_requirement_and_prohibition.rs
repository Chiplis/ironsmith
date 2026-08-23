use super::*;

fn exact_optional_target_creature(spec: &ChooseSpec) -> bool {
    let ChooseSpec::WithCount(inner, count) = spec else {
        return false;
    };
    if *count != crate::effect::ChoiceCount::up_to(1) {
        return false;
    }
    let ChooseSpec::Target(inner) = inner.as_ref() else {
        return false;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return false;
    };
    let mut expected = ObjectFilter::creature().in_zone(Zone::Battlefield);
    expected.set_explicit_card_type_noun(Some(CardType::Creature));
    filter == &expected
}

fn exact_combat_requirement(effect: &Effect) -> bool {
    let Some(apply) = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    else {
        return false;
    };
    if apply.until != Until::EndOfCombat
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || apply.source_type.is_some()
        || apply.source_reference_surface.is_some()
        || apply.set_quantifier_surface.is_some()
        || apply.type_retention_surface.is_some()
        || apply.animation_pt_surface.is_some()
        || apply.animation_duration_surface.is_some()
        || apply.lock_filter_at_resolution
        || apply.resolve_set_pt_values_at_resolution
        || apply.require_creature_target
        || !apply
            .target_spec
            .as_ref()
            .is_some_and(exact_optional_target_creature)
    {
        return false;
    }
    let Some(crate::continuous::Modification::AddAbility(first)) = &apply.modification else {
        return false;
    };
    let [crate::continuous::Modification::AddAbility(second)] =
        apply.additional_modifications.as_slice()
    else {
        return false;
    };
    first.id() == crate::static_abilities::StaticAbilityId::MustAttack
        && second.id() == crate::static_abilities::StaticAbilityId::MustBlock
}

fn exact_combat_prohibition(target_effect: &Effect, cant_effect: &Effect) -> bool {
    let Some(tag) = wrapped_effect_tag(target_effect) else {
        return false;
    };
    let Some(target) = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    else {
        return false;
    };
    if target.chooser.is_some()
        || target.explicit_declaration
        || !exact_optional_target_creature(&target.target)
    {
        return false;
    }

    let Some(cant) =
        structural_unwrap_render_wrappers(cant_effect).downcast_ref::<crate::effects::CantEffect>()
    else {
        return false;
    };
    let crate::effect::Restriction::AttackOrBlock(filter) = &cant.restriction else {
        return false;
    };
    let mut expected = ObjectFilter::creature().in_zone(Zone::Battlefield);
    expected.set_explicit_card_type_noun(Some(CardType::Creature));
    expected
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: tag.clone(),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
    filter == &expected
        && cant.duration == Until::EndOfCombat
        && cant.start == crate::effect::RestrictionStart::Immediate
        && cant.duration_surface == ironsmith_core::RestrictionDurationSurface::Default
}

/// Restores the authored asymmetric combat pair from its exact executable
/// requirement/target/prohibition structure. Both target counts and both
/// end-of-combat windows are checked independently, so a generic temporary
/// grant or prohibition cannot acquire this surface accidentally.
pub(super) fn describe_combat_requirement_then_prohibition(effects: &[Effect]) -> Option<String> {
    let effects = if let [effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
    {
        if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
            return None;
        }
        sequence.effects.as_slice()
    } else {
        effects
    };
    let [requirement, target, prohibition] = effects else {
        return None;
    };
    if !exact_combat_requirement(requirement) || !exact_combat_prohibition(target, prohibition) {
        return None;
    }
    Some(
        "Up to one target creature attacks or blocks this combat if able and up to one target creature can't attack or block this combat"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(first_duration: Until, second_duration: Until) -> Effect {
        let mut creature = ObjectFilter::creature().in_zone(Zone::Battlefield);
        creature.set_explicit_card_type_noun(Some(CardType::Creature));
        let target = ChooseSpec::target(ChooseSpec::Object(creature.clone()))
            .with_count(crate::effect::ChoiceCount::up_to(1));
        let requirement = Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec(
                target.clone(),
                crate::continuous::Modification::AddAbility(
                    crate::static_abilities::StaticAbility::must_attack(),
                ),
                first_duration,
            )
            .with_additional_modification(crate::continuous::Modification::AddAbility(
                crate::static_abilities::StaticAbility::must_block(),
            )),
        )
        .tag("granted_0");

        let target_tag = crate::tag::TagKey::from("targeted_1");
        let target_only =
            Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(target_tag.clone());
        creature
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: target_tag,
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        let prohibition = Effect::new(crate::effects::CantEffect::new(
            crate::effect::Restriction::attack_or_block(creature),
            second_duration,
        ));
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            requirement,
            target_only,
            prohibition,
        ]))
    }

    #[test]
    fn asymmetric_combat_pair_preserves_requirement_and_prohibition_surfaces() {
        assert_eq!(
            describe_combat_requirement_then_prohibition(&[fixture(
                Until::EndOfCombat,
                Until::EndOfCombat,
            )])
            .as_deref(),
            Some(
                "Up to one target creature attacks or blocks this combat if able and up to one target creature can't attack or block this combat"
            )
        );
    }

    #[test]
    fn asymmetric_combat_pair_rejects_a_turn_long_prohibition() {
        assert!(
            describe_combat_requirement_then_prohibition(&[fixture(
                Until::EndOfCombat,
                Until::EndOfTurn,
            )])
            .is_none()
        );
    }
}
