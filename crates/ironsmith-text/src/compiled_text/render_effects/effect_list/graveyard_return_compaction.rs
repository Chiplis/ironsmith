use super::*;

/// An announced pool, a random resolving subset, and its complement captured
/// before either move. Every link is checked; the text never reconstructs a
/// missing choice or infers the remainder from the eventual move outcome.
pub(crate) fn describe_declared_graveyard_random_partition(effects: &[Effect]) -> Option<String> {
    let [declared_effect, choice_effect, remainder_effect, return_effect, bottom_effect] = effects else {
        return None;
    };
    let declared = structural_unwrap_render_wrappers(declared_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let pool = effect_outer_tag(declared_effect)?;
    let choose = structural_unwrap_render_wrappers(choice_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let returned = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;
    let bottom = structural_unwrap_render_wrappers(bottom_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let announced = declared.target.count();
    let selected = choose_exact_count(choose)?;
    let ChooseSpec::Object(filter) = declared.target.base() else { return None; };
    if !declared.explicit_declaration || !declared.target.is_target()
        || announced.dynamic_x || announced.random || announced.max != Some(announced.min)
        || announced.min <= selected || !choose.count.is_random()
        || filter.zone != Some(Zone::Graveyard) || filter.owner != Some(PlayerFilter::You)
    { return None; }
    let expected_choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::tagged(pool.clone()).in_zone(Zone::Graveyard),
        choose.count, PlayerFilter::You, choose.tag.clone(),
    ).in_zone(Zone::Graveyard);
    let expected_remainder = crate::effects::TagMatchingObjectsEffect::new(
        ObjectFilter::tagged(pool.clone()).not_tagged(choose.tag.clone()), remainder.tag.clone(),
    ).in_zones(vec![Zone::Graveyard]);
    // The plural-object surface ("put the rest") is presentation only.
    let mut bottom = bottom.clone();
    bottom.target_plural_surface = false;
    if *choose != expected_choose || *remainder != expected_remainder
        || *returned != crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(
            ChooseSpec::Tagged(choose.tag.clone()), false)
        || bottom != crate::effects::MoveToZoneEffect::new(
            ChooseSpec::All(ObjectFilter::default().in_zone(Zone::Graveyard).match_tagged(
                remainder.tag.clone(), crate::filter::TaggedOpbjectRelation::SameObjectId,
            )), Zone::Library, false).with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
    { return None; }
    let count = number_word(i32::try_from(selected).ok()?).unwrap_or_else(|| selected.to_string());
    let rest = if announced.min - selected == 1 { "the other" } else { "the rest" };
    Some(format!(
        "Choose {}. Return {count} of them at random to the battlefield and put {rest} on the bottom of your library",
        describe_choose_spec(&declared.target),
    ))
}

/// Render the resolution-time choice used by an untargeted graveyard return
/// as the single return action that Oracle presents.
///
/// The choice remains a distinct runtime effect: this is deliberately only a
/// structural text view over an exact, adjacent choose-and-return pair.
pub(crate) fn describe_choose_then_return_from_graveyard(
    choose_effect: &Effect,
    return_effect: &Effect,
) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    // "That player returns a card from their graveyard to their hand"
    // (Skullwinder): the chooser returns its own chosen card.
    if let Some(to_hand) = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        if choose.is_search
            || choose.reveal
            || choose.top_only
            || choose.bottom_only
            || choose.replace_tagged_objects
            || choose.count_value.is_some()
            || choose.aggregate_constraint.is_some()
            || choose_exact_count(choose) != Some(1)
            || choose_primary_zone(choose) != Some(Zone::Graveyard)
            || !choose.additional_zones.is_empty()
            || choose.filter.owner.as_ref() != Some(&choose.chooser)
            || !matches!(
                to_hand.target.unhinted(),
                ChooseSpec::Tagged(tag) if tag == &choose.tag
            )
        {
            return None;
        }
        let chooser = describe_player_filter(&choose.chooser);
        let verb = player_verb(&chooser, "return", "returns");
        let possessive = if choose.chooser == PlayerFilter::You { "your" } else { "their" };
        let mut owned = choose.filter.clone();
        owned.owner = None;
        owned.zone = None;
        let selection = with_indefinite_article(strip_leading_article(&owned.description()));
        return Some(format!(
            "{} {verb} {selection} from {possessive} graveyard to {possessive} hand",
            capitalize_first(&chooser)
        ));
    }
    let returned = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;

    if choose.is_search
        || choose.reveal
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.count_value.is_some()
        || (choose_exact_count(choose) != Some(1)
            && !(choose.count.is_random() && choose_exact_count(choose).is_some())
            && choose.aggregate_constraint.is_none())
        || choose_primary_zone(choose) != Some(Zone::Graveyard)
        || !choose.additional_zones.is_empty()
        || returned.as_aura.is_some()
        || !matches!(
            returned.target.unhinted(),
            ChooseSpec::Tagged(tag) if tag == &choose.tag
        )
    {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let verb = player_verb(&chooser, "return", "returns");
    let mut selection = if choose.top_only {
        let mut ordinary_choice = choose.clone();
        ordinary_choice.top_only = false;
        let ordinary_selection = describe_choose_selection(&ordinary_choice);
        let noun = ordinary_selection
            .strip_prefix("a ")
            .or_else(|| ordinary_selection.strip_prefix("an "))
            .unwrap_or(ordinary_selection.as_str());
        format!("the top {noun}")
    } else {
        describe_choose_selection(choose)
    };
    let where_x = if let Some((head, tail)) = selection.split_once(", where X is ") {
        let tail = tail.to_string();
        selection = head.to_string();
        format!(", where X is {tail}")
    } else {
        String::new()
    };
    let origin = if matches!(&choose.chooser, PlayerFilter::TaggedPlayer(_))
        && choose.filter.owner.as_ref() == Some(&choose.chooser)
    {
        "from their graveyard".to_string()
    } else {
        describe_choose_zone_origin(choose, "graveyard")
    };
    let origin = if choose.top_only {
        origin
            .strip_prefix("from ")
            .map_or(origin.clone(), |rest| format!("of {rest}"))
    } else {
        origin
    };
    let tapped = if returned.tapped { " tapped" } else { "" };
    let actor = if choose.aggregate_constraint.is_some() && choose.chooser == PlayerFilter::You {
        String::new()
    } else {
        format!("{chooser} ")
    };

    Some(append_battlefield_entry_counter_surface(
        format!("{actor}{verb} {selection} {origin} to the battlefield{tapped}{where_x}"),
        &returned.enters_with_counters,
    ))
}

/// Keep an exact graveyard choice, its linked return, and a counter placed on
/// that returned object as one Oracle action. The runtime effects remain
/// separate; the outer return tag is the proof that the counter cannot apply
/// to an unrelated earlier choice.
pub(crate) fn describe_choose_then_return_from_graveyard_with_counters(
    choose_effect: &Effect,
    return_effect: &Effect,
    counter_effect: &Effect,
) -> Option<String> {
    let returned = describe_choose_then_return_from_graveyard(choose_effect, return_effect)?;
    let returned_tag = effect_outer_tag(return_effect)?;
    let counters = structural_unwrap_render_wrappers(counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if counters.distributed
        || counters.target_count.is_some()
        || !choose_spec_references_exact_tag(&counters.target, returned_tag)
    {
        return None;
    }

    Some(format!(
        "{returned} with {} on it",
        describe_put_counter_phrase(&counters.amount, counters.counter_type)
    ))
}

#[cfg(test)]
mod random_partition_tests {
    use super::*;
    #[test]
    fn random_partition_surface_requires_the_actual_pool_subset_and_complement_links() {
        let pool = crate::TagKey::from("announced");
        let selected = crate::TagKey::from("selected");
        let rest = crate::TagKey::from("rest");
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()
            .in_zone(Zone::Graveyard).owned_by(PlayerFilter::You)))
            .with_count(crate::ChoiceCount::exactly(3));
        let mut effects = vec![
            Effect::new(crate::effects::TargetOnlyEffect::explicit(target)).tag_all(pool.clone()),
            Effect::new(crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::tagged(pool.clone()).in_zone(Zone::Graveyard),
                crate::ChoiceCount::exactly(2).at_random(), PlayerFilter::You, selected.clone(),
            ).in_zone(Zone::Graveyard)),
            Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                ObjectFilter::tagged(pool.clone()).not_tagged(selected.clone()), rest.clone(),
            ).in_zones(vec![Zone::Graveyard])),
            Effect::return_from_graveyard_to_battlefield(ChooseSpec::Tagged(selected), false),
            Effect::new(crate::effects::MoveToZoneEffect::new(ChooseSpec::All(ObjectFilter::default().in_zone(Zone::Graveyard)
                .match_tagged(rest, crate::filter::TaggedOpbjectRelation::SameObjectId)), Zone::Library, false)
                .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)),
        ];
        let rendered = describe_declared_graveyard_random_partition(&effects).unwrap();
        assert!(rendered.contains("three target creature cards"), "{rendered}");
        assert!(rendered.contains("Return two of them at random"), "{rendered}");
        assert!(rendered.contains("put the other on the bottom of your library"), "{rendered}");
        effects[2] = Effect::new(crate::effects::TagMatchingObjectsEffect::new(
            ObjectFilter::tagged(pool), "rest",
        ).in_zones(vec![Zone::Graveyard]));
        assert!(describe_declared_graveyard_random_partition(&effects).is_none(),
            "a missing exclusion cannot be rendered as the complement");
    }
}
