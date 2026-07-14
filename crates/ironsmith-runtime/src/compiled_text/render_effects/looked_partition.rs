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
    if !(selected_then_library_top || selected_hand_then_graveyard) {
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
        LookedPartitionDestination::LibraryTop(_) => return None,
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
        _ => return None,
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
            any_order,
        );
        assert_eq!(
            describe_effect_list(&ransack),
            "Look at the top five cards of target player's library. Put any number of them on the bottom of that library in any order and the rest on top of the library in any order"
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
