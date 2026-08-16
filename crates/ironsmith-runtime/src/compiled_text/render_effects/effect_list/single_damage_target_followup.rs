use super::*;

fn single_creature_damage_target(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::DealDamageEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let damage = structural_unwrap_render_wrappers(&tagged.effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !damage.target.is_target() || !damage.target.is_single() {
        return None;
    }
    let ChooseSpec::Object(filter) = damage.target.base() else {
        return None;
    };
    let mut semantic_filter = filter.clone();
    semantic_filter.controller = None;
    semantic_filter.union_surface = Default::default();
    if semantic_filter != ObjectFilter::creature().in_zone(Zone::Battlefield) {
        return None;
    }
    Some((&tagged.tag, damage))
}

fn exact_damaged_creature_reference(spec: &ChooseSpec, tag: &TagKey) -> bool {
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

fn damage_target_tag(effect: &Effect) -> Option<&TagKey> {
    structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>()
        .map(|tag| &tag.tag)
}

fn damage_prefix(effect: &Effect) -> String {
    describe_effect(effect).trim_end_matches('.').to_string()
}

fn exact_tagged_filter(filter: &ObjectFilter, tag: &TagKey) -> bool {
    let mut semantic_filter = filter.clone();
    semantic_filter.union_surface = Default::default();
    semantic_filter == ObjectFilter::tagged(tag.clone())
}

fn describe_tap_then_untap_lock(action: &Effect, damaged_tag: &TagKey) -> Option<String> {
    let sequence = structural_unwrap_render_wrappers(action)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || sequence.result_label.is_some()
    {
        return None;
    }
    let [tap_effect, cant_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let tagged_tap = tap_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let tap = structural_unwrap_render_wrappers(&tagged_tap.effect)
        .downcast_ref::<crate::effects::TapEffect>()?;
    if !exact_damaged_creature_reference(&tap.target, damaged_tag) {
        return None;
    }
    let cant = structural_unwrap_render_wrappers(cant_effect)
        .downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    if !exact_tagged_filter(filter, &tagged_tap.tag)
        || cant.duration != Until::ControllersNextUntapStep
        || cant.start != crate::effect::RestrictionStart::Immediate
        || cant.duration_surface != crate::effect::RestrictionDurationSurface::Default
    {
        return None;
    }
    Some(
        "Tap that creature and it doesn't untap during its controller's next untap step"
            .to_string(),
    )
}

fn describe_delayed_end_of_combat_destroy(action: &Effect, damaged_tag: &TagKey) -> Option<String> {
    let schedule = structural_unwrap_render_wrappers(action)
        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if schedule
        .trigger
        .downcast_ref::<crate::triggers::EndOfCombatTrigger>()
        .is_none()
        || schedule.trigger.intro_surface().is_some()
        || !schedule.one_shot
        || schedule.start_next_turn
        || schedule.duration != ironsmith_core::DelayedTriggerDuration::Forever
        || schedule.until_end_of_turn
        || schedule.until_end_of_combat
        || schedule.leading_duration_surface
        || schedule.watch_ability_source
        || schedule.watch_all_object_targets
        || schedule.either_of_watched_objects
        || schedule.while_any_tagged_object_in_zone.is_some()
        || !schedule.target_objects.is_empty()
        || schedule.target_tag.is_some()
        || schedule.target_filter.is_some()
        || schedule.controller != PlayerFilter::You
        || schedule.prepayment.is_some()
        || schedule.event_value_from_prior_prevention
    {
        return None;
    }
    let [segment] = schedule.effects.segments.as_slice() else {
        return None;
    };
    if segment.starts_new_source_line || !segment.self_replacements.is_empty() {
        return None;
    }
    let [destroy_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    if !exact_damaged_creature_reference(&destroy.spec, damaged_tag) {
        return None;
    }
    Some("Destroy that creature at end of combat".to_string())
}

fn describe_destroy_unless_controller_pays(
    action: &Effect,
    damaged_tag: &TagKey,
) -> Option<String> {
    let unless = structural_unwrap_render_wrappers(action)
        .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    let [destroy_effect] = unless.effects.as_slice() else {
        return None;
    };
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    if !exact_damaged_creature_reference(&destroy.spec, damaged_tag)
        || unless.player
            != PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(damaged_tag.clone()))
        || unless.leading_surface
        || unless.before_delayed_step
    {
        return None;
    }
    Some(format!(
        "Destroy that creature unless its controller pays {}",
        describe_total_cost(&unless.cost)
    ))
}

fn describe_coin_flip_destroy(effects: &[Effect], damaged_tag: &TagKey) -> Option<String> {
    let [flip_effect, branch_effect] = effects else {
        return None;
    };
    let with_id = flip_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let flip = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::FlipCoinEffect>()?;
    if flip.player != PlayerFilter::You
        || flip.kind != ironsmith_core::CoinFlipKind::Called
        || flip.forced_face.is_some()
        || flip.forced_winner.is_some()
        || flip.forced_loser.is_some()
    {
        return None;
    }
    let branch = structural_unwrap_render_wrappers(branch_effect)
        .downcast_ref::<crate::effects::IfEffect>()?;
    if branch.condition != with_id.id
        || branch.predicate != EffectPredicate::Happened
        || !branch.else_.is_empty()
        || branch.per_player_result
        || branch.prior_result_replacement_surface
    {
        return None;
    }
    let [destroy_effect] = branch.then.as_slice() else {
        return None;
    };
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    if !exact_damaged_creature_reference(&destroy.spec, damaged_tag) {
        return None;
    }
    Some("Flip a coin. If you win the flip, destroy that creature".to_string())
}

pub(in crate::compiled_text) fn describe_single_damage_target_followup_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [damage_segment, followup_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if damage_segment.starts_new_source_line
        || followup_segment.starts_new_source_line
        || !damage_segment.self_replacements.is_empty()
        || !followup_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [damage_effect] = damage_segment.default_effects.as_slice() else {
        return None;
    };
    let [followup_effect] = followup_segment.default_effects.as_slice() else {
        return None;
    };
    let (tag, _) = single_creature_damage_target(damage_effect)?;
    let followup = structural_unwrap_render_wrappers(followup_effect);
    let clause = if followup
        .downcast_ref::<crate::effects::TapEffect>()
        .is_some_and(|tap| exact_damaged_creature_reference(&tap.target, tag))
    {
        "Tap that creature".to_string()
    } else if let Some(put) = followup.downcast_ref::<crate::effects::PutCountersEffect>() {
        if put.distributed
            || put.target_count.is_some()
            || !exact_damaged_creature_reference(&put.target, tag)
        {
            return None;
        }
        format!(
            "Put {} on that creature",
            describe_put_counter_phrase(&put.amount, put.counter_type)
        )
    } else {
        return None;
    };
    Some((format!("{}. {clause}", damage_prefix(damage_effect)), 2))
}

pub(in crate::compiled_text) fn describe_single_damage_target_trigger_followup(
    effects: &[Effect],
) -> Option<String> {
    let [tag_effect, trailing_effects @ ..] = effects else {
        return None;
    };
    let tag = damage_target_tag(tag_effect)?;
    if let Some(text) = describe_coin_flip_destroy(trailing_effects, tag) {
        return Some(text);
    }
    let [action_effect] = trailing_effects else {
        return None;
    };
    let action = structural_unwrap_render_wrappers(action_effect);
    if let Some(destroy) = action.downcast_ref::<crate::effects::DestroyEffect>() {
        if !exact_damaged_creature_reference(&destroy.spec, tag) {
            return None;
        }
        return Some("Destroy that creature".to_string());
    }
    if let Some(destroy) = action.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        if !exact_damaged_creature_reference(&destroy.spec, tag) {
            return None;
        }
        return Some("Destroy that creature. It can't be regenerated".to_string());
    }
    describe_tap_then_untap_lock(action_effect, tag)
        .or_else(|| describe_delayed_end_of_combat_destroy(action_effect, tag))
        .or_else(|| describe_destroy_unless_controller_pays(action_effect, tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tagged_creature(tag: &TagKey) -> ChooseSpec {
        ChooseSpec::Object(
            ObjectFilter::creature()
                .in_zone(Zone::Battlefield)
                .match_tagged(
                    tag.clone(),
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                ),
        )
    }

    #[test]
    fn targeted_damage_then_counter_uses_that_creature() {
        let tag = TagKey::from("damaged_0");
        let segments = vec![
            crate::resolution::ResolutionSegment::from_effects(vec![
                Effect::deal_damage(1, ChooseSpec::target_creature()).tag(tag.clone()),
            ]),
            crate::resolution::ResolutionSegment::from_effects(vec![Effect::put_counters(
                CounterType::MinusOneMinusOne,
                1,
                tagged_creature(&tag),
            )]),
        ];
        assert_eq!(
            describe_single_damage_target_followup_window(&segments, 0),
            Some((
                "Deal 1 damage to target creature. Put a -1/-1 counter on that creature"
                    .to_string(),
                2,
            ))
        );

        let mut changed = segments;
        changed[1].default_effects = vec![Effect::tap(tagged_creature(&TagKey::from("other")))];
        assert!(describe_single_damage_target_followup_window(&changed, 0).is_none());
    }

    #[test]
    fn damage_trigger_destroy_requires_the_trigger_target_tag() {
        let tag = TagKey::from("triggering_damage_target");
        let effects = vec![
            Effect::new(crate::effects::TagTriggeringDamageTargetEffect::new(
                tag.clone(),
            )),
            Effect::destroy(tagged_creature(&tag)),
        ];
        assert_eq!(
            describe_single_damage_target_trigger_followup(&effects).as_deref(),
            Some("Destroy that creature")
        );

        let changed = vec![
            effects[0].clone(),
            Effect::destroy(tagged_creature(&TagKey::from("other"))),
        ];
        assert!(describe_single_damage_target_trigger_followup(&changed).is_none());
    }

    #[test]
    fn damage_trigger_tap_lock_requires_both_linked_tags_and_exact_duration() {
        let damaged = TagKey::from("damaged");
        let tapped = TagKey::from("tapped_0");
        let tap = Effect::tap(tagged_creature(&damaged)).tag(tapped.clone());
        let cant = Effect::new(crate::effects::CantEffect::new(
            crate::effect::Restriction::Untap(ObjectFilter::tagged(tapped.clone())),
            Until::ControllersNextUntapStep,
        ));
        let effects = vec![
            Effect::new(crate::effects::TagTriggeringDamageTargetEffect::new(
                damaged.clone(),
            )),
            Effect::new(crate::effects::SequenceEffect::coordinated(vec![
                tap.clone(),
                cant.clone(),
            ])),
        ];
        assert_eq!(
            describe_single_damage_target_trigger_followup(&effects).as_deref(),
            Some("Tap that creature and it doesn't untap during its controller's next untap step")
        );

        let changed = vec![
            effects[0].clone(),
            Effect::new(crate::effects::SequenceEffect::coordinated(vec![
                tap,
                Effect::new(crate::effects::CantEffect::new(
                    crate::effect::Restriction::Untap(ObjectFilter::tagged(tapped)),
                    Until::EndOfTurn,
                )),
            ])),
        ];
        assert!(describe_single_damage_target_trigger_followup(&changed).is_none());
    }

    #[test]
    fn damage_trigger_delayed_destroy_requires_end_of_combat_and_same_tag() {
        let damaged = TagKey::from("damaged");
        let delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
            crate::triggers::Trigger::new(crate::triggers::EndOfCombatTrigger),
            vec![Effect::destroy(tagged_creature(&damaged))],
            true,
            Vec::new(),
            PlayerFilter::You,
        );
        let effects = vec![
            Effect::new(crate::effects::TagTriggeringDamageTargetEffect::new(
                damaged.clone(),
            )),
            Effect::new(delayed.clone()),
        ];
        assert_eq!(
            describe_single_damage_target_trigger_followup(&effects).as_deref(),
            Some("Destroy that creature at end of combat")
        );

        let changed = vec![
            effects[0].clone(),
            Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                delayed.trigger,
                vec![Effect::destroy(tagged_creature(&TagKey::from("other")))],
                true,
                Vec::new(),
                PlayerFilter::You,
            )),
        ];
        assert!(describe_single_damage_target_trigger_followup(&changed).is_none());
    }

    #[test]
    fn damage_trigger_unless_payment_binds_the_damaged_creatures_controller() {
        let damaged = TagKey::from("damaged");
        let effects = vec![
            Effect::new(crate::effects::TagTriggeringDamageTargetEffect::new(
                damaged.clone(),
            )),
            Effect::new(crate::effects::UnlessPaysEffect::new(
                vec![Effect::destroy(tagged_creature(&damaged))],
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(damaged.clone())),
                vec![crate::mana::ManaSymbol::Generic(2)],
            )),
        ];
        assert_eq!(
            describe_single_damage_target_trigger_followup(&effects).as_deref(),
            Some("Destroy that creature unless its controller pays {2}")
        );

        let changed = vec![
            effects[0].clone(),
            Effect::new(crate::effects::UnlessPaysEffect::new(
                vec![Effect::destroy(tagged_creature(&damaged))],
                PlayerFilter::You,
                vec![crate::mana::ManaSymbol::Generic(2)],
            )),
        ];
        assert!(describe_single_damage_target_trigger_followup(&changed).is_none());
    }

    #[test]
    fn damage_trigger_coin_branch_requires_the_same_result_and_damage_tag() {
        let damaged = TagKey::from("damaged");
        let effects = vec![
            Effect::new(crate::effects::TagTriggeringDamageTargetEffect::new(
                damaged.clone(),
            )),
            Effect::with_id(7, Effect::flip_coin(PlayerFilter::You)),
            Effect::if_then_else(
                crate::effect::EffectId(7),
                EffectPredicate::Happened,
                vec![Effect::destroy(tagged_creature(&damaged))],
                vec![],
            ),
        ];
        assert_eq!(
            describe_single_damage_target_trigger_followup(&effects).as_deref(),
            Some("Flip a coin. If you win the flip, destroy that creature")
        );

        let changed = vec![
            effects[0].clone(),
            effects[1].clone(),
            Effect::if_then_else(
                crate::effect::EffectId(8),
                EffectPredicate::Happened,
                vec![Effect::destroy(tagged_creature(&damaged))],
                vec![],
            ),
        ];
        assert!(describe_single_damage_target_trigger_followup(&changed).is_none());
    }
}
