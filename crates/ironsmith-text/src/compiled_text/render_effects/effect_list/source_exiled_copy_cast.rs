use super::*;

/// Render the executable linked-exile representation of
/// "copy the exiled card; if you do, cast the copy" without exposing the
/// internal source-linked card selection. The first optional action chooses
/// exactly one card exiled by this source; `CastTagged(as_copy)` is the typed
/// operation that creates and casts the corresponding spell copy.
pub(in crate::compiled_text) fn describe_optional_source_exiled_copy_then_cast_pair(
    producer_effect: &Effect,
    result_effect: &Effect,
) -> Option<String> {
    let producer = producer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let optional_choice = producer
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let [choice_effect] = optional_choice.effects.as_slice() else {
        return None;
    };
    let choice = choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let expected_filter = ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile);

    let result = result_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [optional_cast_effect] = result.then.as_slice() else {
        return None;
    };
    let optional_cast = optional_cast_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = optional_cast.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;

    if optional_choice.decider != Some(PlayerFilter::You)
        || optional_choice.fallback != crate::decision::FallbackStrategy::Decline
        || choice.filter != expected_filter
        || choice.count != ChoiceCount::exactly(1)
        || choice.count_value.is_some()
        || choice.aggregate_constraint.is_some()
        || choice.chooser != PlayerFilter::You
        || choice.zone != Some(Zone::Exile)
        || !choice.additional_zones.is_empty()
        || choice.description != "Choose"
        || choice.is_search
        || choice.reveal
        || choice.search_mode != crate::effect::SearchSelectionMode::Exact
        || choice.search_reveal_reference_surface.is_some()
        || choice.search_result_reference_surface.is_some()
        || choice.search_top_in_any_order_surface.is_some()
        || choice.top_only
        || choice.bottom_only
        || choice.replace_tagged_objects
        || choice.remember_as_chosen_object
        || result.condition != producer.id
        || result.predicate != crate::effect::EffectPredicate::Happened
        || !result.else_.is_empty()
        || result.prior_result_replacement_surface
        || optional_cast.decider.is_some()
        || optional_cast.fallback != crate::decision::FallbackStrategy::Decline
        || cast.tag != choice.tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || !cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.additional_mana_cost.is_some()
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    Some(
        "You may copy the exiled card. If you do, you may cast the copy without paying its mana cost"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(
        producer_id: crate::effect::EffectId,
        result_id: crate::effect::EffectId,
        source_linked: bool,
        as_copy: bool,
    ) -> (Effect, Effect) {
        let filter = if source_linked {
            ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile)
        } else {
            ObjectFilter::default().in_zone(Zone::Exile)
        };
        let mut choice = crate::effects::ChooseObjectsEffect::new(
            filter,
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            "chosen_exiled",
        );
        choice.zone = Some(Zone::Exile);
        let producer = Effect::new(crate::effects::WithIdEffect::new(
            producer_id,
            Effect::new(crate::effects::MayEffect::new_for_player(
                vec![Effect::new(choice)],
                PlayerFilter::You,
            )),
        ));

        let mut cast = crate::effects::CastTaggedEffect::new("chosen_exiled", PlayerFilter::You)
            .without_paying_mana_cost();
        if as_copy {
            cast = cast.as_copy();
        }
        let cast = Effect::new(cast);
        let result = Effect::new(crate::effects::IfEffect::new(
            result_id,
            crate::effect::EffectPredicate::Happened,
            vec![Effect::new(crate::effects::MayEffect::new(vec![cast]))],
            vec![],
        ));
        (producer, result)
    }

    #[test]
    fn renders_exact_linked_exile_copy_cast_pair() {
        let (producer, result) = pair(
            crate::effect::EffectId(7),
            crate::effect::EffectId(7),
            true,
            true,
        );
        assert_eq!(
            describe_optional_source_exiled_copy_then_cast_pair(&producer, &result).as_deref(),
            Some(
                "You may copy the exiled card. If you do, you may cast the copy without paying its mana cost"
            )
        );
    }

    #[test]
    fn rejects_wrong_result_id_source_link_or_noncopy_cast() {
        let (producer, wrong_id) = pair(
            crate::effect::EffectId(7),
            crate::effect::EffectId(8),
            true,
            true,
        );
        assert_eq!(
            describe_optional_source_exiled_copy_then_cast_pair(&producer, &wrong_id),
            None
        );

        let (wrong_source, result) = pair(
            crate::effect::EffectId(7),
            crate::effect::EffectId(7),
            false,
            true,
        );
        assert_eq!(
            describe_optional_source_exiled_copy_then_cast_pair(&wrong_source, &result),
            None
        );

        let (producer, noncopy) = pair(
            crate::effect::EffectId(7),
            crate::effect::EffectId(7),
            true,
            false,
        );
        assert_eq!(
            describe_optional_source_exiled_copy_then_cast_pair(&producer, &noncopy),
            None
        );
    }
}
