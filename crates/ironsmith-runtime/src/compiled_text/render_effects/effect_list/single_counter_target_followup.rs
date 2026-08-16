use super::*;

fn single_creature_counter_target(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::PutCountersEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let put = structural_unwrap_render_wrappers(&tagged.effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed
        || put.target_count.is_some()
        || !put.target.is_target()
        || !put.target.is_single()
    {
        return None;
    }
    let ChooseSpec::Object(filter) = put.target.base() else {
        return None;
    };
    let mut semantic_filter = filter.clone();
    semantic_filter.controller = None;
    semantic_filter.union_surface = Default::default();
    if semantic_filter != ObjectFilter::creature().in_zone(Zone::Battlefield) {
        return None;
    }
    Some((&tagged.tag, put))
}

fn is_exact_countered_creature_reference(spec: &ChooseSpec, tag: &TagKey) -> bool {
    if matches!(spec.base(), ChooseSpec::Tagged(found) if found == tag) {
        return true;
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        return false;
    };
    let [membership] = filter.tagged_constraints.as_slice() else {
        return false;
    };
    if membership.tag != *tag
        || membership.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return false;
    }
    let mut semantic_filter = filter.clone();
    semantic_filter.tagged_constraints.clear();
    semantic_filter.union_surface = Default::default();
    semantic_filter == ObjectFilter::creature().in_zone(Zone::Battlefield)
}

fn exact_resolving_tap_or_untap_choice(effect: &Effect, tag: &TagKey) -> bool {
    let Some(may) =
        structural_unwrap_render_wrappers(effect).downcast_ref::<crate::effects::MayEffect>()
    else {
        return false;
    };
    if may.decider != Some(PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return false;
    }
    let [choice_effect] = may.effects.as_slice() else {
        return false;
    };
    let Some(choice) = structural_unwrap_render_wrappers(choice_effect)
        .downcast_ref::<crate::effects::ChooseModeEffect>()
    else {
        return false;
    };
    if choice.chooser.is_some()
        || choice.min != Value::Fixed(1)
        || choice.max != Value::Fixed(1)
        || choice.choose_count != Value::Fixed(1)
        || choice.min_choose_count != Value::Fixed(1)
        || choice.allow_repeat
        || choice.random
        || choice.allow_repeated_modes
        || choice.spree
        || choice.tiered
        || !choice.common_prefix_effects.is_empty()
        || choice.common_suffix_effect_count != 0
        || !choice.mode_additional_mana_costs.is_empty()
        || choice.mode_point_costs != [1, 1]
        || choice.disallow_previously_chosen_modes
        || choice.disallow_previously_chosen_modes_this_turn
        || choice.distinct_player_targets_per_mode
        || choice.conditional_mode_range.is_some()
        || choice.presentation_label.is_some()
    {
        return false;
    }
    let [first, second] = choice.modes.as_slice() else {
        return false;
    };
    let exact_mode = |mode: &crate::effect::EffectMode, expected_name: &str, tap: bool| -> bool {
        if mode.source_text != expected_name {
            return false;
        }
        let [effect] = mode.effects.as_slice() else {
            return false;
        };
        let effect = structural_unwrap_render_wrappers(effect);
        if tap {
            effect
                .downcast_ref::<crate::effects::TapEffect>()
                .is_some_and(|effect| is_exact_countered_creature_reference(&effect.target, tag))
        } else {
            effect
                .downcast_ref::<crate::effects::UntapEffect>()
                .is_some_and(|effect| is_exact_countered_creature_reference(&effect.target, tag))
        }
    };
    exact_mode(first, "Tap", true) && exact_mode(second, "Untap", false)
}

fn describe_counter_producer(effect: &Effect) -> String {
    describe_effect(effect).trim_end_matches('.').to_string()
}

fn exact_fight_followup(
    target_effect: &Effect,
    fight_effect: &Effect,
    tag: &TagKey,
) -> Option<String> {
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let fight_target = exact_fight_target(fight_effect, tag)?;
    let count = target.target.count();
    if target.chooser.is_some()
        || target.explicit_declaration
        || !target.target.is_target()
        || count.max != Some(1)
        || count.dynamic_x
        || count.random
        || fight_target != &target.target
    {
        return None;
    }
    Some(describe_choose_spec(&target.target))
}

fn exact_fight_target<'a>(fight_effect: &'a Effect, tag: &TagKey) -> Option<&'a ChooseSpec> {
    let fight = structural_unwrap_render_wrappers(fight_effect)
        .downcast_ref::<crate::effects::FightEffect>()?;
    let count = fight.creature2.count();
    if !is_exact_countered_creature_reference(&fight.creature1, tag)
        || !fight.creature2.is_target()
        || count.max != Some(1)
        || count.dynamic_x
        || count.random
    {
        return None;
    }
    Some(&fight.creature2)
}

pub(in crate::compiled_text) fn describe_single_counter_target_followup_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, consumer_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if producer_segment.starts_new_source_line
        || consumer_segment.starts_new_source_line
        || !producer_segment.self_replacements.is_empty()
        || !consumer_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [producer] = producer_segment.default_effects.as_slice() else {
        return None;
    };
    let (tag, _) = single_creature_counter_target(producer)?;
    let prefix = describe_counter_producer(producer);

    let rendered = match consumer_segment.default_effects.as_slice() {
        [consumer] => {
            let consumer = structural_unwrap_render_wrappers(consumer);
            if consumer
                .downcast_ref::<crate::effects::UntapEffect>()
                .is_some_and(|untap| is_exact_countered_creature_reference(&untap.target, tag))
            {
                format!("{prefix}. Untap that creature")
            } else if exact_resolving_tap_or_untap_choice(consumer, tag) {
                format!("{prefix}. You may tap or untap that creature")
            } else {
                return None;
            }
        }
        [_, _] => {
            let target = exact_fight_followup(
                &consumer_segment.default_effects[0],
                &consumer_segment.default_effects[1],
                tag,
            )?;
            format!("{prefix}. Then that creature fights {target}",)
        }
        _ => return None,
    };
    Some((rendered, 2))
}

pub(in crate::compiled_text) fn describe_single_counter_target_then_fight(
    effects: &[Effect],
) -> Option<String> {
    let (producer, target) = match effects {
        [producer, fight_effect] => {
            let (tag, _) = single_creature_counter_target(producer)?;
            (producer, exact_fight_target(fight_effect, tag)?)
        }
        [first, second, third] => {
            let (producer, target_effect, fight_effect) =
                if single_creature_counter_target(first).is_some() {
                    (first, second, third)
                } else if single_creature_counter_target(second).is_some() {
                    (second, first, third)
                } else {
                    return None;
                };
            let (tag, _) = single_creature_counter_target(producer)?;
            let target_text = exact_fight_followup(target_effect, fight_effect, tag)?;
            return Some(format!(
                "{}. Then that creature fights {target_text}",
                describe_counter_producer(producer)
            ));
        }
        _ => return None,
    };
    Some(format!(
        "{}. Then that creature fights {}",
        describe_counter_producer(producer),
        describe_choose_spec(target)
    ))
}

pub(in crate::compiled_text) fn describe_single_counter_target_then_double(
    effects: &[Effect],
) -> Option<String> {
    let [producer, double_effect] = effects else {
        return None;
    };
    let (tag, put) = single_creature_counter_target(producer)?;
    if !put
        .amount
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupThen)
    {
        return None;
    }
    let double = structural_unwrap_render_wrappers(double_effect)
        .downcast_ref::<crate::effects::DoubleCountersEffect>()?;
    if double.counter_type != Some(put.counter_type)
        || !is_exact_countered_creature_reference(&double.target, tag)
    {
        return None;
    }
    Some(format!(
        "{}, then double the number of {} counters on that creature",
        describe_counter_producer(producer),
        put.counter_type.description()
    ))
}

pub(in crate::compiled_text) fn describe_single_counter_target_self_replacement(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    let [default_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let [branch] = segment.self_replacements.as_slice() else {
        return None;
    };
    if !branch.condition_after_replacement
        || branch.leading_instead_surface
        || !branch.starts_new_source_line
    {
        return None;
    }
    let (tag, default_put) = single_creature_counter_target(default_effect)?;
    let [replacement_effect] = branch.replacement_effects.as_slice() else {
        return None;
    };
    let replacement_tagged = replacement_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let replacement = structural_unwrap_render_wrappers(&replacement_tagged.effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if replacement_tagged.tag != *tag
        || replacement.counter_type != default_put.counter_type
        || replacement.target_count.is_some()
        || replacement.distributed
        || !is_exact_countered_creature_reference(&replacement.target, tag)
    {
        return None;
    }
    Some(format!(
        "{}. Put {} on that creature instead if {}",
        describe_counter_producer(default_effect),
        describe_put_counter_phrase(&replacement.amount, replacement.counter_type),
        describe_condition(&branch.condition)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};

    fn counter_target(tag: &TagKey) -> Effect {
        Effect::put_counters(
            CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::creature()
                    .in_zone(Zone::Battlefield)
                    .you_control(),
            )),
        )
        .tag(tag.clone())
    }

    fn tagged_creature(tag: &TagKey) -> ChooseSpec {
        let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        ChooseSpec::Object(filter)
    }

    #[test]
    fn singular_counter_target_untap_uses_that_creature() {
        let tag = TagKey::from("counters_0");
        let segments = vec![
            crate::resolution::ResolutionSegment::from_effects(vec![counter_target(&tag)]),
            crate::resolution::ResolutionSegment::from_effects(vec![Effect::untap(
                tagged_creature(&tag),
            )]),
        ];
        assert_eq!(
            describe_single_counter_target_followup_window(&segments, 0),
            Some((
                "Put a +1/+1 counter on target creature you control. Untap that creature"
                    .to_string(),
                2,
            ))
        );

        let mut changed = segments;
        changed[1].default_effects = vec![Effect::untap(tagged_creature(&TagKey::from("other")))];
        assert!(describe_single_counter_target_followup_window(&changed, 0).is_none());
    }

    #[test]
    fn singular_counter_target_double_requires_same_counter_and_tag() {
        let tag = TagKey::from("counters_0");
        let producer = Effect::put_counters(
            CounterType::PlusOnePlusOne,
            Value::Fixed(1)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::CounterFollowupThen),
            ChooseSpec::target_creature(),
        )
        .tag(tag.clone());
        let effects = vec![
            producer,
            Effect::double_counters(Some(CounterType::PlusOnePlusOne), tagged_creature(&tag)),
        ];
        assert_eq!(
            describe_single_counter_target_then_double(&effects).as_deref(),
            Some(
                "Put a +1/+1 counter on target creature, then double the number of +1/+1 counters on that creature"
            )
        );

        let wrong_counter = vec![
            effects[0].clone(),
            Effect::double_counters(Some(CounterType::MinusOneMinusOne), tagged_creature(&tag)),
        ];
        assert!(describe_single_counter_target_then_double(&wrong_counter).is_none());
    }

    #[test]
    fn singular_counter_target_fight_requires_the_producer_tag() {
        let tag = TagKey::from("counters_0");
        let opponent = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature()
                .in_zone(Zone::Battlefield)
                .controlled_by(PlayerFilter::NotYou),
        ))
        .with_count(ChoiceCount::up_to(1));
        let effects = vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(opponent.clone())),
            counter_target(&tag),
            Effect::fight(tagged_creature(&tag), opponent.clone()),
        ];
        assert_eq!(
            describe_single_counter_target_then_fight(&effects).as_deref(),
            Some(
                "Put a +1/+1 counter on target creature you control. Then that creature fights up to one target creature you don't control"
            )
        );

        let wrong_tag = vec![
            effects[0].clone(),
            effects[1].clone(),
            Effect::fight(tagged_creature(&TagKey::from("other")), opponent),
        ];
        assert!(describe_single_counter_target_then_fight(&wrong_tag).is_none());
    }
}
