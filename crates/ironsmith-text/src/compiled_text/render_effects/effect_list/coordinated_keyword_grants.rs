use super::*;

fn keyword_grant(
    effect: &Effect,
) -> Option<(
    &crate::effects::ApplyContinuousEffect,
    crate::static_abilities::StaticAbilityId,
)> {
    let apply = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !apply.additional_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
        return None;
    };
    keyword_label_from_static_ability_id(ability.id()).map(|_| (apply, ability.id()))
}

fn same_grant_shell(
    first: &crate::effects::ApplyContinuousEffect,
    candidate: &crate::effects::ApplyContinuousEffect,
    first_anchor: Option<&TagKey>,
) -> bool {
    let same_direct_target =
        first.target == candidate.target && first.target_spec == candidate.target_spec;
    let same_anchored_target = if same_direct_target {
        true
    } else {
        let Some(anchor) = first_anchor else {
            return false;
        };
        let Some(candidate_spec) = candidate.target_spec.as_ref() else {
            return false;
        };
        choose_spec_references_exact_tag(candidate_spec, anchor)
    };
    same_anchored_target
        && first.until == candidate.until
        && first.condition == candidate.condition
        && (!same_direct_target
            || first.lock_filter_at_resolution == candidate.lock_filter_at_resolution)
        && first.resolve_set_pt_values_at_resolution
            == candidate.resolve_set_pt_values_at_resolution
        && first.require_creature_target == candidate.require_creature_target
}

pub(super) fn describe_coordinated_keyword_grants(effects: &[Effect]) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let (first, first_id) = keyword_grant(effects.first()?)?;
    let first_anchor = wrapped_effect_tag(effects.first()?);
    let mut labels = vec![keyword_label_from_static_ability_id(first_id)?.to_string()];
    for effect in &effects[1..] {
        let (candidate, ability_id) = keyword_grant(effect)?;
        if !same_grant_shell(first, candidate, first_anchor) {
            return None;
        }
        let label = keyword_label_from_static_ability_id(ability_id)?.to_string();
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    if labels.len() != effects.len() {
        return None;
    }

    let first_text = describe_effect(effects.first()?)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let first_suffix = format!(" {} until end of turn", labels[0]);
    let subject_and_verb = first_text.strip_suffix(&first_suffix)?;
    Some(format!(
        "{subject_and_verb} {} until end of turn",
        join_with_and(&labels)
    ))
}

pub(super) fn describe_put_counters_then_coordinated_keyword_grants(
    effects: &[Effect],
) -> Option<String> {
    let [put_effect, grant_effects @ ..] = effects else {
        return None;
    };
    if grant_effects.len() < 2 {
        return None;
    }
    let (put_text, put_filter, put_tag) = put_counters_each_filter_view(put_effect)?;
    if !put_filter.card_types.contains(&CardType::Creature) {
        return None;
    }
    let (first_grant, _) = keyword_grant(grant_effects.first()?)?;
    let same_group = if let Some(put_tag) = put_tag {
        first_grant
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_references_exact_tag(spec, put_tag))
    } else {
        apply_continuous_filter(first_grant).is_some_and(|filter| filter == put_filter)
    };
    if !same_group {
        return None;
    }
    let grant_text = describe_coordinated_keyword_grants(grant_effects)?;
    let grant_tail = grant_text
        .split_once(" gains ")
        .or_else(|| grant_text.split_once(" gain "))?
        .1;
    Some(format!("{put_text}. Those creatures gain {grant_tail}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(ability: crate::static_abilities::StaticAbility, until: Until) -> Effect {
        Effect::new(crate::effects::ApplyContinuousEffect::new(
            crate::continuous::EffectTarget::Filter(
                ObjectFilter::default()
                    .with_subtype(Subtype::Gremlin)
                    .you_control(),
            ),
            crate::continuous::Modification::AddAbility(ability),
            until,
        ))
    }

    #[test]
    fn same_target_keyword_grants_share_one_plural_verb_and_duration() {
        let effects = vec![
            grant(
                crate::static_abilities::StaticAbility::menace(),
                Until::EndOfTurn,
            ),
            grant(
                crate::static_abilities::StaticAbility::lifelink(),
                Until::EndOfTurn,
            ),
            grant(
                crate::static_abilities::StaticAbility::haste(),
                Until::EndOfTurn,
            ),
        ];
        assert_eq!(
            describe_coordinated_keyword_grants(&effects).as_deref(),
            Some("Gremlins you control gain menace, lifelink, and haste until end of turn")
        );

        let changed_duration = vec![
            effects[0].clone(),
            grant(
                crate::static_abilities::StaticAbility::lifelink(),
                Until::Forever,
            ),
            effects[2].clone(),
        ];
        assert!(describe_coordinated_keyword_grants(&changed_duration).is_none());

        let changed_target = vec![
            effects[0].clone(),
            Effect::new(crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(
                    ObjectFilter::default()
                        .with_subtype(Subtype::Goblin)
                        .you_control(),
                ),
                crate::continuous::Modification::AddAbility(
                    crate::static_abilities::StaticAbility::lifelink(),
                ),
                Until::EndOfTurn,
            )),
            effects[2].clone(),
        ];
        assert!(describe_coordinated_keyword_grants(&changed_target).is_none());

        let countered = TagKey::from("countered");
        let put = Effect::put_counters(
            CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::all(ObjectFilter::creature().you_control()),
        )
        .tag(countered.clone());
        let tagged_grant = |ability| {
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Source,
                crate::continuous::Modification::AddAbility(ability),
                Until::EndOfTurn,
            );
            apply.target_spec = Some(ChooseSpec::Tagged(countered.clone()));
            Effect::new(apply)
        };
        let counter_then_grants = vec![
            put,
            tagged_grant(crate::static_abilities::StaticAbility::vigilance()),
            tagged_grant(crate::static_abilities::StaticAbility::trample()),
            tagged_grant(crate::static_abilities::StaticAbility::indestructible()),
        ];
        assert_eq!(
            describe_put_counters_then_coordinated_keyword_grants(&counter_then_grants).as_deref(),
            Some(
                "Put a +1/+1 counter on each creature you control. Those creatures gain vigilance, trample, and indestructible until end of turn"
            )
        );
    }
}
