use super::*;

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
    let returned = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;

    if choose.is_search
        || choose.reveal
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.aggregate_constraint.is_some()
        || choose.count_value.is_some()
        || choose_exact_count(choose) != Some(1)
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
    let origin = describe_choose_zone_origin(choose, "graveyard");
    let origin = if choose.top_only {
        origin
            .strip_prefix("from ")
            .map_or(origin.clone(), |rest| format!("of {rest}"))
    } else {
        origin
    };
    let tapped = if returned.tapped { " tapped" } else { "" };

    Some(format!(
        "{chooser} {verb} {selection} {origin} to the battlefield{tapped}{where_x}"
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
