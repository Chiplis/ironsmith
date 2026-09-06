use super::*;

fn exact_tagged(spec: &ChooseSpec, expected: &TagKey) -> bool {
    matches!(spec.base(), ChooseSpec::Tagged(found) if found == expected)
}

fn ordinary_one_shot_schedule(schedule: &crate::effects::ScheduleDelayedTriggerEffect) -> bool {
    schedule.trigger.intro_surface().is_none()
        && schedule.one_shot
        && !schedule.start_next_turn
        && schedule.duration == ironsmith_core::DelayedTriggerDuration::Forever
        && !schedule.until_end_of_turn
        && !schedule.until_end_of_combat
        && !schedule.leading_duration_surface
        && !schedule.watch_ability_source
        && !schedule.watch_all_object_targets
        && !schedule.either_of_watched_objects
        && schedule.while_any_tagged_object_in_zone.is_none()
        && schedule.target_objects.is_empty()
        && schedule.target_tag.is_none()
        && schedule.target_filter.is_none()
        && schedule.controller == PlayerFilter::You
        && schedule.prepayment.is_none()
        && !schedule.event_value_from_prior_prevention
}

/// Preserve three authored sentences that share one per-opponent creature set:
/// select and tap, goad that exact set, then register a combat-damage watcher
/// for the same objects. The tag checks prevent unrelated adjacent effects
/// from borrowing the set-reference surface.
pub(in crate::compiled_text) fn describe_quantified_tap_goad_then_watch_set(
    effects: &[Effect],
) -> Option<String> {
    let [for_players_effect, goad_effect, schedule_effect] = effects else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Opponent
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let [tap_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let tagged_tap = tap_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let tap = tagged_tap
        .effect
        .downcast_ref::<crate::effects::TapEffect>()?;
    let ChoiceCount {
        min: 0,
        max: Some(1),
        ..
    } = tap.target.count()
    else {
        return None;
    };
    if !tap.target.is_target() {
        return None;
    }

    let (goad_action, watched_tag) = if let Some(tagged) = goad_effect.downcast_ref::<crate::effects::TaggedEffect>() {
        (tagged.effect.as_ref(), &tagged.tag)
    } else {
        (goad_effect, &tagged_tap.tag)
    };
    let goad = goad_action.downcast_ref::<crate::effects::GoadEffect>()?;
    let ChooseSpec::Object(goad_filter) = goad.target.base() else {
        return None;
    };
    if goad.duration != Until::YourNextTurn
        || !filter_is_tagged_as(goad_filter, tagged_tap.tag.as_str())
    {
        return None;
    }

    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    let damage = schedule
        .trigger
        .downcast_ref::<crate::triggers::DealsCombatDamageToPlayerTrigger>()?;
    let watched_filter = schedule.target_filter.as_ref()?;
    if schedule.one_shot
        || schedule.duration != ironsmith_core::DelayedTriggerDuration::UntilControllerNextTurn
        || !schedule.leading_duration_surface
        || schedule.target_tag.as_ref() != Some(watched_tag)
        || !filter_is_tagged_as(watched_filter, watched_tag.as_str())
        || damage.player != PlayerFilter::Any
    {
        return None;
    }
    let [draw_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count != Value::Fixed(1) {
        return None;
    }

    let first = describe_effect(for_players_effect);
    let first = first.trim().trim_end_matches('.');
    let second = capitalize_first(describe_effect(goad_effect).trim().trim_end_matches('.'));
    let third = capitalize_first(
        describe_effect(schedule_effect)
            .trim()
            .trim_end_matches('.'),
    );
    Some(format!("{first}. {second}. {third}"))
}

/// Render the typed cross-delay correlation:
///
/// `destroy the other creature at end of combat. At the beginning of the
/// next end step, if that creature was destroyed this way, put a +1/+1
/// counter on the first creature.`
///
/// The matcher requires the combat-participant and attached-object tags, the
/// exact destruction result id, and both delayed trigger kinds. It therefore
/// cannot fold unrelated adjacent schedules or a counter placed on a
/// different object.
pub(in crate::compiled_text) fn describe_end_combat_destroy_then_next_end_counter(
    effects: &[Effect],
) -> Option<String> {
    let [participant_effect, attached_effect, outer_effect] = effects else {
        return None;
    };
    let participant =
        participant_effect.downcast_ref::<crate::effects::TagOtherBlockParticipantEffect>()?;
    let subject_filter = participant.subject_filter.as_ref()?;
    let other_filter = participant.filter.as_ref()?;
    let blocking_tag = &participant.tag;
    if blocking_tag.as_str() != "blocking"
        || !subject_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "enchanted"
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        || !subject_filter.card_types.contains(&CardType::Creature)
        || !other_filter.card_types.contains(&CardType::Creature)
    {
        return None;
    }

    let attached = attached_effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()?;
    if attached.tag.as_str() != "enchanted" {
        return None;
    }
    let enchanted_tag = &attached.tag;

    let outer = outer_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if outer
        .trigger
        .downcast_ref::<crate::triggers::EndOfCombatTrigger>()
        .is_none()
        || !ordinary_one_shot_schedule(outer)
    {
        return None;
    }
    let [outer_segment] = outer.effects.segments.as_slice() else {
        return None;
    };
    if outer_segment.starts_new_source_line || !outer_segment.self_replacements.is_empty() {
        return None;
    }
    let [destroy_effect, result_effect] = outer_segment.default_effects.as_slice() else {
        return None;
    };
    let with_id = destroy_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let destroy = structural_unwrap_render_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    if !exact_tagged(&destroy.spec, blocking_tag) {
        return None;
    }

    let result = result_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let EffectPredicate::PriorEffectResult(surface) = &result.predicate else {
        return None;
    };
    if result.condition != with_id.id
        || !result.else_.is_empty()
        || result.per_player_result
        || result.prior_result_replacement_surface
        || surface.action != ironsmith_core::PriorEffectAction::Destroyed
        || surface.actor != ironsmith_core::PriorEffectResultActor::Passive
        || surface.quantifier != ironsmith_core::PriorEffectResultQuantifier::One
        || surface.required_count.is_some()
        || surface.shared_characteristic.is_some()
    {
        return None;
    }
    let [inner_effect] = result.then.as_slice() else {
        return None;
    };
    let inner = inner_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    let end_step = inner
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfEndStepTrigger>()?;
    if end_step.player != PlayerFilter::Any || !ordinary_one_shot_schedule(inner) {
        return None;
    }
    let [inner_segment] = inner.effects.segments.as_slice() else {
        return None;
    };
    if inner_segment.starts_new_source_line || !inner_segment.self_replacements.is_empty() {
        return None;
    }
    let [counter_effect] = inner_segment.default_effects.as_slice() else {
        return None;
    };
    let counter = structural_unwrap_render_wrappers(counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if counter.counter_type != crate::object::CounterType::PlusOnePlusOne
        || counter.amount != Value::Fixed(1)
        || counter.target_count.is_some()
        || counter.distributed
        || !exact_tagged(&counter.target, enchanted_tag)
    {
        return None;
    }

    Some("Destroy the other creature at end of combat. At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or less, destroy the other creature at end of combat. At the beginning of the next end step, if that creature was destroyed this way, put a +1/+1 counter on the first creature.";

    fn parsed_effects() -> Vec<Effect> {
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Correlated Delayed Combat Probe",
        )
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(format!("Enchant creature\n{LINE}"))
        .expect("correlated delayed combat program should compile");
        let triggered = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("triggered ability");
        triggered.effects.flattened_default_effects().to_vec()
    }

    #[test]
    fn public_route_preserves_both_delays_and_combat_referents() {
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Correlated Delayed Combat Probe",
        )
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(format!("Enchant creature\n{LINE}"))
        .expect("correlated delayed combat program should compile");
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            format!("Enchant creature\n{LINE}")
        );
        assert!(describe_end_combat_destroy_then_next_end_counter(&parsed_effects()).is_some());
    }

    #[test]
    fn changed_attached_referent_is_not_compacted() {
        let mut effects = parsed_effects();
        let attached = effects[1]
            .downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            .expect("attached-object prelude")
            .clone();
        effects[1] = Effect::new(crate::effects::TagAttachedToSourceEffect::new(
            if attached.tag.as_str() == "enchanted" {
                "different_attached"
            } else {
                "enchanted"
            },
        ));
        assert!(describe_end_combat_destroy_then_next_end_counter(&effects).is_none());
    }
}
