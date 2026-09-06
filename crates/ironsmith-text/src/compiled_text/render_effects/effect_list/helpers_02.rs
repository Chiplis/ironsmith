use super::helpers_00::*;
use super::helpers_01::*;
use super::*;

pub(crate) fn choose_exact_target_type(
    effect: &Effect,
    card_type: crate::types::CardType,
    exact: usize,
) -> Option<&str> {
    let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_exact_count(choose) != Some(exact) {
        return None;
    }
    if choose_primary_zone(choose) != Some(Zone::Battlefield) {
        return None;
    }
    if choose.filter.card_types != vec![card_type] {
        return None;
    }
    Some(choose.tag.as_str())
}

pub(crate) fn exact_counted_target_filter(
    spec: &ChooseSpec,
    exact: usize,
    target_context: bool,
) -> Option<&crate::filter::ObjectFilter> {
    fn object_filter(spec: &ChooseSpec) -> Option<&crate::filter::ObjectFilter> {
        match spec.unhinted() {
            ChooseSpec::Target(inner) => object_filter(inner),
            ChooseSpec::Object(filter) => Some(filter),
            _ => None,
        }
    }

    match spec.unhinted() {
        ChooseSpec::Target(inner) => exact_counted_target_filter(inner, exact, true),
        ChooseSpec::WithCount(inner, count) => {
            if count.min != exact
                || count.max != Some(exact)
                || count.dynamic_x
                || count.random
                || (!target_context && !inner.is_target())
            {
                return None;
            }
            object_filter(inner)
        }
        _ => None,
    }
}

pub(crate) fn exile_exact_target_type(
    effect: &Effect,
    card_type: crate::types::CardType,
    exact: usize,
) -> bool {
    let exile = unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::ExileEffect>();
    let Some(exile) = exile else {
        return false;
    };
    let Some(filter) = exact_counted_target_filter(&exile.spec, exact, false) else {
        return false;
    };
    filter.zone == Some(Zone::Battlefield) && filter.card_types == vec![card_type]
}

pub(crate) fn tagged_exile_exact_target_type(
    effect: &Effect,
    card_type: crate::types::CardType,
    exact: usize,
) -> Option<&str> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = tagged
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let Some(filter) = exact_counted_target_filter(&exile.spec, exact, false) else {
        return None;
    };
    if filter.zone != Some(Zone::Battlefield) || filter.card_types != vec![card_type] {
        return None;
    }
    Some(tagged.tag.as_str())
}

pub(crate) fn is_move_to_exile_of_tag(effect: &Effect, tag: &str) -> bool {
    let effect = unwrap_tag_wrappers(effect);
    let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() else {
        return false;
    };
    if move_to_zone.zone != Zone::Exile {
        return false;
    }
    matches!(&move_to_zone.target, ChooseSpec::Tagged(effect_tag) if effect_tag.as_str() == tag)
}

pub(crate) fn optional_nonland_permanent_choice<'a>(
    effect: &'a Effect,
    zone: Zone,
    tag: Option<&str>,
) -> Option<&'a str> {
    let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.count.min != 0
        || choose.count.max != Some(1)
        || choose_primary_zone(choose) != Some(zone)
        || !is_nonland_permanent_filter_in_zone(&choose.filter, zone)
    {
        return None;
    }
    if let Some(tag) = tag
        && choose.tag.as_str() != tag
    {
        return None;
    }
    Some(choose.tag.as_str())
}

pub(crate) fn choose_targets_schedule_trigger(
    choose: &crate::effects::ChooseObjectsEffect,
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> bool {
    if choose.count.min != 1
        || choose.count.max != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
    {
        return false;
    }
    let Some(target_tag) = schedule.target_tag.as_ref() else {
        return false;
    };
    if choose.tag != *target_tag {
        return false;
    }
    let Some(target_filter) = schedule.target_filter.as_ref() else {
        return false;
    };
    let mut stripped = target_filter.clone();
    stripped.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == choose.tag)
    });
    // A destination-owner condition belongs to the watched zone change
    // ("is put into your graveyard"), not to the target declaration made
    // while the delayed trigger is created.
    if choose.filter.owner.is_none() && stripped.owner == Some(crate::target::PlayerFilter::You) {
        stripped.owner = None;
    }
    stripped == choose.filter
}

pub(crate) fn describe_may_choose_graveyard_then_return(effects: &[Effect]) -> Option<String> {
    let [may_effect, result_effect] = effects else {
        return None;
    };
    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let [choose_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.filter.zone != Some(Zone::Graveyard)
        || choose.filter.owner != Some(PlayerFilter::You)
        || choose.is_search
        || choose.top_only
    {
        return None;
    }

    let if_effect = result_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [return_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let returned = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;
    if returned.as_aura.is_some()
        || !matches!(returned.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let mut selection = choose.filter.clone();
    selection.zone = None;
    selection.owner = None;
    selection.tagged_constraints.clear();
    let selection = with_indefinite_article(&selection.description());
    let return_clause = append_battlefield_entry_counter_surface(
        format!(
            "return it to the battlefield{}",
            if returned.tapped { " tapped" } else { "" }
        ),
        &returned.enters_with_counters,
    );
    Some(format!(
        "You may choose {selection} in your graveyard. If you do, {return_clause}"
    ))
}

pub(crate) fn consult_reveal_put_battlefield_then_shuffle_effects(
    effects: &[Effect],
) -> Option<String> {
    fn same_tagged_iteration_player(left: &PlayerFilter, right: &PlayerFilter) -> bool {
        if left == right {
            return true;
        }
        let is_iterated_controller = |player: &PlayerFilter| {
            matches!(
                player,
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
                    if tag.as_str() == "__it__"
            )
        };
        (matches!(left, PlayerFilter::IteratedPlayer) && is_iterated_controller(right))
            || (matches!(right, PlayerFilter::IteratedPlayer) && is_iterated_controller(left))
    }

    if effects.len() != 3 {
        return None;
    }

    let consult = unwrap_tag_wrappers(&effects[0])
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return None;
    }
    if !matches!(
        consult.stop_rule,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
            | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
    ) {
        return None;
    }

    let direct_match_move = || {
        let move_effect = unwrap_tag_wrappers(&effects[1]);
        let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        (move_to_zone.zone == Zone::Battlefield
            && !move_to_zone.to_top
            && matches!(
                move_to_zone.target.base(),
                ChooseSpec::Tagged(tag) if tag == &consult.match_tag
            ))
        .then_some(())
    };
    let iterated_match_move = || {
        let for_each = unwrap_tag_wrappers(&effects[1])
            .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
        if for_each.tag != consult.match_tag {
            return None;
        }
        let nested = if let [sequence] = for_each.effects.as_slice()
            && let Some(sequence) = sequence.downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            for_each.effects.as_slice()
        };
        let [move_effect] = nested else {
            return None;
        };
        let move_to_zone =
            unwrap_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        (move_to_zone.zone == Zone::Battlefield
            && !move_to_zone.to_top
            && (matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
                || matches!(
                    move_to_zone.target.base(),
                    ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"
                )))
        .then_some(())
    };
    direct_match_move().or_else(iterated_match_move)?;

    let shuffle =
        unwrap_tag_wrappers(&effects[2]).downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let shuffle_uses_revealed_card_controller = matches!(
        &shuffle.player,
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
            if tag == &consult.match_tag
    );
    if !same_tagged_iteration_player(&shuffle.player, &consult.player)
        && !shuffle_uses_revealed_card_controller
    {
        return None;
    }

    Some(strip_leading_article(&consult.filter.description()).to_string())
}

/// Render the exact reveal-until / matched-card-to-battlefield / shuffle
/// procedure as one semantic clause. Keeping this structural avoids exposing
/// the internal `consult_match` helper tag when the move is lowered through a
/// tagged iteration (Reweave), without teaching the generic tag renderer to
/// prettify unrelated internal tags.
pub(crate) fn describe_consult_reveal_put_battlefield_then_shuffle_effects(
    effects: &[Effect],
) -> Option<String> {
    consult_reveal_put_battlefield_then_shuffle_effects(effects)?;
    let consult = unwrap_tag_wrappers(&effects[0])
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let player = describe_player_filter(&consult.player);
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let put_verb = player_verb(&player, "put", "puts");
    let shuffle_verb = player_verb(&player, "shuffle", "shuffles");
    let pronoun = if player == "you" { "you" } else { "they" };
    let library_owner = if player == "you" { "your" } else { "their" };
    let selection = describe_single_search_filter_in_zone(&consult.filter, Zone::Library);

    Some(format!(
        "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} reveal {selection}, {put_verb} that card onto the battlefield, then {shuffle_verb}"
    ))
}

pub(crate) fn consult_reveal_put_battlefield_then_shuffle_selection(
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    let effects = if for_each.effects.len() == 1 {
        if let Some(sequence) = for_each.effects[0].downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            for_each.effects.as_slice()
        }
    } else {
        for_each.effects.as_slice()
    };
    consult_reveal_put_battlefield_then_shuffle_effects(effects)
}

pub(crate) fn describe_destroy_for_each_destroyed_consult_exile_put_shuffle(
    destroy_effect: &Effect,
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    fn same_tagged_iteration_player(left: &PlayerFilter, right: &PlayerFilter) -> bool {
        if left == right {
            return true;
        }
        let is_iterated_controller = |player: &PlayerFilter| {
            matches!(
                player,
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
                    if tag.as_str() == "__it__"
            )
        };
        (matches!(left, PlayerFilter::IteratedPlayer) && is_iterated_controller(right))
            || (matches!(right, PlayerFilter::IteratedPlayer) && is_iterated_controller(left))
    }

    fn destroyed_subject(destroy: &crate::effects::DestroyEffect) -> &'static str {
        let Some(filter) = (match destroy.spec.base() {
            ChooseSpec::Object(filter) => Some(filter),
            _ => None,
        }) else {
            return "permanent";
        };
        if filter.card_types == vec![CardType::Creature] {
            "creature"
        } else if filter.card_types == vec![CardType::Artifact] {
            "artifact"
        } else if filter.card_types.contains(&CardType::Creature)
            || filter.card_types.contains(&CardType::Artifact)
            || filter.card_types.contains(&CardType::Enchantment)
            || filter.card_types.contains(&CardType::Planeswalker)
            || filter.card_types.contains(&CardType::Battle)
            || filter.card_types.contains(&CardType::Land)
        {
            "permanent"
        } else {
            "object"
        }
    }

    let destroyed_tag = effect_tag(destroy_effect)?;
    if for_each.tag != *destroyed_tag {
        return None;
    }
    let destroy =
        unwrap_tag_wrappers(destroy_effect).downcast_ref::<crate::effects::DestroyEffect>()?;

    let effects = if for_each.effects.len() == 1 {
        if let Some(sequence) = for_each.effects[0].downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            for_each.effects.as_slice()
        }
    } else {
        for_each.effects.as_slice()
    };
    if effects.len() != 4 {
        return None;
    }
    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let single_match_stop = matches!(
        &consult.stop_rule,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
            | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
    );
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !single_match_stop
    {
        return None;
    }
    let exile_effect = unwrap_tag_wrappers(&effects[1]);
    let exiles_match = exile_effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .is_some_and(|exile| {
            matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        })
        || exile_effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_to_zone| {
                move_to_zone.zone == Zone::Exile
                    && matches!(
                        move_to_zone.target.base(),
                        ChooseSpec::Tagged(tag) if tag == &consult.match_tag
                    )
            });
    if !exiles_match {
        return None;
    }
    let move_to_zone =
        unwrap_tag_wrappers(&effects[2]).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return None;
    }
    let shuffle = effects[3].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let shuffle_uses_revealed_card_controller = matches!(
        &shuffle.player,
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
            if tag == &consult.match_tag
    );
    if !same_tagged_iteration_player(&shuffle.player, &consult.player)
        && !shuffle_uses_revealed_card_controller
    {
        return None;
    }

    let mut destroy_text = describe_effect(destroy_effect)
        .trim_end_matches('.')
        .to_string();
    destroy_text = destroy_text.replace("artifacts or creatures", "artifacts and/or creatures");
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    Some(format!(
        "{destroy_text}. For each {} destroyed this way, its controller reveals cards from the top of their library until {selection} is revealed and exiles that card. Those players put the exiled cards onto the battlefield, then shuffle",
        destroyed_subject(destroy)
    ))
}

/// Render the executable, correctly staged form of the destroy / consult /
/// exile collection procedure. The tagged per-object loop proves that every
/// destroyed object gets one consult and exile; the sibling move proves the
/// exiled results are collected before entering the battlefield; and the
/// controller-grouped loop proves each participating player shuffles once.
pub(crate) fn describe_destroy_consult_exile_collected_then_shuffle(
    effects: &[&Effect],
) -> Option<String> {
    let [
        destroy_effect,
        collected_loop_effect,
        move_effect,
        shuffle_loop_effect,
    ] = effects
    else {
        return None;
    };
    let destroyed_tag = effect_tag(destroy_effect)?;
    let collection_tag = effect_tag(collected_loop_effect)?;
    let per_object = unwrap_tag_wrappers(collected_loop_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [consult_effect, exile_effect] = per_object.effects.as_slice() else {
        return None;
    };
    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if per_object.tag != *destroyed_tag {
        return None;
    }

    let move_to_zone =
        unwrap_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == collection_tag
        )
    {
        return None;
    }

    let shuffle_loop = unwrap_tag_wrappers(shuffle_loop_effect)
        .downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()?;
    let [shuffle_effect] = shuffle_loop.effects.as_slice() else {
        return None;
    };
    let shuffle = unwrap_tag_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle_loop.tag != *destroyed_tag
        || shuffle.player != PlayerFilter::IteratedPlayer
        || shuffle.target_spec.is_some()
    {
        return None;
    }

    // Reuse the established surface renderer through a legacy-shaped view
    // after the executable collection and controller relationships above have
    // been proved.
    let mut legacy_move = move_to_zone.clone();
    legacy_move.target = ChooseSpec::Tagged(consult.match_tag.clone());
    let legacy_loop = crate::effects::ForEachTaggedEffect::new(
        per_object.tag.clone(),
        vec![
            consult_effect.clone(),
            exile_effect.clone(),
            Effect::new(legacy_move),
            shuffle_effect.clone(),
        ],
    );
    describe_destroy_for_each_destroyed_consult_exile_put_shuffle(destroy_effect, &legacy_loop)
}

pub(crate) fn is_consult_reveal_put_battlefield_then_bottom(
    for_each: &crate::effects::ForEachTaggedEffect,
) -> bool {
    let effects = if for_each.effects.len() == 1 {
        if let Some(sequence) = for_each.effects[0].downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            for_each.effects.as_slice()
        }
    } else {
        for_each.effects.as_slice()
    };

    if effects.len() != 3 {
        return false;
    }

    let Some(consult) = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
    else {
        return false;
    };
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return false;
    }
    if consult.stop_rule != crate::effects::ConsultTopOfLibraryStopRule::FirstMatch {
        return false;
    }

    let move_effect = unwrap_tag_wrappers(&effects[1]);
    let Some(move_to_zone) = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>() else {
        return false;
    };
    if move_to_zone.zone != Zone::Battlefield || move_to_zone.to_top {
        return false;
    }
    if !matches!(
        &move_to_zone.target,
        ChooseSpec::Tagged(tag) if tag == &consult.match_tag
    ) {
        return false;
    }

    let Some(remainder) =
        effects[2].downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
    else {
        return false;
    };
    remainder.tag == consult.all_tag
        && remainder.keep_tagged.as_ref() == Some(&consult.match_tag)
        && remainder.order == crate::effects::consult_helpers::LibraryBottomOrder::Random
}

pub(crate) fn consult_reveal_put_battlefield_then_bottom_selection(
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if !is_consult_reveal_put_battlefield_then_bottom(for_each) {
        return None;
    }
    let effects = if for_each.effects.len() == 1 {
        if let Some(sequence) = for_each.effects[0].downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            for_each.effects.as_slice()
        }
    } else {
        for_each.effects.as_slice()
    };
    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    Some(strip_leading_article(&consult.filter.description()).to_string())
}

pub(crate) fn tagged_exile_any_number_target_creatures(effect: &Effect) -> Option<&str> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = tagged
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::WithCount(target, count) = &exile.spec else {
        return None;
    };
    if count.min != 0 || count.max.is_some() || count.random {
        return None;
    }
    let ChooseSpec::Target(inner) = target.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    if !matches!(filter.zone, None | Some(Zone::Battlefield))
        || filter.controller.is_some()
        || filter.card_types != vec![crate::types::CardType::Creature]
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
    {
        return None;
    }
    Some(tagged.tag.as_str())
}

/// A consult match can be moved either as the tagged collection directly or
/// through the generic `ForEachTagged(match_tag)` lowering. Those shapes are
/// semantically identical; collection-oriented renderers should not fall back
/// to exposing the iteration merely because lowering chose the latter form.
fn consult_match_move_to_zone<'a>(
    effect: &'a Effect,
    consult: &crate::effects::ConsultTopOfLibraryEffect,
    zone: Zone,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    let direct = unwrap_render_wrappers(effect);
    if let Some(move_to_zone) = direct.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == zone
        && !move_to_zone.to_top
        && matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return Some(move_to_zone);
    }

    let for_each = direct.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag != consult.match_tag {
        return None;
    }
    let nested = if let [sequence] = for_each.effects.as_slice()
        && let Some(sequence) =
            unwrap_render_wrappers(sequence).downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        for_each.effects.as_slice()
    };
    let [move_effect] = nested else {
        return None;
    };
    let move_to_zone =
        unwrap_render_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    (move_to_zone.zone == zone
        && !move_to_zone.to_top
        && (matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
            || matches!(
                move_to_zone.target.base(),
                ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"
            )))
    .then_some(move_to_zone)
}

pub(crate) fn render_consult_reveal_put_hand_then_bottom(effects: &[&Effect]) -> Option<String> {
    if effects.len() != 3 {
        return None;
    }

    let consult = structural_unwrap_render_wrappers(effects[0])
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return None;
    }

    consult_match_move_to_zone(effects[1], consult, Zone::Hand)?;

    let remainder = structural_unwrap_render_wrappers(effects[2])
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != consult.all_tag
        || remainder.keep_tagged.as_ref() != Some(&consult.match_tag)
    {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = if player == "you" {
        "your".to_string()
    } else {
        "their".to_string()
    };
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let pronoun_reveal_verb = if pronoun == "you" || pronoun == "they" {
        "reveal"
    } else {
        "reveals"
    };
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch => {
            with_indefinite_article(&selection)
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {
            with_indefinite_article(&selection)
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => {
            describe_counted_consult_stop(count, &selection)
        }
    };
    let order_text = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in any order".to_string()
        }
    };

    let matched_collection_is_singular = matches!(
        &consult.stop_rule,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
            | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
    );
    let matched_reference = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {
            "that card".to_string()
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(_) => format!(
            "the {} revealed this way",
            pluralize_noun_phrase(strip_leading_article(&selection))
        ),
    };

    if player == "you" && matched_collection_is_singular {
        Some(format!(
            "Reveal cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}. Put {matched_reference} into your hand and the rest on the bottom of {library_owner} library{order_text}"
        ))
    } else if player == "you" {
        Some(format!(
            "Reveal cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}. Put {matched_reference} into your hand, then put the rest of the revealed cards on the bottom of {library_owner} library{order_text}"
        ))
    } else {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, then {player} {put_verb} {matched_reference} into their hand and {put_verb} the rest of the revealed cards on the bottom of {library_owner} library{order_text}"
        ))
    }
}

/// Render a reveal-until partition whose matching cards move in a separately
/// authored instruction before the revealed remainder goes to the library
/// bottom. The shared consult tags, rather than a card-specific surface, prove
/// that the three effects form one partition while preserving the sentence
/// boundary before its disposition instructions.
pub(crate) fn render_consult_reveal_move_matches_then_bottom(effects: &[Effect]) -> Option<String> {
    let [consult_effect, move_effect, bottom_effect] = effects else {
        return None;
    };

    let consult = unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return None;
    }

    let move_to_zone =
        unwrap_render_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.to_top || move_to_zone.zone == Zone::Battlefield {
        return None;
    }
    let moves_consult_matches = match move_to_zone.target.base() {
        ChooseSpec::Tagged(tag) => tag == &consult.match_tag,
        ChooseSpec::All(filter) | ChooseSpec::Object(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == consult.match_tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
        }
        _ => false,
    };
    if !moves_consult_matches {
        return None;
    }

    let bottom = unwrap_render_wrappers(bottom_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if bottom.tag != consult.all_tag || bottom.keep_tagged.as_ref() != Some(&consult.match_tag) {
        return None;
    }

    let consult_text =
        capitalize_first(describe_effect(consult_effect).trim().trim_end_matches('.'));
    let move_text = capitalize_first(describe_effect(move_effect).trim().trim_end_matches('.'));
    let rendered_bottom = describe_effect(bottom_effect);
    let bottom_text = rendered_bottom
        .trim()
        .trim_end_matches('.')
        .strip_prefix("Put the rest of the revealed cards")
        .map(|suffix| format!("put the rest{suffix}"))
        .unwrap_or_else(|| lowercase_first(rendered_bottom.trim().trim_end_matches('.')));

    Some(cleanup_decompiled_text(&format!(
        "{consult_text}. {move_text}, then {bottom_text}"
    )))
}

pub(crate) fn render_consult_reveal_put_hand_rest_exile(effects: &[&Effect]) -> Option<String> {
    if effects.len() != 3 {
        return None;
    }

    let consult = unwrap_render_wrappers(effects[0]).downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return None;
    }

    consult_match_move_to_zone(effects[1], consult, Zone::Hand)?;

    let remainder = unwrap_render_wrappers(effects[2]).downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if remainder.tag != consult.all_tag {
        return None;
    }
    let [conditional_effect] = remainder.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let matches_consult_hit = matches!(
        &conditional.condition,
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag == &consult.match_tag
                && *filter
                    == ObjectFilter::default()
                        .same_stable_id_as_tagged(crate::tag::TagKey::from("__it__"))
    ) || matches!(
        &conditional.condition,
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag.as_str() == "__it__"
                && *filter
                    == ObjectFilter::tagged(crate::tag::TagKey::from(
                        consult.match_tag.as_str()
                    ))
    );
    if !matches_consult_hit {
        return None;
    }
    if !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
        return None;
    }
    let exile_effect = unwrap_tag_wrappers(&conditional.if_false[0]);
    let move_remainder = exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_remainder.zone != Zone::Exile || !move_remainder.to_top {
        return None;
    }
    if !matches!(
        &move_remainder.target,
        ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"
    ) && !matches!(&move_remainder.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let pronoun_reveal_verb = if pronoun == "you" || pronoun == "they" {
        "reveal"
    } else {
        "reveals"
    };
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch => selection.clone(),
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {
            selection.clone()
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => {
            describe_counted_consult_stop(count, &selection)
        }
    };

    if player == "you" {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}. Put that card into your hand and exile all other cards revealed this way"
        ))
    } else {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, then {player} {put_verb} that card into their hand and exile all other cards revealed this way"
        ))
    }
}

pub(crate) fn render_consult_reveal_put_battlefield_rest_graveyard(
    effects: &[&Effect],
) -> Option<String> {
    if effects.len() != 3 {
        return None;
    }

    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return None;
    }

    let (move_to_zone, conditional_followups) = if let Some(move_to_zone) =
        consult_match_move_to_zone(effects[1], consult, Zone::Battlefield)
    {
        (move_to_zone, None)
    } else {
        let conditional = unwrap_render_wrappers(effects[1])
            .downcast_ref::<crate::effects::ConditionalEffect>()?;
        if !conditional.if_false.is_empty()
            || !matches!(
                &conditional.condition,
                crate::ConditionExpr::TaggedObjectMatches(tag, filter)
                    if tag == &consult.match_tag && filter == &consult.filter
            )
        {
            return None;
        }
        let (move_effect, followups) = conditional.if_true.split_first()?;
        let move_to_zone = consult_match_move_to_zone(move_effect, consult, Zone::Battlefield)?;
        (move_to_zone, Some(followups))
    };
    if !move_to_zone.enters_with_counters.is_empty()
        || move_to_zone.enters_attacking
        || move_to_zone.attack_target_mode.is_some()
        || move_to_zone.enters_face_down
        || move_to_zone.enters_transformed
        || move_to_zone.transfer_exiled_with_source_links
    {
        return None;
    }

    let remainder =
        unwrap_render_wrappers(effects[2]).downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if !for_each_moves_unselected_to_zone(
        remainder,
        consult.all_tag.as_str(),
        consult.match_tag.as_str(),
        Zone::Graveyard,
    ) {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = if matches!(consult.player, PlayerFilter::Target(_)) {
        "their".to_string()
    } else {
        describe_possessive_player_filter(&consult.player)
    };
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let pronoun_reveal_verb = if pronoun == "you" || pronoun == "they" {
        "reveal"
    } else {
        "reveals"
    };
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch => {
            with_indefinite_article(&selection)
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {
            with_indefinite_article(&selection)
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => {
            describe_counted_consult_stop(count, &selection)
        }
    };

    let control_suffix = match move_to_zone.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => " under its owner's control",
        crate::effects::BattlefieldController::You => " under your control",
    };
    let tapped_suffix = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };

    if let Some(followups) = conditional_followups {
        let mut rendered_followups = Vec::new();
        for effect in followups {
            let rendered = describe_effect(effect)
                .trim()
                .trim_end_matches('.')
                .to_string();
            if rendered.is_empty() || rendered.contains(". ") {
                return None;
            }
            rendered_followups.push(lowercase_first(&rendered));
        }
        let followup_suffix = if rendered_followups.is_empty() {
            String::new()
        } else {
            format!(" and {}", rendered_followups.join(" and "))
        };
        let consult_text =
            capitalize_first(describe_effect(effects[0]).trim().trim_end_matches('.'));
        let graveyard_owner = if matches!(consult.player, PlayerFilter::Target(_)) {
            "that player's".to_string()
        } else {
            describe_possessive_player_filter(&consult.player)
        };
        return Some(format!(
            "{consult_text}. If {selection} is revealed this way, put it onto the battlefield{tapped_suffix}{control_suffix}{followup_suffix}. Put the rest of the revealed cards into {graveyard_owner} graveyard"
        ));
    }

    if player == "you" {
        Some(format!(
            "Reveal cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}. Put that card onto the battlefield{tapped_suffix}{control_suffix} and put all other cards revealed this way into your graveyard"
        ))
    } else if move_to_zone.battlefield_controller == crate::effects::BattlefieldController::You {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}. Put that card onto the battlefield{tapped_suffix}{control_suffix} and the rest into their graveyard"
        ))
    } else {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, then {player} {put_verb} that card onto the battlefield{tapped_suffix}{control_suffix} and {put_verb} all other cards revealed this way into their graveyard"
        ))
    }
}

pub(crate) fn render_consult_reveal_put_hand_rest_graveyard(effects: &[&Effect]) -> Option<String> {
    if effects.len() != 3 {
        return None;
    }

    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return None;
    }

    consult_match_move_to_zone(effects[1], consult, Zone::Hand)?;

    let remainder = effects[2].downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if remainder.tag != consult.all_tag {
        return None;
    }
    let [conditional_effect] = remainder.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    // Accept two equivalent conditional formats:
    // Format 1: TaggedObjectMatches(match_tag, filter_with_it_constraint)
    // Format 2: TaggedObjectMatches(__it__, filter_with_match_tag_constraint)
    let condition_ok = match &conditional.condition {
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag == &consult.match_tag
                && *filter
                    == ObjectFilter::default()
                        .same_stable_id_as_tagged(crate::tag::TagKey::from("__it__")) =>
        {
            true
        }
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag.as_str() == "__it__"
                && filter
                    .tagged_constraints
                    .iter()
                    .any(|c| c.tag == consult.match_tag) =>
        {
            true
        }
        _ => false,
    };
    if !condition_ok {
        return None;
    }
    if !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
        return None;
    }
    let graveyard_effect = unwrap_tag_wrappers(&conditional.if_false[0]);
    let move_remainder = graveyard_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_remainder.zone != Zone::Graveyard || move_remainder.to_top {
        return None;
    }
    if !matches!(
        &move_remainder.target,
        ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"
    ) && !matches!(&move_remainder.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let pronoun_reveal_verb = if pronoun == "you" || pronoun == "they" {
        "reveal"
    } else {
        "reveals"
    };
    let selection = describe_search_selection_with_cards(&consult.filter.description());
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => {
            describe_counted_consult_stop(count, &selection)
        }
    };

    if player == "you" {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, put that card into your hand, and put all other cards revealed this way into your graveyard"
        ))
    } else {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, then {player} {put_verb} that card into their hand and {put_verb} all other cards revealed this way into their graveyard"
        ))
    }
}

pub(crate) fn render_consult_reveal_put_all_revealed_into_graveyard(
    effects: &[&Effect],
) -> Option<String> {
    if effects.len() != 2 {
        return None;
    }

    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return None;
    }

    let move_effect = unwrap_tag_wrappers(effects[1]);
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard || move_to_zone.to_top {
        return None;
    }
    if !matches!(
        &move_to_zone.target,
        ChooseSpec::Tagged(tag) if tag == &consult.all_tag
    ) {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let pronoun_reveal_verb = if pronoun == "you" || pronoun == "they" {
        "reveal"
    } else {
        "reveals"
    };
    let selection = describe_search_selection_with_cards(&consult.filter.description());
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => {
            describe_counted_consult_stop(count, &selection)
        }
    };
    let graveyard_owner = describe_possessive_player_filter(&consult.player);

    if consult.player == PlayerFilter::Opponent {
        return Some(format!(
            "Each opponent reveals cards from the top of their library until they reveal {stop_text}, then puts all cards revealed this way into their graveyard"
        ));
    }
    if player == "you" {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, put all cards revealed this way into {graveyard_owner} graveyard"
        ))
    } else {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, then {player} {put_verb} all cards revealed this way into {graveyard_owner} graveyard"
        ))
    }
}

pub(crate) fn render_consult_reveal_put_all_revealed_into_hand(
    effects: &[&Effect],
) -> Option<String> {
    if effects.len() != 2 {
        return None;
    }

    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal {
        return None;
    }

    let move_effect = unwrap_tag_wrappers(effects[1]);
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand || move_to_zone.to_top {
        return None;
    }
    if !matches!(
        &move_to_zone.target,
        ChooseSpec::Tagged(tag) if tag == &consult.all_tag
    ) {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let reveal_verb = player_verb(&player, "reveal", "reveals");
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let pronoun_reveal_verb = if pronoun == "you" || pronoun == "they" {
        "reveal"
    } else {
        "reveals"
    };
    let selection = describe_search_selection_with_cards(&consult.filter.description());
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => {
            describe_counted_consult_stop(count, &selection)
        }
    };
    let hand_owner = describe_possessive_player_filter(&consult.player);

    if player == "you" {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, then put all cards revealed this way into {hand_owner} hand"
        ))
    } else {
        Some(format!(
            "{player} {reveal_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_reveal_verb} {stop_text}, then {player} {put_verb} all cards revealed this way into {hand_owner} hand"
        ))
    }
}

pub(crate) fn render_each_player_exile_top_then_cast_any_number(
    effects: &[&Effect],
) -> Option<String> {
    let [for_players_effect, for_each_effect] = effects else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 1 {
        return None;
    }
    let exile_top =
        for_players.effects[0].downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile_top.player != PlayerFilter::IteratedPlayer
        || exile_top.count != Value::Fixed(1)
        || exile_top.accumulated_tags.len() != 1
    {
        return None;
    }
    let exiled_tag = &exile_top.accumulated_tags[0];

    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.filter.zone != Some(Zone::Exile)
        || !for_each
            .filter
            .excluded_card_types
            .contains(&crate::types::CardType::Land)
        || !for_each.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *exiled_tag
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }

    let [may_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag.as_str() != "__it__"
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    Some(
        "Exile the top card of each player's library, then you may cast any number of spells from among those cards without paying their mana costs"
            .to_string(),
    )
}

/// The collective free-cast permission on its own, for the very common authoring
/// where oracle puts the exile and the permission in SEPARATE sentences:
///
/// `Exile the top four cards of your library. You may cast any number of spells
///  with mana value 5 or less from among them without paying their mana costs.`
///
/// `render_exile_top_then_cast_any_number_with_mana_value_cap` only matches the
/// one-sentence "exile ..., then you may cast ..." form, because it needs both
/// effects in a single slice — across two resolution segments that pair never
/// forms, and the permission segment fell through to the generic `MayEffect`
/// wrapper, which prefixed "You may " onto a clause that already carried its own
/// permission ("You may For each nonland card ... you may cast it ...").
pub(crate) fn render_may_cast_any_number_from_among_exiled(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if !matches!(may.decider, None | Some(PlayerFilter::You)) {
        return None;
    }
    let [for_each_effect] = may.effects.as_slice() else {
        return None;
    };
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.filter.zone != Some(Zone::Exile)
        || !for_each
            .filter
            .excluded_card_types
            .contains(&crate::types::CardType::Land)
    {
        return None;
    }
    // The "them" antecedent is the set some earlier sentence exiled, which is
    // exactly what a sentence-helper exiled tag records.
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
            && crate::cards::is_sentence_helper_tag(constraint.tag.as_str(), "exiled")
    }) {
        return None;
    }

    let [inner_may_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let inner_may = inner_may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = inner_may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag.as_str() != "__it__"
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    let cap = match &for_each.filter.mana_value {
        None => String::new(),
        Some(crate::filter::Comparison::LessThanOrEqual(value)) => {
            format!(" with mana value {value} or less")
        }
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) => {
            format!(" with mana value {} or less", describe_value(value))
        }
        _ => return None,
    };
    Some(format!(
        "You may cast any number of spells{cap} from among them without paying their mana costs"
    ))
}

pub(crate) fn render_exile_top_then_cast_any_number_with_mana_value_cap(
    effects: &[&Effect],
) -> Option<String> {
    let [exile_effect, may_effect] = effects else {
        return None;
    };
    let exile_top = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile_top.moved_tags.len() != 1 {
        return None;
    }
    let exiled_tag = &exile_top.moved_tags[0];

    let outer_may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(outer_may.decider, None | Some(PlayerFilter::You)) || outer_may.effects.len() != 1
    {
        return None;
    }
    let for_each = outer_may.effects[0].downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.filter.zone != Some(Zone::Exile)
        || !for_each
            .filter
            .excluded_card_types
            .contains(&crate::types::CardType::Land)
        || !for_each.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *exiled_tag
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }
    let mana_value = match &for_each.filter.mana_value {
        Some(crate::filter::Comparison::LessThanOrEqual(value)) => value.to_string(),
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) => describe_value(value),
        _ => return None,
    };

    let [inner_may_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let inner_may = inner_may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = inner_may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag.as_str() != "__it__"
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    let exile_text = if matches!(
        exile_top.player,
        PlayerFilter::Target(ref target) if **target == PlayerFilter::Opponent
    ) {
        let count_text = describe_value(&exile_top.count);
        format!("Target opponent exiles the top {count_text} cards of their library")
    } else {
        describe_exile_top_clause(exile_top, false)?.0
    };
    Some(format!(
        "{exile_text}. You may cast any number of spells with mana value {mana_value} or less from among them without paying their mana costs"
    ))
}

pub(crate) fn render_shuffle_exile_top_then_cast_any_number_with_mana_value_cap(
    effects: &[&Effect],
) -> Option<String> {
    let [shuffle_effect, exile_effect, may_effect] = effects else {
        return None;
    };
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return None;
    }
    let tail =
        render_exile_top_then_cast_any_number_with_mana_value_cap(&[exile_effect, may_effect])?;
    // When the immediately preceding typed action already says "Shuffle your
    // library", Oracle commonly elides the repeated source from "exile the
    // top N cards". This compactor is gated to that exact same-player
    // shuffle/exile/permission triple, so the shorter surface remains backed
    // by structural library provenance.
    let tail = tail.replacen(" of your library.", ".", 1);
    Some(format!(
        "Shuffle your library, then {}",
        lowercase_first(&tail)
    ))
}

fn exiled_collection_filter_text(
    filter: &ObjectFilter,
    exiled_tag: &crate::TagKey,
) -> Option<String> {
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *exiled_tag
            && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }
    let mut visible = filter.clone();
    visible.zone = None;
    visible.owner = None;
    visible.controller = None;
    visible.tagged_constraints.retain(|constraint| {
        !(constraint.tag == *exiled_tag
            && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject)
    });
    let mut text = strip_leading_article(&visible.description()).to_string();
    if visible.excluded_card_types.len() > 1 {
        let flat = visible
            .excluded_card_types
            .iter()
            .map(|card_type| format!("non{}", describe_card_type_word_local(*card_type)))
            .collect::<Vec<_>>()
            .join(" ");
        let punctuated = visible
            .excluded_card_types
            .iter()
            .map(|card_type| format!("non{}", describe_card_type_word_local(*card_type)))
            .collect::<Vec<_>>()
            .join(", ");
        text = text.replacen(&flat, &punctuated, 1);
    }
    if text.contains("permanent") {
        text = text.replacen("permanent", "card", 1);
    } else if !text.contains("card") {
        text.push_str(" card");
    }
    Some(text)
}

/// Renders a structurally linked hidden pile followed by one manifest/cloak
/// operation. The shared accumulating tag proves that the targeted object and
/// the top-library cards are exactly the cards shuffled and turned face down.
pub(crate) fn describe_face_down_pile_then_manifest(effects: &[Effect]) -> Option<String> {
    fn wrapper_chain_contains_tag(effect: &Effect, expected: &crate::TagKey) -> bool {
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return wrapper_chain_contains_tag(&with_id.effect, expected);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return tag_all.tag == *expected
                || wrapper_chain_contains_tag(&tag_all.effect, expected);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return tagged.tag == *expected || wrapper_chain_contains_tag(&tagged.effect, expected);
        }
        false
    }

    let (target_only_effect, target_only, body) = if let [target_only_effect, body @ ..] = effects
        && let Some(target_only) = unwrap_basic_tag_wrappers(target_only_effect)
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
    {
        (Some(target_only_effect), Some(target_only), body)
    } else {
        (None, None, effects)
    };
    let [target_exile_effect, library_exile_effect, manifest_effect] = body else {
        return None;
    };
    let manifest = unwrap_basic_tag_wrappers(manifest_effect)
        .downcast_ref::<crate::effects::ManifestObjectsEffect>()?;
    let ChooseSpec::Tagged(pile_tag) = manifest.target.base() else {
        return None;
    };
    let target_exile = unwrap_basic_tag_wrappers(target_exile_effect)
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let library_exile = unwrap_basic_tag_wrappers(library_exile_effect)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let target_choice_matches = target_only.is_none_or(|target_only| {
        target_specs_select_same_objects(&target_only.target, &target_exile.spec)
            || matches!(
                target_exile.spec.base(),
                ChooseSpec::Tagged(target_tag)
                    if target_only_effect.is_some_and(|effect| {
                        wrapper_chain_contains_tag(effect, target_tag)
                    })
            )
    });

    if !target_exile.face_down
        || !library_exile.face_down
        || !library_exile.moved_tags.is_empty()
        || library_exile.accumulated_tags.as_slice() != [pile_tag.clone()]
        || !wrapper_chain_contains_tag(target_exile_effect, pile_tag)
        || manifest.controller != PlayerFilter::You
        || !target_choice_matches
    {
        return None;
    }

    let described_target;
    let target = if let Some(target_only) = target_only {
        described_target = describe_choose_spec(&target_only.target);
        described_target.as_str()
    } else {
        // The typed exile already proves both the selection and its hidden
        // movement. Describe the selection directly instead of round-tripping
        // through effect prose, whose capitalization and wrapper surface are
        // intentionally contextual.
        described_target = describe_choose_spec(&target_exile.spec);
        described_target.as_str()
    };
    let (library, _) = describe_exile_top_clause(library_exile, false)?;
    let library = library.strip_prefix("Exile ")?;
    let action = if manifest.cloak { "cloak" } else { "manifest" };
    let transition = if manifest.shuffle {
        format!("shuffle that pile, then {action} those cards")
    } else {
        format!("then {action} those cards")
    };
    let tapped = if manifest.tapped {
        ". They enter tapped"
    } else {
        ""
    };

    Some(format!(
        "Exile {target} and {library} in a face-down pile, {transition}{tapped}"
    ))
}

fn for_each_puts_chosen_onto_battlefield(
    effect: &Effect,
    chosen_tag: &crate::TagKey,
) -> Option<(bool, PlayerFilter)> {
    let for_each = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag != *chosen_tag || for_each.effects.len() != 1 {
        return None;
    }
    let put = for_each.effects[0].downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    if !matches!(put.target.base(), ChooseSpec::Iterated) {
        return None;
    }
    Some((put.tapped, put.controller.clone()))
}

pub(crate) fn render_exile_top_then_put_from_among_onto_battlefield(
    effects: &[&Effect],
) -> Option<String> {
    let [exile_effect, selection_effect, put_effect] = effects else {
        return None;
    };
    let exile_top = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let [exiled_tag] = exile_top.moved_tags.as_slice() else {
        return None;
    };

    let (selection, chosen_tag, optional, _all_matching) = if let Some(choose) =
        selection_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
    {
        if choose.chooser != PlayerFilter::You
            || choose.zone != Some(Zone::Exile)
            || !choose.additional_zones.is_empty()
            || choose.is_search
            || choose.reveal
        {
            return None;
        }
        let selection = exiled_collection_filter_text(&choose.filter, exiled_tag)?;
        let count_text = match (choose.count.min, choose.count.max) {
            (1, Some(1)) => with_indefinite_article(&selection),
            (0, Some(1)) => format!("up to one {selection}"),
            (0, None) => format!("any number of {}", pluralize_noun_phrase(&selection)),
            _ => return None,
        };
        (count_text, &choose.tag, choose.count.min == 0, false)
    } else {
        let tag_matching =
            selection_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        if tag_matching.zone != Some(Zone::Exile) || !tag_matching.additional_zones.is_empty() {
            return None;
        }
        let selection = exiled_collection_filter_text(&tag_matching.filter, exiled_tag)?;
        (
            format!("all {}", pluralize_noun_phrase(&selection)),
            &tag_matching.tag,
            false,
            true,
        )
    };

    let (tapped, controller) = for_each_puts_chosen_onto_battlefield(put_effect, chosen_tag)?;
    if controller != PlayerFilter::You {
        return None;
    }
    let tapped_suffix = if tapped { " tapped" } else { "" };
    let control_suffix = if exile_top.player != PlayerFilter::You {
        " under your control"
    } else {
        ""
    };
    let exile_text = describe_exile_top_clause(exile_top, false)?.0;
    let transition = if exile_top.player != PlayerFilter::You {
        if optional {
            format!(", then you may put {selection} from among them")
        } else {
            format!(", then put {selection} from among them")
        }
    } else if optional {
        format!(". You may put {selection} from among them")
    } else {
        format!(". Put {selection} from among them")
    };
    Some(format!(
        "{exile_text}{transition} onto the battlefield{tapped_suffix}{control_suffix}"
    ))
}

pub(crate) fn render_random_exile_choose_copy_then_cast_copy(
    effects: &[&Effect],
) -> Option<String> {
    let [exile_effect, choose_effect, may_effect] = effects else {
        return None;
    };
    let (exiled_tag, exile) =
        if let Some(tag_all) = exile_effect.downcast_ref::<crate::effects::TagAllEffect>() {
            (
                &tag_all.tag,
                tag_all
                    .effect
                    .downcast_ref::<crate::effects::ExileEffect>()?,
            )
        } else {
            let tagged = exile_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
            (
                &tagged.tag,
                tagged
                    .effect
                    .downcast_ref::<crate::effects::ExileEffect>()?,
            )
        };
    if exile.face_down {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Exile)
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count.random
    {
        return None;
    }
    let selection = exiled_collection_filter_text(&choose.filter, exiled_tag)?;
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| decider != &PlayerFilter::You)
        || may.effects.len() != 1
    {
        return None;
    }
    let cast = may.effects[0].downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != choose.tag
        || cast.player != PlayerFilter::You
        || !cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.allow_land
        || cast.cost_reduction.is_some()
    {
        return None;
    }
    let exile_text = describe_effect(exile_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{exile_text}. Choose {} from among them and copy it. You may cast the copy without paying its mana cost",
        with_indefinite_article(&selection)
    ))
}

pub(crate) fn render_sacrifice_then_consult_reveal_put_battlefield_rest_bottom(
    effects: &[&Effect],
) -> Option<String> {
    if effects.len() != 5 {
        return None;
    }

    let choose = effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_primary_zone(choose) != Some(Zone::Battlefield) || choose.is_search {
        return None;
    }

    let sacrifice = sacrifice_view_unwrapped(effects[1])?;
    let exact_sentence_helper_set =
        sacrifice_tracks_exact_sentence_helper_chosen_set(sacrifice, choose);
    if (sacrifice.player != &choose.chooser && !exact_sentence_helper_set)
        || !sacrifice_uses_chosen_tag(sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    let consult = effects[2].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.player != choose.chooser
    {
        return None;
    }

    let move_effect = unwrap_tag_wrappers(effects[3]);
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield || move_to_zone.to_top {
        return None;
    }
    if !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &consult.match_tag) {
        return None;
    }

    let bottom =
        effects[4].downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if bottom.tag != consult.all_tag || bottom.keep_tagged.as_ref() != Some(&consult.match_tag) {
        return None;
    }

    let sacrificed_subject = if choose.filter.card_types == vec![crate::types::CardType::Creature]
        && choose.filter.subtypes.len() == 1
    {
        pluralize_word(&choose.filter.subtypes[0].to_string().to_ascii_lowercase())
    } else {
        let mut desc = choose.filter.description();
        for suffix in [" you control", " you own"] {
            if let Some(rest) = desc.strip_suffix(suffix) {
                desc = rest.to_string();
                break;
            }
        }
        pluralize_noun_phrase(strip_leading_article(&desc))
    };
    let sacrifice_text = if choose.count.is_dynamic_x() {
        format!("Sacrifice X {sacrificed_subject}")
    } else if let Some(exact) = choose_exact_count(choose) {
        let count_text = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
        format!("Sacrifice {count_text} {sacrificed_subject}")
    } else {
        format!(
            "Sacrifice {} {sacrificed_subject}",
            describe_choice_count(&choose.count)
        )
    };

    let selection = describe_search_selection_with_cards(&consult.filter.description());
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => selection,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count)
            if is_effect_count_reference(count, None) =>
        {
            format!(
                "a number of {} equal to the number of {} sacrificed this way",
                pluralize_noun_phrase(&selection),
                sacrificed_subject
            )
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => {
            describe_counted_consult_stop(count, &selection)
        }
    };

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let reveal_clause = if player == "you" {
        format!("reveal cards from the top of {library_owner} library")
    } else {
        format!(
            "{player} {} cards from the top of {library_owner} library",
            player_verb(&player, "reveal", "reveals")
        )
    };
    let pronoun = if player == "you" { "you" } else { "they" };
    let pronoun_reveal_verb = if pronoun == "you" || pronoun == "they" {
        "reveal"
    } else {
        "reveals"
    };
    let rest_order = match bottom.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => format!(
            " in an order chosen by {}",
            describe_player_filter(&bottom.player)
        ),
    };

    Some(format!(
        "{sacrifice_text}, then {reveal_clause} until {pronoun} {pronoun_reveal_verb} {stop_text}. Put those cards onto the battlefield and the rest on the bottom of {library_owner} library{rest_order}"
    ))
}

pub(crate) fn render_repeated_named_searches_to_hand(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let first_choose = effects
        .first()?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !first_choose.is_search || choose_exact_count(first_choose) != Some(1) {
        return None;
    }
    let search_origin = describe_search_origin_zones(first_choose)?;
    let search_owner_filter = first_choose
        .filter
        .owner
        .as_ref()
        .unwrap_or(&first_choose.chooser);

    let mut chooses = Vec::new();
    let mut idx = 0usize;
    while idx < effects.len() {
        let Some(choose) = effects[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        else {
            break;
        };
        if choose.tag != first_choose.tag
            || !choose.is_search
            || choose_exact_count(choose) != Some(1)
            || choose.chooser != first_choose.chooser
            || choose_search_zones(choose) != choose_search_zones(first_choose)
        {
            break;
        }
        if choose.filter.name.is_none()
            || choose.filter.card_types != first_choose.filter.card_types
            || choose.filter.owner != first_choose.filter.owner
        {
            return None;
        }
        chooses.push(choose);
        idx += 1;
    }
    if chooses.len() <= 1 || idx + 1 >= effects.len() {
        return None;
    }
    let reveal = effects[idx].downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != first_choose.tag {
        return None;
    }
    let move_effect = unwrap_tag_wrappers(effects[idx + 1]);
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand || move_to_zone.to_top {
        return None;
    }
    if !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &first_choose.tag) {
        return None;
    }
    let shuffle = effects
        .get(idx + 2)
        .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
    if let Some(shuffle) = shuffle
        && shuffle.player != *search_owner_filter
    {
        return None;
    }

    let names = chooses
        .iter()
        .map(|choose| format!("a card named {}", choose.filter.name.as_ref().unwrap()))
        .collect::<Vec<_>>();
    let shuffle_text = if shuffle.is_some() {
        if describe_player_filter(search_owner_filter) == "you" {
            ", then shuffle".to_string()
        } else {
            ", then that player shuffles".to_string()
        }
    } else {
        String::new()
    };
    Some((
        format!(
            "Search {search_origin} for {}. Reveal those cards, put them into your hand{shuffle_text}",
            join_with_and(&names)
        ),
        chooses.len() + 2 + usize::from(shuffle.is_some()),
    ))
}

pub(crate) fn describe_single_search_selection_with_cards(
    choose: &crate::effects::ChooseObjectsEffect,
) -> String {
    let mut display_filter = choose.filter.clone();
    if display_filter.owner.as_ref() == Some(&choose.chooser) {
        display_filter.owner = None;
    }
    display_filter.zone = None;
    if let Some(name) = display_filter.name.as_ref() {
        return format!("a card named {name}");
    }
    let filter_text = display_filter.description();
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    for owner_prefix in [
        "target player's ",
        "that player's ",
        "their ",
        "your ",
        "an opponent's ",
    ] {
        if let Some(rest) = card_desc.strip_prefix(owner_prefix) {
            card_desc = rest.to_string();
            break;
        }
    }
    card_desc = strip_leading_article(&card_desc).to_string();
    card_desc = card_desc.replace("permanent named ", "card named ");
    if let Some(name) = card_desc.strip_prefix("named ") {
        card_desc = format!("card named {name}");
    } else if let Some(rest) = card_desc.strip_prefix("card ") {
        card_desc = format!("{rest} card");
    }
    if !card_desc.contains(" card") {
        card_desc = format!("{card_desc} card");
    }
    with_indefinite_article(&card_desc)
}

pub(crate) fn render_single_named_search_to_hand_with_conditional_shuffle(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let choose = effects
        .first()?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search || choose_exact_count(choose) != Some(1) {
        return None;
    }
    let search_zones = choose_search_zones(choose)?;
    if !search_zones.contains(&Zone::Library) || search_zones.len() < 2 {
        return None;
    }

    let reveal = effects
        .get(1)?
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != choose.tag {
        return None;
    }

    let (move_effect_id, move_to_hand) = if let Some(with_id) = effects
        .get(2)?
        .downcast_ref::<crate::effects::WithIdEffect>(
    ) {
        let for_each = with_id
            .effect
            .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
        (Some(with_id.id), for_each)
    } else {
        (
            None,
            effects
                .get(2)?
                .downcast_ref::<crate::effects::ForEachTaggedEffect>()?,
        )
    };
    if move_to_hand.tag != choose.tag
        || !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
    {
        return None;
    }

    let mut consumed = 3usize;
    let mut shuffle_text = String::new();
    if let Some(if_effect) = effects
        .get(3)
        .and_then(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
    {
        let move_effect_id = move_effect_id?;
        if if_effect.condition != move_effect_id
            || if_effect.predicate != EffectPredicate::Happened
            || !if_effect.else_.is_empty()
            || if_effect.then.len() != 1
        {
            return None;
        }
        let shuffle = if_effect.then[0].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
        if shuffle.player != choose.chooser {
            return None;
        }
        let player = describe_player_filter(&choose.chooser);
        shuffle_text = if player == "you" {
            ". If you search your library this way, shuffle".to_string()
        } else {
            format!(
                ". If {player} searched their library this way, {player} {}",
                player_verb(&player, "shuffle", "shuffles")
            )
        };
        consumed = 4;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let selection = describe_single_search_selection_with_cards(choose);
    let hand = describe_possessive_player_filter(&choose.chooser);
    Some((
        format!(
            "Search {search_origin} for {selection}, reveal it, and put it into {hand} hand{shuffle_text}"
        ),
        consumed,
    ))
}

pub(crate) fn keyword_label_from_static_ability(
    ability: crate::static_abilities::StaticAbilityId,
) -> Option<&'static str> {
    Some(match ability {
        crate::static_abilities::StaticAbilityId::Flying => "flying",
        crate::static_abilities::StaticAbilityId::FirstStrike => "first strike",
        crate::static_abilities::StaticAbilityId::DoubleStrike => "double strike",
        crate::static_abilities::StaticAbilityId::Deathtouch => "deathtouch",
        crate::static_abilities::StaticAbilityId::Haste => "haste",
        crate::static_abilities::StaticAbilityId::Hexproof => "hexproof",
        crate::static_abilities::StaticAbilityId::Indestructible => "indestructible",
        crate::static_abilities::StaticAbilityId::Lifelink => "lifelink",
        crate::static_abilities::StaticAbilityId::Menace => "menace",
        crate::static_abilities::StaticAbilityId::Reach => "reach",
        crate::static_abilities::StaticAbilityId::Trample => "trample",
        crate::static_abilities::StaticAbilityId::Vigilance => "vigilance",
        _ => return None,
    })
}

pub(crate) fn choose_references_revealed_tag(
    choose: &crate::effects::ChooseObjectsEffect,
    revealed_tag: &crate::TagKey,
) -> bool {
    choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == *revealed_tag
    })
}

pub(crate) fn choose_excludes_chosen_tag(
    choose: &crate::effects::ChooseObjectsEffect,
    chosen_tag: &crate::TagKey,
) -> bool {
    choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            && constraint.tag == *chosen_tag
    })
}

pub(crate) fn revealed_choice_label(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if let Some(label) = structural_revealed_choice_label(choose) {
        return Some(label);
    }
    if choose.filter.card_types.is_empty()
        && choose.filter.all_card_types.is_empty()
        && choose.filter.subtypes.len() == 1
        && choose.filter.static_abilities.is_empty()
        && choose.filter.any_of.is_empty()
    {
        return Some(format!("{} card", choose.filter.subtypes[0]));
    }
    if choose.filter.card_types.is_empty() && choose.filter.static_abilities.len() == 1 {
        return Some(format!(
            "card with {}",
            keyword_label_from_static_ability(choose.filter.static_abilities[0])?
        ));
    }
    None
}

pub(crate) fn valid_revealed_choice<'a>(
    choose: &'a crate::effects::ChooseObjectsEffect,
    revealed_tag: &crate::TagKey,
    chosen_tag: Option<&crate::TagKey>,
) -> Option<(String, &'a crate::TagKey)> {
    if choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose.count.min > 1
        || choose.count.max != Some(1)
        || !choose_references_revealed_tag(choose, revealed_tag)
    {
        return None;
    }
    if let Some(chosen_tag) = chosen_tag
        && (choose.tag != *chosen_tag || !choose_excludes_chosen_tag(choose, chosen_tag))
    {
        return None;
    }
    Some((revealed_choice_label(choose)?, &choose.tag))
}

pub(crate) fn conditional_revealed_choice<'a>(
    conditional: &'a crate::effects::ConditionalEffect,
    revealed_tag: &crate::TagKey,
    chosen_tag: Option<&crate::TagKey>,
) -> Option<(String, &'a crate::TagKey)> {
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let choose = conditional.if_true[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (label, tag) = valid_revealed_choice(choose, revealed_tag, chosen_tag)?;
    let Condition::ValueComparison {
        left:
            Value::SpellsCastThisTurnMatching {
                player: PlayerFilter::You,
                filter,
                exclude_source: false,
            },
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    } = &conditional.condition
    else {
        return None;
    };
    if filter.card_types != choose.filter.card_types
        || !filter.excluded_card_types.contains(&CardType::Creature)
    {
        return None;
    }
    Some((label, tag))
}

pub(crate) fn for_each_returns_iterated_to_hand(effect: &Effect, tag: &crate::TagKey) -> bool {
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    if for_each.tag != *tag || for_each.effects.len() != 1 {
        return false;
    }
    let effect = unwrap_tag_wrappers(&for_each.effects[0]);
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        return matches!(return_to_hand.spec.base(), ChooseSpec::Iterated);
    }
    matches!(
        effect.downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == Zone::Hand
                && matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    )
}

pub(crate) fn for_each_reveals_iterated_tag(effect: &Effect, tag: &crate::TagKey) -> bool {
    if let Some(reveal) =
        unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::RevealTaggedEffect>()
    {
        return reveal.tag == *tag;
    }
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    if for_each.tag != *tag || for_each.effects.len() != 1 {
        return false;
    }
    let effect = unwrap_tag_wrappers(&for_each.effects[0]);
    matches!(
        effect.downcast_ref::<crate::effects::RevealTaggedEffect>(),
        Some(reveal) if reveal.tag.as_str() == "__it__" || reveal.tag == *tag
    )
}

fn for_each_moves_partition_remainder_to_zone(
    effect: &Effect,
    iterated_tag: &crate::TagKey,
    kept_tag: &crate::TagKey,
    zone: Zone,
) -> bool {
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    if for_each.tag != *iterated_tag || for_each.effects.len() != 1 {
        return false;
    }
    let Some(conditional) = structural_unwrap_render_wrappers(&for_each.effects[0])
        .downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return false;
    };
    let Condition::TaggedObjectMatches(iterated, filter) = &conditional.condition else {
        return false;
    };
    if iterated.as_str() != "__it__"
        || filter != &ObjectFilter::tagged(kept_tag.clone())
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !conditional.if_true.is_empty()
        || conditional.if_false.len() != 1
    {
        return false;
    }
    matches!(
        structural_unwrap_render_wrappers(&conditional.if_false[0])
            .downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == zone
                && !move_to_zone.to_top
                && !move_to_zone.enters_tapped
                && matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    )
}

fn choose_is_exact_tagged_library_partition(
    choose: &crate::effects::ChooseObjectsEffect,
    tag: &crate::TagKey,
) -> bool {
    if choose_primary_zone(choose) != Some(Zone::Library) || !choose.additional_zones.is_empty() {
        return false;
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter == ObjectFilter::tagged(tag.clone())
}

pub(crate) fn for_each_moves_any_remainder_to_zone(
    effect: &Effect,
    tag: &crate::TagKey,
    zone: Zone,
) -> bool {
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    if for_each.tag != *tag || for_each.effects.len() != 1 {
        return false;
    }
    let Some(conditional) = for_each.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return false;
    };
    if !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
        return false;
    }
    matches!(
        unwrap_tag_wrappers(&conditional.if_false[0]).downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == zone
                && matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    )
}

pub(crate) fn put_battlefield_uses_tag(effect: &Effect, tag: &crate::TagKey) -> bool {
    let effect = unwrap_tag_wrappers(effect);
    if let Some(put) = effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>() {
        return !put.tapped
            && matches!(put.target.base(), ChooseSpec::Tagged(found) if found == tag);
    }
    matches!(
        effect.downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == Zone::Battlefield
                && !move_to_zone.enters_tapped
                && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found == tag)
    )
}

pub(crate) fn describe_look_choose_reveal_to_hand_rest_bottom(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let look = effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = effects
        .get(1)?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if look.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose_references_revealed_tag(choose, &look.tag)
    {
        return None;
    }
    let mut idx = 2usize;
    let mut reveals_selection = false;
    if effects
        .get(idx)
        .is_some_and(|effect| for_each_reveals_iterated_tag(effect, &choose.tag))
    {
        reveals_selection = true;
        idx += 1;
    }
    if !effects
        .get(idx)
        .is_some_and(|effect| for_each_returns_iterated_to_hand(effect, &choose.tag))
    {
        return None;
    }
    idx += 1;
    let selected_condition = effects.get(idx).and_then(|effect| {
        let conditional = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ConditionalEffect>()?;
        let crate::ConditionExpr::TaggedObjectMatches(tag, filter) = &conditional.condition else {
            return None;
        };
        (tag == &choose.tag
            && filter.zone.is_none()
            && filter.tagged_constraints.is_empty()
            && !conditional.if_true.is_empty()
            && conditional.if_false.is_empty()
            && conditional.surface == ironsmith_core::ConditionalSurface::LeadingIf)
            .then_some(*effect)
    });
    if selected_condition.is_some() {
        idx += 1;
    }
    let remainder = effects
        .get(idx)?
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != look.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != PlayerFilter::You
    {
        return None;
    }

    let order_text = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == look.tag)
    });
    let mut filter_text = filter.description();
    let unfiltered_looked_cards = filter == ObjectFilter::default();
    if unfiltered_looked_cards {
        filter_text = if choose.count.max == Some(1) {
            "card".to_string()
        } else {
            "cards".to_string()
        };
    }
    filter_text = normalize_looked_card_filter_description(&filter, &filter_text);
    if !filter_text.contains("card") {
        if let Some((head, tail)) = filter_text.split_once(" with ") {
            let noun = if choose.count.max == Some(1) {
                "card"
            } else {
                "cards"
            };
            filter_text = format!("{head} {noun} with {tail}");
        } else {
            filter_text.push_str(if choose.count.max == Some(1) {
                " card"
            } else {
                " cards"
            });
        }
    }
    let selection = if unfiltered_looked_cards {
        match (choose.count.min, choose.count.max) {
            (0, Some(1)) => "up to one of them".to_string(),
            (1, Some(1)) => "one of them".to_string(),
            (0, Some(max)) => {
                let max_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
                format!("up to {max_text} of them")
            }
            _ => format!("{} of them", describe_choice_count(&choose.count)),
        }
    } else if choose.count.max == Some(1) {
        with_indefinite_article(&filter_text)
    } else {
        let count_text = match (choose.count.min, choose.count.max) {
            (0, Some(max)) => {
                let max_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
                format!("up to {max_text}")
            }
            _ => describe_choice_count(&choose.count),
        };
        format!("{count_text} {filter_text}")
    };
    let hand_pronoun = if choose.count.max == Some(1) {
        "it"
    } else {
        "them"
    };
    let remainder_prefix = match remainder.surface {
        ironsmith_core::LibraryRemainderSurface::SentenceLeadingThenRest => "Then put",
        ironsmith_core::LibraryRemainderSurface::Rest
        | ironsmith_core::LibraryRemainderSurface::RestBare => "Put",
        _ => return None,
    };
    Some((
        if reveals_selection {
            let mut sentences = vec![
                describe_effect(effects[0]),
                format!(
                    "You may reveal {selection} from among them and put {hand_pronoun} into your hand"
                ),
            ];
            if let Some(condition) = selected_condition {
                sentences.push(describe_effect(condition));
            }
            sentences.push(format!(
                "{remainder_prefix} the rest on the bottom of your library{order_text}"
            ));
            sentences.join(". ")
        } else {
            if selected_condition.is_some() {
                return None;
            }
            if unfiltered_looked_cards {
                format!(
                    "{}. Put {selection} into your hand and the rest on the bottom of your library{order_text}",
                    describe_effect(effects[0])
                )
            } else {
                format!(
                    "{}. Put {selection} from among them into your hand. Put the rest on the bottom of your library{order_text}",
                    describe_effect(effects[0])
                )
            }
        },
        idx + 1,
    ))
}

pub(crate) struct TaggedLookMove<'a> {
    tag: &'a crate::TagKey,
    move_to_zone: &'a crate::effects::MoveToZoneEffect,
    count: &'a ChoiceCount,
    explicit_may: bool,
}

pub(crate) fn tagged_move_from_looked_view(effect: &Effect) -> Option<TaggedLookMove<'_>> {
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        if may.decider.as_ref() != Some(&PlayerFilter::You) || may.effects.len() != 1 {
            return None;
        }
        let mut move_view = tagged_move_from_looked_view(&may.effects[0])?;
        move_view.explicit_may = true;
        return Some(move_view);
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let move_to_zone = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::WithCount(inner, count) = &move_to_zone.target else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    if count.max.is_some_and(|max| count.min > max)
        || !matches!(move_to_zone.zone, Zone::Battlefield | Zone::Hand)
        || filter.zone != Some(move_to_zone.zone)
    {
        return None;
    }
    Some(TaggedLookMove {
        tag: &tagged.tag,
        move_to_zone,
        count,
        explicit_may: false,
    })
}

pub(crate) fn tagged_may_move_one_from_looked_view(
    effect: &Effect,
) -> Option<(
    &crate::TagKey,
    &crate::effects::MoveToZoneEffect,
    &ChoiceCount,
)> {
    let move_view = tagged_move_from_looked_view(effect)?;
    if !move_view.explicit_may || move_view.count.max != Some(1) || move_view.count.min > 1 {
        return None;
    }
    Some((move_view.tag, move_view.move_to_zone, move_view.count))
}

pub(crate) fn for_each_tagged_moves_nonmatching_to_library_bottom(
    effect: &Effect,
    tag: &crate::TagKey,
) -> bool {
    let Some((_, for_each)) = for_each_tagged_for_compaction(effect) else {
        return false;
    };
    if for_each.tag != *tag || for_each.effects.len() != 1 {
        return false;
    }
    let Some(conditional) = for_each.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return false;
    };
    if !conditional.if_true.is_empty() || conditional.if_false.len() != 1 {
        return false;
    }
    matches!(
        unwrap_tag_wrappers(&conditional.if_false[0]).downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == Zone::Library
                && !move_to_zone.to_top
                && matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    )
}

pub(crate) fn describe_look_may_exile_from_among_rest_bottom_cast(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let look = effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = effects
        .get(1)?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile =
        unwrap_tag_wrappers(effects.get(2)?).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let rest = effects
        .get(3)?
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let permission = *effects.get(4)?;
    if look.player != PlayerFilter::You
        || look.reveal
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose.count.max != Some(1)
        || choose.count.min > 1
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == look.tag
        })
        || exile.zone != Zone::Exile
        || !matches!(exile.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || exile.enters_face_down
        || rest.tag != look.tag
        || rest.keep_tagged.as_ref() != Some(&choose.tag)
        || rest.player != PlayerFilter::You
    {
        return None;
    }
    let permission_text =
        if let Some(grant) = permission.downcast_ref::<crate::effects::GrantPlayTaggedEffect>() {
            if grant.tag != choose.tag
                || grant.player != PlayerFilter::You
                || grant.allow_any_color_for_cast
                || grant.while_on_top_of_library
                || grant.filter.is_some()
                || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
            {
                return None;
            }
            if grant.allow_land {
                "You may play the exiled card this turn"
            } else {
                "You may cast the exiled card this turn"
            }
        } else if let Some(cast) = permission.downcast_ref::<crate::effects::CastTaggedEffect>() {
            if cast.tag != choose.tag
                || cast.player != PlayerFilter::You
                || cast.allow_land
                || cast.as_copy
                || cast.cost_reduction.is_some()
            {
                return None;
            }
            if cast.without_paying_mana_cost {
                "You may cast the exiled card without paying its mana cost"
            } else {
                "You may cast the exiled card"
            }
        } else {
            return None;
        };
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == look.tag)
    });
    let permission_text = if filter == ObjectFilter::default() {
        permission_text.replace("the exiled card", "that card")
    } else {
        permission_text.to_string()
    };
    let explicit_up_to_surface = choose
        .tag
        .as_str()
        .starts_with("__sentence_helper_exiled_up_to_");
    let exile_action = if explicit_up_to_surface {
        "Exile up to one"
    } else if choose.count.min == 0 {
        "You may exile"
    } else {
        "Exile"
    };
    let exile_clause = if filter == ObjectFilter::default() {
        if explicit_up_to_surface {
            format!("{exile_action} of those cards")
        } else {
            format!("{exile_action} one of those cards")
        }
    } else {
        let mut selection =
            normalize_looked_card_filter_description(&filter, &filter.description());
        if !selection.contains("card") {
            selection.push_str(" card");
        }
        let selection = if explicit_up_to_surface {
            strip_leading_article(&selection).to_string()
        } else {
            with_indefinite_article(&selection)
        };
        format!("{exile_action} {selection} from among them")
    };
    let order_text = match rest.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };
    let rendered = if explicit_up_to_surface {
        format!(
            "{}. {exile_clause} and put the rest on the bottom of your library{order_text}. {permission_text}",
            describe_effect(effects[0])
        )
    } else {
        format!(
            "{}. {exile_clause}. Put the rest on the bottom of your library{order_text}. {permission_text}",
            describe_effect(effects[0])
        )
    };
    Some((rendered, 5))
}

pub(crate) fn describe_look_may_move_one_rest_bottom(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let look = effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if look.player != PlayerFilter::You {
        return None;
    }
    let (moved_tag, move_to_zone, count) = tagged_may_move_one_from_looked_view(effects.get(1)?)?;
    if !for_each_tagged_moves_nonmatching_to_library_bottom(effects.get(2)?, moved_tag) {
        return None;
    }
    let ChooseSpec::WithCount(inner, _) = &move_to_zone.target else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    let mut filter = filter.clone();
    filter.zone = None;
    let mut selection = filter.description();
    if !selection.contains("card") {
        selection.push_str(" card");
    }
    let selection = if count.min == 0 {
        format!("up to one {selection}")
    } else {
        with_indefinite_article(&selection)
    };
    let destination = match move_to_zone.zone {
        Zone::Battlefield => "onto the battlefield",
        Zone::Hand => "into your hand",
        _ => return None,
    };
    Some((
        format!(
            "{}. You may put {selection} from among them {destination}. Put the rest on the bottom of your library",
            describe_effect(effects[0])
        ),
        3,
    ))
}

pub(crate) fn describe_look_move_counted_rest_bottom(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let look = effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if look.player != PlayerFilter::You {
        return None;
    }
    let move_view = tagged_move_from_looked_view(effects.get(1)?)?;
    if !for_each_tagged_moves_nonmatching_to_library_bottom(effects.get(2)?, move_view.tag) {
        return None;
    }
    let ChooseSpec::WithCount(inner, _) = &move_view.move_to_zone.target else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    let selection = describe_moved_card_selection_from_looked(filter, move_view.count);
    let selection = match (move_view.count.min, move_view.count.max) {
        (0, Some(1)) => format!("up to one {selection}"),
        (1, Some(1)) => with_indefinite_article(&selection),
        (0, Some(max)) => {
            let max_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
            format!("up to {max_text} {selection}")
        }
        _ => format!("{} {selection}", describe_choice_count(move_view.count)),
    };
    let destination = match move_view.move_to_zone.zone {
        Zone::Battlefield => {
            let tapped = if move_view.move_to_zone.enters_tapped {
                " tapped"
            } else {
                ""
            };
            format!("onto the battlefield{tapped}")
        }
        Zone::Hand => "into your hand".to_string(),
        _ => return None,
    };
    let action = if move_view.explicit_may {
        "You may put"
    } else {
        "Put"
    };
    Some((
        format!(
            "{}. {action} {selection} from among them {destination}. Put the rest on the bottom of your library",
            describe_effect(effects[0])
        ),
        3,
    ))
}

pub(crate) fn describe_moved_card_selection_from_looked(
    filter: &ObjectFilter,
    count: &ChoiceCount,
) -> String {
    let mut base_filter = filter.clone();
    base_filter.zone = None;
    let filter_text = base_filter.description();
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    card_desc = normalize_looked_card_filter_description(&base_filter, &card_desc);
    if !card_desc.contains(" card") {
        let plural = count.max != Some(1);
        let (head, tail) = card_desc
            .split_once(" with ")
            .map(|(head, tail)| (head.to_string(), format!(" with {tail}")))
            .unwrap_or_else(|| (card_desc.clone(), String::new()));
        let mut head = head;
        if plural {
            head = singularize_simple_noun_phrase(&head);
        }
        card_desc = format!("{head} card{tail}");
    }
    if count.max == Some(1) {
        card_desc
    } else {
        pluralize_noun_phrase(&card_desc)
    }
}

pub(crate) fn singularize_simple_noun_phrase(phrase: &str) -> String {
    let mut words = phrase.split_whitespace().collect::<Vec<_>>();
    let Some(last) = words.pop() else {
        return phrase.to_string();
    };
    let singular = if let Some(stem) = last.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = last.strip_suffix('s') {
        if last.ends_with("ss") {
            last.to_string()
        } else {
            stem.to_string()
        }
    } else {
        last.to_string()
    };
    if words.is_empty() {
        singular
    } else {
        format!("{} {singular}", words.join(" "))
    }
}

pub(crate) fn join_with_and_or_articles(labels: &[String]) -> String {
    let labels = labels
        .iter()
        .map(|label| with_indefinite_article(label))
        .collect::<Vec<_>>();
    match labels.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [one, two] => format!("{one} and/or {two}"),
        many => {
            let (last, head) = many.split_last().expect("non-empty labels");
            format!("{}, and/or {last}", head.join(", "))
        }
    }
}

pub(crate) fn is_card_type_choice_label(label: &str) -> bool {
    matches!(
        label,
        "artifact card"
            | "battle card"
            | "creature card"
            | "enchantment card"
            | "instant card"
            | "kindred card"
            | "land card"
            | "planeswalker card"
            | "sorcery card"
    )
}

pub(crate) fn render_look_reveal_repeated_choices(effects: &[&Effect]) -> Option<(String, usize)> {
    // Effect ids and result tags carry execution identity, but do not change
    // the authored look/reveal/choice partition. Normalize those transparent
    // wrappers before matching the complete typed sequence so parser-assigned
    // ids cannot force the renderer back to one sentence per internal effect.
    let effects = effects
        .iter()
        .map(|effect| structural_unwrap_render_wrappers(effect))
        .collect::<Vec<_>>();
    let look = effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if look.player != PlayerFilter::You {
        return None;
    }

    let explicitly_reveals_looked = effects
        .get(1)
        .and_then(|effect| effect.downcast_ref::<crate::effects::RevealTaggedEffect>())
        .is_some_and(|reveal| reveal.tag == look.tag);
    let looked_pool_is_public = look.reveal || explicitly_reveals_looked;

    let mut idx = if explicitly_reveals_looked {
        2usize
    } else {
        1usize
    };
    let mut labels = Vec::new();
    let mut chosen_tag: Option<crate::TagKey> = None;
    let mut conditional_choices = false;
    while idx < effects.len() {
        if let Some(choose) = effects[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            if let Some((label, tag)) =
                valid_revealed_choice(choose, &look.tag, chosen_tag.as_ref())
            {
                chosen_tag.get_or_insert_with(|| tag.clone());
                labels.push(label);
                idx += 1;
                continue;
            }
            if labels.is_empty() {
                return None;
            }
            break;
        }
        if let Some(conditional) = effects[idx].downcast_ref::<crate::effects::ConditionalEffect>()
        {
            if let Some((label, tag)) =
                conditional_revealed_choice(conditional, &look.tag, chosen_tag.as_ref())
            {
                chosen_tag.get_or_insert_with(|| tag.clone());
                labels.push(label);
                conditional_choices = true;
                idx += 1;
                continue;
            }
            if labels.is_empty() {
                return None;
            }
            break;
        }
        break;
    }

    let chosen_tag = chosen_tag?;
    if labels.is_empty() {
        return None;
    }

    let mut reveals_selection = false;
    if effects
        .get(idx)
        .and_then(|effect| effect.downcast_ref::<crate::effects::RevealTaggedEffect>())
        .is_some_and(|reveal| reveal.tag == chosen_tag)
    {
        reveals_selection = true;
        idx += 1;
    }

    let look_text = describe_effect(effects[0]);
    let labels_are_card_types = labels
        .iter()
        .all(|label| is_card_type_choice_label(label.as_str()));
    let labels_are_keyword_cards = labels.iter().all(|label| label.starts_with("card with "));

    if labels.len() == 1
        && idx + 1 < effects.len()
        && for_each_returns_iterated_to_hand(effects[idx], &chosen_tag)
        && for_each_moves_any_remainder_to_zone(effects[idx + 1], &look.tag, Zone::Graveyard)
    {
        let reveal_text = if look.reveal { "" } else { ". Reveal them" };
        return Some((
            format!(
                "{look_text}{reveal_text}. You may put {} from among them into your hand. Put the rest into your graveyard",
                with_indefinite_article(&labels[0])
            ),
            idx + 2,
        ));
    }

    if idx + 1 < effects.len()
        && for_each_returns_iterated_to_hand(effects[idx], &chosen_tag)
        && let Some(remainder) = effects[idx + 1]
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
        )
        && remainder.tag == look.tag
        && remainder.keep_tagged.as_ref() == Some(&chosen_tag)
        && remainder.player == PlayerFilter::You
    {
        let order_text = match remainder.order {
            crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
            crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses
                if !looked_pool_is_public && reveals_selection =>
            {
                " in any order"
            }
            crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => "",
        };
        let choice_text = if labels_are_card_types && (conditional_choices || labels.len() > 2) {
            if conditional_choices {
                "For each card type among noncreature spells you cast this turn, you may put a card of that type from among the revealed cards into your hand"
                    .to_string()
            } else {
                "For each card type, you may put a card of that type from among the revealed cards into your hand"
                    .to_string()
            }
        } else if !looked_pool_is_public && reveals_selection {
            let hand_reference = if labels.len() == 1 {
                // The revealed object and the immediately moved object share
                // the same typed result tag.  For a singular selection, keep
                // that identity relationship concise instead of redeclaring
                // the noun after "reveal a ... card".
                "it"
            } else {
                "those cards"
            };
            format!(
                "You may reveal {} from among them and put {hand_reference} into your hand",
                join_with_and_or_articles(&labels)
            )
        } else {
            format!(
                "Choose from among them {}. Put those cards into your hand",
                join_with_and(&labels)
            )
        };
        let reveal_text = if explicitly_reveals_looked {
            ". Reveal them"
        } else {
            ""
        };
        let remainder_prefix = match remainder.surface {
            ironsmith_core::LibraryRemainderSurface::SentenceLeadingThenRest => "Then put",
            ironsmith_core::LibraryRemainderSurface::Rest
            | ironsmith_core::LibraryRemainderSurface::RestBare => "Put",
            _ => return None,
        };
        return Some((
            format!(
                "{look_text}{reveal_text}. {choice_text}. {remainder_prefix} the rest on the bottom of your library{order_text}"
            ),
            idx + 2,
        ));
    }

    if labels_are_keyword_cards
        && idx + 3 < effects.len()
        && let Some(battlefield_choose) =
            effects[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && battlefield_choose.chooser == PlayerFilter::You
        && battlefield_choose.count.min <= 1
        && battlefield_choose.count.max == Some(1)
        && choose_is_exact_tagged_library_partition(battlefield_choose, &chosen_tag)
        && put_battlefield_uses_tag(effects[idx + 1], &battlefield_choose.tag)
        && for_each_moves_partition_remainder_to_zone(
            effects[idx + 2],
            &chosen_tag,
            &battlefield_choose.tag,
            Zone::Hand,
        )
        && for_each_moves_partition_remainder_to_zone(
            effects[idx + 3],
            &look.tag,
            &chosen_tag,
            Zone::Graveyard,
        )
    {
        let labels = labels
            .iter()
            .map(|label| with_indefinite_article(label))
            .collect::<Vec<_>>();
        let choice_text =
            if labels.len() > 2 && labels.iter().all(|label| label.starts_with("a card with ")) {
                let remaining = labels[2..]
                    .iter()
                    .map(|label| label.trim_start_matches("a card with ").to_string())
                    .collect::<Vec<_>>();
                format!(
                    "{}, {}, and so on for {}",
                    labels[0],
                    labels[1],
                    join_with_and(&remaining)
                )
            } else {
                join_with_and(&labels)
            };
        let reveal_text = look_text
            .strip_prefix("Look at ")
            .map(|tail| format!("Reveal {tail}"))
            .unwrap_or_else(|| format!("{look_text}. Reveal them"));
        return Some((
            format!(
                "{reveal_text}. Choose from among them {choice_text}. Put one of the chosen cards onto the battlefield, the other chosen cards into your hand, and the rest into your graveyard"
            ),
            idx + 4,
        ));
    }

    if labels_are_keyword_cards {
        let labels = labels
            .iter()
            .map(|label| with_indefinite_article(label))
            .collect::<Vec<_>>();
        return Some((
            format!(
                "{look_text}. Reveal them. Choose from among them {}",
                join_with_and(&labels)
            ),
            idx,
        ));
    }

    None
}

pub(crate) fn count_override_look_choice(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    effects: &[Effect],
) -> Option<(u32, Option<LibraryBottomOrder>)> {
    let [choose_effect, move_effect, rest_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let move_to_hand = move_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let legacy_rest = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>();
    let typed_rest =
        rest_effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>();
    let rest_matches = legacy_rest.is_some_and(|rest| {
        for_each_moves_unselected_to_zone(
            rest,
            look_at_top.tag.as_str(),
            choose.tag.as_str(),
            Zone::Library,
        )
    }) || typed_rest.is_some_and(|rest| {
        rest.tag == look_at_top.tag
            && rest.keep_tagged.as_ref() == Some(&choose.tag)
            && rest.player == look_at_top.player
    });
    if choose.chooser != look_at_top.player
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.is_search
        || !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
        || !rest_matches
    {
        return None;
    }
    let exact_count = match (choose.count.min, choose.count.max) {
        (n, Some(max)) if n == max && n > 0 => n,
        _ => return None,
    };

    let mut base_filter = choose.filter.clone();
    base_filter.zone = None;
    base_filter.tagged_constraints.retain(|constraint| {
        !(matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str())
    });
    if base_filter != ObjectFilter::default() {
        return None;
    }

    Some((exact_count as u32, typed_rest.map(|rest| rest.order)))
}

pub(crate) fn render_look_at_top_count_override_with_conditional(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let look_at_top = effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let conditional = effects
        .get(1)?
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.condition != Condition::ThisSpellWasKicked {
        return None;
    }

    let (kicked_count, kicked_order) =
        count_override_look_choice(look_at_top, &conditional.if_true)?;
    let (normal_count, normal_order) =
        count_override_look_choice(look_at_top, &conditional.if_false)?;
    if normal_count != 1 || kicked_count <= normal_count {
        return None;
    }
    let order = match (normal_order, kicked_order) {
        (Some(normal), Some(kicked)) if normal == kicked => Some(normal),
        (None, None) => None,
        _ => return None,
    };

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let kicked_count_text =
        small_number_word(kicked_count).unwrap_or_else(|| kicked_count.to_string());
    let order_text = match order {
        Some(LibraryBottomOrder::Random) => " in a random order",
        Some(LibraryBottomOrder::ChooserChooses) => " in any order",
        None => "",
    };

    Some((
        format!(
            "Look at the top {count_text} {noun} of {owner} library{count_where_clause}. Put one of those cards into {owner} hand. If this spell was kicked, put {kicked_count_text} of those cards into {owner} hand instead. Put the rest on the bottom of {owner} library{order_text}"
        ),
        2,
    ))
}

pub(crate) fn render_reveal_top_choose_to_hand(effects: &[&Effect]) -> Option<(String, usize)> {
    let reveal = effects
        .first()?
        .downcast_ref::<crate::effects::RevealTopEffect>()?;
    let revealed_tag = reveal.tag.as_ref()?;
    let choose = effects
        .get(1)?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if reveal.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose_references_revealed_tag(choose, revealed_tag)
        || !for_each_returns_iterated_to_hand(effects.get(2)?, &choose.tag)
    {
        return None;
    }
    let consumed = if effects.get(3).is_some_and(|effect| {
        for_each_moves_any_remainder_to_zone(effect, revealed_tag, Zone::Graveyard)
    }) {
        4
    } else {
        3
    };
    Some((
        "Reveal the top card of your library. Put it into your hand".to_string(),
        consumed,
    ))
}

pub(crate) fn render_look_reveal_choice_to_hand_rest_graveyard(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let look = effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reveal = effects
        .get(1)?
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let choose = effects
        .get(2)?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if look.player != PlayerFilter::You
        || reveal.tag != look.tag
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose.count.min != 0
        || choose.count.max != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose_references_revealed_tag(choose, &look.tag)
        || !for_each_returns_iterated_to_hand(effects.get(3)?, &choose.tag)
        || !for_each_moves_any_remainder_to_zone(effects.get(4)?, &look.tag, Zone::Graveyard)
    {
        return None;
    }
    Some((
        format!(
            "{}. Reveal them. You may put {} from among them into your hand. Put the rest into your graveyard",
            describe_effect(effects[0]),
            with_indefinite_article(&revealed_choice_label(choose)?)
        ),
        5,
    ))
}
