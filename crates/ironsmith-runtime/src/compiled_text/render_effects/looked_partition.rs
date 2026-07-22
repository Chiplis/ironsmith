use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LookedPartitionDestination {
    Hand,
    Graveyard,
    LibraryTop(&'static str),
    LibraryBottom(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreeWayDestination {
    Hand,
    Graveyard,
    LibraryTop,
    LibraryBottom,
}

fn exact_tagged_library_filter(
    filter: &ObjectFilter,
    expected: &[(&crate::TagKey, crate::filter::TaggedOpbjectRelation)],
) -> bool {
    if filter.zone != Some(Zone::Library) || filter.tagged_constraints.len() != expected.len() {
        return false;
    }
    if expected.iter().any(|(tag, relation)| {
        filter
            .tagged_constraints
            .iter()
            .filter(|constraint| &constraint.tag == *tag && constraint.relation == *relation)
            .count()
            != 1
    }) {
        return false;
    }

    let mut plain = filter.clone();
    plain.zone = None;
    plain.tagged_constraints.clear();
    plain == ObjectFilter::default()
}

fn move_targets_exact_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &crate::TagKey,
) -> bool {
    matches!(&move_to_zone.target, ChooseSpec::Tagged(candidate) if candidate == tag)
}

fn placement_order_suffix(order: &crate::effects::LibraryPlacementOrder) -> Option<&'static str> {
    match order {
        crate::effects::LibraryPlacementOrder::Random => Some(" in a random order"),
        crate::effects::LibraryPlacementOrder::ChosenBy(PlayerFilter::You) => Some(" in any order"),
        crate::effects::LibraryPlacementOrder::ChosenBy(_) => None,
    }
}

fn looked_partition_destination(
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<LookedPartitionDestination> {
    match (move_to_zone.zone, move_to_zone.to_top) {
        (Zone::Hand, false) if move_to_zone.library_order.is_none() => {
            Some(LookedPartitionDestination::Hand)
        }
        (Zone::Graveyard, false) if move_to_zone.library_order.is_none() => {
            Some(LookedPartitionDestination::Graveyard)
        }
        (Zone::Library, true) => Some(LookedPartitionDestination::LibraryTop(
            placement_order_suffix(move_to_zone.library_order.as_ref()?)?,
        )),
        (Zone::Library, false) => Some(LookedPartitionDestination::LibraryBottom(
            placement_order_suffix(move_to_zone.library_order.as_ref()?)?,
        )),
        _ => None,
    }
}

fn exact_nonrandom_choice_count(count: &ChoiceCount) -> Option<usize> {
    (!count.dynamic_x && !count.random && count.min > 0 && count.max == Some(count.min))
        .then_some(count.min)
}

fn bounded_nonrandom_up_to_count(count: &ChoiceCount) -> Option<usize> {
    let max = count.max?;
    (!count.dynamic_x && !count.random && count.min == 0)
        .then_some(max)
        .filter(|max| *max > 0)
}

fn describe_selected_partition_reference(
    count: &ChoiceCount,
    destination: LookedPartitionDestination,
) -> Option<String> {
    if count.is_any_number() && !count.random {
        return Some("any number of them".to_string());
    }
    if let Some(max) = bounded_nonrandom_up_to_count(count) {
        let max = small_number_word(max as u32).unwrap_or_else(|| max.to_string());
        return Some(format!("up to {max} of them"));
    }
    let exact = exact_nonrandom_choice_count(count)?;
    if exact == 1 {
        return Some(
            if destination == LookedPartitionDestination::Graveyard {
                "one of those cards"
            } else {
                "one of them"
            }
            .to_string(),
        );
    }
    let count = small_number_word(exact as u32).unwrap_or_else(|| exact.to_string());
    Some(format!("{count} of them"))
}

fn simple_three_way_destination(
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<ThreeWayDestination> {
    if move_to_zone.library_order.is_some() {
        return None;
    }
    match (move_to_zone.zone, move_to_zone.to_top) {
        (Zone::Hand, false) => Some(ThreeWayDestination::Hand),
        (Zone::Graveyard, false) => Some(ThreeWayDestination::Graveyard),
        (Zone::Library, true) => Some(ThreeWayDestination::LibraryTop),
        (Zone::Library, false) => Some(ThreeWayDestination::LibraryBottom),
        _ => None,
    }
}

fn is_plain_disjoint_looked_choice(
    choose: &crate::effects::ChooseObjectsEffect,
    expected: &[(&crate::TagKey, crate::filter::TaggedOpbjectRelation)],
) -> bool {
    choose.chooser == PlayerFilter::You
        && choose_primary_zone(choose) == Some(Zone::Library)
        && choose.additional_zones.is_empty()
        && choose.count.is_single()
        && !choose.count.random
        && choose.count_value.is_none()
        && choose.aggregate_constraint.is_none()
        && !choose.is_search
        && !choose.reveal
        && !choose.top_only
        && !choose.bottom_only
        && !choose.replace_tagged_objects
        && exact_tagged_library_filter(&choose.filter, expected)
}

/// Compacts three independently chosen cards from one looked-at pool. Each
/// later choice must exclude every earlier choice, which is the structural
/// proof that the hand/top/bottom or hand/graveyard/bottom destinations are
/// three distinct cards rather than three references to the same implicit
/// object.
pub(super) fn describe_three_way_looked_card_partition(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [
        look_effect,
        first_choice_effect,
        second_choice_effect,
        third_choice_effect,
        first_move_effect,
        second_move_effect,
        third_move_effect,
    ] = effects.get(..7)?
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let first = first_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let second = second_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let third = third_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if look.reveal
        || look.player != PlayerFilter::You
        || look.count != Value::Fixed(3)
        || first.tag == second.tag
        || first.tag == third.tag
        || second.tag == third.tag
        || !is_plain_disjoint_looked_choice(
            first,
            &[(
                &look.tag,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            )],
        )
        || !is_plain_disjoint_looked_choice(
            second,
            &[
                (
                    &look.tag,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                ),
                (
                    &first.tag,
                    crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                ),
            ],
        )
        || !is_plain_disjoint_looked_choice(
            third,
            &[
                (
                    &look.tag,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                ),
                (
                    &first.tag,
                    crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                ),
                (
                    &second.tag,
                    crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                ),
            ],
        )
    {
        return None;
    }

    let moves = [first_move_effect, second_move_effect, third_move_effect];
    let tags = [&first.tag, &second.tag, &third.tag];
    let mut destinations = Vec::with_capacity(3);
    for (effect, tag) in moves.into_iter().zip(tags) {
        let move_to_zone =
            unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        if !move_targets_exact_tag(move_to_zone, tag) {
            return None;
        }
        destinations.push(simple_three_way_destination(move_to_zone)?);
    }

    let disposition = match destinations.as_slice() {
        [
            ThreeWayDestination::Hand,
            ThreeWayDestination::LibraryTop,
            ThreeWayDestination::LibraryBottom,
        ] => "one on top of your library",
        [
            ThreeWayDestination::Hand,
            ThreeWayDestination::Graveyard,
            ThreeWayDestination::LibraryBottom,
        ] => "one into your graveyard",
        _ => return None,
    };

    Some((
        format!(
            "Look at the top three cards of your library. Put one of those cards into your hand, {disposition}, and one on the bottom of your library"
        ),
        7,
    ))
}

/// Compacts the self-library form used by effects such as "look, reorder,
/// then you may shuffle". This consumes only the three-card procedure so a
/// following draw remains its own sentence in the outer effect-list renderer.
pub(super) fn describe_self_look_reorder_then_may_shuffle(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [look_effect, reorder_effect, may_effect] = effects.get(..3)? else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reorder = reorder_effect.downcast_ref::<crate::effects::ReorderLibraryTopEffect>()?;
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [shuffle_effect] = may.effects.as_slice() else {
        return None;
    };
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if look.reveal
        || look.player != PlayerFilter::You
        || reorder.tag != look.tag
        || shuffle.player != PlayerFilter::You
        || !matches!(may.decider, None | Some(PlayerFilter::You))
    {
        return None;
    }
    let (count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    Some((
        format!(
            "Look at the top {count} {noun} of your library{where_clause}, then put them back in any order. You may shuffle"
        ),
        3,
    ))
}

/// Compacts the same self-library procedure when no optional shuffle follows.
/// The shared tag proves that the reordered cards are exactly those just
/// looked at, so the two runtime effects form one authored instruction.
pub(super) fn describe_self_look_reorder(effects: &[&Effect]) -> Option<(String, usize)> {
    let [look_effect, reorder_effect] = effects.get(..2)? else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reorder = reorder_effect.downcast_ref::<crate::effects::ReorderLibraryTopEffect>()?;
    if look.reveal || look.player != PlayerFilter::You || reorder.tag != look.tag {
        return None;
    }
    let (count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    Some((
        format!(
            "Look at the top {count} {noun} of your library{where_clause}, then put them back in any order"
        ),
        2,
    ))
}

/// Compacts a structurally complete partition of a looked-card pool:
/// choose a subset, tag its exact complement, then move the two groups to
/// independently ordered destinations. The exact tag relationships are the
/// proof that "the rest" means all and only unselected looked-at cards.
pub(super) fn describe_looked_card_selected_partition(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let (target_only, offset) = if let Some(target) = effects
        .first()
        .and_then(|effect| effect.downcast_ref::<crate::effects::TargetOnlyEffect>())
    {
        (Some(target), 1)
    } else {
        (None, 0)
    };
    let [
        look_effect,
        choose_effect,
        tag_remainder_effect,
        move_selected_effect,
        move_remainder_effect,
    ] = effects.get(offset..offset + 5)?
    else {
        return None;
    };

    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let tag_remainder =
        tag_remainder_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let move_selected = unwrap_basic_tag_wrappers(move_selected_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_remainder = unwrap_basic_tag_wrappers(move_remainder_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    if look.reveal
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || (!choose.count.is_any_number()
            && exact_nonrandom_choice_count(&choose.count).is_none()
            && bounded_nonrandom_up_to_count(&choose.count).is_none())
        || !exact_tagged_library_filter(
            &choose.filter,
            &[(
                &look.tag,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            )],
        )
        || tag_remainder.zone != Some(Zone::Library)
        || !tag_remainder.additional_zones.is_empty()
        || !exact_tagged_library_filter(
            &tag_remainder.filter,
            &[
                (
                    &look.tag,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                ),
                (
                    &choose.tag,
                    crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
                ),
            ],
        )
        || !move_targets_exact_tag(move_selected, &choose.tag)
        || !move_targets_exact_tag(move_remainder, &tag_remainder.tag)
    {
        return None;
    }

    let target_player = if let Some(target) = target_only {
        Some(choose_spec_player_filter(&target.target)?)
    } else {
        None
    };
    if target_player
        .as_ref()
        .is_some_and(|target| !player_filters_refer_to_same_player(target, &look.player))
    {
        return None;
    }

    let selected_destination = looked_partition_destination(move_selected)?;
    let remainder_destination = looked_partition_destination(move_remainder)?;
    let selected_then_library_top = matches!(
        selected_destination,
        LookedPartitionDestination::Hand
            | LookedPartitionDestination::Graveyard
            | LookedPartitionDestination::LibraryBottom(_)
    ) && matches!(
        remainder_destination,
        LookedPartitionDestination::LibraryTop(_)
    );
    let selected_hand_then_graveyard = matches!(
        (selected_destination, remainder_destination),
        (
            LookedPartitionDestination::Hand,
            LookedPartitionDestination::Graveyard
        )
    );
    let selected_library_top_then_bottom = matches!(
        (selected_destination, remainder_destination),
        (
            LookedPartitionDestination::LibraryTop(_),
            LookedPartitionDestination::LibraryBottom(_)
        )
    );
    if !(selected_then_library_top
        || selected_hand_then_graveyard
        || selected_library_top_then_bottom)
    {
        return None;
    }

    let selected_reference =
        describe_selected_partition_reference(&choose.count, selected_destination)?;
    let selected_destination_text = match selected_destination {
        LookedPartitionDestination::Hand => "into your hand".to_string(),
        LookedPartitionDestination::Graveyard => {
            if look.player == PlayerFilter::You {
                "into your graveyard".to_string()
            } else {
                "into that player's graveyard".to_string()
            }
        }
        LookedPartitionDestination::LibraryBottom(order) => {
            let library = if look.player == PlayerFilter::You {
                "your library"
            } else {
                "that library"
            };
            format!("on the bottom of {library}{order}")
        }
        LookedPartitionDestination::LibraryTop(order) => {
            let library = if look.player == PlayerFilter::You {
                "your library"
            } else {
                "that library"
            };
            let order = if choose.count.max == Some(1) {
                ""
            } else {
                order
            };
            format!("on top of {library}{order}")
        }
    };
    let remainder_destination_text = match remainder_destination {
        LookedPartitionDestination::LibraryTop(order) => {
            let library = if look.player == PlayerFilter::You {
                "your library"
            } else if selected_destination == LookedPartitionDestination::Graveyard {
                "their library"
            } else {
                "the library"
            };
            format!("on top of {library}{order}")
        }
        LookedPartitionDestination::Graveyard => {
            if look.player == PlayerFilter::You {
                "into your graveyard".to_string()
            } else {
                "into that player's graveyard".to_string()
            }
        }
        LookedPartitionDestination::LibraryBottom(order) => {
            let library = if look.player == PlayerFilter::You {
                "your library"
            } else {
                "that library"
            };
            format!("on the bottom of {library}{order}")
        }
        LookedPartitionDestination::Hand => return None,
    };
    let remainder_reference = match (
        look.count.clone(),
        exact_nonrandom_choice_count(&choose.count),
    ) {
        (Value::Fixed(looked), Some(selected))
            if looked > 0 && looked as usize == selected.saturating_add(1) =>
        {
            "the other"
        }
        _ => "the rest",
    };
    let owner = describe_possessive_player_filter(&look.player);
    let (count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);

    Some((
        format!(
            "Look at the top {count} {noun} of {owner} library{where_clause}. Put {selected_reference} {selected_destination_text} and {remainder_reference} {remainder_destination_text}"
        ),
        offset + 5,
    ))
}

fn exact_selected_move<'a>(
    effect: &'a Effect,
    selected_tag: &crate::TagKey,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    let (move_to_zone, target_matches) = if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        (
            move_to_zone,
            move_targets_exact_tag(move_to_zone, selected_tag),
        )
    } else {
        let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() else {
            return None;
        };
        let [inner] = for_each.effects.as_slice() else {
            return None;
        };
        let Some(move_to_zone) =
            unwrap_basic_tag_wrappers(inner).downcast_ref::<crate::effects::MoveToZoneEffect>()
        else {
            return None;
        };
        (
            move_to_zone,
            for_each.tag == *selected_tag
                && matches!(move_to_zone.target.base(), ChooseSpec::Iterated),
        )
    };

    target_matches.then_some(move_to_zone)
}

fn exact_selected_move_to_hand(effect: &Effect, selected_tag: &crate::TagKey) -> bool {
    let Some(move_to_zone) = exact_selected_move(effect, selected_tag) else {
        return false;
    };
    move_to_zone.zone == Zone::Hand
        && !move_to_zone.to_top
        && move_to_zone.library_order.is_none()
        && !move_to_zone.enters_tapped
        && !move_to_zone.enters_attacking
        && !move_to_zone.enters_face_down
}

fn exact_plain_selected_group_move_to_zone(
    effect: &Effect,
    selected_tag: &crate::TagKey,
    zone: Zone,
) -> bool {
    let Some(move_to_zone) = exact_selected_move(effect, selected_tag) else {
        return false;
    };
    move_to_zone.zone == zone
        && !move_to_zone.to_top
        && move_to_zone.library_order.is_none()
        && move_to_zone.battlefield_controller == crate::effects::BattlefieldController::Preserve
        && !move_to_zone.controller_surface_explicit
        && move_to_zone.enters_with_counters.is_empty()
        && !move_to_zone.enters_tapped
        && !move_to_zone.enters_attacking
        && move_to_zone.attack_target_mode.is_none()
        && !move_to_zone.enters_face_down
        && !move_to_zone.transfer_exiled_with_source_links
}

fn exact_looked_and_or_type_choice(
    look: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    selected_tag: &crate::TagKey,
) -> Option<CardType> {
    if choose.chooser != look.player
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.count.min != 0
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || choose.count.up_to_x
        || choose.count.random
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.tag != *selected_tag
        || choose.description != "Choose"
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.tagged_constraints.len() != 2
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == look.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *selected_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        })
    {
        return None;
    }

    let mut plain = choose.filter.clone();
    plain.zone = None;
    plain.tagged_constraints.clear();
    let [card_type] = plain.card_types.as_slice() else {
        return None;
    };
    let mut expected = ObjectFilter::default();
    expected.card_types.push(*card_type);
    (plain == expected).then_some(*card_type)
}

fn exact_chosen_group_battlefield_or_hand_choice(
    effect: &Effect,
    selected_tag: &crate::TagKey,
) -> bool {
    let Some(choice) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() else {
        return false;
    };
    let [battlefield_mode, hand_mode] = choice.modes.as_slice() else {
        return false;
    };
    choice.chooser.is_none()
        && choice.min == Value::Fixed(1)
        && choice.max == Value::Fixed(1)
        && choice.choose_count == Value::Fixed(1)
        && choice.min_choose_count == Value::Fixed(1)
        && !choice.allow_repeat
        && !choice.random
        && !choice.allow_repeated_modes
        && !choice.spree
        && !choice.disallow_previously_chosen_modes
        && !choice.disallow_previously_chosen_modes_this_turn
        && !choice.distinct_player_targets_per_mode
        && choice.conditional_mode_range.is_none()
        && matches!(
            battlefield_mode.effects.as_slice(),
            [move_effect]
                if exact_plain_selected_group_move_to_zone(
                    move_effect,
                    selected_tag,
                    Zone::Battlefield,
                )
        )
        && matches!(
            hand_mode.effects.as_slice(),
            [move_effect]
                if exact_plain_selected_group_move_to_zone(
                    move_effect,
                    selected_tag,
                    Zone::Hand,
                )
        )
}

fn describe_revealed_count_with_plus_one(count: &Value) -> String {
    match count.unhinted() {
        Value::Add(left, right)
            if matches!(left.unhinted(), Value::X)
                && matches!(right.unhinted(), Value::Fixed(1)) =>
        {
            "X plus one".to_string()
        }
        Value::Add(left, right)
            if matches!(left.unhinted(), Value::Fixed(1))
                && matches!(right.unhinted(), Value::X) =>
        {
            "X plus one".to_string()
        }
        value => describe_value(value),
    }
}

/// Renders a typed looked-card partition whose X threshold replaces the
/// selected collection's destination with one shared battlefield-or-hand
/// choice. Every reference is proven by tags: both independent card-type
/// choices draw from the revealed pool, both append to the same selected set,
/// and both dispositions retain the exact complement on the library bottom.
pub(in crate::compiled_text) fn describe_looked_and_or_destination_self_replacement(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    let [branch] = segment.self_replacements.as_slice() else {
        return None;
    };
    if branch.condition_after_replacement
        || !matches!(&branch.condition, crate::ConditionExpr::XValueAtLeast(_))
    {
        return None;
    }
    let [
        default_look_effect,
        default_first_choice_effect,
        default_second_choice_effect,
        default_move_effect,
        default_remainder_effect,
    ] = segment.default_effects.as_slice()
    else {
        return None;
    };
    let [
        replacement_look_effect,
        replacement_first_choice_effect,
        replacement_second_choice_effect,
        replacement_destination_effect,
        replacement_remainder_effect,
    ] = branch.replacement_effects.as_slice()
    else {
        return None;
    };

    let look = default_look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let replacement_look =
        replacement_look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let first =
        default_first_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let second =
        default_second_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let replacement_first =
        replacement_first_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let replacement_second =
        replacement_second_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let remainder = default_remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let replacement_remainder = replacement_remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(
    )?;

    if look != replacement_look
        || first != replacement_first
        || second != replacement_second
        || !look.reveal
        || look.player != PlayerFilter::You
        || first.tag != second.tag
        || remainder != replacement_remainder
        || remainder.tag != look.tag
        || remainder.keep_tagged.as_ref() != Some(&first.tag)
        || remainder.player != look.player
        || !exact_plain_selected_group_move_to_zone(default_move_effect, &first.tag, Zone::Hand)
        || !exact_chosen_group_battlefield_or_hand_choice(
            replacement_destination_effect,
            &first.tag,
        )
    {
        return None;
    }

    let first_type = exact_looked_and_or_type_choice(look, first, &first.tag)?;
    let second_type = exact_looked_and_or_type_choice(look, second, &first.tag)?;
    if first_type == second_type {
        return None;
    }
    let first_label = format!("{} card", first_type.name().to_ascii_lowercase());
    let second_label = format!("{} card", second_type.name().to_ascii_lowercase());
    let selection = format!(
        "{} and/or {}",
        with_indefinite_article(&first_label),
        with_indefinite_article(&second_label),
    );
    let count = describe_revealed_count_with_plus_one(&look.count);
    let noun = if matches!(look.count.unhinted(), Value::Fixed(1)) {
        "card"
    } else {
        "cards"
    };
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };
    let condition = describe_condition(&branch.condition);
    Some(format!(
        "Reveal the top {count} {noun} of your library. Choose {selection} from among them. Put those cards into your hand and the rest on the bottom of your library{order}. If {condition}, instead put the chosen cards onto the battlefield or into your hand and the rest on the bottom of your library{order}"
    ))
}

fn tagged_group_move<'a>(
    effect: &'a Effect,
) -> Option<(&'a crate::TagKey, &'a crate::effects::ForEachTaggedEffect)> {
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some((
            &tag_all.tag,
            tag_all
                .effect
                .downcast_ref::<crate::effects::ForEachTaggedEffect>()?,
        ));
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    Some((
        &tagged.tag,
        tagged
            .effect
            .downcast_ref::<crate::effects::ForEachTaggedEffect>()?,
    ))
}

/// Compacts a two-stage partition of one looked-at collection. The first
/// optional singleton goes to hand, any number of the still-unselected cards
/// matching a typed filter move to a public zone, and the exact complement of
/// both moved groups goes to the graveyard. The shared affected-object tag is
/// the structural proof that the final complement excludes both earlier sets.
pub(super) fn describe_two_stage_looked_card_partition(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [
        look_effect,
        hand_choice_effect,
        hand_move_effect,
        matching_choice_effect,
        matching_move_effect,
        remainder_effect,
    ] = effects.get(..6)?
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let hand_choice = hand_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let matching_choice =
        matching_choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (kept_tag, hand_move) = tagged_group_move(hand_move_effect)?;
    let (matching_kept_tag, matching_move) = tagged_group_move(matching_move_effect)?;
    let remainder = remainder_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;

    if matching_kept_tag != kept_tag
        || look.reveal
        || look.player != PlayerFilter::You
        || hand_choice.chooser != PlayerFilter::You
        || hand_choice.count.min != 0
        || hand_choice.count.max != Some(1)
        || hand_choice.count.dynamic_x
        || hand_choice.count.random
        || hand_choice.count_value.is_some()
        || hand_choice.aggregate_constraint.is_some()
        || hand_choice.is_search
        || hand_choice.reveal
        || !exact_tagged_library_filter(
            &hand_choice.filter,
            &[(
                &look.tag,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            )],
        )
        || matching_choice.chooser != hand_choice.chooser
        || !matching_choice.count.is_any_number()
        || matching_choice.count.random
        || matching_choice.count_value.is_some()
        || matching_choice.aggregate_constraint.is_some()
        || matching_choice.is_search
        || matching_choice.reveal
        || matching_choice.top_only
        || matching_choice.bottom_only
        || matching_choice.replace_tagged_objects
        || matching_choice.filter.zone != Some(Zone::Library)
        || matching_choice.filter.tagged_constraints.len() != 2
        || !matching_choice
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == look.tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
        || !matching_choice
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == hand_choice.tag
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            })
        || !for_each_moves_tag_to_hand(hand_move, hand_choice.tag.as_str())
        || !for_each_moves_unselected_to_zone(
            remainder,
            look.tag.as_str(),
            kept_tag.as_str(),
            Zone::Graveyard,
        )
    {
        return None;
    }

    let (matching_zone, matching_tapped) =
        for_each_moves_tag_to_public_zone(matching_move, matching_choice.tag.as_str())?;
    if matching_zone != Zone::Battlefield {
        return None;
    }
    let mut matching_without_exclusion = matching_choice.clone();
    matching_without_exclusion
        .filter
        .tagged_constraints
        .retain(|constraint| {
            !(constraint.tag == hand_choice.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject)
        });
    let matching = describe_any_number_filter_from_looked_cards(look, &matching_without_exclusion)?;
    let opener = looked_set_sentence(look, false);
    Some((
        format!(
            "{opener}. You may put one of them into your hand. Then put any number of {matching} from among them onto the battlefield{} and the rest into your graveyard",
            if matching_tapped { " tapped" } else { "" }
        ),
        6,
    ))
}

/// Compacts the private exact-singleton program used by "look at the top two,
/// put one into a graveyard" effects. The source tag, exact selected tag, and
/// owner filter prove that the unselected card remains in the same library.
pub(super) fn describe_private_look_choose_one_graveyard(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let mut cursor = 0usize;
    let target_only = effects
        .get(cursor)
        .and_then(|effect| effect.downcast_ref::<crate::effects::TargetOnlyEffect>());
    if target_only.is_some() {
        cursor += 1;
    }
    let look = effects
        .get(cursor)?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    cursor += 1;
    let choose = effects
        .get(cursor)?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    cursor += 1;
    let move_to_zone = exact_selected_move(*effects.get(cursor)?, &choose.tag)?;
    cursor += 1;

    if let Some(target) = target_only
        && !choose_spec_player_filter(&target.target)
            .is_some_and(|player| player_filters_refer_to_same_player(&player, &look.player))
    {
        return None;
    }
    let exact_pool_reference = choose.filter.tagged_constraints.len() == 1
        && choose.filter.tagged_constraints[0].tag == look.tag
        && choose.filter.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::IsTaggedObject;
    let owner_matches = choose
        .filter
        .owner
        .as_ref()
        .is_some_and(|owner| player_filters_refer_to_same_player(owner, &look.player));
    let mut plain_filter = choose.filter.clone();
    plain_filter.zone = None;
    plain_filter.owner = None;
    plain_filter.tagged_constraints.clear();
    let destination_matches = if look.player == PlayerFilter::You {
        move_to_zone
            .destination_player_surface
            .as_ref()
            .is_some_and(|player| player_filters_refer_to_same_player(player, &look.player))
            && move_to_zone.destination_player_reference_surface.is_none()
    } else {
        move_to_zone.destination_player_surface.is_none()
            && move_to_zone.destination_player_reference_surface
                == Some(ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer)
    };
    if look.reveal
        || choose.chooser != PlayerFilter::You
        || exact_nonrandom_choice_count(&choose.count) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || !exact_pool_reference
        || !owner_matches
        || plain_filter != ObjectFilter::default()
        || move_to_zone.zone != Zone::Graveyard
        || move_to_zone.to_top
        || move_to_zone.library_order.is_some()
        || move_to_zone.target_plural_surface
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || !destination_matches
    {
        return None;
    }

    let owner = describe_possessive_player_filter(&look.player);
    let graveyard_owner = if look.player == PlayerFilter::You {
        "your"
    } else {
        "their"
    };
    let (count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    Some((
        format!(
            "Look at the top {count} {noun} of {owner} library{where_clause}. Put one of them into {graveyard_owner} graveyard"
        ),
        cursor,
    ))
}

/// Compacts the direct selected-set form used by looked-card effects: choose
/// an exact nonempty subset from the looked pool, move that subset to hand,
/// then put the exact complement on the bottom of the same library. The
/// shared source/selected tags prove both halves of the partition. Both a
/// direct tagged move and a `ForEachTagged` iterated move are accepted because
/// they move exactly the same structurally proven selected set.
pub(super) fn describe_looked_card_selected_hand_remainder_bottom(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [look_effect, choose_effect, move_effect, remainder_effect] = effects.get(..4)? else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let selected_count = exact_nonrandom_choice_count(&choose.count)?;

    let mut plain_filter = choose.filter.clone();
    plain_filter.zone = None;
    plain_filter.tagged_constraints.clear();
    if look.reveal
        || look.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || !exact_looked_library_choice(choose, &look.tag)
        || plain_filter != ObjectFilter::default()
        || !exact_selected_move_to_hand(move_effect, &choose.tag)
        || remainder.tag != look.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != look.player
    {
        return None;
    }

    let selected_phrase = if selected_count == 1 {
        if matches!(look.count.unhinted(), Value::Fixed(_)) {
            "one of them".to_string()
        } else {
            "one of those cards".to_string()
        }
    } else {
        let selected_count =
            small_number_word(selected_count as u32).unwrap_or_else(|| selected_count.to_string());
        format!("{selected_count} of those cards")
    };
    let (look_count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };

    Some((
        format!(
            "Look at the top {look_count} {noun} of your library{where_clause}. Put {selected_phrase} into your hand and the rest on the bottom of your library{order}"
        ),
        4,
    ))
}

/// Compacts a public top-of-library pool followed by an exact one-card choice
/// and a graveyard move. The optional target declaration and trailing draw
/// cover both controller-chosen and opponent-chosen forms without inferring a
/// card from surface text.
pub(super) fn describe_revealed_top_choose_one_graveyard(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let mut cursor = 0usize;
    if effects.first().is_some_and(|effect| {
        effect
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    }) {
        cursor += 1;
    }
    let look_effect = *effects.get(cursor)?;
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    cursor += 1;
    let target_only = effects
        .get(cursor)
        .and_then(|effect| effect.downcast_ref::<crate::effects::TargetOnlyEffect>());
    if target_only.is_some() {
        cursor += 1;
    }
    let choose = effects
        .get(cursor)?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    cursor += 1;
    let move_to_zone = exact_selected_move(*effects.get(cursor)?, &choose.tag)?;
    cursor += 1;
    let trailing_draw = effects
        .get(cursor)
        .and_then(|effect| effect.downcast_ref::<crate::effects::DrawCardsEffect>());
    if trailing_draw.is_some() {
        cursor += 1;
    }

    let mut plain_filter = choose.filter.clone();
    plain_filter.zone = None;
    plain_filter.tagged_constraints.clear();
    if plain_filter.owner.as_ref() == Some(&look.player) {
        plain_filter.owner = None;
    }
    let exact_pool_reference = choose.filter.tagged_constraints.len() == 1
        && choose.filter.tagged_constraints[0].tag == look.tag
        && choose.filter.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::IsTaggedObject;
    let target_matches_chooser = match (target_only, &choose.chooser) {
        (None, PlayerFilter::You) => true,
        (Some(target), chooser) => choose_spec_player_filter(&target.target)
            .is_some_and(|target| player_filters_refer_to_same_player(&target, chooser)),
        _ => false,
    };
    if !look.reveal
        || !target_matches_chooser
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || !choose.count.is_single()
        || choose.count.random
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || !exact_pool_reference
        || plain_filter != ObjectFilter::default()
        || move_to_zone.zone != Zone::Graveyard
        || move_to_zone.to_top
        || move_to_zone.library_order.is_some()
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
    {
        return None;
    }

    let draw_text = if let Some(draw) = trailing_draw {
        let Value::Fixed(revealed_count) = look.count.unhinted() else {
            return None;
        };
        if draw.player != PlayerFilter::You
            || draw.count != Value::Fixed(revealed_count.saturating_sub(1))
        {
            return None;
        }
        Some(format!("draw {}", describe_card_count(&draw.count)))
    } else {
        if choose.chooser != PlayerFilter::You {
            return None;
        }
        None
    };

    let (count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    let reveal = match &look.player {
        PlayerFilter::You => {
            format!("Reveal the top {count} {noun} of your library{where_clause}")
        }
        PlayerFilter::DamagedPlayer => {
            format!("That player reveals the top {count} {noun} of their library{where_clause}")
        }
        _ => {
            let player = describe_player_filter(&look.player);
            let verb = player_verb(&player, "reveal", "reveals");
            let library = describe_possessive_player_filter(&look.player);
            format!(
                "{} {verb} the top {count} {noun} of {library} library{where_clause}",
                capitalize_first(&player)
            )
        }
    };
    let chooser = if choose.chooser == PlayerFilter::You {
        "You choose".to_string()
    } else {
        let player = describe_player_filter(&choose.chooser);
        format!(
            "{} {}",
            capitalize_first(&player),
            player_verb(&player, "choose", "chooses")
        )
    };
    let graveyard = if look.player == PlayerFilter::You {
        "your"
    } else {
        "their"
    };
    let rendered = if let Some(draw) = draw_text {
        format!(
            "{reveal}. {chooser} one of those cards. Put that card into {graveyard} graveyard, then {draw}"
        )
    } else {
        format!("{reveal}. {chooser} one of those cards and put it into {graveyard} graveyard")
    };
    Some((rendered, cursor))
}

fn exact_looked_library_choice(
    choose: &crate::effects::ChooseObjectsEffect,
    looked_tag: &crate::TagKey,
) -> bool {
    choose.chooser == PlayerFilter::You
        && choose_primary_zone(choose) == Some(Zone::Library)
        && choose.additional_zones.is_empty()
        && choose.count_value.is_none()
        && choose.aggregate_constraint.is_none()
        && !choose.is_search
        && !choose.reveal
        && !choose.top_only
        && !choose.bottom_only
        && !choose.replace_tagged_objects
        && choose.filter.zone == Some(Zone::Library)
        && choose.filter.tagged_constraints.len() == 1
        && choose.filter.tagged_constraints[0].tag == *looked_tag
        && choose.filter.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
}

fn exact_tagged_move_to_hand<'a>(
    effect: &'a Effect,
    selected_tag: &crate::TagKey,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    let (move_to_zone, target_matches) = if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        (
            move_to_zone,
            move_targets_exact_tag(move_to_zone, selected_tag),
        )
    } else {
        let for_each = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
        let [inner] = for_each.effects.as_slice() else {
            return None;
        };
        let move_to_zone =
            unwrap_basic_tag_wrappers(inner).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        (
            move_to_zone,
            for_each.tag == *selected_tag
                && matches!(move_to_zone.target.base(), ChooseSpec::Iterated),
        )
    };
    (move_to_zone.zone == Zone::Hand
        && !move_to_zone.to_top
        && move_to_zone.library_order.is_none()
        && target_matches)
        .then_some(move_to_zone)
}

fn conditional_hand_partition_branch<'a>(
    branch: &'a [Effect],
    looked_tag: &crate::TagKey,
) -> Option<(&'a crate::effects::ChooseObjectsEffect, usize)> {
    let [choose_effect, move_effect] = branch else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !exact_looked_library_choice(choose, looked_tag) {
        return None;
    }
    let mut plain_filter = choose.filter.clone();
    plain_filter.zone = None;
    plain_filter.tagged_constraints.clear();
    if plain_filter != ObjectFilter::default() {
        return None;
    }
    let count = exact_nonrandom_choice_count(&choose.count)?;
    exact_tagged_move_to_hand(move_effect, &choose.tag)?;
    Some((choose, count))
}

/// Compacts a conditional cardinality choice whose two branches write the
/// same selected tag, followed by the exact looked-minus-selected remainder.
/// Sharing the tag is essential: it proves that the trailing remainder is
/// valid regardless of which branch ran.
pub(super) fn describe_conditional_looked_hand_partition(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [look_effect, conditional_effect, remainder_effect] = effects.get(..3)? else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let (if_true, true_count) = conditional_hand_partition_branch(&conditional.if_true, &look.tag)?;
    let (if_false, false_count) =
        conditional_hand_partition_branch(&conditional.if_false, &look.tag)?;
    if look.reveal
        || look.player != PlayerFilter::You
        || if_true.tag != if_false.tag
        || remainder.tag != look.tag
        || remainder.keep_tagged.as_ref() != Some(&if_true.tag)
        || remainder.player != look.player
    {
        return None;
    }

    let true_count = small_number_word(true_count as u32).unwrap_or_else(|| true_count.to_string());
    let false_count =
        small_number_word(false_count as u32).unwrap_or_else(|| false_count.to_string());
    let owner = describe_possessive_player_filter(&look.player);
    let (look_count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    let condition = describe_condition(&conditional.condition);
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => "in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => "in any order",
    };

    Some((
        format!(
            "Look at the top {look_count} {noun} of {owner} library{where_clause}. If {condition}, put {true_count} of those cards into your hand. Otherwise, put {false_count} of them into your hand. Then put the rest on the bottom of {owner} library {order}"
        ),
        3,
    ))
}

fn exact_sacrifice_may<'a>(
    effect: &'a Effect,
) -> Option<(
    &'a crate::effects::WithIdEffect,
    &'a crate::effects::ChooseObjectsEffect,
    SacrificeView<'a>,
)> {
    let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [choose_effect, sacrifice_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    (may.decider
        .as_ref()
        .is_none_or(|decider| decider == &PlayerFilter::You)
        && choose.chooser == PlayerFilter::You
        && sacrifice.player == &PlayerFilter::You
        && describe_choose_then_sacrifice(choose, sacrifice).is_some())
    .then_some((with_id, choose, sacrifice))
}

fn tagged_sacrificed_characteristic(value: &Value, sacrificed_tag: &crate::TagKey) -> bool {
    match value.unhinted() {
        Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec) => {
            matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == sacrificed_tag)
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            tagged_sacrificed_characteristic(left, sacrificed_tag)
                || tagged_sacrificed_characteristic(right, sacrificed_tag)
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => tagged_sacrificed_characteristic(inner, sacrificed_tag),
        _ => false,
    }
}

/// Compacts "look, optionally sacrifice, if you do select from the looked
/// cards, put the rest". Every cross-sentence reference is checked by tag and
/// effect id, including the dynamic characteristic of the sacrificed object.
pub(super) fn describe_look_may_sacrifice_select_battlefield_rest_bottom(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [look_effect, sacrifice_effect, if_effect, remainder_effect] = effects.get(..4)? else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let (sacrifice_with_id, sacrifice_choose, sacrifice) = exact_sacrifice_may(sacrifice_effect)?;
    let conditional = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let [choose_effect, move_effect] = conditional.then.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = move_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let crate::filter::Comparison::LessThanOrEqualExpr(maximum) =
        choose.filter.mana_value.as_ref()?
    else {
        return None;
    };
    if look.reveal
        || look.player != PlayerFilter::You
        || conditional.condition != sacrifice_with_id.id
        || conditional.predicate != EffectPredicate::Happened
        || !conditional.else_.is_empty()
        || !exact_looked_library_choice(choose, &look.tag)
        || choose.count.min != 0
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || choose.count.random
        || for_each.tag != choose.tag
        || !matches!(put.target.base(), ChooseSpec::Iterated)
        || put.tapped
        || put.controller != PlayerFilter::You
        || !tagged_sacrificed_characteristic(maximum, &sacrifice_choose.tag)
        || remainder.tag != look.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != look.player
    {
        return None;
    }

    let sacrifice_clause = describe_choose_then_sacrifice(sacrifice_choose, sacrifice)?;
    let sacrifice_clause = sacrifice_clause.strip_prefix("you ")?;
    let mut x_choice = choose.clone();
    x_choice.filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(
        Value::X,
    )));
    let selection = describe_looked_battlefield_selection(&x_choice)?;
    let selection = selection.strip_prefix("up to one ")?;
    let selection = with_indefinite_article(selection);
    let (look_count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    let order = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => "in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => "in any order",
    };

    Some((
        format!(
            "Look at the top {look_count} {noun} of your library{where_clause}. Then you may {sacrifice_clause}. If you do, you may put {selection} from among those cards onto the battlefield, where X is {}. Put the rest on the bottom of your library {order}",
            describe_value(maximum)
        ),
        4,
    ))
}

fn exact_tagged_remainder_to_graveyard(
    effect: &Effect,
    looked_tag: &crate::TagKey,
    selected_tag: &crate::TagKey,
) -> bool {
    let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() else {
        return false;
    };
    let [conditional_effect] = for_each.effects.as_slice() else {
        return false;
    };
    let Some(conditional) = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return false;
    };
    let [move_effect] = conditional.if_false.as_slice() else {
        return false;
    };
    let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, membership) =
        &conditional.condition
    else {
        return false;
    };
    let exact_membership = membership.tagged_constraints.len() == 1
        && membership.tagged_constraints[0].tag.as_str() == "__it__"
        && membership.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::SameStableId
        && {
            let mut plain = membership.clone();
            plain.tagged_constraints.clear();
            plain == ObjectFilter::default()
        };
    for_each.tag == *looked_tag
        && condition_tag == selected_tag
        && exact_membership
        && conditional.if_true.is_empty()
        && move_to_zone.zone == Zone::Graveyard
        && !move_to_zone.to_top
        && move_to_zone.library_order.is_none()
        && matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
}

pub(super) fn describe_look_at_top_choose_battlefield_rest_graveyard(
    effects: &[Effect],
) -> Option<String> {
    let (look_effect, reveal_effect, choose_effect, move_effect, remainder_effect) = match effects {
        [look, choose, move_effect, remainder] => (look, None, choose, move_effect, remainder),
        [look, reveal, choose, move_effect, remainder] => {
            (look, Some(reveal), choose, move_effect, remainder)
        }
        _ => return None,
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if look.player != PlayerFilter::You {
        return None;
    }
    if let Some(reveal_effect) = reveal_effect {
        let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
        if reveal.tag != look.tag {
            return None;
        }
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose.count.is_any_number()
        || !choose_references_tag(choose, &look.tag)
        || !exact_tagged_remainder_to_graveyard(remainder_effect, &look.tag, &choose.tag)
    {
        return None;
    }
    let (_, for_each) = for_each_tagged_for_compaction(move_effect)?;
    let [move_effect] = for_each.effects.as_slice() else {
        return None;
    };
    if for_each.tag != choose.tag {
        return None;
    }
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }

    let selection = describe_looked_battlefield_selection(choose)?;
    let (count_text, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    let opener = if look.reveal || reveal_effect.is_some() {
        "Reveal"
    } else {
        "Look at"
    };
    let control_suffix = match move_to_zone.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => " under its owner's control",
        crate::effects::BattlefieldController::You => " under your control",
    };
    let battlefield_suffix = format!(
        "{}{control_suffix}",
        describe_battlefield_entry_state_for_looked_move(move_to_zone)
    );
    Some(format!(
        "{opener} the top {count_text} {noun} of your library{where_clause}. Put {selection} from among them onto the battlefield{battlefield_suffix}. Put the rest into your graveyard"
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LookedCountReplacementRemainder {
    LibraryBottom(crate::effects::consult_helpers::LibraryBottomOrder),
    Graveyard,
}

struct LookedCountReplacementBranch<'a> {
    look: &'a crate::effects::LookAtTopCardsEffect,
    explicitly_revealed: bool,
    choose: &'a crate::effects::ChooseObjectsEffect,
    remainder: LookedCountReplacementRemainder,
}

fn exact_looked_count_replacement_branch(
    effects: &[Effect],
) -> Option<LookedCountReplacementBranch<'_>> {
    let look = effects
        .first()?
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if look.player != PlayerFilter::You {
        return None;
    }

    let mut cursor = 1usize;
    let explicitly_revealed = effects
        .get(cursor)
        .and_then(|effect| effect.downcast_ref::<crate::effects::RevealTaggedEffect>())
        .is_some_and(|reveal| reveal.tag == look.tag);
    if explicitly_revealed {
        cursor += 1;
    }
    let [choose_effect, move_effect, remainder_effect] = effects.get(cursor..)? else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !exact_looked_library_choice(choose, &look.tag)
        || choose.count.dynamic_x
        || choose.count.up_to_x
        || choose.count.random
        || choose
            .count
            .max
            .is_none_or(|max| max == 0 || choose.count.min > max)
        || (choose.count.min != 0 && choose.count.max != Some(choose.count.min))
        || !exact_plain_selected_group_move_to_zone(move_effect, &choose.tag, Zone::Hand)
    {
        return None;
    }

    let remainder = if let Some(bottom) =
        remainder_effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
    {
        if bottom.tag != look.tag
            || bottom.keep_tagged.as_ref() != Some(&choose.tag)
            || bottom.player != look.player
        {
            return None;
        }
        LookedCountReplacementRemainder::LibraryBottom(bottom.order)
    } else if exact_tagged_remainder_to_graveyard(remainder_effect, &look.tag, &choose.tag) {
        LookedCountReplacementRemainder::Graveyard
    } else {
        return None;
    };

    Some(LookedCountReplacementBranch {
        look,
        explicitly_revealed,
        choose,
        remainder,
    })
}

fn same_looked_choice_except_count_and_result_tag(
    left: &crate::effects::ChooseObjectsEffect,
    right: &crate::effects::ChooseObjectsEffect,
) -> bool {
    let normalized_tag = crate::TagKey::from("__count_replacement_selected__");
    let mut left = left.clone();
    left.count = ChoiceCount::exactly(1);
    left.tag = normalized_tag.clone();
    let mut right = right.clone();
    right.count = ChoiceCount::exactly(1);
    right.tag = normalized_tag;
    left == right
}

fn looked_count_replacement_selection(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let maximum = choose.count.max?;
    let mut plain_filter = choose.filter.clone();
    plain_filter.zone = None;
    plain_filter.tagged_constraints.clear();
    if plain_filter == ObjectFilter::default() {
        let count = small_number_word(maximum as u32).unwrap_or_else(|| maximum.to_string());
        return Some(match (choose.count.min, maximum) {
            (0, 1) | (1, 1) => "one of them".to_string(),
            (0, _) => format!("up to {count} of them"),
            (_, _) => format!("{count} of them"),
        });
    }

    let mut surface_choice = choose.clone();
    if choose.count.min == 0 && maximum == 1 {
        surface_choice.count = ChoiceCount::exactly(1);
    }
    Some(format!(
        "{} from among them",
        describe_looked_battlefield_selection(&surface_choice)?
    ))
}

fn looked_count_replacement_hand_clause(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let prefix = if choose.count.min == 0 && choose.count.max == Some(1) {
        "You may put"
    } else {
        "Put"
    };
    Some(format!(
        "{prefix} {} into your hand",
        looked_count_replacement_selection(choose)?
    ))
}

fn looked_count_replacement_remainder_clause(
    remainder: LookedCountReplacementRemainder,
) -> &'static str {
    match remainder {
        LookedCountReplacementRemainder::LibraryBottom(
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
        ) => "the rest on the bottom of your library in a random order",
        LookedCountReplacementRemainder::LibraryBottom(
            crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
        ) => "the rest on the bottom of your library in any order",
        LookedCountReplacementRemainder::Graveyard => "the rest into your graveyard",
    }
}

fn looked_count_replacement_branch_body(
    branch: &LookedCountReplacementBranch<'_>,
) -> Option<String> {
    let selection = looked_count_replacement_hand_clause(branch.choose)?;
    let remainder = looked_count_replacement_remainder_clause(branch.remainder);
    Some(match branch.remainder {
        LookedCountReplacementRemainder::LibraryBottom(_) => {
            format!("{selection} and {remainder}")
        }
        LookedCountReplacementRemainder::Graveyard => {
            format!("{selection}. Put {remainder}")
        }
    })
}

/// Renders a looked-card partition whose self-replacement changes only the
/// number selected for hand. Both branches must repeat the same typed
/// look/reveal prelude and dispose of the exact looked-minus-selected
/// complement identically. That proof lets the default remainder stay before
/// a labeled replacement clause, while an unlabeled whole-branch replacement
/// retains its own remainder instead of moving it after the condition.
pub(in crate::compiled_text) fn describe_looked_count_self_replacement(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    let [replacement] = segment.self_replacements.as_slice() else {
        return None;
    };
    if replacement.condition_after_replacement {
        return None;
    }
    let default = exact_looked_count_replacement_branch(&segment.default_effects)?;
    let alternate = exact_looked_count_replacement_branch(&replacement.replacement_effects)?;
    if default.look != alternate.look
        || default.explicitly_revealed != alternate.explicitly_revealed
        || default.remainder != alternate.remainder
        || !same_looked_choice_except_count_and_result_tag(default.choose, alternate.choose)
    {
        return None;
    }
    let default_max = default.choose.count.max?;
    let alternate_max = alternate.choose.count.max?;
    if alternate_max <= default_max
        || (default.choose.count.min == 0) != (alternate.choose.count.min == 0)
    {
        return None;
    }

    let (look_count, noun, where_clause) =
        describe_top_count_noun_and_where_clause(&default.look.count);
    let opener = if default.look.reveal || default.explicitly_revealed {
        format!("Reveal the top {look_count} {noun} of your library{where_clause}")
    } else {
        format!("Look at the top {look_count} {noun} of your library{where_clause}")
    };
    let default_body = looked_count_replacement_branch_body(&default)?;
    let default_text = format!("{opener}. {default_body}");
    let condition = describe_condition(&replacement.condition);
    let visible_label = replacement
        .presentation_label
        .as_ref()
        .and_then(PresentationLabel::display_prefix)
        .is_some_and(|label| {
            let label = label.trim();
            !label.is_empty() && !label.starts_with("__ironsmith_")
        });

    if visible_label {
        let default_count =
            small_number_word(default_max as u32).unwrap_or_else(|| default_max.to_string());
        let alternate_selection = looked_count_replacement_hand_clause(alternate.choose)?;
        return Some(format!(
            "{default_text}. If {condition}, {} instead of {default_count}",
            lowercase_first(&alternate_selection)
        ));
    }
    if !matches!(
        default.remainder,
        LookedCountReplacementRemainder::LibraryBottom(_)
    ) {
        return None;
    }
    let alternate_body = looked_count_replacement_branch_body(&alternate)?;
    Some(format!(
        "{default_text}. If {condition}, instead {}",
        lowercase_first(&alternate_body)
    ))
}

/// Compacts a face-down selected card plus the exact graveyard complement and
/// a permission tied to that same selected tag.
pub(super) fn describe_look_exile_face_down_rest_graveyard_then_cast(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [
        look_effect,
        choose_effect,
        exile_effect,
        remainder_effect,
        grant_effect,
    ] = effects.get(..5)?
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = exile_effect.downcast_ref::<crate::effects::ExileEffect>()?;
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if look.reveal
        || !exact_looked_library_choice(choose, &look.tag)
        || exact_nonrandom_choice_count(&choose.count) != Some(1)
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || !exile.face_down
        || !exact_tagged_remainder_to_graveyard(remainder_effect, &look.tag, &choose.tag)
        || grant.tag != choose.tag
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || grant.allow_land
        || grant.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::AnyType
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.cast_pool_is_plural
    {
        return None;
    }

    let owner = describe_possessive_player_filter(&look.player);
    let graveyard_owner = if look.player == PlayerFilter::You {
        "your"
    } else {
        "their"
    };
    let (look_count, noun, where_clause) = describe_top_count_noun_and_where_clause(&look.count);
    Some((
        format!(
            "Look at the top {look_count} {noun} of {owner} library{where_clause}, exile one of them face down, then put the rest into {graveyard_owner} graveyard. You may cast that card for as long as it remains exiled, and mana of any type can be spent to cast that spell"
        ),
        5,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looked_count_replacement_partition(
        looked: crate::TagKey,
        selected: crate::TagKey,
        count: ChoiceCount,
        reveal: bool,
        creature_only: bool,
        remainder: LookedCountReplacementRemainder,
    ) -> Vec<Effect> {
        let mut filter = ObjectFilter::tagged(looked.clone()).in_zone(Zone::Library);
        if creature_only {
            filter.card_types.push(CardType::Creature);
        }
        let choose = Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                filter,
                count,
                PlayerFilter::You,
                selected.clone(),
            )
            .in_zone(Zone::Library),
        );
        let move_selected = Effect::for_each_tagged(
            selected.clone(),
            vec![Effect::move_to_zone(
                ChooseSpec::Iterated,
                Zone::Hand,
                false,
            )],
        );
        let remainder = match remainder {
            LookedCountReplacementRemainder::LibraryBottom(order) => Effect::new(
                crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                    looked.clone(),
                    Some(selected),
                    order,
                    PlayerFilter::You,
                ),
            ),
            LookedCountReplacementRemainder::Graveyard => {
                let mut membership = ObjectFilter::default();
                membership
                    .tagged_constraints
                    .push(crate::filter::TaggedObjectConstraint {
                        tag: crate::TagKey::from("__it__"),
                        relation: crate::filter::TaggedOpbjectRelation::SameStableId,
                    });
                Effect::for_each_tagged(
                    looked.clone(),
                    vec![Effect::conditional(
                        Condition::TaggedObjectMatches(selected, membership),
                        vec![],
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Graveyard,
                            false,
                        )],
                    )],
                )
            }
        };

        let mut effects = vec![Effect::new(crate::effects::LookAtTopCardsEffect::new(
            PlayerFilter::You,
            if creature_only { 5 } else { 3 },
            looked.clone(),
        ))];
        if reveal {
            effects.push(Effect::new(crate::effects::RevealTaggedEffect::new(looked)));
        }
        effects.extend([choose, move_selected, remainder]);
        effects
    }

    fn count_replacement_test_condition() -> Condition {
        Condition::PlayerControls {
            player: PlayerFilter::You,
            filter: ObjectFilter::creature().in_zone(Zone::Battlefield),
        }
    }

    #[test]
    fn renders_unlabeled_looked_count_replacement_as_two_complete_partitions() {
        let looked = crate::TagKey::from("flow_looked");
        let default_effects = looked_count_replacement_partition(
            looked.clone(),
            crate::TagKey::from("flow_one"),
            ChoiceCount::exactly(1),
            false,
            false,
            LookedCountReplacementRemainder::LibraryBottom(
                crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
            ),
        );
        let replacement_effects = looked_count_replacement_partition(
            looked,
            crate::TagKey::from("flow_two"),
            ChoiceCount::exactly(2),
            false,
            false,
            LookedCountReplacementRemainder::LibraryBottom(
                crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
            ),
        );
        let program =
            crate::resolution::ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
                default_effects,
                self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                    count_replacement_test_condition(),
                    replacement_effects,
                )],
            }]);

        assert_eq!(
            super::super::super::ast_render::describe_resolution_program(&program),
            "Look at the top three cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order. If you control a creature, instead put two of them into your hand and the rest on the bottom of your library in any order"
        );
    }

    #[test]
    fn renders_labeled_reveal_count_replacement_after_the_default_remainder() {
        let looked = crate::TagKey::from("gather_revealed");
        let default_effects = looked_count_replacement_partition(
            looked.clone(),
            crate::TagKey::from("gather_one"),
            ChoiceCount::up_to(1),
            true,
            true,
            LookedCountReplacementRemainder::Graveyard,
        );
        let replacement_effects = looked_count_replacement_partition(
            looked,
            crate::TagKey::from("gather_two"),
            ChoiceCount::up_to(2),
            true,
            true,
            LookedCountReplacementRemainder::Graveyard,
        );
        let branch = crate::resolution::SelfReplacementBranch::new(
            count_replacement_test_condition(),
            replacement_effects,
        )
        .with_presentation_label(Some(PresentationLabel::from_ability_word("Spell mastery")));
        let program =
            crate::resolution::ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
                default_effects,
                self_replacements: vec![branch],
            }]);

        assert_eq!(
            super::super::super::ast_render::describe_resolution_program(&program),
            "Reveal the top five cards of your library. You may put a creature card from among them into your hand. Put the rest into your graveyard. Spell mastery — If you control a creature, put up to two creature cards from among them into your hand instead of one"
        );
    }

    fn ordered_move(
        tag: crate::TagKey,
        to_top: bool,
        order: crate::effects::LibraryPlacementOrder,
    ) -> Effect {
        Effect::new(
            crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(tag), Zone::Library, to_top)
                .with_library_order(order),
        )
    }

    fn exact_hand_remainder_bottom_effects(selected_count: usize) -> Vec<Effect> {
        let looked = crate::TagKey::from("dynamic_looked_pool");
        let selected = crate::TagKey::from("dynamic_selected_set");
        let choose = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::tagged(looked.clone()).in_zone(Zone::Library),
            ChoiceCount::exactly(selected_count),
            PlayerFilter::You,
            selected.clone(),
        )
        .in_zone(Zone::Library);
        let move_selected = crate::effects::ForEachTaggedEffect::new(
            selected.clone(),
            vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Iterated,
                Zone::Hand,
                false,
            ))],
        );

        vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::PartySize(PlayerFilter::You).with_surface_hint(ValueSurfaceHint::WhereXIs),
                looked.clone(),
            )),
            Effect::new(choose),
            Effect::new(move_selected),
            Effect::new(
                crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                    looked,
                    Some(selected),
                    crate::effects::consult_helpers::LibraryBottomOrder::Random,
                    PlayerFilter::You,
                ),
            ),
        ]
    }

    #[test]
    fn renders_exact_hand_subset_and_tagged_remainder() {
        for (selected_count, selected_word) in [(1, "one"), (2, "two"), (3, "three")] {
            let effects = exact_hand_remainder_bottom_effects(selected_count);
            let expected = format!(
                "Look at the top X cards of your library, where X is the number of creatures in your party. Put {selected_word} of those cards into your hand and the rest on the bottom of your library in a random order"
            );

            assert_eq!(describe_effect_list(&effects), expected);
            assert_eq!(
                describe_pre_clause_structural_effect_list(&effects).as_deref(),
                Some(expected.as_str())
            );
        }
    }

    #[test]
    fn renders_exact_singleton_hand_subset_with_direct_tagged_move() {
        let mut effects = exact_hand_remainder_bottom_effects(1);
        effects[2] = Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(crate::TagKey::from("dynamic_selected_set")),
            Zone::Hand,
            false,
        ));

        let expected = "Look at the top X cards of your library, where X is the number of creatures in your party. Put one of those cards into your hand and the rest on the bottom of your library in a random order";
        assert_eq!(describe_effect_list(&effects), expected);
        assert_eq!(
            describe_pre_clause_structural_effect_list(&effects).as_deref(),
            Some(expected)
        );
    }

    fn private_top_choice_to_graveyard_effects(target_player: bool) -> Vec<Effect> {
        let looked = crate::TagKey::from("private_top_cards");
        let selected = crate::TagKey::from("private_top_choice");
        let look_player = if target_player {
            PlayerFilter::target_player()
        } else {
            PlayerFilter::You
        };
        let owner = if target_player {
            PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any))
        } else {
            PlayerFilter::You
        };
        let mut effects = Vec::new();
        if target_player {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_player(),
            )));
        }
        effects.push(Effect::new(crate::effects::LookAtTopCardsEffect::new(
            look_player,
            Value::Fixed(2),
            looked.clone(),
        )));
        effects.push(Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::tagged(looked)
                    .in_zone(Zone::Library)
                    .owned_by(owner),
                ChoiceCount::exactly(1),
                PlayerFilter::You,
                selected.clone(),
            )
            .in_zone(Zone::Library),
        ));
        let move_selected = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(selected),
            Zone::Graveyard,
            false,
        );
        let move_selected = if target_player {
            move_selected.with_destination_player_reference_surface(
                ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer,
            )
        } else {
            move_selected.with_destination_player_surface(PlayerFilter::You)
        };
        effects.push(Effect::new(move_selected));
        effects
    }

    #[test]
    fn renders_private_exact_singleton_move_without_moving_the_remainder() {
        for (target_player, expected) in [
            (
                false,
                "Look at the top two cards of your library. Put one of them into your graveyard",
            ),
            (
                true,
                "Look at the top two cards of target player's library. Put one of them into their graveyard",
            ),
        ] {
            let effects = private_top_choice_to_graveyard_effects(target_player);
            assert_eq!(describe_effect_list(&effects), expected);
            assert_eq!(
                describe_pre_clause_structural_effect_list(&effects).as_deref(),
                Some(expected)
            );
        }
    }

    fn revealed_top_choice_to_graveyard_effects(opponent_chooses: bool) -> Vec<Effect> {
        let revealed = crate::TagKey::from("public_top_cards");
        let selected = crate::TagKey::from("public_top_choice");
        let look_player = if opponent_chooses {
            PlayerFilter::You
        } else {
            PlayerFilter::DamagedPlayer
        };
        let chooser = if opponent_chooses {
            PlayerFilter::target_opponent()
        } else {
            PlayerFilter::You
        };
        let mut effects = vec![Effect::new(
            crate::effects::LookAtTopCardsEffect::revealing(
                look_player,
                if opponent_chooses {
                    Value::Fixed(3)
                } else {
                    Value::Fixed(2)
                },
                revealed.clone(),
            ),
        )];
        if opponent_chooses {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target_opponent(),
            )));
        }
        effects.push(Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::tagged(revealed).in_zone(Zone::Library),
                ChoiceCount::exactly(1),
                chooser,
                selected.clone(),
            )
            .in_zone(Zone::Library),
        ));
        effects.push(Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(selected),
            Zone::Graveyard,
            false,
        )));
        if opponent_chooses {
            effects.push(Effect::draw(2));
        }
        effects
    }

    #[test]
    fn renders_public_top_choice_move_for_controller_or_target_opponent() {
        for (opponent_chooses, expected) in [
            (
                false,
                "That player reveals the top two cards of their library. You choose one of those cards and put it into their graveyard",
            ),
            (
                true,
                "Reveal the top three cards of your library. Target opponent chooses one of those cards. Put that card into your graveyard, then draw two cards",
            ),
        ] {
            let effects = revealed_top_choice_to_graveyard_effects(opponent_chooses);
            assert_eq!(describe_effect_list(&effects), expected);
            assert_eq!(
                describe_pre_clause_structural_effect_list(&effects).as_deref(),
                Some(expected)
            );
        }
    }

    fn partition_effects(
        player: PlayerFilter,
        count: ChoiceCount,
        selected_zone: Zone,
        selected_order: Option<crate::effects::LibraryPlacementOrder>,
        remainder_order: crate::effects::LibraryPlacementOrder,
    ) -> Vec<Effect> {
        let looked = crate::TagKey::from("looked_partition");
        let selected = crate::TagKey::from("partition_selected");
        let remainder = crate::TagKey::from("partition_remainder");
        let mut effects = Vec::new();
        if let PlayerFilter::Target(inner) = &player {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                ChooseSpec::target(ChooseSpec::Player((**inner).clone())),
            )));
        }
        effects.push(Effect::new(crate::effects::LookAtTopCardsEffect::new(
            player,
            Value::Fixed(5),
            looked.clone(),
        )));
        effects.push(Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::tagged(looked.clone()).in_zone(Zone::Library),
                count,
                PlayerFilter::You,
                selected.clone(),
            )
            .in_zone(Zone::Library),
        ));
        effects.push(Effect::new(
            crate::effects::TagMatchingObjectsEffect::new(
                ObjectFilter::tagged(looked)
                    .not_tagged(selected.clone())
                    .in_zone(Zone::Library),
                remainder.clone(),
            )
            .in_zone(Zone::Library),
        ));
        let selected_move = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(selected),
            selected_zone,
            false,
        );
        effects.push(Effect::new(if let Some(order) = selected_order {
            selected_move.with_library_order(order)
        } else {
            selected_move
        }));
        effects.push(ordered_move(remainder, true, remainder_order));
        effects
    }

    #[test]
    fn renders_hand_graveyard_and_two_library_partition_surfaces() {
        let any_order = crate::effects::LibraryPlacementOrder::ChosenBy(PlayerFilter::You);
        let diabolic = partition_effects(
            PlayerFilter::You,
            ChoiceCount::exactly(1),
            Zone::Hand,
            None,
            any_order.clone(),
        );
        assert_eq!(
            describe_effect_list(&diabolic),
            "Look at the top five cards of your library. Put one of them into your hand and the rest on top of your library in any order"
        );

        let cruel = partition_effects(
            PlayerFilter::target_opponent(),
            ChoiceCount::exactly(1),
            Zone::Graveyard,
            None,
            any_order.clone(),
        );
        assert_eq!(
            describe_effect_list(&cruel),
            "Look at the top five cards of target opponent's library. Put one of those cards into that player's graveyard and the rest on top of their library in any order"
        );

        let ransack = partition_effects(
            PlayerFilter::target_player(),
            ChoiceCount::any_number(),
            Zone::Library,
            Some(any_order.clone()),
            any_order.clone(),
        );
        assert_eq!(
            describe_effect_list(&ransack),
            "Look at the top five cards of target player's library. Put any number of them on the bottom of that library in any order and the rest on top of the library in any order"
        );

        let looked = crate::TagKey::from("optional_top_looked");
        let selected = crate::TagKey::from("optional_top_selected");
        let remainder = crate::TagKey::from("optional_top_remainder");
        let optional_top = vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::Fixed(4),
                looked.clone(),
            )),
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::tagged(looked.clone()).in_zone(Zone::Library),
                    ChoiceCount::up_to(1),
                    PlayerFilter::You,
                    selected.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(
                crate::effects::TagMatchingObjectsEffect::new(
                    ObjectFilter::tagged(looked)
                        .not_tagged(selected.clone())
                        .in_zone(Zone::Library),
                    remainder.clone(),
                )
                .in_zone(Zone::Library),
            ),
            ordered_move(selected, true, any_order.clone()),
            ordered_move(
                remainder,
                false,
                crate::effects::LibraryPlacementOrder::Random,
            ),
        ];
        assert_eq!(
            describe_effect_list(&optional_top),
            "Look at the top four cards of your library. Put up to one of them on top of your library and the rest on the bottom of your library in a random order"
        );

        let looked = crate::TagKey::from("dark_bargain_looked");
        let selected = crate::TagKey::from("dark_bargain_hand");
        let remainder = crate::TagKey::from("dark_bargain_remainder");
        let dark_bargain = vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::Fixed(3),
                looked.clone(),
            )),
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::tagged(looked.clone()).in_zone(Zone::Library),
                    ChoiceCount::exactly(2),
                    PlayerFilter::You,
                    selected.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(
                crate::effects::TagMatchingObjectsEffect::new(
                    ObjectFilter::tagged(looked)
                        .not_tagged(selected.clone())
                        .in_zone(Zone::Library),
                    remainder.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(selected),
                Zone::Hand,
                false,
            )),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(remainder),
                Zone::Graveyard,
                false,
            )),
        ];
        assert_eq!(
            describe_effect_list(&dark_bargain),
            "Look at the top three cards of your library. Put two of them into your hand and the other into your graveyard"
        );
    }

    fn bounded_hand_graveyard_partition(exact_complement: bool) -> Vec<Effect> {
        let looked = crate::TagKey::from("shadow_prophecy_looked");
        let selected = crate::TagKey::from("shadow_prophecy_hand");
        let remainder = crate::TagKey::from("shadow_prophecy_remainder");
        let mut remainder_filter = ObjectFilter::tagged(looked.clone());
        if exact_complement {
            remainder_filter = remainder_filter.not_tagged(selected.clone());
        }
        vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::Fixed(5),
                looked.clone(),
            )),
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::tagged(looked).in_zone(Zone::Library),
                    ChoiceCount::up_to(2),
                    PlayerFilter::You,
                    selected.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(
                crate::effects::TagMatchingObjectsEffect::new(
                    remainder_filter.in_zone(Zone::Library),
                    remainder.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(selected),
                Zone::Hand,
                false,
            )),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(remainder),
                Zone::Graveyard,
                false,
            )),
        ]
    }

    #[test]
    fn renders_bounded_up_to_partition_only_with_an_exact_complement() {
        let effects = bounded_hand_graveyard_partition(true);
        let refs = effects.iter().collect::<Vec<_>>();
        assert_eq!(
            describe_looked_card_selected_partition(&refs),
            Some((
                "Look at the top five cards of your library. Put up to two of them into your hand and the rest into your graveyard".to_string(),
                5,
            ))
        );

        let inexact = bounded_hand_graveyard_partition(false);
        let refs = inexact.iter().collect::<Vec<_>>();
        assert_eq!(describe_looked_card_selected_partition(&refs), None);
    }

    fn three_way_partition(middle_zone: Zone) -> Vec<Effect> {
        let looked = crate::TagKey::from("looked_three_way");
        let first = crate::TagKey::from("three_way_first");
        let second = crate::TagKey::from("three_way_second");
        let third = crate::TagKey::from("three_way_third");
        vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::Fixed(3),
                looked.clone(),
            )),
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::tagged(looked.clone()).in_zone(Zone::Library),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    first.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::tagged(looked.clone())
                        .not_tagged(first.clone())
                        .in_zone(Zone::Library),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    second.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::tagged(looked)
                        .not_tagged(first.clone())
                        .not_tagged(second.clone())
                        .in_zone(Zone::Library),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    third.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(first),
                Zone::Hand,
                false,
            )),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(second),
                middle_zone,
                middle_zone == Zone::Library,
            )),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(third),
                Zone::Library,
                false,
            )),
        ]
    }

    #[test]
    fn renders_three_way_disjoint_partitions_without_choice_scaffolding() {
        for (middle_zone, expected) in [
            (
                Zone::Library,
                "Look at the top three cards of your library. Put one of those cards into your hand, one on top of your library, and one on the bottom of your library",
            ),
            (
                Zone::Graveyard,
                "Look at the top three cards of your library. Put one of those cards into your hand, one into your graveyard, and one on the bottom of your library",
            ),
        ] {
            let effects = three_way_partition(middle_zone);
            assert_eq!(describe_effect_list(&effects), expected);
            assert_eq!(
                describe_pre_clause_structural_effect_list(&effects).as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn renders_self_look_reorder_optional_shuffle_as_separate_sentences() {
        let looked = crate::TagKey::from("looked_reorder");
        let effects = vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::Fixed(3),
                looked.clone(),
            )),
            Effect::new(crate::effects::ReorderLibraryTopEffect::new(looked)),
            Effect::may(vec![Effect::new(
                crate::effects::ShuffleLibraryEffect::new(PlayerFilter::You),
            )]),
            Effect::draw(1),
        ];
        assert_eq!(
            describe_effect_list(&effects),
            "Look at the top three cards of your library, then put them back in any order. You may shuffle. Draw a card"
        );
        assert_eq!(
            describe_pre_clause_structural_effect_list(&effects).as_deref(),
            Some(
                "Look at the top three cards of your library, then put them back in any order. You may shuffle. Draw a card"
            )
        );
    }

    fn hand_then_rest_bottom(count: i32) -> Vec<Effect> {
        let looked = crate::TagKey::from("looked_control");
        let selected = crate::TagKey::from("selected_control");
        vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::Fixed(count),
                looked.clone(),
            )),
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    ObjectFilter::tagged(looked.clone()).in_zone(Zone::Library),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    selected.clone(),
                )
                .in_zone(Zone::Library),
            ),
            Effect::new(crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(selected.clone()),
                Zone::Hand,
                false,
            )),
            Effect::new(
                crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                    looked,
                    Some(selected),
                    crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses,
                    PlayerFilter::You,
                ),
            ),
        ]
    }

    #[test]
    fn existing_anticipate_impulse_and_index_surfaces_remain_on_their_paths() {
        for (count, expected) in [
            (
                3,
                "Look at the top three cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order",
            ),
            (
                4,
                "Look at the top four cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order",
            ),
        ] {
            assert_eq!(
                describe_effect_list(&hand_then_rest_bottom(count)),
                expected
            );
        }

        let looked = crate::TagKey::from("index_control");
        let index = vec![
            Effect::new(crate::effects::LookAtTopCardsEffect::new(
                PlayerFilter::You,
                Value::Fixed(5),
                looked.clone(),
            )),
            Effect::new(crate::effects::ReorderLibraryTopEffect::new(looked)),
        ];
        assert_eq!(
            describe_effect_list(&index),
            "Look at the top five cards of your library, then put them back in any order"
        );
    }
}
