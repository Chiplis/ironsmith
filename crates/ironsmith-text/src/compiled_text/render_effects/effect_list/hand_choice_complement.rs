use super::super::*;

/// A hand selection kept by its chooser, followed by discarding precisely the
/// complement of that selection. The shared tag and identical count/card
/// filters are required: an ordinary chosen-card discard must keep its own
/// rendering.
pub(in crate::compiled_text::render_effects) fn describe_hand_choice_discard_complement(
    effects: &[Effect],
) -> Option<String> {
    let (draw, choose_effect, discard_effect) = match effects {
        [choose, discard] => (None, choose, discard),
        [draw, choose, discard] => (Some(draw), choose, discard),
        _ => return None,
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let discard = structural_unwrap_render_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if choose.is_search
        || choose.reveal
        || choose.replace_tagged_objects
        || choose.zone != Some(Zone::Hand)
        || !choose.additional_zones.is_empty()
        || choose.filter.zone != Some(Zone::Hand)
        || choose.filter.owner != Some(PlayerFilter::You)
        || !choose.filter.tagged_constraints.is_empty()
        || choose.chooser != PlayerFilter::You
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || discard.player != PlayerFilter::You
        || discard.random
        || discard.any_number
    {
        return None;
    }
    let discard_filter = discard.card_filter.as_ref()?;
    if !matches!(discard.count.unhinted(), Value::Count(filter) if filter == discard_filter) {
        return None;
    }
    let [constraint] = discard_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.tag != choose.tag
        || constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    {
        return None;
    }
    let mut unselected_filter = discard_filter.clone();
    unselected_filter.tagged_constraints.clear();
    if unselected_filter != choose.filter {
        return None;
    }

    let selection = describe_choose_selection(choose).replace(" from your hand", " in your hand");
    let selection = selection.trim_end_matches(" in your hand");
    let choice = format!("choose {selection} in your hand and discard the rest");
    match draw {
        Some(draw_effect) => {
            let draw = structural_unwrap_render_wrappers(draw_effect)
                .downcast_ref::<crate::effects::DrawCardsEffect>()?;
            if draw.player != PlayerFilter::You {
                return None;
            }
            Some(format!(
                "{}, then {choice}",
                describe_effect(draw_effect).trim_end_matches('.')
            ))
        }
        None => Some(capitalize_first(&choice)),
    }
}

/// Color selections accumulate into one tag. The final discard uses the
/// opposite of that tag, so presenting it as a discard of "those cards"
/// reverses the effect's meaning.
pub(in crate::compiled_text::render_effects) fn describe_each_player_color_hand_complement(
    players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if players.filter != PlayerFilter::Any
        || players.starting_with_controller
        || players.stop_after_first_happened
    {
        return None;
    }
    let [sequence_effect] = players.effects.as_slice() else {
        return None;
    };
    let sequence = structural_unwrap_render_wrappers(sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let [look_effect, choices @ .., discard_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal || look.target != ChooseSpec::Player(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let expected_colors = [
        crate::color::ColorSet::WHITE,
        crate::color::ColorSet::BLUE,
        crate::color::ColorSet::BLACK,
        crate::color::ColorSet::RED,
        crate::color::ColorSet::GREEN,
    ];
    if choices.len() != expected_colors.len() {
        return None;
    }
    let first = structural_unwrap_render_wrappers(&choices[0])
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let [first_exclusion] = first.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if first_exclusion.tag != first.tag
        || first_exclusion.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    {
        return None;
    }
    let mut common_filter = first.filter.clone();
    common_filter.colors = None;
    for (effect, color) in choices.iter().zip(expected_colors) {
        let choose = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let mut filter = choose.filter.clone();
        filter.colors = None;
        if filter != common_filter
            || choose.filter.colors != Some(color)
            || choose.tag != first.tag
            || choose.chooser != PlayerFilter::IteratedPlayer
            || choose.zone != Some(Zone::Hand)
            || !choose.additional_zones.is_empty()
            || !choose.count.is_single()
            || choose.count_value.is_some()
            || choose.aggregate_constraint.is_some()
            || choose.is_search
            || choose.reveal
            || choose.replace_tagged_objects
        {
            return None;
        }
    }
    if common_filter.zone != Some(Zone::Hand)
        || common_filter.owner != Some(PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let discard = structural_unwrap_render_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let discarded_filter = discard.card_filter.as_ref()?;
    if discard.player != PlayerFilter::IteratedPlayer
        || discard.random
        || discard.any_number
        || !matches!(discard.count.unhinted(), Value::Count(filter) if filter == discarded_filter)
    {
        return None;
    }
    common_filter.excluded_card_types = vec![CardType::Land];
    common_filter.other = true;
    if *discarded_filter != common_filter {
        return None;
    }
    Some("Each player reveals their hand, chooses one card of each color from it, then discards all other nonland cards".to_string())
}
