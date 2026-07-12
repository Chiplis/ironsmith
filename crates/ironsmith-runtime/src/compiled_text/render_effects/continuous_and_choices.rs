use super::*;

pub(super) fn normalize_looked_card_filter_description(
    filter: &ObjectFilter,
    card_desc: &str,
) -> String {
    if let Some(desc) = describe_card_type_looked_card_disjunction(filter) {
        return desc;
    }

    if let Some(desc) = describe_creature_vehicle_looked_card_disjunction(filter) {
        return desc;
    }

    if filter.type_or_subtype_union
        && filter.card_types == [CardType::Creature]
        && filter.subtypes == [Subtype::Vehicle]
    {
        let mut desc = "creature and/or Vehicle card".to_string();
        if filter.distinct_names {
            desc.push_str(" with different names");
        }
        if filter.distinct_powers {
            desc.push_str(" with different powers");
        }
        return desc;
    }

    let mut card_desc = strip_leading_article(card_desc).to_string();
    if matches!(
        card_desc.as_str(),
        "instant or sorcery"
            | "instant or sorcery card"
            | "instant or sorcery cards"
            | "instants or sorcery"
            | "instants or sorcery cards"
    ) {
        return "instant and/or sorcery card".to_string();
    }
    card_desc = card_desc.replace("permanent named ", "card named ");
    if filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && !filter.token
        && card_desc == "permanent"
    {
        card_desc = "card".to_string();
    } else if filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && !filter.token
        && let Some(prefix) = card_desc.strip_suffix(" permanent")
    {
        card_desc = format!("{prefix} card");
    }
    card_desc
}

pub(super) fn describe_card_type_looked_card_disjunction(filter: &ObjectFilter) -> Option<String> {
    let types = simple_looked_card_disjunction_types(filter)?;
    if types.len() < 2 {
        return None;
    }

    let mut desc = format!(
        "{} card",
        types
            .iter()
            .map(|card_type| describe_card_type_word_local(*card_type))
            .collect::<Vec<_>>()
            .join(" and/or ")
    );
    let all_branches_distinct_names =
        !filter.any_of.is_empty() && filter.any_of.iter().all(|branch| branch.distinct_names);
    let all_branches_distinct_powers =
        !filter.any_of.is_empty() && filter.any_of.iter().all(|branch| branch.distinct_powers);
    if filter.distinct_names || all_branches_distinct_names {
        desc.push_str(" with different names");
    }
    if filter.distinct_powers || all_branches_distinct_powers {
        desc.push_str(" with different powers");
    }
    Some(desc)
}

pub(super) fn simple_looked_card_disjunction_types(filter: &ObjectFilter) -> Option<Vec<CardType>> {
    if filter.type_or_subtype_union
        && filter.subtypes.is_empty()
        && filter.all_card_types.is_empty()
        && filter.any_of.is_empty()
        && filter.card_types.len() >= 2
    {
        return Some(filter.card_types.clone());
    }

    if filter.any_of.len() >= 2
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.all_card_types.is_empty()
    {
        let mut types = Vec::new();
        for branch in &filter.any_of {
            if !branch.any_of.is_empty()
                || !branch.subtypes.is_empty()
                || !branch.all_card_types.is_empty()
                || branch.card_types.len() != 1
            {
                return None;
            }
            types.push(branch.card_types[0]);
        }
        return Some(types);
    }

    None
}

pub(super) fn describe_creature_vehicle_looked_card_disjunction(
    filter: &ObjectFilter,
) -> Option<String> {
    if filter.any_of.len() != 2 {
        return None;
    }
    let has_creature_branch = filter.any_of.iter().any(|branch| {
        branch.card_types == [CardType::Creature]
            && branch.subtypes.is_empty()
            && branch.any_of.is_empty()
    });
    let has_vehicle_branch = filter.any_of.iter().any(|branch| {
        branch.card_types.is_empty()
            && branch.subtypes == [Subtype::Vehicle]
            && branch.any_of.is_empty()
    });
    if !has_creature_branch || !has_vehicle_branch {
        return None;
    }

    let mut desc = "creature and/or Vehicle card".to_string();
    if filter.distinct_names || filter.any_of.iter().all(|branch| branch.distinct_names) {
        desc.push_str(" with different names");
    }
    if filter.distinct_powers || filter.any_of.iter().all(|branch| branch.distinct_powers) {
        desc.push_str(" with different powers");
    }
    Some(desc)
}

pub(super) fn looked_filter_can_include_card_type(
    filter: &ObjectFilter,
    card_type: CardType,
) -> bool {
    filter.card_types.contains(&card_type)
        || filter
            .any_of
            .iter()
            .any(|branch| looked_filter_can_include_card_type(branch, card_type))
}

pub(super) fn looked_filter_is_creature_land_union(filter: &ObjectFilter) -> bool {
    looked_filter_can_include_card_type(filter, CardType::Creature)
        && looked_filter_can_include_card_type(filter, CardType::Land)
        && filter.subtypes.is_empty()
}

pub(super) fn describe_land_or_legendary_permanent_looked_filter(
    filter: &ObjectFilter,
) -> Option<String> {
    if filter.any_of.len() != 2 {
        return None;
    }

    let mut land_branch = None;
    let mut legendary_branch = None;
    for branch in &filter.any_of {
        if branch.card_types == [CardType::Land]
            && branch.supertypes.is_empty()
            && branch.any_of.is_empty()
        {
            land_branch = Some(branch);
            continue;
        }
        if branch.card_types == ObjectFilter::permanent_card().card_types
            && branch.supertypes == [Supertype::Legendary]
            && branch.any_of.is_empty()
        {
            legendary_branch = Some(branch);
        }
    }

    let land = land_branch?;
    let legendary = legendary_branch?;
    if land.mana_value != legendary.mana_value {
        return None;
    }

    let suffix = match land.mana_value.as_ref() {
        Some(crate::filter::Comparison::LessThanOrEqual(value)) => {
            format!(" with mana value {value} or less")
        }
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) => {
            format!(" with mana value {} or less", describe_value(value))
        }
        None => String::new(),
        _ => return None,
    };
    Some(format!("land and/or legendary permanent cards{suffix}"))
}

pub(super) fn unwrap_tag_wrapped_effect<'a>(effect: &'a Effect) -> &'a Effect {
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_tag_wrapped_effect(&tag_all.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_tag_wrapped_effect(&tagged.effect);
    }
    effect
}

pub(super) fn tag_all_wrapper_tag_for_effect<'a>(effect: &'a Effect) -> Option<&'a str> {
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(tag_all.tag.as_str());
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tag_all_wrapper_tag_for_effect(&with_id.effect);
    }
    None
}

pub(super) fn land_move_from_chosen_effect(effect: &Effect, tag: &str) -> Option<bool> {
    let unwrapped = unwrap_tag_wrapped_effect(effect);
    if let Some(put) = unwrapped.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>() {
        if matches!(put.target.base(), ChooseSpec::Tagged(t) if t.as_str() == tag) {
            return Some(put.tapped);
        }
    }
    if let Some(move_to_zone) = unwrapped.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_battlefield_uses_chosen_tag(move_to_zone, tag)
    {
        return Some(move_to_zone.enters_tapped);
    }
    None
}

pub(crate) fn describe_look_at_top_then_put_onto_battlefield_and_into_hand_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    land_choose: &crate::effects::ChooseObjectsEffect,
    land_move_effect: &Effect,
    hand_choose: &crate::effects::ChooseObjectsEffect,
    hand_move_effect: &Effect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if let Some(reveal_top) = reveal_top
        && reveal_top.tag.as_str() != look_at_top.tag.as_str()
    {
        return None;
    }
    let keep_tag = remainder.keep_tagged.as_ref()?;
    if tag_all_wrapper_tag_for_effect(land_move_effect) != Some(keep_tag.as_str())
        || tag_all_wrapper_tag_for_effect(hand_move_effect) != Some(keep_tag.as_str())
    {
        return None;
    }
    let tapped = land_move_from_chosen_effect(land_move_effect, land_choose.tag.as_str())?;
    let hand_move = unwrap_tag_wrapped_effect(hand_move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_hand_uses_chosen_tag(hand_move, hand_choose.tag.as_str())
        || remainder.tag.as_str() != look_at_top.tag.as_str()
    {
        return None;
    }
    let excludes_battlefield_choice =
        hand_choose
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == land_choose.tag.as_str()
                    && matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                    )
            });
    if !excludes_battlefield_choice {
        return None;
    }

    let land = strip_leading_article(&describe_choose_filter_from_looked_cards(
        look_at_top,
        land_choose,
    )?)
    .to_string();
    let hand = strip_leading_article(&describe_choose_filter_from_looked_cards_with_ignored_tags(
        look_at_top,
        hand_choose,
        &[land_choose.tag.as_str()],
    )?)
    .to_string();
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand_owner = describe_possessive_player_filter(&hand_choose.chooser);
    let (count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
    let opener = if reveal_top.is_some() || look_at_top.reveal {
        "Reveal"
    } else {
        "Look at"
    };
    let may_prefix = if hand_choose.chooser == PlayerFilter::You {
        "You may".to_string()
    } else {
        format!(
            "{} may",
            capitalize_first(&describe_player_filter(&hand_choose.chooser))
        )
    };
    let tapped_suffix = if tapped { " tapped" } else { "" };
    let order_text = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => format!(
            " in an order chosen by {}",
            describe_player_filter(&remainder.player)
        ),
    };

    Some(format!(
        "{opener} the top {count_text} {noun} of {owner} library. {may_prefix} put up to one {land} from among them onto the battlefield{tapped_suffix} and up to one {hand} from among them into {hand_owner} hand. Put the rest on the bottom of {owner} library{order_text}"
    ))
}

pub(super) fn for_each_puts_tag_onto_battlefield(
    for_each: &crate::effects::ForEachTaggedEffect,
    tag: &str,
) -> Option<bool> {
    if for_each.tag.as_str() != tag || for_each.effects.len() != 1 {
        return None;
    }
    if let Some(put) =
        for_each.effects[0].downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
        && matches!(put.target, ChooseSpec::Iterated)
        && matches!(put.controller, PlayerFilter::You)
    {
        return Some(put.tapped);
    }
    if let Some(move_to_zone) =
        for_each.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == Zone::Battlefield
        && matches!(move_to_zone.target, ChooseSpec::Iterated)
        && move_to_zone.battlefield_controller == crate::effects::BattlefieldController::You
    {
        return Some(move_to_zone.enters_tapped);
    }
    None
}

pub(crate) fn describe_look_at_top_then_may_put_battlefield_else_hand_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    battlefield_choose: &crate::effects::ChooseObjectsEffect,
    battlefield_move_id: Option<&crate::effects::WithIdEffect>,
    battlefield_move: &crate::effects::ForEachTaggedEffect,
    if_not_moved: &crate::effects::IfEffect,
    rest: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    let move_id = battlefield_move_id?;
    if if_not_moved.condition != move_id.id
        || if_not_moved.predicate != EffectPredicate::DidNotHappen
        || !if_not_moved.else_.is_empty()
        || if_not_moved.then.len() != 2
    {
        return None;
    }
    if choose_primary_zone(battlefield_choose) != Some(Zone::Library)
        || battlefield_choose.is_search
        || battlefield_choose.count.min != 0
        || battlefield_choose.count.max != Some(1)
    {
        return None;
    }
    let tapped =
        for_each_puts_tag_onto_battlefield(battlefield_move, battlefield_choose.tag.as_str())?;
    let hand_choose = if_not_moved.then[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let hand_move = if_not_moved.then[1].downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if choose_primary_zone(hand_choose) != Some(Zone::Library)
        || hand_choose.is_search
        || choose_exact_count(hand_choose) != Some(1)
        || !for_each_moves_tag_to_hand(hand_move, hand_choose.tag.as_str())
        || !for_each_moves_unselected_from_any_to_zone(
            rest,
            look_at_top.tag.as_str(),
            &[battlefield_choose.tag.as_str(), hand_choose.tag.as_str()],
            Zone::Library,
        )
    {
        return None;
    }

    let battlefield_choice =
        describe_choose_filter_from_looked_cards(look_at_top, battlefield_choose)?;
    let hand_choice = describe_choose_filter_from_looked_cards(look_at_top, hand_choose)?;
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&hand_choose.chooser);
    let (count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
    let tapped_suffix = if tapped { " tapped" } else { "" };
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library. You may put {battlefield_choice} from among them onto the battlefield{tapped_suffix}. If you don't, put {hand_choice} from among them into {hand} hand. Put the rest on the bottom of {owner} library"
    ))
}

pub(crate) fn describe_look_at_top_then_reveal_put_into_hand_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: Option<&crate::effects::ForEachTaggedEffect>,
    move_to_hand: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if let Some(reveal) = reveal
        && !for_each_reveals_tag(reveal, choose.tag.as_str())
    {
        return None;
    }
    if !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
        || rest.tag.as_str() != look_at_top.tag.as_str()
        || rest
            .keep_tagged
            .as_ref()
            .is_none_or(|tag| tag.as_str() != choose.tag.as_str())
    {
        return None;
    }

    let chosen = describe_choose_filter_from_looked_cards(look_at_top, choose)?;
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
    let may_prefix = if choose.chooser == PlayerFilter::You {
        "You may".to_string()
    } else {
        format!(
            "{} may",
            capitalize_first(&describe_player_filter(&choose.chooser))
        )
    };
    let order_text = match rest.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in any order".to_string()
        }
    };

    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library. {may_prefix} reveal {chosen} from among them and put it into {hand} hand. Put the rest on the bottom of {owner} library{order_text}"
    ))
}

pub(super) fn for_each_moves_tag_to_library_top(
    for_each: &crate::effects::ForEachTaggedEffect,
    tag: &str,
) -> bool {
    fn iterated_or_tagged(spec: &ChooseSpec, tag: &str) -> bool {
        match spec.base() {
            ChooseSpec::Iterated => true,
            ChooseSpec::Tagged(move_tag) => move_tag.as_str() == tag,
            _ => false,
        }
    }

    if for_each.tag.as_str() != tag || for_each.effects.len() != 1 {
        return false;
    }
    let unwrapped = unwrap_tag_wrapped_effect(&for_each.effects[0]);
    matches!(
        unwrapped.downcast_ref::<crate::effects::MoveToZoneEffect>(),
        Some(move_to_zone)
            if move_to_zone.zone == Zone::Library
                && move_to_zone.to_top
                && iterated_or_tagged(&move_to_zone.target, tag)
    )
}

pub(crate) fn describe_look_at_top_then_reveal_put_on_top_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: &crate::effects::ForEachTaggedEffect,
    move_to_top: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if !for_each_reveals_tag(reveal, choose.tag.as_str())
        || !for_each_moves_tag_to_library_top(move_to_top, choose.tag.as_str())
        || rest.tag.as_str() != look_at_top.tag.as_str()
        || rest
            .keep_tagged
            .as_ref()
            .is_none_or(|tag| tag.as_str() != choose.tag.as_str())
    {
        return None;
    }

    let chosen = describe_choose_filter_from_looked_cards(look_at_top, choose)?;
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let may_prefix = if choose.chooser == PlayerFilter::You {
        "You may".to_string()
    } else {
        format!(
            "{} may",
            capitalize_first(&describe_player_filter(&choose.chooser))
        )
    };
    let order_text = match rest.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in any order".to_string()
        }
    };

    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library{count_where_clause}. {may_prefix} reveal {chosen} from among them and put it on top of {owner} library. Put the rest on the bottom of {owner} library{order_text}"
    ))
}

pub(super) fn for_each_moves_tag_to_public_zone(
    for_each: &crate::effects::ForEachTaggedEffect,
    tag: &str,
) -> Option<(Zone, bool)> {
    if for_each.tag.as_str() != tag || for_each.effects.len() != 1 {
        return None;
    }
    let move_to_zone = for_each.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !matches!(move_to_zone.target.base(), ChooseSpec::Iterated) {
        return None;
    }
    match move_to_zone.zone {
        Zone::Hand => Some((Zone::Hand, false)),
        Zone::Battlefield => Some((Zone::Battlefield, move_to_zone.enters_tapped)),
        _ => None,
    }
}

pub(super) fn describe_look_at_top_then_put_any_matching_to_zone_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    choose: &crate::effects::ChooseObjectsEffect,
    move_chosen: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    let opener = if let Some(reveal_top) = reveal_top {
        if reveal_top.tag.as_str() != look_at_top.tag.as_str() {
            return None;
        }
        "Reveal"
    } else {
        "Look at"
    };
    if rest.tag.as_str() != look_at_top.tag.as_str()
        || rest
            .keep_tagged
            .as_ref()
            .is_none_or(|tag| tag.as_str() != choose.tag.as_str())
    {
        return None;
    }
    let (zone, tapped) = for_each_moves_tag_to_public_zone(move_chosen, choose.tag.as_str())?;
    let matching = describe_any_number_filter_from_looked_cards(look_at_top, choose)?;
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let direct_put_any_number = choose.filter.distinct_powers
        || (zone == Zone::Hand && looked_filter_is_creature_land_union(&choose.filter));
    let put_prefix = if direct_put_any_number {
        if choose.chooser == PlayerFilter::You {
            "Put".to_string()
        } else {
            format!(
                "{} puts",
                capitalize_first(&describe_player_filter(&choose.chooser))
            )
        }
    } else if choose.chooser == PlayerFilter::You {
        "You may put".to_string()
    } else {
        format!(
            "{} may put",
            capitalize_first(&describe_player_filter(&choose.chooser))
        )
    };
    let destination = match zone {
        Zone::Hand => format!(
            "into {} hand",
            describe_possessive_player_filter(&choose.chooser)
        ),
        Zone::Battlefield => {
            let tapped_suffix = if tapped { " tapped" } else { "" };
            format!("onto the battlefield{tapped_suffix}")
        }
        _ => return None,
    };
    let order_text = match rest.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in any order".to_string()
        }
    };

    if zone == Zone::Hand
        && looked_filter_is_creature_land_union(&choose.filter)
        && !choose.filter.distinct_powers
    {
        return Some(format!(
            "{opener} the top {count_text} {noun} of {owner} library{count_where_clause}. {put_prefix} any number of {matching} from among them {destination} and the rest on the bottom of {owner} library{order_text}"
        ));
    }

    Some(format!(
        "{opener} the top {count_text} {noun} of {owner} library{count_where_clause}. {put_prefix} any number of {matching} from among them {destination}. Put the rest on the bottom of {owner} library{order_text}"
    ))
}

pub(super) fn describe_look_at_top_then_reveal_any_matching_to_hand_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: &crate::effects::RevealTaggedEffect,
    move_chosen: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if reveal.tag.as_str() != choose.tag.as_str()
        || !for_each_moves_tag_to_hand(move_chosen, choose.tag.as_str())
        || rest.tag.as_str() != look_at_top.tag.as_str()
        || rest
            .keep_tagged
            .as_ref()
            .is_none_or(|tag| tag.as_str() != choose.tag.as_str())
    {
        return None;
    }
    let matching = if choose.count.is_any_number() {
        format!(
            "any number of {}",
            describe_any_number_filter_from_looked_cards(look_at_top, choose)?
        )
    } else {
        describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let may_prefix = if choose.chooser == PlayerFilter::You {
        "You may".to_string()
    } else {
        format!(
            "{} may",
            capitalize_first(&describe_player_filter(&choose.chooser))
        )
    };
    let order_text = match rest.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in any order".to_string()
        }
    };

    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library{count_where_clause}. {may_prefix} reveal {matching} from among them and put the revealed cards into {hand} hand. Put the rest on the bottom of {owner} library{order_text}"
    ))
}

pub(super) fn describe_look_at_top_then_reveal_split_matching_to_hand_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    chooses: &[&crate::effects::ChooseObjectsEffect],
    reveal: &crate::effects::RevealTaggedEffect,
    move_chosen: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    let first = *chooses.first()?;
    let chosen_tag = first.tag.as_str();
    if chooses.len() < 2
        || reveal.tag.as_str() != chosen_tag
        || !for_each_moves_tag_to_hand(move_chosen, chosen_tag)
        || rest.tag.as_str() != look_at_top.tag.as_str()
        || rest
            .keep_tagged
            .as_ref()
            .is_none_or(|tag| tag.as_str() != chosen_tag)
    {
        return None;
    }

    let mut labels = Vec::new();
    for choose in chooses {
        if choose.tag.as_str() != chosen_tag
            || choose.chooser != first.chooser
            || choose.count != ChoiceCount::up_to(1)
            || choose_primary_zone(choose) != Some(Zone::Library)
            || choose.is_search
            || !choose_references_tag(choose, &look_at_top.tag)
        {
            return None;
        }
        labels.push(with_indefinite_article(&structural_revealed_choice_label(
            choose,
        )?));
    }

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&first.chooser);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let may_prefix = if first.chooser == PlayerFilter::You {
        "You may".to_string()
    } else {
        format!(
            "{} may",
            capitalize_first(&describe_player_filter(&first.chooser))
        )
    };
    let order_text = match rest.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in any order".to_string()
        }
    };

    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library{count_where_clause}. {may_prefix} reveal {} from among them and put the revealed cards into {hand} hand. Put the rest on the bottom of {owner} library{order_text}",
        labels.join(" and/or ")
    ))
}

pub(super) fn sacrificed_count_noun_for_reveal_split(filter: &ObjectFilter) -> Option<String> {
    if filter.card_types == [CardType::Land]
        && filter.subtypes.is_empty()
        && filter.supertypes.is_empty()
    {
        return Some("lands".to_string());
    }

    let mut bare = filter.clone();
    bare.zone = None;
    if bare.controller == Some(PlayerFilter::You) {
        bare.controller = None;
    }
    let description = bare.description();
    let description = description
        .strip_suffix(" on the battlefield")
        .unwrap_or(&description);
    Some(pluralize_noun_phrase(strip_leading_article(description)))
}

pub(super) fn any_number_revealed_choice_text(
    choose: &crate::effects::ChooseObjectsEffect,
) -> String {
    let mut base = choose.filter.clone();
    base.zone = None;
    base.tagged_constraints.retain(|constraint| {
        !matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        )
    });

    if base.card_types.len() == 2
        && base.card_types.contains(&CardType::Artifact)
        && base.card_types.contains(&CardType::Land)
        && base.all_card_types.is_empty()
        && base.subtypes.is_empty()
        && base.any_of.is_empty()
    {
        return "artifact and/or land cards".to_string();
    }

    let description = base.description();
    let mut card_desc = description
        .split(" in ")
        .next()
        .unwrap_or(description.as_str())
        .trim()
        .to_string();
    card_desc = normalize_looked_card_filter_description(&base, &card_desc);
    if !card_desc.contains(" card") {
        card_desc = format!("{card_desc} card");
    }
    pluralize_noun_phrase(strip_leading_article(&card_desc))
}

pub(super) fn put_iterated_onto_battlefield(
    effect: &Effect,
    tapped: bool,
    controller: &PlayerFilter,
) -> bool {
    let effect = unwrap_tag_wrapped_effect(effect);
    matches!(
        effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>(),
        Some(put)
            if put.tapped == tapped
                && &put.controller == controller
                && matches!(put.target, ChooseSpec::Iterated)
    )
}

pub(super) fn chosen_land_nonland_battlefield_split(
    for_each: &crate::effects::ForEachTaggedEffect,
    chosen_tag: &crate::TagKey,
    controller: &PlayerFilter,
) -> bool {
    if for_each.tag != *chosen_tag || for_each.effects.len() != 1 {
        return false;
    }
    let Some(conditional) = for_each.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
    else {
        return false;
    };
    let land_condition = matches!(
        &conditional.condition,
        crate::effect::Condition::TaggedObjectMatches(tag, filter)
            if tag.as_str() == "__it__" && filter.card_types == [CardType::Land]
    );
    land_condition
        && conditional.if_true.len() == 1
        && conditional.if_false.len() == 1
        && put_iterated_onto_battlefield(&conditional.if_true[0], true, controller)
        && put_iterated_onto_battlefield(&conditional.if_false[0], false, controller)
}

pub(super) fn describe_sacrifice_reveal_top_choose_land_nonland_split_rest_bottom(
    sacrifice_choose: &crate::effects::ChooseObjectsEffect,
    sacrifice_with_id: &crate::effects::WithIdEffect,
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    split: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    let sacrifice = sacrifice_view(&sacrifice_with_id.effect)?;
    describe_choose_then_sacrifice(sacrifice_choose, sacrifice)?;
    if look_at_top.player != PlayerFilter::You
        || !look_at_top.reveal
        || !value_prefers_where_x(&look_at_top.count)
        || !is_effect_count_reference(&look_at_top.count, Some(sacrifice_with_id.id))
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.count.is_any_number()
        || choose.is_search
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == look_at_top.tag
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject
                )
        })
        || !chosen_land_nonland_battlefield_split(split, &choose.tag, &PlayerFilter::You)
        || rest.tag != look_at_top.tag
        || rest.keep_tagged.as_ref() != Some(&choose.tag)
        || rest.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
        || rest.player != PlayerFilter::You
    {
        return None;
    }

    let sacrificed = sacrificed_count_noun_for_reveal_split(&sacrifice_choose.filter)?;
    let matching = any_number_revealed_choice_text(choose);
    Some(format!(
        "Sacrifice any number of {sacrificed}. Reveal the top X cards of your library, where X is the number of {sacrificed} sacrificed this way. Choose any number of {matching} revealed this way. Put all nonland cards chosen this way onto the battlefield, then put all land cards chosen this way onto the battlefield tapped, then put the rest on the bottom of your library in a random order"
    ))
}

pub(super) fn describe_look_at_top_then_cast_matching_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    cast: &crate::effects::CastTaggedEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if cast.tag.as_str() != choose.tag.as_str()
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || rest.tag.as_str() != look_at_top.tag.as_str()
        || rest
            .keep_tagged
            .as_ref()
            .is_none_or(|tag| tag.as_str() != choose.tag.as_str())
    {
        return None;
    }

    let mut chosen = describe_choose_filter_from_looked_cards(look_at_top, choose)?;
    if let Some(rest) = chosen.strip_prefix("a nonland card with ") {
        let rest = rest.strip_suffix(" card").unwrap_or(rest);
        chosen = format!("a spell with {rest}");
    } else if chosen.starts_with("a nonland ") && chosen.contains(" with ") {
        let suffix = chosen.split_once(" with ")?.1;
        let suffix = suffix.strip_suffix(" card").unwrap_or(suffix);
        chosen = format!("a spell with {suffix}");
    } else if chosen == "a nonland card" {
        chosen = "a spell".to_string();
    }
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let may_prefix = if choose.chooser == PlayerFilter::You {
        "You may".to_string()
    } else {
        format!(
            "{} may",
            capitalize_first(&describe_player_filter(&choose.chooser))
        )
    };
    let order_text = match rest.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
            " in any order".to_string()
        }
    };

    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library{count_where_clause}. {may_prefix} cast {chosen} from among them without paying its mana cost. Put the rest on the bottom of {owner} library{order_text}"
    ))
}

pub(super) fn move_tag_to_zone(effect: &Effect, tag: &crate::TagKey, zone: Zone) -> bool {
    fn unwrap(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap(tagged.effect.as_ref());
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap(with_id.effect.as_ref());
        }
        effect
    }

    unwrap(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .is_some_and(|move_to_zone| {
            move_to_zone.zone == zone
                && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if *found == *tag)
        })
}

pub(super) fn describe_look_at_top_reveal_matching_bargain_battlefield_else_hand(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: &crate::effects::RevealTaggedEffect,
    conditional: &crate::effects::ConditionalEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    if reveal.tag != choose.tag
        || shuffle.player != look_at_top.player
        || !matches!(
            &conditional.condition,
            Condition::ThisSpellPaidLabel(label)
                if label.display_label().eq_ignore_ascii_case("bargain")
        )
        || conditional.if_true.len() != 1
        || conditional.if_false.len() != 1
        || !move_tag_to_zone(&conditional.if_true[0], &choose.tag, Zone::Battlefield)
        || !move_tag_to_zone(&conditional.if_false[0], &choose.tag, Zone::Hand)
    {
        return None;
    }
    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == look_at_top.tag
    });
    if !references_looked || choose.count.min != 0 {
        return None;
    }

    let mut base_filter = choose.filter.clone();
    base_filter.zone = None;
    base_filter.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == look_at_top.tag)
    });
    let mut filter_text = base_filter.description();
    if !filter_text.contains("card") {
        filter_text.push_str(" cards");
    }
    let choice_text = if let Some(max) = choose.count.max {
        let max_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
        format!("up to {max_text} {filter_text}")
    } else {
        format!("any number of {filter_text}")
    };

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library{count_where_clause}. You may reveal {choice_text} from among them. If this spell was bargained, put the revealed cards onto the battlefield. Otherwise, put the revealed cards into {hand} hand. Shuffle {owner} library"
    ))
}

pub(crate) fn describe_look_at_top_then_put_one_hand_other_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_hand: &crate::effects::MoveToZoneEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if !move_to_hand_uses_chosen_tag(move_to_hand, choose.tag.as_str())
        || rest.tag.as_str() != look_at_top.tag.as_str()
        || rest
            .keep_tagged
            .as_ref()
            .is_none_or(|tag| tag.as_str() != choose.tag.as_str())
    {
        return None;
    }
    let chosen = describe_choose_filter_from_looked_cards(look_at_top, choose)?;
    if chosen != "a card" {
        return None;
    }

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    if owner != "your" || hand != "your" {
        return None;
    }
    let (count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
    // A two-card look leaves a single remainder card ("the other", no order
    // phrase); larger looks follow the oracle's "the rest ... in any order".
    let (remainder, order_text) = if look_at_top.count == Value::Fixed(2) {
        ("the other", "")
    } else {
        let order_text = match rest.order {
            crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
            crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
        };
        ("the rest", order_text)
    };

    Some(format!(
        "Look at the top {count_text} {noun} of your library. Put one of them into your hand and {remainder} on the bottom of your library{order_text}"
    ))
}

pub(crate) fn describe_look_at_top_then_put_chosen_hand_rest_bottom_from_for_each(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_hand: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
        || !for_each_moves_unselected_to_zone(
            rest,
            look_at_top.tag.as_str(),
            choose.tag.as_str(),
            Zone::Library,
        )
    {
        return None;
    }
    if choose_primary_zone(choose) != Some(Zone::Library) || choose.is_search {
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

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let chosen = match exact_count {
        1 => "one of them".to_string(),
        n => format!(
            "{} of them",
            small_number_word(n as u32).unwrap_or_else(|| n.to_string())
        ),
    };
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library{count_where_clause}. Put {chosen} into {hand} hand and the rest on the bottom of {owner} library"
    ))
}

pub(super) fn describe_look_top_cards_sentence(
    opener: &str,
    count_text: &str,
    noun: &str,
    owner: &str,
) -> String {
    if count_text.starts_with("twice ") {
        return format!("{opener} {count_text} {noun} from the top of {owner} library");
    }
    format!("{opener} the top {count_text} {noun} of {owner} library")
}

pub(crate) fn describe_look_at_top_then_put_into_hand_rest_graveyard(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_top: Option<&crate::effects::RevealTaggedEffect>,
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: Option<&crate::effects::ForEachTaggedEffect>,
    move_to_hand: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if let Some(reveal_top) = reveal_top
        && reveal_top.tag.as_str() != look_at_top.tag.as_str()
    {
        return None;
    }
    if let Some(reveal) = reveal
        && !for_each_reveals_tag(reveal, choose.tag.as_str())
    {
        return None;
    }
    if !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
        || !for_each_moves_unselected_to_zone(
            rest,
            look_at_top.tag.as_str(),
            choose.tag.as_str(),
            Zone::Graveyard,
        )
    {
        return None;
    }

    let mut base_filter = choose.filter.clone();
    base_filter.zone = None;
    base_filter.tagged_constraints.retain(|constraint| {
        !(matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str())
    });
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
    let opener = if reveal_top.is_some() || look_at_top.reveal {
        "Reveal"
    } else {
        "Look at"
    };
    let look_sentence = describe_look_top_cards_sentence(opener, &count_text, noun, &owner);
    let exact_count = match (choose.count.min, choose.count.max) {
        (n, Some(max)) if n == max && n > 0 => Some(n),
        _ => None,
    };
    if base_filter == crate::filter::ObjectFilter::default()
        && let Some(n) = exact_count
        && reveal.is_none()
    {
        if n == 1 && look_at_top.count == Value::Fixed(2) {
            return Some(format!(
                "{look_sentence}. Put one of those cards into {hand} hand and the other into {owner} graveyard"
            ));
        }
        let chosen = match n {
            1 => "one of those cards".to_string(),
            n => format!(
                "{} of them",
                small_number_word(n as u32).unwrap_or_else(|| n.to_string())
            ),
        };
        return Some(format!(
            "{look_sentence}. Put {chosen} into {hand} hand and the rest into {owner} graveyard"
        ));
    }
    let chosen = if base_filter == crate::filter::ObjectFilter::default() {
        match exact_count {
            Some(1) => "one of those cards".to_string(),
            Some(n) => format!(
                "{} of them",
                small_number_word(n as u32).unwrap_or_else(|| n.to_string())
            ),
            None => describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?,
        }
    } else {
        describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?
    };
    let exact_choice = exact_count.is_some();
    let dynamic_exact_choice = choose.count.dynamic_x
        && !choose.count.up_to_x
        && choose.search_mode != SearchSelectionMode::Optional;
    let choice_says_up_to = choose.count.min == 0 && choose.count.max.is_some_and(|max| max > 1);
    let actor_prefix = if exact_choice || dynamic_exact_choice || choice_says_up_to {
        String::new()
    } else if choose.chooser == PlayerFilter::You {
        "You may ".to_string()
    } else {
        format!(
            "{} may ",
            capitalize_first(&describe_player_filter(&choose.chooser))
        )
    };
    let choice_clause = if reveal.is_some() {
        format!("{actor_prefix}reveal {chosen} from among them and put it into {hand} hand")
    } else if base_filter == crate::filter::ObjectFilter::default() && exact_choice {
        format!("{actor_prefix}put {chosen} into {hand} hand")
    } else {
        format!("{actor_prefix}put {chosen} from among them into {hand} hand")
    };

    if (choice_says_up_to || dynamic_exact_choice) && reveal.is_none() {
        return Some(format!(
            "{look_sentence}. {} and the rest into {owner} graveyard",
            capitalize_first(&choice_clause)
        ));
    }

    Some(format!(
        "{look_sentence}. {choice_clause}. Put the rest into {owner} graveyard"
    ))
}

pub(super) fn describe_look_at_top_then_put_matching_to_zone_rest_hand(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_chosen: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if choose_primary_zone(choose) != Some(Zone::Library) || choose.is_search {
        return None;
    }
    let rest_zone = if for_each_moves_unselected_to_zone(
        rest,
        look_at_top.tag.as_str(),
        choose.tag.as_str(),
        Zone::Hand,
    ) {
        Zone::Hand
    } else if for_each_moves_unselected_to_zone(
        rest,
        look_at_top.tag.as_str(),
        choose.tag.as_str(),
        Zone::Graveyard,
    ) {
        Zone::Graveyard
    } else {
        return None;
    };
    let (zone, tapped) = for_each_moves_tag_to_public_zone(move_chosen, choose.tag.as_str())?;
    if zone == Zone::Hand {
        return None;
    }

    let matching = if choose.count.is_any_number() {
        format!(
            "any number of {}",
            describe_any_number_filter_from_looked_cards(look_at_top, choose)?
        )
    } else {
        describe_counted_choose_filter_from_looked_cards(look_at_top, choose)?
    };
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let (count_text, noun, count_where_clause) =
        describe_top_count_noun_and_where_clause(&look_at_top.count);
    let opener = if look_at_top.reveal {
        "Reveal"
    } else {
        "Look at"
    };
    let destination = match zone {
        Zone::Battlefield => {
            let tapped_suffix = if tapped { " tapped" } else { "" };
            format!("onto the battlefield{tapped_suffix}")
        }
        _ => return None,
    };
    let put_prefix = if rest_zone == Zone::Graveyard && choose.count.is_any_number() {
        if choose.chooser == PlayerFilter::You {
            "You may put".to_string()
        } else {
            format!(
                "{} may put",
                capitalize_first(&describe_player_filter(&choose.chooser))
            )
        }
    } else {
        "Put".to_string()
    };
    let rest_clause = match rest_zone {
        Zone::Hand => format!(" and the rest into {hand} hand"),
        Zone::Graveyard => format!(". Put the rest into {owner} graveyard"),
        _ => return None,
    };

    Some(format!(
        "{opener} the top {count_text} {noun} of {owner} library{count_where_clause}. {put_prefix} {matching} from among them {destination}{rest_clause}"
    ))
}

pub(crate) fn describe_if_didnt_put_card_into_hand_this_way(
    chooser: &PlayerFilter,
    move_to_hand_id: crate::effect::EffectId,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if if_effect.condition != move_to_hand_id
        || if_effect.predicate != EffectPredicate::DidNotHappen
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let then_text = describe_effect_list(&if_effect.then);
    if then_text.is_empty() {
        return None;
    }

    let condition = if *chooser == PlayerFilter::You {
        "If you didn't put a card into your hand this way".to_string()
    } else {
        let who = describe_player_filter(chooser);
        let hand = describe_possessive_player_filter(chooser);
        format!("If {who} didn't put a card into {hand} hand this way")
    };

    Some(format!("{condition}, {then_text}"))
}

pub(crate) fn describe_look_at_top_then_reveal_put_into_hand_rest_bottom_then_if_not_into_hand(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: &crate::effects::ForEachTaggedEffect,
    move_to_hand_with_id: &crate::effects::WithIdEffect,
    move_to_hand: &crate::effects::ForEachTaggedEffect,
    rest: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    let base = describe_look_at_top_then_reveal_put_into_hand_rest_bottom(
        look_at_top,
        choose,
        Some(reveal),
        move_to_hand,
        rest,
    )?;
    let follow_up = describe_if_didnt_put_card_into_hand_this_way(
        &choose.chooser,
        move_to_hand_with_id.id,
        if_effect,
    )?;
    Some(format!("{base}. {follow_up}"))
}

pub(crate) fn for_each_moves_matching_to_zone_else_graveyard<'a>(
    for_each: &'a crate::effects::ForEachTaggedEffect,
    looked_tag: &str,
) -> Option<(&'a crate::filter::ObjectFilter, Zone)> {
    fn move_to_zone_for_compaction(effect: &Effect) -> Option<&crate::effects::MoveToZoneEffect> {
        if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            return Some(move_to_zone);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return tagged
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>();
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return with_id
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>();
        }
        None
    }

    fn uses_iterated_or_looked_tag(spec: &ChooseSpec, looked_tag: &str) -> bool {
        matches!(spec, ChooseSpec::Iterated)
            || matches!(spec, ChooseSpec::Tagged(tag) if tag.as_str() == looked_tag)
    }

    if for_each.tag.as_str() != looked_tag || for_each.effects.len() != 1 {
        return None;
    }
    let conditional = for_each.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.if_true.len() != 1 || conditional.if_false.len() != 1 {
        return None;
    }
    let move_to_hand = move_to_zone_for_compaction(&conditional.if_true[0])?;
    let move_to_graveyard = move_to_zone_for_compaction(&conditional.if_false[0])?;
    if move_to_graveyard.zone != Zone::Graveyard
        || !uses_iterated_or_looked_tag(&move_to_hand.target, looked_tag)
        || !uses_iterated_or_looked_tag(&move_to_graveyard.target, looked_tag)
    {
        return None;
    }
    let filter = match &conditional.condition {
        crate::effect::Condition::TaggedObjectMatches(tag, filter)
            if tag.as_str() == "__it__" || tag.as_str() == looked_tag =>
        {
            filter
        }
        crate::effect::Condition::PlayerTaggedObjectMatches { tag, filter, .. }
            if tag.as_str() == "__it__" || tag.as_str() == looked_tag =>
        {
            filter
        }
        _ => return None,
    };
    Some((filter, move_to_hand.zone))
}

pub(crate) fn for_each_moves_matching_to_hand_else_graveyard<'a>(
    for_each: &'a crate::effects::ForEachTaggedEffect,
    looked_tag: &str,
) -> Option<&'a crate::filter::ObjectFilter> {
    let (filter, zone) = for_each_moves_matching_to_zone_else_graveyard(for_each, looked_tag)?;
    if zone != Zone::Hand {
        return None;
    }
    Some(filter)
}

pub(super) fn filter_is_only_same_name_as_tag(
    filter: &crate::filter::ObjectFilter,
    tag: &str,
) -> bool {
    let mut base = filter.clone();
    let before = base.tagged_constraints.len();
    base.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::SameNameAsTagged
            ))
    });
    before != base.tagged_constraints.len() && base == crate::filter::ObjectFilter::default()
}

pub(crate) fn describe_look_at_top_then_reveal_put_matching_into_hand_rest_graveyard(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_tagged: &crate::effects::RevealTaggedEffect,
    distribute: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if reveal_tagged.tag.as_str() != look_at_top.tag.as_str() {
        return None;
    }
    let filter =
        for_each_moves_matching_to_hand_else_graveyard(distribute, look_at_top.tag.as_str())?;
    let owner = if look_at_top.player == PlayerFilter::IteratedPlayer {
        "their".to_string()
    } else {
        describe_possessive_player_filter(&look_at_top.player)
    };
    let (mut count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
    if look_at_top.player == PlayerFilter::IteratedPlayer {
        count_text = count_text.replace("that player controls", "they control");
    }
    if filter_is_only_same_name_as_tag(filter, "__chosen_name__") {
        return Some(format!(
            "Reveal the top {count_text} {noun} of {owner} library and put all of them with that name into {owner} hand. Put the rest into {owner} graveyard"
        ));
    }
    let matching =
        pluralize_noun_phrase(&describe_search_selection_with_cards(&filter.description()));

    Some(format!(
        "Reveal the top {count_text} {noun} of {owner} library. Put all {matching} revealed this way into {owner} hand and the rest into {owner} graveyard"
    ))
}

pub(crate) fn describe_choose_name_then_reveal_matching_hand_rest_graveyard(
    choose_name: &crate::effects::ChooseCardNameEffect,
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_tagged: &crate::effects::RevealTaggedEffect,
    distribute: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if choose_name.chooser != PlayerFilter::You {
        return None;
    }
    let filter =
        for_each_moves_matching_to_hand_else_graveyard(distribute, look_at_top.tag.as_str())?;
    if !filter_is_only_same_name_as_tag(filter, choose_name.tag.as_str()) {
        return None;
    }
    let choose_text = describe_effect(&Effect::new(choose_name.clone()));
    let choose_text = choose_text
        .strip_prefix("You choose ")
        .map(|selection| format!("Choose {selection}"))?;
    let reveal_text = describe_look_at_top_then_reveal_put_matching_into_hand_rest_graveyard(
        look_at_top,
        reveal_tagged,
        distribute,
    )?;
    Some(format!("{choose_text}. {reveal_text}"))
}

pub(crate) fn describe_look_at_top_then_reveal_put_matching_into_hand_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_tagged: &crate::effects::RevealTaggedEffect,
    tag_matching: &crate::effects::TagMatchingObjectsEffect,
    move_matching: &crate::effects::ForEachTaggedEffect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if reveal_tagged.tag.as_str() != look_at_top.tag.as_str()
        || tag_matching.tag.as_str() != move_matching.tag.as_str()
        || remainder.tag.as_str() != look_at_top.tag.as_str()
        || !remainder
            .keep_tagged
            .as_ref()
            .is_some_and(|tag| tag.as_str() == tag_matching.tag.as_str())
        || !for_each_moves_tag_to_hand(move_matching, tag_matching.tag.as_str())
        || tag_matching.zone != Some(Zone::Library)
    {
        return None;
    }

    let has_looked_constraint = tag_matching
        .filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            constraint.tag.as_str() == look_at_top.tag.as_str()
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject
                )
        });
    if !has_looked_constraint {
        return None;
    }

    let mut filter = tag_matching.filter.clone();
    filter.zone = None;
    filter.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == look_at_top.tag.as_str()
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            ))
    });

    let owner = if look_at_top.player == PlayerFilter::IteratedPlayer {
        "their".to_string()
    } else {
        describe_possessive_player_filter(&look_at_top.player)
    };
    let (mut count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
    if look_at_top.player == PlayerFilter::IteratedPlayer {
        count_text = count_text.replace("that player controls", "they control");
    }
    let matching =
        pluralize_noun_phrase(&describe_search_selection_with_cards(&filter.description()));
    let order_text = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };

    Some(format!(
        "Reveal the top {count_text} {noun} of {owner} library. Put all {matching} revealed this way into {owner} hand and the rest on the bottom of {owner} library{order_text}"
    ))
}

pub(crate) fn describe_look_at_top_then_reveal_put_matching_onto_battlefield_rest_graveyard(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    reveal_tagged: &crate::effects::RevealTaggedEffect,
    distribute: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if reveal_tagged.tag.as_str() != look_at_top.tag.as_str() {
        return None;
    }
    let (filter, zone) =
        for_each_moves_matching_to_zone_else_graveyard(distribute, look_at_top.tag.as_str())?;
    if zone != Zone::Battlefield {
        return None;
    }
    let owner = if look_at_top.player == PlayerFilter::IteratedPlayer {
        "their".to_string()
    } else {
        describe_possessive_player_filter(&look_at_top.player)
    };
    let (mut count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
    if look_at_top.player == PlayerFilter::IteratedPlayer {
        count_text = count_text.replace("that player controls", "they control");
    }
    let matching = pluralize_noun_phrase(&describe_revealed_selection_with_cards(
        &filter.description(),
    ));

    Some(format!(
        "Reveal the top {count_text} {noun} of {owner} library. Put all {matching} revealed this way onto the battlefield and the rest into {owner} graveyard"
    ))
}

pub(crate) fn describe_look_count_and_noun(count: &Value) -> (String, &'static str, bool) {
    if let Value::Fixed(n) = count
        && *n >= 0
    {
        let count_u32 = *n as u32;
        let text = small_number_word(count_u32).unwrap_or_else(|| n.to_string());
        let singular = *n == 1;
        return (text, if singular { "card" } else { "cards" }, singular);
    }
    if let Value::XTimes(factor) = count
        && *factor > 0
    {
        let text = if *factor == 1 {
            "X".to_string()
        } else if *factor == 2 {
            "twice X".to_string()
        } else {
            format!("{factor} times X")
        };
        return (text, "cards", false);
    }
    (describe_value(count), "cards", false)
}

pub(super) fn describe_look_at_top_choose_battlefield_rest_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    remainder: &crate::effects::PutTaggedRemainderOnLibraryBottomEffect,
) -> Option<String> {
    if look_at_top.reveal
        || look_at_top.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose.count.is_any_number()
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose_references_tag(choose, &look_at_top.tag)
        || remainder.tag != look_at_top.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != look_at_top.player
    {
        return None;
    }

    let Some((_, for_each)) = for_each_tagged_for_compaction(move_effect) else {
        return None;
    };
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }
    let move_to_zone = unwrap_basic_tag_wrappers(&for_each.effects[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }

    let (count_text, noun, singular_count) = describe_look_count_and_noun(&look_at_top.count);
    if singular_count {
        return None;
    }
    let owner = describe_possessive_player_filter(&look_at_top.player);
    let mut selection = describe_looked_battlefield_selection(choose)?;
    let put_prefix = if move_to_zone.enters_attacking
        && choose.count.min == 0
        && choose.count.max == Some(1)
        && let Some(rest) = selection.strip_prefix("up to one ")
    {
        selection = with_indefinite_article(rest);
        "You may put"
    } else {
        "Put"
    };
    let battlefield_suffix = describe_battlefield_entry_state_for_looked_move(move_to_zone);
    let order_text = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
    };

    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library. {put_prefix} {selection} from among them onto the battlefield{battlefield_suffix}. Put the rest on the bottom of {owner} library{order_text}"
    ))
}

pub(super) fn describe_battlefield_entry_state_for_looked_move(
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> &'static str {
    match (
        move_to_zone.enters_tapped,
        move_to_zone.enters_attacking,
        &move_to_zone.attack_target_mode,
    ) {
        (
            true,
            true,
            Some(crate::effects::MoveToZoneAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
                PlayerFilter::Defending,
            )),
        ) => " tapped and attacking that player",
        (
            false,
            true,
            Some(crate::effects::MoveToZoneAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
                PlayerFilter::Defending,
            )),
        ) => " attacking that player",
        (true, true, _) => " tapped and attacking",
        (false, true, _) => " attacking",
        (true, false, _) => " tapped",
        _ => "",
    }
}

pub(super) fn describe_looked_battlefield_selection(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.tagged_constraints.retain(|constraint| {
        constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    });

    let raw = filter.description();
    let raw = raw.split(" in ").next().unwrap_or(raw.as_str()).trim();
    let mut card_desc = normalize_looked_card_filter_description(&filter, raw);
    card_desc = normalize_battlefield_looked_card_description(&filter, &card_desc);
    if !card_desc.contains(" card") {
        if let Some((head, tail)) = card_desc.split_once(" with ") {
            card_desc = format!("{head} card with {tail}");
        } else {
            card_desc.push_str(" card");
        }
    }

    let plural = pluralize_noun_phrase(&card_desc);
    if choose.count.is_any_number() {
        return Some(format!("any number of {plural}"));
    }
    if choose.count.is_up_to_dynamic_x() {
        return Some(format!("up to X {plural}"));
    }
    if choose.count.is_dynamic_x() {
        return Some(format!("X {plural}"));
    }

    match (choose.count.min, choose.count.max) {
        (0, Some(1)) => Some(format!("up to one {card_desc}")),
        (1, Some(1)) => Some(with_indefinite_article(&card_desc)),
        (0, Some(max)) => {
            let max_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
            Some(format!("up to {max_text} {plural}"))
        }
        (min, Some(max)) if min == max => {
            let count_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
            Some(format!("{count_text} {plural}"))
        }
        _ => Some(format!("{} {plural}", describe_choice_count(&choose.count))),
    }
}

pub(super) fn normalize_battlefield_looked_card_description(
    filter: &ObjectFilter,
    card_desc: &str,
) -> String {
    let mut card_desc = card_desc.to_string();
    if filter.card_types == [CardType::Artifact, CardType::Creature]
        || (filter.any_of.len() == 2
            && filter
                .any_of
                .iter()
                .any(|branch| branch.card_types == [CardType::Artifact])
            && filter
                .any_of
                .iter()
                .any(|branch| branch.card_types == [CardType::Creature]))
    {
        if let Some(rest) = card_desc
            .strip_prefix("artifacts or creatures")
            .or_else(|| card_desc.strip_prefix("artifact or creature"))
        {
            card_desc = format!("artifact and/or creature card{rest}");
        }
    }

    if !card_desc.contains("mana value")
        && let Some(suffix) = common_looked_filter_mana_value_suffix(filter)
    {
        card_desc.push_str(&suffix);
    }
    card_desc
}

pub(super) fn common_looked_filter_mana_value_suffix(filter: &ObjectFilter) -> Option<String> {
    let mana_value = if let Some(mana_value) = filter.mana_value.as_ref() {
        mana_value
    } else {
        let first = filter.any_of.first()?.mana_value.as_ref()?;
        if !filter
            .any_of
            .iter()
            .all(|branch| branch.mana_value.as_ref() == Some(first))
        {
            return None;
        }
        first
    };

    match mana_value {
        crate::filter::Comparison::LessThanOrEqual(value) => {
            Some(format!(" with mana value {value} or less"))
        }
        crate::filter::Comparison::LessThanOrEqualExpr(value) => Some(format!(
            " with mana value {} or less",
            describe_value(value)
        )),
        _ => None,
    }
}

pub(super) fn describe_top_count_noun_and_where_clause(
    count: &Value,
) -> (String, &'static str, String) {
    match count {
        Value::SurfaceHinted { value, .. } if value_prefers_where_x(count) => (
            "X".to_string(),
            "cards",
            format!(", where X is {}", describe_value(value)),
        ),
        Value::SourcePower => (
            "X".to_string(),
            "cards",
            ", where X is its power".to_string(),
        ),
        Value::GreatestManaValue(filter)
            if greatest_commander_mana_value_owned_by(filter, PlayerFilter::You) =>
        {
            (
                "X".to_string(),
                "cards",
                ", where X is the greatest mana value of a commander you own on the battlefield or in the command zone"
                    .to_string(),
            )
        }
        _ => {
            let (count_text, noun, _) = describe_look_count_and_noun(count);
            (count_text, noun, String::new())
        }
    }
}

pub(crate) fn describe_draw_then_discard(
    draw: &crate::effects::DrawCardsEffect,
    discard: &crate::effects::DiscardEffect,
) -> Option<String> {
    if draw.player != discard.player {
        return None;
    }
    if draw.player == PlayerFilter::You {
        let mut text = format!(
            "Draw {}, then discard {}",
            describe_card_count(&draw.count),
            describe_discard_count(&discard.count, discard.card_filter.as_ref())
        );
        if discard.random {
            text.push_str(" at random");
        }
        return Some(text);
    }
    let player = describe_player_filter(&draw.player);
    let mut text = format!(
        "{player} {} {}, then {} {}",
        player_verb(&player, "draw", "draws"),
        describe_card_count(&draw.count),
        player_verb(&player, "discard", "discards"),
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    );
    if discard.random {
        text.push_str(" at random");
    }
    Some(text)
}

pub(super) fn shared_draw_partner(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Specific(_)
        | PlayerFilter::TaggedPlayer(_)
        | PlayerFilter::Active
        | PlayerFilter::DamagedPlayer
        | PlayerFilter::IteratedPlayer => "that player".to_string(),
        _ => describe_player_filter(filter),
    }
}

pub(super) fn draw_cards_view(effect: &Effect) -> Option<&crate::effects::DrawCardsEffect> {
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        return Some(draw);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return draw_cards_view(&with_id.effect);
    }
    None
}

pub(crate) fn describe_shared_draw(
    first: &crate::effects::DrawCardsEffect,
    second: &crate::effects::DrawCardsEffect,
) -> Option<String> {
    if first.player == second.player {
        return None;
    }
    if first.count != second.count
        && !(is_effect_count_reference(&first.count, None)
            && is_effect_count_reference(&second.count, None))
    {
        return None;
    }
    if first.player != PlayerFilter::You {
        return None;
    }

    let partner = shared_draw_partner(&second.player);
    Some(format!(
        "you and {partner} each draw {}",
        describe_card_count(&first.count)
    ))
}

/// Render a Draw + GainLife pair as "Draw a card, then you gain life equal to
/// the number of [filter]" when the gain amount is `Value::Count`.  This
/// matches the oracle phrasing used by Union of the Third Path and similar
/// cards where the life gain scales with a zone-based count.
///
/// Only fires for `Value::Count` amounts so that fixed-amount cards like
/// "Draw a card. You gain 3 life." keep their existing separate-sentence
/// rendering (which downstream normalisation already merges with "and").
pub(crate) fn describe_draw_then_gain_life(
    draw: &crate::effects::DrawCardsEffect,
    gain: &crate::effects::GainLifeEffect,
) -> Option<String> {
    // Only combine when the drawing player is "you".
    if !matches!(draw.player, PlayerFilter::You) {
        return None;
    }
    // Only combine when the gaining player is "you".
    if !matches!(gain.player, ChooseSpec::Player(PlayerFilter::You)) {
        return None;
    }
    if let (Value::PowerOf(power_spec), Value::ToughnessOf(toughness_spec)) =
        (draw.count.unhinted(), gain.amount.unhinted())
        && toughness_life_basis_matches_power_draw(power_spec, toughness_spec)
    {
        return Some(format!(
            "You draw cards equal to {}, then you gain life equal to its toughness",
            describe_power_card_count_basis(power_spec)
        ));
    }
    // Only combine when the gain amount is a dynamic count—oracle
    // cards with fixed gain amounts use "and" rather than "then".
    let Value::Count(filter) = gain.amount.unhinted() else {
        return None;
    };
    let draw_clause = format!("Draw {}", describe_card_count(&draw.count));
    let filter_text = pluralize_noun_phrase(&describe_for_each_count_filter(filter));
    Some(format!(
        "{draw_clause}, then you gain life equal to the number of {filter_text}"
    ))
}

pub(super) fn toughness_life_basis_matches_power_draw(
    power_spec: &ChooseSpec,
    toughness_spec: &ChooseSpec,
) -> bool {
    if power_spec == toughness_spec {
        return true;
    }
    matches!(
        (power_spec.base(), toughness_spec.base()),
        (ChooseSpec::Tagged(tag), ChooseSpec::Source)
            if tag.as_str() == "__it__" || tag.as_str().starts_with("sacrifice_cost_")
    )
}

pub(crate) fn describe_draw_then_lose_life(
    draw: &crate::effects::DrawCardsEffect,
    lose: &crate::effects::LoseLifeEffect,
) -> Option<String> {
    if !matches!(draw.player, PlayerFilter::You)
        || !matches!(lose.player, ChooseSpec::Player(PlayerFilter::You))
        || draw.count != lose.amount
    {
        return None;
    }
    if draw.count == Value::X {
        return Some("You draw X cards and you lose X life".to_string());
    }
    if let Value::Fixed(amount) = &draw.count {
        return Some(format!(
            "You draw {} and lose {amount} life",
            describe_card_count(&draw.count)
        ));
    }
    let where_x =
        describe_where_x_basis(&draw.count)?.replace("that player controls", "they control");
    Some(format!(
        "You draw X cards and you lose X life, where X is {where_x}"
    ))
}

pub(super) fn describe_you_and_attacking_player_draw_and_lose(
    draw_you: &crate::effects::DrawCardsEffect,
    draw_attacking: &crate::effects::DrawCardsEffect,
    lose_you: &crate::effects::LoseLifeEffect,
    lose_attacking: &crate::effects::LoseLifeEffect,
) -> Option<String> {
    if draw_you.player != PlayerFilter::You
        || draw_attacking.player != PlayerFilter::Attacking
        || lose_you.player != ChooseSpec::Player(PlayerFilter::You)
        || lose_attacking.player != ChooseSpec::Player(PlayerFilter::Attacking)
        || draw_you.count != draw_attacking.count
        || lose_you.amount != lose_attacking.amount
    {
        return None;
    }

    Some(format!(
        "You and the attacking player each draw {} and lose {}",
        describe_card_count(&draw_you.count),
        describe_life_amount_phrase(&lose_you.amount)
    ))
}

pub(super) fn describe_target_player_draw_then_lose_life(
    draw: &crate::effects::DrawCardsEffect,
    target_only: &crate::effects::TargetOnlyEffect,
    lose: &crate::effects::LoseLifeEffect,
) -> Option<String> {
    if draw.player != PlayerFilter::target_player()
        || target_only.target != ChooseSpec::target_player()
        || lose.player != ChooseSpec::Player(PlayerFilter::target_player())
        || draw.count != lose.amount
    {
        return None;
    }
    if draw.count == Value::X {
        return Some("Target player draws X cards and loses X life".to_string());
    }
    if let Value::Fixed(amount) = &draw.count {
        return Some(format!(
            "Target player draws {} and loses {amount} life",
            describe_card_count(&draw.count)
        ));
    }
    let where_x =
        describe_where_x_basis(&draw.count)?.replace("that player controls", "they control");
    Some(format!(
        "Target player draws X cards and loses X life, where X is {where_x}"
    ))
}

pub(super) fn describe_target_player_lose_then_you_gain_life(
    target_only: &crate::effects::TargetOnlyEffect,
    lose: &crate::effects::LoseLifeEffect,
    gain: &crate::effects::GainLifeEffect,
) -> Option<String> {
    if target_only.target != ChooseSpec::target_player()
        || lose.player != ChooseSpec::Player(PlayerFilter::target_player())
        || gain.player != ChooseSpec::Player(PlayerFilter::You)
        || lose.amount != gain.amount
    {
        return None;
    }
    if lose.amount == Value::X {
        return Some("Target player loses X life and you gain X life".to_string());
    }
    let where_x = describe_where_x_basis(&lose.amount)?;
    Some(format!(
        "Target player loses X life and you gain X life, where X is {where_x}"
    ))
}

pub(super) fn describe_damaged_player_reveal_choose_graveyard(
    effects: &[&Effect],
) -> Option<String> {
    let effects = if effects.first().is_some_and(|effect| {
        effect
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    }) {
        &effects[1..]
    } else {
        effects
    };
    let [look_effect, choose_effect, move_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if !look.reveal || look.player != PlayerFilter::DamagedPlayer {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You || !choose.count.is_single() {
        return None;
    }
    let chooses_revealed_card = choose.filter.owner == Some(PlayerFilter::DamagedPlayer)
        && choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == look.tag
        });
    if !chooses_revealed_card {
        return None;
    }
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !matches!(move_to_zone.target, ChooseSpec::Iterated)
        || move_to_zone.zone != Zone::Graveyard
        || move_to_zone.enters_tapped
    {
        return None;
    }

    let (count_text, noun, _) = describe_look_count_and_noun(&look.count);
    Some(format!(
        "The damaged player reveals the top {count_text} {noun} of the damaged player's library. You choose one of those cards and put it into the damaged player's graveyard"
    ))
}

pub(super) fn describe_target_opponent_create_tokens_with_count(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, create_effect] = effects else {
        return None;
    };
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !matches!(
        target_only.target.base(),
        ChooseSpec::Player(PlayerFilter::Opponent)
    ) {
        return None;
    }
    let create = created_token_effect(create_effect)?;
    let Value::SurfaceHinted { value, .. } = &create.count else {
        return None;
    };
    let Value::Count(filter) = value.unhinted() else {
        return None;
    };
    if filter.controller != Some(PlayerFilter::target_opponent()) {
        return None;
    }

    let token_blueprint = describe_token_blueprint(&create.token);
    let token_phrase = pluralize_token_phrase(&token_blueprint);
    let (token_main, token_ability) = split_token_ability_sentence(&token_phrase);
    let mut text = format!(
        "Choose target opponent. {}",
        describe_create_token_action(&format!("X {token_main}"), &create.controller)
    );
    if create.enters_tapped && create.enters_attacking {
        text.push_str(" that are tapped and attacking");
    } else {
        if create.enters_tapped {
            text.push_str(", tapped");
        }
        if create.enters_attacking {
            text.push_str(", attacking");
        }
    }

    let mut count_filter = filter.clone();
    count_filter.controller = None;
    text.push_str(&format!(
        ", where X is the number of {} that player controls",
        pluralize_noun_phrase(&describe_for_each_count_filter(&count_filter))
    ));
    Some(append_token_ability_sentence(text, token_ability))
}

pub(super) fn describe_draw_lose_life_get_energy(
    draw: &crate::effects::DrawCardsEffect,
    lose: &crate::effects::LoseLifeEffect,
    energy: &crate::effects::EnergyCountersEffect,
) -> Option<String> {
    if !matches!(draw.player, PlayerFilter::You)
        || !matches!(lose.player, ChooseSpec::Player(PlayerFilter::You))
        || energy.player != PlayerFilter::You
    {
        return None;
    }
    let energy_text = match &energy.count {
        Value::Fixed(amount) if *amount > 0 => repeated_energy_symbols(*amount as usize),
        _ => return None,
    };
    Some(format!(
        "You draw {}, lose {} life, and get {energy_text}",
        describe_card_count(&draw.count),
        describe_value(&lose.amount)
    ))
}

pub(crate) fn describe_mill_then_may_return(
    mill: &crate::effects::MillEffect,
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if may.effects.len() != 1 {
        return None;
    }
    let return_effect = may.effects.first()?;
    let is_return_to_hand = return_effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some()
        || return_effect
            .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
            .is_some();
    if !is_return_to_hand {
        return None;
    }

    let decider = may.decider.as_ref().unwrap_or(&mill.player);
    if decider != &mill.player {
        return None;
    }

    let player = describe_player_filter(&mill.player);
    let mill_clause = format!(
        "{player} {} {}",
        player_verb(&player, "mill", "mills"),
        describe_mill_count_for_player(&mill.count, &mill.player)
    );
    let return_clause = lowercase_first(&describe_effect(return_effect));
    Some(format!("{mill_clause}, then {player} may {return_clause}"))
}

pub(super) fn describe_mill_count_for_player(count: &Value, player: &PlayerFilter) -> String {
    fn half_library_count(count: &Value) -> Option<(&PlayerFilter, bool)> {
        let Value::HalfRoundedDown(inner) = count else {
            return None;
        };
        match inner.as_ref() {
            Value::CardsInLibrary(filter) => Some((filter, false)),
            Value::Add(left, right) => match (left.as_ref(), right.as_ref()) {
                (Value::CardsInLibrary(filter), Value::Fixed(1))
                | (Value::Fixed(1), Value::CardsInLibrary(filter)) => Some((filter, true)),
                _ => None,
            },
            _ => None,
        }
    }

    if let Some((library_player, rounded_up)) = half_library_count(count)
        && library_player == player
    {
        let possessive = if matches!(player, PlayerFilter::You) {
            "your"
        } else {
            "their"
        };
        let rounding = if rounded_up { "up" } else { "down" };
        return format!("half {possessive} library, rounded {rounding}");
    }

    describe_card_count(count)
}

pub(super) fn describe_choose_filter_from_tagged_cards(
    choose: &crate::effects::ChooseObjectsEffect,
    source_tag: &str,
) -> Option<String> {
    if !matches!(
        choose_primary_zone(choose),
        Some(Zone::Library | Zone::Graveyard)
    ) || choose.is_search
        || choose.count.min > 1
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || choose.count.random
    {
        return None;
    }
    let references_source = choose.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == source_tag
    });
    if !references_source {
        return None;
    }

    let mut base_filter = choose.filter.clone();
    base_filter.zone = None;
    base_filter.tagged_constraints.retain(|constraint| {
        !(matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == source_tag)
    });
    if base_filter == ObjectFilter::default() {
        return Some("a card".to_string());
    }

    let filter_text = base_filter.description();
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    card_desc = strip_leading_article(&card_desc).to_string();
    if let Some(rest) = card_desc.strip_prefix("card ") {
        card_desc = format!("{rest} card");
    }
    if !card_desc.contains(" card") {
        card_desc = format!("{card_desc} card");
    }
    Some(with_indefinite_article(&card_desc))
}

pub(super) fn describe_tagged_mill_clause(mill: &crate::effects::MillEffect) -> String {
    if matches!(mill.player, PlayerFilter::You) {
        format!(
            "Mill {}",
            describe_mill_count_for_player(&mill.count, &mill.player)
        )
    } else {
        let player = describe_player_filter(&mill.player);
        format!(
            "{player} {} {}",
            player_verb(&player, "mill", "mills"),
            describe_mill_count_for_player(&mill.count, &mill.player)
        )
    }
}

pub(crate) fn describe_tagged_mill_then_put_milled_card_into_hand(
    tagged_mill: &crate::effects::TaggedEffect,
    mill: &crate::effects::MillEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_hand: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if choose.chooser != mill.player
        || !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
    {
        return None;
    }
    let chosen = describe_choose_filter_from_tagged_cards(choose, tagged_mill.tag.as_str())?;
    let mill_clause = describe_tagged_mill_clause(mill);
    let hand = describe_possessive_player_filter(&choose.chooser);
    let may = if choose.count.min == 0 { " may" } else { "" };
    Some(format!(
        "{mill_clause}. You{may} put {chosen} from among the cards milled this way into {hand} hand"
    ))
}

pub(crate) fn describe_tagged_mill_then_put_milled_card_into_hand_with_fallback(
    tagged_mill: &crate::effects::TaggedEffect,
    mill: &crate::effects::MillEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_hand_with_id: &crate::effects::WithIdEffect,
    move_to_hand: &crate::effects::ForEachTaggedEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if mill.player != PlayerFilter::You
        || choose.chooser != PlayerFilter::You
        || !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
        || if_effect.condition != move_to_hand_with_id.id
        || if_effect.predicate != EffectPredicate::DidNotHappen
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let chosen = describe_choose_filter_from_tagged_cards(choose, tagged_mill.tag.as_str())?;
    let action = match (choose.count.min, choose.count.max) {
        (1, Some(1)) => format!("Put {chosen} from among the milled cards into your hand"),
        (0, Some(1)) => format!("You may put {chosen} from among the milled cards into your hand"),
        _ => return None,
    };
    let condition = if choose.count.min == 0 {
        "If you don't"
    } else {
        "If you can't"
    };
    let then_text = lowercase_first(describe_effect_list(&if_effect.then).trim_end_matches('.'));
    if then_text.is_empty() {
        return None;
    }
    Some(format!(
        "{}. {action}. {condition}, {then_text}",
        describe_tagged_mill_clause(mill)
    ))
}

pub(crate) fn describe_tagged_mill_then_may_put_milled_card_into_hand(
    tagged_mill: &crate::effects::TaggedEffect,
    mill: &crate::effects::MillEffect,
    may: &crate::effects::MayEffect,
) -> Option<String> {
    let [choose_effect, move_effect, rest_effect] = may.effects.as_slice() else {
        return None;
    };
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != mill.player)
    {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let move_to_hand = move_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let rest = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if choose.chooser != mill.player
        || !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
        || !for_each_moves_unselected_to_zone(
            rest,
            tagged_mill.tag.as_str(),
            choose.tag.as_str(),
            Zone::Graveyard,
        )
    {
        return None;
    }
    let chosen = describe_choose_filter_from_tagged_cards(choose, tagged_mill.tag.as_str())?;
    let mill_clause = describe_tagged_mill_clause(mill);
    let hand = describe_possessive_player_filter(&choose.chooser);
    Some(format!(
        "{mill_clause}. You may put {chosen} from among the cards milled this way into {hand} hand"
    ))
}

pub(super) fn describe_optional_payment_cost(
    may: &crate::effects::MayEffect,
    payer: &PlayerFilter,
) -> Option<String> {
    if may.effects.is_empty() || may.decider.as_ref().is_some_and(|decider| decider != payer) {
        return None;
    }

    let mut parts = Vec::new();
    for effect in &may.effects {
        if let Some(pay_mana) = effect.downcast_ref::<crate::effects::PayManaEffect>() {
            if !matches!(&pay_mana.player, ChooseSpec::Player(player) if player == payer) {
                return None;
            }
            parts.push(pay_mana.cost.to_oracle());
        } else if let Some(lose_life) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
            if !matches!(&lose_life.player, ChooseSpec::Player(player) if player == payer) {
                return None;
            }
            parts.push(format!("{} life", describe_value(&lose_life.amount)));
        } else {
            return None;
        }
    }

    (!parts.is_empty()).then(|| parts.join(" and "))
}

pub(super) fn describe_may_have_target_block_source(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You)) || may.effects.len() != 2 {
        return None;
    }
    let (tag, target_only) = tagged_target_only_effect(&may.effects[0])?;
    let cant =
        unwrap_basic_tag_wrappers(&may.effects[1]).downcast_ref::<crate::effects::CantEffect>()?;
    if cant.duration != Until::EndOfTurn {
        return None;
    }
    let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
        &cant.restriction
    else {
        return None;
    };
    if !filter_is_exactly_tagged(blockers, tag) || !attacker.source {
        return None;
    }

    Some(format!(
        "You may have {} block this creature this turn if able",
        describe_choose_spec(&target_only.target)
    ))
}

pub(crate) fn describe_tagged_mill_then_payment_if_you_do_put_milled_card_into_hand(
    tagged_mill: &crate::effects::TaggedEffect,
    mill: &crate::effects::MillEffect,
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let payment = describe_optional_payment_cost(may, &mill.player)?;
    let [choose_effect, move_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let move_to_hand = move_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if choose.chooser != mill.player
        || !for_each_moves_tag_to_hand(move_to_hand, choose.tag.as_str())
    {
        return None;
    }

    let chosen = describe_choose_filter_from_tagged_cards(choose, tagged_mill.tag.as_str())?;
    let mill_clause = describe_tagged_mill_clause(mill);
    let hand = describe_possessive_player_filter(&choose.chooser);
    Some(format!(
        "{mill_clause}. Then you may pay {payment}. If you do, put {chosen} from among those cards into {hand} hand"
    ))
}

pub(crate) fn describe_may_search_library_and_or_nonlibrary(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    fn downcast_search_library<'a>(
        effect: &'a Effect,
    ) -> Option<&'a crate::effects::SearchLibraryEffect> {
        if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>() {
            return Some(search);
        }
        effect
            .downcast_ref::<crate::effects::TaggedEffect>()?
            .effect
            .downcast_ref::<crate::effects::SearchLibraryEffect>()
    }

    fn downcast_move_to_zone<'a>(
        effect: &'a Effect,
    ) -> Option<&'a crate::effects::MoveToZoneEffect> {
        if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            return Some(move_to_zone);
        }
        effect
            .downcast_ref::<crate::effects::TaggedEffect>()?
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
    }

    fn zone_name(zone: Zone) -> Option<&'static str> {
        match zone {
            Zone::Graveyard => Some("graveyard"),
            Zone::Hand => Some("hand"),
            Zone::Exile => Some("exile"),
            Zone::Battlefield => Some("battlefield"),
            Zone::Command => Some("command zone"),
            Zone::Stack => Some("stack"),
            Zone::OutsideGame => Some("outside the game"),
            Zone::Library => None,
        }
    }

    fn destination_phrase(zone: Zone, player: &PlayerFilter) -> String {
        let owner = describe_possessive_player_filter(player);
        match zone {
            Zone::Hand => format!("into {owner} hand"),
            Zone::Battlefield => "onto the battlefield".to_string(),
            Zone::Library => format!("on top of {owner} library"),
            Zone::Graveyard => format!("into {owner} graveyard"),
            Zone::Exile => "into exile".to_string(),
            Zone::Stack => "onto the stack".to_string(),
            Zone::Command => "into the command zone".to_string(),
            Zone::OutsideGame => "outside the game".to_string(),
        }
    }

    let [choose_effect, found_effect, fallback_effect] = may.effects.as_slice() else {
        return None;
    };

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_primary_zone(choose) == Some(Zone::Library)
        || choose.is_search
        || choose.count.min != 0
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
    {
        return None;
    }

    let found = found_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if found.predicate != EffectPredicate::Happened || !found.else_.is_empty() {
        return None;
    }

    let mut reveal_chosen = false;
    let mut move_to_zone: Option<&crate::effects::MoveToZoneEffect> = None;
    for effect in &found.then {
        if let Some(reveal) = effect.downcast_ref::<crate::effects::RevealTaggedEffect>() {
            if reveal.tag != choose.tag {
                return None;
            }
            reveal_chosen = true;
            continue;
        }

        let Some(candidate_move) = downcast_move_to_zone(effect) else {
            return None;
        };
        if !matches!(candidate_move.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag) {
            return None;
        }
        move_to_zone = Some(candidate_move);
    }
    let move_to_zone = move_to_zone?;

    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if fallback.predicate != EffectPredicate::DidNotHappen || !fallback.else_.is_empty() {
        return None;
    }
    let [search_effect] = fallback.then.as_slice() else {
        return None;
    };
    let search = downcast_search_library(search_effect)?;

    if search.destination != move_to_zone.zone
        || search.reveal != reveal_chosen
        || choose.filter.name != search.filter.name
    {
        return None;
    }

    let nonlibrary_zone = zone_name(choose_primary_zone(choose)?)?;
    let actor = may.decider.as_ref().unwrap_or(&search.chooser);
    let actor_text = describe_player_filter(actor);
    let actor_sentence = capitalize_first(&actor_text);
    let possessive = describe_possessive_player_filter(&search.player);
    let mut display_filter = search.filter.clone();
    if display_filter.owner.as_ref() == Some(&search.player) {
        display_filter.owner = None;
    }
    let filter_desc = if is_generic_owned_card_search_filter(&display_filter) {
        "a card".to_string()
    } else {
        describe_search_selection_with_cards(&display_filter.description())
    };

    let mut text = format!(
        "{actor_sentence} may search {possessive} library and/or {nonlibrary_zone} for {filter_desc}"
    );
    if search.reveal && search.destination != Zone::Battlefield {
        text.push_str(", reveal it, and put it ");
    } else {
        text.push_str(", and put it ");
    }
    text.push_str(&destination_phrase(search.destination, &search.player));

    text.push_str(". If ");
    text.push_str(&actor_text);
    text.push(' ');
    text.push_str(player_verb(&actor_text, "search", "searches"));
    text.push(' ');
    text.push_str(&format!("{possessive} library this way, shuffle"));

    Some(text)
}

pub(crate) fn describe_may_search_then_put_onto_battlefield(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    fn downcast_move_to_zone<'a>(
        effect: &'a Effect,
    ) -> Option<&'a crate::effects::MoveToZoneEffect> {
        if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            return Some(move_to_zone);
        }
        effect
            .downcast_ref::<crate::effects::TaggedEffect>()?
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
    }

    let [choose_effect, move_effect, shuffle_effect] = may.effects.as_slice() else {
        return None;
    };

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
    {
        return None;
    }

    let move_to_zone = downcast_move_to_zone(move_effect)?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != choose.chooser {
        return None;
    }

    let actor = may.decider.as_ref().unwrap_or(&choose.chooser);
    if actor != &choose.chooser {
        return None;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let mut display_filter = choose.filter.clone();
    if display_filter.owner.as_ref() == Some(&choose.chooser) {
        display_filter.owner = None;
    }
    let selection = describe_search_selection_with_cards(&display_filter.description());
    let actor_text = describe_player_filter(actor);
    let may_clause = if actor_text == "you" {
        "You may".to_string()
    } else {
        format!("{} may", capitalize_first(&actor_text))
    };
    let pronoun = if choose.count.max == Some(1) {
        "it"
    } else if selection.contains(" cards") {
        "those cards"
    } else {
        "them"
    };

    Some(format!(
        "{may_clause} search {search_origin} for {selection}. Put {pronoun} onto the battlefield, then shuffle"
    ))
}

pub(crate) fn describe_may_search_reveal_shuffle_then_conditional_move(
    may: &crate::effects::MayEffect,
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    fn downcast_move_to_zone<'a>(
        effect: &'a Effect,
    ) -> Option<&'a crate::effects::MoveToZoneEffect> {
        if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            return Some(move_to_zone);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return tagged
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>();
        }
        effect
            .downcast_ref::<crate::effects::WithIdEffect>()?
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
    }

    fn downcast_search_reveal(effect: &Effect) -> Option<&crate::effects::RevealTaggedEffect> {
        if let Some(reveal) = effect.downcast_ref::<crate::effects::RevealTaggedEffect>() {
            return Some(reveal);
        }
        effect
            .downcast_ref::<crate::effects::WithIdEffect>()?
            .effect
            .downcast_ref::<crate::effects::RevealTaggedEffect>()
    }

    fn conditional_search_shuffle_player(effect: &Effect) -> Option<&PlayerFilter> {
        if let Some(shuffle) = effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>() {
            return Some(&shuffle.player);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return conditional_search_shuffle_player(&with_id.effect);
        }
        let if_effect = effect.downcast_ref::<crate::effects::IfEffect>()?;
        if if_effect.predicate != EffectPredicate::SearchedLibrary
            || !if_effect.else_.is_empty()
            || if_effect.then.len() != 1
        {
            return None;
        }
        if_effect.then[0]
            .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            .map(|shuffle| &shuffle.player)
    }

    fn move_uses_tag(move_to_zone: &crate::effects::MoveToZoneEffect, tag: &TagKey) -> bool {
        choose_spec_references_exact_tag(&move_to_zone.target, tag)
    }

    let [choose_effect, reveal_effect, shuffle_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = downcast_search_reveal(reveal_effect)?;
    let shuffle_player = conditional_search_shuffle_player(shuffle_effect)?;

    if !choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || reveal.tag != choose.tag
    {
        return None;
    }

    let actor = may.decider.as_ref().unwrap_or(&choose.chooser);
    if actor != &choose.chooser {
        return None;
    }
    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    if shuffle_player != search_owner_filter {
        return None;
    }

    let [true_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let [false_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    let true_move = downcast_move_to_zone(true_effect)?;
    let false_move = downcast_move_to_zone(false_effect)?;
    if true_move.zone != Zone::Hand
        || false_move.zone != Zone::Library
        || !false_move.to_top
        || !move_uses_tag(true_move, &choose.tag)
        || !move_uses_tag(false_move, &choose.tag)
    {
        return None;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let mut display_filter = choose.filter.clone();
    display_filter.owner = None;
    if searched_library && display_filter.zone == Some(Zone::Library) {
        display_filter.zone = None;
    }
    let raw_filter_text = if display_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        normalize_search_descriptor_for_origin(&display_filter.description(), searched_library)
    };
    let selection = describe_search_selection_with_cards_preserving_where(
        &describe_search_selection_from_filter_text(choose, &raw_filter_text),
    );
    let actor_text = describe_player_filter(actor);
    let may_clause = if actor_text == "you" {
        "You may".to_string()
    } else {
        format!("{} may", capitalize_first(&actor_text))
    };
    let owner_possessive = describe_possessive_player_filter(search_owner_filter);
    let condition = describe_condition(&conditional.condition);

    Some(format!(
        "{may_clause} search {search_origin} for {selection}, reveal it, then shuffle. If {condition}, put that card into {owner_possessive} hand. Otherwise, put that card on top of {owner_possessive} library"
    ))
}

pub(crate) fn describe_may_search_reveal_conditional_move_then_shuffle(
    may: &crate::effects::MayEffect,
    conditional: &crate::effects::ConditionalEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    let [choose_effect, reveal_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if !choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || reveal.tag != choose.tag
    {
        return None;
    }
    let actor = may.decider.as_ref().unwrap_or(&choose.chooser);
    if actor != &PlayerFilter::You || choose.chooser != PlayerFilter::You {
        return None;
    }
    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    if shuffle.player != *search_owner_filter {
        return None;
    }

    let true_move = conditional_search_branch_move(
        &conditional.if_true,
        Zone::Battlefield,
        choose.tag.as_str(),
    )?;
    let false_move =
        conditional_search_branch_move(&conditional.if_false, Zone::Hand, choose.tag.as_str())?;
    if false_move.to_top {
        return None;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let mut display_filter = choose.filter.clone();
    display_filter.owner = None;
    if searched_library && display_filter.zone == Some(Zone::Library) {
        display_filter.zone = None;
    }
    let raw_filter_text = if display_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        normalize_search_descriptor_for_origin(&display_filter.description(), searched_library)
    };
    let selection = describe_search_selection_with_cards_preserving_where(
        &describe_search_selection_from_filter_text(choose, &raw_filter_text),
    );
    let owner_possessive = describe_possessive_player_filter(search_owner_filter);
    let condition =
        describe_condition_for_searched_card(&conditional.condition, choose.tag.as_str());
    let true_clause = capitalize_first(&describe_conditional_search_move_clause(
        true_move,
        "it",
        &owner_possessive,
    )?);
    let false_clause =
        describe_conditional_search_move_clause(false_move, "it", &owner_possessive)?;
    let shuffle_clause = if describe_player_filter(search_owner_filter) == "you" {
        "shuffle".to_string()
    } else {
        "that player shuffles".to_string()
    };

    Some(format!(
        "You may search {search_origin} for {selection} and reveal it. {true_clause} if {condition}. Otherwise, {false_clause}. If you search {search_origin} this way, {shuffle_clause}"
    ))
}

pub(crate) fn describe_search_reveal_named_conditional_move_then_shuffle(
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: &crate::effects::RevealTaggedEffect,
    conditional: &crate::effects::ConditionalEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    fn branch_move<'a>(
        effects: &'a [Effect],
        zone: Zone,
        tag: &str,
    ) -> Option<&'a crate::effects::MoveToZoneEffect> {
        let [effect] = effects else {
            return None;
        };
        let move_to_zone = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        if move_to_zone.zone != zone
            || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found.as_str() == tag)
        {
            return None;
        }
        Some(move_to_zone)
    }

    if !choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || reveal.tag != choose.tag
    {
        return None;
    }
    let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition else {
        return None;
    };
    let card_name = filter.name.as_ref()?;
    if condition_tag != &choose.tag {
        return None;
    }
    branch_move(&conditional.if_true, Zone::Battlefield, choose.tag.as_str())?;
    branch_move(&conditional.if_false, Zone::Hand, choose.tag.as_str())?;

    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    if shuffle.player != *search_owner_filter {
        return None;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let mut display_filter = choose.filter.clone();
    display_filter.owner = None;
    if searched_library && display_filter.zone == Some(Zone::Library) {
        display_filter.zone = None;
    }
    let raw_filter_text = if display_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        normalize_search_descriptor_for_origin(&display_filter.description(), searched_library)
    };
    let selection = describe_search_selection_with_cards_preserving_where(
        &describe_search_selection_from_filter_text(choose, &raw_filter_text),
    );
    let owner_possessive = describe_possessive_player_filter(search_owner_filter);

    Some(format!(
        "Search {search_origin} for {selection} and reveal it. If you reveal a card named {card_name} this way, put it onto the battlefield. Otherwise, put that card into {owner_possessive} hand. Then shuffle"
    ))
}

pub(super) fn move_to_zone_from_effect(
    effect: &Effect,
) -> Option<&crate::effects::MoveToZoneEffect> {
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return Some(move_to_zone);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return move_to_zone_from_effect(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return move_to_zone_from_effect(&with_id.effect);
    }
    None
}

pub(super) fn conditional_search_branch_move<'a>(
    effects: &'a [Effect],
    zone: Zone,
    tag: &str,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    let [effect] = effects else {
        return None;
    };
    let move_to_zone = move_to_zone_from_effect(effect)?;
    if move_to_zone.zone != zone
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found.as_str() == tag)
    {
        return None;
    }
    Some(move_to_zone)
}

pub(crate) fn describe_search_reveal_conditional_move_then_shuffle(
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: &crate::effects::RevealTaggedEffect,
    conditional: &crate::effects::ConditionalEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    fn branch_move<'a>(
        effects: &'a [Effect],
        zone: Zone,
        tag: &str,
    ) -> Option<&'a crate::effects::MoveToZoneEffect> {
        let [effect] = effects else {
            return None;
        };
        let move_to_zone = move_to_zone_from_effect(effect)?;
        if move_to_zone.zone != zone
            || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found.as_str() == tag)
        {
            return None;
        }
        Some(move_to_zone)
    }

    if !choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || reveal.tag != choose.tag
    {
        return None;
    }
    let true_move = branch_move(&conditional.if_true, Zone::Battlefield, choose.tag.as_str())?;
    let false_move = branch_move(&conditional.if_false, Zone::Hand, choose.tag.as_str())?;
    if false_move.to_top {
        return None;
    }

    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    if shuffle.player != *search_owner_filter {
        return None;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let mut display_filter = choose.filter.clone();
    display_filter.owner = None;
    if searched_library && display_filter.zone == Some(Zone::Library) {
        display_filter.zone = None;
    }
    let raw_filter_text = if display_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        normalize_search_descriptor_for_origin(&display_filter.description(), searched_library)
    };
    let selection = describe_search_selection_with_cards_preserving_where(
        &describe_search_selection_from_filter_text(choose, &raw_filter_text),
    );
    let owner_possessive = describe_possessive_player_filter(search_owner_filter);
    let condition =
        describe_condition_for_searched_card(&conditional.condition, choose.tag.as_str());
    let true_clause = describe_conditional_search_move_clause(true_move, "it", &owner_possessive)?;
    let false_clause =
        describe_conditional_search_move_clause(false_move, "it", &owner_possessive)?;

    Some(format!(
        "Search {search_origin} for {selection} and reveal it. If {condition}, {true_clause}. Otherwise, {false_clause}. Then shuffle"
    ))
}

pub(crate) fn describe_search_reveal_conditional_may_battlefield_else_hand_then_shuffle(
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: &crate::effects::RevealTaggedEffect,
    battlefield_conditional: &crate::effects::ConditionalEffect,
    hand_conditional: &crate::effects::ConditionalEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    if !choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || reveal.tag != choose.tag
        || !battlefield_conditional.if_false.is_empty()
    {
        return None;
    }

    let [may_effect] = battlefield_conditional.if_true.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::You)
    {
        return None;
    }
    let battlefield_move =
        conditional_search_branch_move(&may.effects, Zone::Battlefield, choose.tag.as_str())?;

    let Condition::Not(not_condition) = &hand_conditional.condition else {
        return None;
    };
    let Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
    } = not_condition.as_ref()
    else {
        return None;
    };
    let battlefield_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    if *player != PlayerFilter::You || tag != &choose.tag || filter != &battlefield_filter {
        return None;
    }
    let hand_move =
        conditional_search_branch_move(&hand_conditional.if_true, Zone::Hand, choose.tag.as_str())?;
    if !hand_conditional.if_false.is_empty() || hand_move.to_top {
        return None;
    }

    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    if shuffle.player != *search_owner_filter {
        return None;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let mut display_filter = choose.filter.clone();
    display_filter.owner = None;
    if searched_library && display_filter.zone == Some(Zone::Library) {
        display_filter.zone = None;
    }
    let raw_filter_text = if display_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        normalize_search_descriptor_for_origin(&display_filter.description(), searched_library)
    };
    let selection = describe_search_selection_with_cards_preserving_where(
        &describe_search_selection_from_filter_text(choose, &raw_filter_text),
    );
    let condition = describe_condition(&battlefield_conditional.condition);
    let tapped = if battlefield_move.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let owner_possessive = describe_possessive_player_filter(search_owner_filter);
    let hand_clause = describe_conditional_search_move_clause(hand_move, "it", &owner_possessive)?;

    Some(format!(
        "Search {search_origin} for {selection} and reveal it. If {condition}, you may put that card onto the battlefield{tapped}. If you don't put the card onto the battlefield, {hand_clause}. Then shuffle"
    ))
}

pub(super) fn describe_condition_for_searched_card(
    condition: &Condition,
    searched_tag: &str,
) -> String {
    if let Condition::TaggedObjectMatches(tag, filter) = condition
        && tag.as_str() == searched_tag
    {
        let desc = filter.description();
        let stripped = strip_leading_article(&desc).to_ascii_lowercase();
        if let Some(rest) = stripped.strip_prefix("permanent with mana value ") {
            return format!("its mana value is {}", rest.trim());
        }
        if stripped == "land" {
            return "it's a land card".to_string();
        }
        if stripped == "creature" {
            return "it's a creature card".to_string();
        }
    }
    describe_condition(condition)
}

pub(super) fn describe_conditional_search_move_clause(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    pronoun: &str,
    owner_possessive: &str,
) -> Option<String> {
    match move_to_zone.zone {
        Zone::Battlefield => {
            let tapped = if move_to_zone.enters_tapped {
                " tapped"
            } else {
                ""
            };
            Some(format!("put {pronoun} onto the battlefield{tapped}"))
        }
        Zone::Hand => Some(format!("put {pronoun} into {owner_possessive} hand")),
        Zone::Library if move_to_zone.to_top => Some(format!(
            "put {pronoun} on top of {owner_possessive} library"
        )),
        _ => None,
    }
}

pub(crate) fn describe_may_choose_reveal_and_move_to_hand(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    fn downcast_move_to_zone<'a>(
        effect: &'a Effect,
    ) -> Option<&'a crate::effects::MoveToZoneEffect> {
        if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            return Some(move_to_zone);
        }
        effect
            .downcast_ref::<crate::effects::TaggedEffect>()?
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
    }

    let [choose_effect, reveal_effect, move_effect] = may.effects.as_slice() else {
        return None;
    };

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let move_to_zone = downcast_move_to_zone(move_effect)?;

    let zones = choose_search_zones(choose)?;
    let choose_from_exile = zones.as_slice() == [Zone::Exile];
    let choose_from_outside_or_exile =
        zones.len() == 2 && zones.contains(&Zone::OutsideGame) && zones.contains(&Zone::Exile);
    if !(choose_from_exile || choose_from_outside_or_exile)
        || choose.is_search
        || !choose.count.is_single()
        || reveal.tag != choose.tag
        || !move_to_hand_uses_chosen_tag(move_to_zone, choose.tag.as_str())
    {
        return None;
    }

    let actor = may.decider.as_ref().unwrap_or(&choose.chooser);
    if actor != &choose.chooser {
        return None;
    }

    let actor_text = describe_player_filter(actor);
    let mut chosen = describe_choose_selection(choose);
    let ensure_selection_mentions_card = |selection: &mut String| {
        if !choose_from_outside_or_exile
            || selection.to_ascii_lowercase().contains(" card")
            || choose.filter.subtypes.is_empty()
        {
            return;
        }
        let subtype_text = choose
            .filter
            .subtypes
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        if !subtype_text.is_empty() {
            *selection = selection.replacen(&subtype_text, &format!("{subtype_text} card"), 1);
        }
    };
    ensure_selection_mentions_card(&mut chosen);
    let chosen_mentions_owner = chosen.to_ascii_lowercase().contains(" own");
    let location = match choose.filter.owner.as_ref() {
        Some(owner) if owner == actor && choose_from_outside_or_exile => {
            if chosen_mentions_owner {
                "from outside the game or in exile".to_string()
            } else if actor_text == "you" {
                "you own from outside the game or in exile".to_string()
            } else {
                format!(
                    "{} owns from outside the game or in exile",
                    describe_player_filter(owner)
                )
            }
        }
        Some(owner) if owner == actor => {
            if actor_text == "you" {
                "you own in exile".to_string()
            } else {
                format!("{} owns in exile", describe_player_filter(owner))
            }
        }
        _ if choose_from_outside_or_exile => "from outside the game or in exile".to_string(),
        _ => "in exile".to_string(),
    };
    let hand = if choose.filter.owner.as_ref() == Some(actor) {
        format!("{} hand", describe_possessive_player_filter(actor))
    } else {
        "its owner's hand".to_string()
    };

    if choose_from_outside_or_exile && choose.filter.face_down == Some(false) {
        let mut outside_choose = choose.clone();
        outside_choose.filter.face_down = None;
        outside_choose.zone = Some(Zone::OutsideGame);
        outside_choose.additional_zones.clear();
        let mut outside_selection = describe_choose_selection(&outside_choose);
        ensure_selection_mentions_card(&mut outside_selection);
        return Some(format!(
            "{} may reveal {outside_selection} from outside the game or choose {chosen} in exile. Put that card into {hand}",
            capitalize_first(&actor_text)
        ));
    }

    Some(format!(
        "{} may choose {chosen} {location} and put that card into {hand}",
        capitalize_first(&actor_text)
    ))
}

pub(crate) fn describe_may_have_you_create_tokens(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if may.effects.len() != 1 {
        return None;
    }
    let create_token = may.effects[0].downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if !matches!(create_token.controller, PlayerFilter::You) {
        return None;
    }
    let decider = may.decider.as_ref()?;
    let who = describe_player_filter(decider);
    if who == "you" {
        return None;
    }

    let inner = describe_effect_list(&may.effects);
    let Some(rest) = inner.strip_prefix("Create ") else {
        return None;
    };

    Some(format!("{who} may have you create {rest}"))
}

pub(super) fn describe_may_discover_from_triggering_toughness(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if may.decider != Some(PlayerFilter::You) {
        return None;
    }
    let [effect] = may.effects.as_slice() else {
        return None;
    };
    let discover = effect.downcast_ref::<crate::effects::DiscoverEffect>()?;
    if discover.player != PlayerFilter::You {
        return None;
    }
    matches!(&discover.count, crate::effect::Value::ToughnessOf(target)
        if matches!(target.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"))
    .then(|| "You may discover X, where X is that creature's toughness".to_string())
}

pub(crate) fn describe_may_enlist(may: &crate::effects::MayEffect) -> Option<String> {
    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            tagged.effect.as_ref()
        } else if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            with_id.effect.as_ref()
        } else {
            effect
        }
    }

    if may.effects.len() != 4 {
        return None;
    }

    if let Some(tag_triggering) =
        unwrap_effect(&may.effects[0]).downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        && tag_triggering.tag.as_str() == "enlist_attacker"
        && let Some(choose) =
            unwrap_effect(&may.effects[1]).downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choose.chooser == PlayerFilter::You
        && choose.tag.as_str() == "enlisted_creature"
        && let Some(tap) =
            unwrap_effect(&may.effects[2]).downcast_ref::<crate::effects::TapEffect>()
        && matches!(&tap.target, ChooseSpec::Tagged(tag) if tag.as_str() == "enlisted_creature")
        && let Some(modify) = unwrap_effect(&may.effects[3])
            .downcast_ref::<crate::effects::ModifyPowerToughnessForEachEffect>()
        && matches!(&modify.target, ChooseSpec::Tagged(tag) if tag.as_str() == "enlist_attacker")
        && modify.power_per == 1
        && modify.toughness_per == 0
        && modify.duration == crate::effect::Until::EndOfTurn
        && matches!(
            &modify.count,
            Value::PowerOf(spec)
                if matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == "enlisted_creature")
        )
    {
        let enlisted_desc =
            strip_leading_article(&choose.filter.description()).replace(" in the battlefield", "");
        let enlisted = if enlisted_desc.starts_with("another ") {
            enlisted_desc
        } else {
            with_indefinite_article(&enlisted_desc)
        };
        return Some(format!(
            "you may tap {enlisted}. When you do, this creature gets +X/+0 until end of turn, where X is that creature's power"
        ));
    }

    let tag_text = describe_effect(unwrap_effect(&may.effects[0])).to_ascii_lowercase();
    let choose_text = describe_effect(unwrap_effect(&may.effects[1])).to_ascii_lowercase();
    let tap_text = describe_effect(unwrap_effect(&may.effects[2])).to_ascii_lowercase();
    let pump_text = describe_effect(unwrap_effect(&may.effects[3])).to_ascii_lowercase();
    if tag_text == "tag the triggering object as 'enlist_attacker'"
        && choose_text.starts_with("you choose exactly 1 ")
        && choose_text.contains(" and tags it as 'enlisted_creature'")
        && tap_text == "tap target tagged object 'enlisted_creature'"
        && pump_text
            == "the tagged object 'enlist_attacker' gets +1/+0 until end of turn for each the tagged object 'enlisted_creature''s power"
    {
        let enlisted = choose_text
            .trim_start_matches("you choose exactly 1 ")
            .split(" and tags it as 'enlisted_creature'")
            .next()
            .unwrap_or("another nonattacking creature you control")
            .replace(" in the battlefield", "");
        return Some(format!(
            "you may tap {enlisted}. When you do, this creature gets +X/+0 until end of turn, where X is that creature's power"
        ));
    }

    None
}

pub(crate) fn describe_may_search_choose_for_each_with_shuffle(
    may: &crate::effects::MayEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    let choose = may
        .effects
        .first()?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (choose, for_each) = if let Some(reveal) = may
        .effects
        .get(1)
        .and_then(|effect| effect.downcast_ref::<crate::effects::RevealTaggedEffect>())
    {
        if reveal.tag != choose.tag {
            return None;
        }
        let mut revealed_choose = choose.clone();
        revealed_choose.reveal = true;
        let for_each = may
            .effects
            .get(2)?
            .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
        (revealed_choose, for_each)
    } else {
        let for_each = may
            .effects
            .get(1)?
            .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
        (choose.clone(), for_each)
    };

    if may.effects.len() != if choose.reveal { 3 } else { 2 } {
        return None;
    }

    let compact = describe_search_choose_for_each(&choose, for_each, Some(shuffle), false)?;
    let compact = if choose_search_zones(&choose).is_some_and(|zones| zones.len() > 1) {
        if let Some(prefix) = compact
            .strip_suffix(", then shuffle your library")
            .or_else(|| compact.strip_suffix(", then shuffle"))
        {
            format!("{prefix}. If you search your library this way, shuffle")
        } else {
            compact
        }
    } else {
        compact
    };
    let actor = describe_player_filter(may.decider.as_ref().unwrap_or(&choose.chooser));
    if let Some(rest) = compact.strip_prefix("Search ") {
        if actor == "you" {
            return Some(format!("You may search {}", lowercase_first(rest)));
        }
        return Some(format!(
            "{} may search {}",
            capitalize_first(&actor),
            lowercase_first(rest)
        ));
    }
    None
}
