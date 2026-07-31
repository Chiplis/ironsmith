use super::*;

pub(in crate::compiled_text) fn describe_choose_color_reveal_hand_discard_that_color(
    effects: &[&Effect],
) -> Option<String> {
    let (choose_color_effect, look_effect, discard_effect) = match effects {
        [choose_color_effect, look_effect, discard_effect] => {
            (*choose_color_effect, *look_effect, *discard_effect)
        }
        [
            choose_color_effect,
            target_only_effect,
            look_effect,
            discard_effect,
        ] if target_only_effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some() =>
        {
            (*choose_color_effect, *look_effect, *discard_effect)
        }
        _ => return None,
    };
    let choose_color = choose_color_effect.downcast_ref::<crate::effects::ChooseColorEffect>()?;
    if choose_color.chooser != PlayerFilter::You {
        return None;
    }
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.random
        || discard.any_number
        || !player_filters_refer_to_same_player(&discard.player, &look_player)
        || !discard.card_filter.as_ref().is_some_and(|filter| {
            filter
                .owner
                .as_ref()
                .is_some_and(|owner| player_filters_refer_to_same_player(owner, &look_player))
                && filter.chosen_color
        })
    {
        return None;
    }
    let Value::Count(count_filter) = discard.count.unhinted() else {
        return None;
    };
    if count_filter.zone != Some(Zone::Hand) || !count_filter.chosen_color {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    Some(format!(
        "Choose a color. {} {} their hand and discards all cards of that color",
        capitalize_first(&revealer),
        reveal_verb
    ))
}

pub(super) fn describe_reveal_top_two_optional_picks_rest_bottom(
    effects: &[&Effect],
) -> Option<String> {
    let [
        look_effect,
        reveal_effect,
        first_choose_effect,
        first_move_effect,
        second_choose_effect,
        second_move_effect,
        rest_effect,
    ] = effects
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let first_choose = first_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let first_keep_tag = effect_outer_tag(first_move_effect)?;
    let first_move = unwrap_basic_tag_wrappers(first_move_effect)
        .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let second_choose =
        second_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let second_keep_tag = effect_outer_tag(second_move_effect)?;
    let second_move = unwrap_basic_tag_wrappers(second_move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let rest =
        rest_effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;

    let Value::Fixed(count) = look.count else {
        return None;
    };
    if look.player != PlayerFilter::You
        || reveal.tag != look.tag
        || first_choose.count.min != 0
        || first_choose.count.max != Some(1)
        || second_choose.count.min != 0
        || second_choose.count.max != Some(1)
        || choose_primary_zone(first_choose) != Some(Zone::Library)
        || choose_primary_zone(second_choose) != Some(Zone::Library)
        || first_choose.filter.card_types != vec![CardType::Land]
        || first_choose
            .filter
            .tagged_constraints
            .iter()
            .all(|constraint| {
                constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    || constraint.tag != look.tag
            })
        || second_choose.filter.subtypes.len() != 1
        || second_choose
            .filter
            .tagged_constraints
            .iter()
            .all(|constraint| {
                constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    || constraint.tag != look.tag
            })
        || !matches!(&first_move.target, ChooseSpec::Tagged(tag) if tag == &first_choose.tag)
        || !first_move.tapped
        || first_move.controller != PlayerFilter::You
        || !matches!(&second_move.target, ChooseSpec::Tagged(tag) if tag == &second_choose.tag)
        || second_move.zone != Zone::Hand
        || rest.tag != look.tag
        || rest.keep_tagged.as_ref() != Some(first_keep_tag)
        || second_keep_tag != first_keep_tag
        || rest.order != LibraryBottomOrder::Random
        || rest.player != PlayerFilter::You
    {
        return None;
    }

    let count_text = small_number_word(count as u32).unwrap_or_else(|| count.to_string());
    let subtype_text = second_choose.filter.subtypes[0].to_string();
    Some(format!(
        "Reveal the top {count_text} cards of your library. You may put up to one land card from among them onto the battlefield tapped and up to one {subtype_text} card from among them into your hand. Put the rest on the bottom of your library in a random order"
    ))
}

pub(super) fn describe_hideaway_effects(effects: &[&Effect]) -> Option<String> {
    let [look_effect, choose_effect, exile_effect, rest_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = exile_effect.downcast_ref::<crate::effects::ExileEffect>()?;
    let rest =
        rest_effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;

    if look.player != PlayerFilter::You
        || look.reveal
        || !choose.count.is_single()
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == look.tag
        })
        || !matches!(exile.spec, ChooseSpec::Tagged(ref tag) if tag == &choose.tag)
        || !exile.face_down
        || rest.tag != look.tag
        || rest.keep_tagged.as_ref() != Some(&choose.tag)
        || rest.player != PlayerFilter::You
    {
        return None;
    }

    let (count_text, noun, _) = describe_look_count_and_noun(&look.count);
    Some(match rest.order {
        LibraryBottomOrder::Random => format!(
            "Look at the top {count_text} {noun} of your library, then exile one of them face down. Put the rest on the bottom of your library in a random order"
        ),
        LibraryBottomOrder::ChooserChooses => format!(
            "Look at the top {count_text} {noun} of your library, exile one face down, then put the rest on the bottom of your library in any order"
        ),
    })
}

/// Recover the keyword surface from Hideaway's fully lowered ETB trigger.
/// The runtime keeps the expanded effects so the ability remains executable;
/// this recognizer only changes how that exact rules-defined shape is shown.
pub(super) fn describe_structural_hideaway_keyword(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }

    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if !trigger.this_object
        || trigger.from != crate::triggers::ZonePattern::Any
        || trigger.to != crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        || trigger.object_filter != ObjectFilter::default()
        || trigger.player != crate::triggers::PlayerRelation::Any
        || trigger.cause_filter.is_some()
        || trigger.count_mode != crate::triggers::CountMode::Each
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let effects = segment.default_effects.iter().collect::<Vec<_>>();
    describe_hideaway_effects(&effects)?;
    let look = effects[0].downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let Value::Fixed(count) = look.count.unhinted() else {
        return None;
    };
    (*count > 0).then(|| format!("Hideaway {count}"))
}

pub(super) fn describe_look_exile_one_rest_bottom_cast_else_hand(
    effects: &[&Effect],
) -> Option<String> {
    if let Some(gonti) = describe_target_opponent_look_exile_one_rest_bottom_cast(effects) {
        return Some(gonti);
    }

    let [
        look_effect,
        choose_effect,
        exile_effect,
        rest_effect,
        may_effect,
        fallback_effect,
    ] = effects
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile =
        unwrap_basic_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let rest =
        rest_effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let if_effect = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;

    if look.player != PlayerFilter::You
        || look.reveal
        || !choose.count.is_single()
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == look.tag
        })
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || !exile.face_down
        || rest.tag != look.tag
        || rest.keep_tagged.as_ref() != Some(&choose.tag)
        || rest.player != PlayerFilter::You
        || rest.order != LibraryBottomOrder::Random
        || may.decider.is_some()
        || if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::DidNotHappen
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let [conditional_effect] = may.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition else {
        return None;
    };
    let [cast_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    if !conditional.if_false.is_empty() {
        return None;
    }
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if condition_tag != &choose.tag
        || cast.tag != choose.tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    let [hand_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let hand_move = unwrap_basic_tag_wrappers(hand_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if hand_move.zone != Zone::Hand
        || !matches!(hand_move.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let (count_text, noun, _) = describe_look_count_and_noun(&look.count);
    let condition_text = describe_exiled_card_cast_condition(filter)?;
    Some(format!(
        "Look at the top {count_text} {noun} of your library. Exile one of them face down and put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost if {condition_text}. If you don't, put that card into your hand"
    ))
}

/// Preserve the collection boundaries in the Gonti-style look/partition/play
/// program. Every clause is justified by an explicit tag edge: the choice is
/// drawn from the looked collection, the exile and permission share the
/// choice tag, and the bottom action is the looked collection minus that tag.
pub(super) fn describe_target_opponent_look_exile_one_rest_bottom_cast(
    effects: &[&Effect],
) -> Option<String> {
    let [
        target_effect,
        look_effect,
        choose_effect,
        exile_effect,
        rest_effect,
        grant_effect,
    ] = effects
    else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile =
        unwrap_basic_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let rest =
        rest_effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let grant = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;

    let targets_opponent = matches!(
        &target.target,
        ChooseSpec::Target(inner)
            if matches!(inner.as_ref(), ChooseSpec::Player(PlayerFilter::Opponent))
    );
    let looks_at_target_opponent = matches!(
        &look.player,
        PlayerFilter::Target(inner) if inner.as_ref() == &PlayerFilter::Opponent
    );
    let mut untagged_filter = choose.filter.clone();
    untagged_filter.zone = None;
    untagged_filter.tagged_constraints.clear();
    let exact_looked_choice = choose.filter.zone == Some(Zone::Library)
        && choose.filter.tagged_constraints.len() == 1
        && choose.filter.tagged_constraints[0].tag == look.tag
        && choose.filter.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        && untagged_filter == ObjectFilter::default();

    if !targets_opponent
        || !looks_at_target_opponent
        || look.reveal
        || !choose.count.is_single()
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.is_search
        || !exact_looked_choice
        || !matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || !exile.face_down
        || rest.tag != look.tag
        || rest.keep_tagged.as_ref() != Some(&choose.tag)
        || rest.player != look.player
        || rest.order != LibraryBottomOrder::Random
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

    let (count_text, noun, where_clause) = describe_look_count_and_noun(&look.count);
    Some(format!(
        "Look at the top {count_text} {noun} of target opponent's library{where_clause}, exile one of them face down, then put the rest on the bottom of that library in a random order. You may cast that card for as long as it remains exiled, and mana of any type can be spent to cast that spell"
    ))
}

pub(super) fn describe_exiled_card_cast_condition(filter: &ObjectFilter) -> Option<String> {
    let mut display_filter = filter.clone();
    display_filter.zone = None;
    display_filter.has_mana_cost = false;
    if display_filter.card_types.len() == 1 {
        let card_type = display_filter.card_types[0]
            .to_string()
            .to_ascii_lowercase();
        let spell_text = with_indefinite_article(&format!("{card_type} spell"));
        let mana_value = display_filter.mana_value.clone();
        let mut remaining_filter = display_filter.clone();
        remaining_filter.card_types.clear();
        remaining_filter.mana_value = None;
        if remaining_filter == ObjectFilter::default() {
            match mana_value.as_ref() {
                Some(crate::filter::Comparison::LessThanOrEqual(value)) => {
                    return Some(format!("it's {spell_text} with mana value {value} or less"));
                }
                Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) => {
                    return Some(format!(
                        "it's {spell_text} with mana value {} or less",
                        describe_value(value)
                    ));
                }
                _ => {}
            }
        }
    }
    let desc = strip_indefinite_article(&display_filter.description()).to_ascii_lowercase();
    Some(format!("it's {}", with_indefinite_article(&desc)))
}

pub(super) fn describe_exile_targets_opponent_piles_return_chosen(
    effects: &[&Effect],
) -> Option<String> {
    let [
        exile_effect,
        tag_source_effect,
        choose_effect,
        move_chosen_effect,
        rest_effect,
    ] = effects
    else {
        return None;
    };
    let exiled = exile_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = exiled
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::WithCount(inner, count) = &exile.spec else {
        return None;
    };
    let ChooseSpec::Target(target) = inner.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(exile_filter) = target.as_ref() else {
        return None;
    };
    let tag_source =
        tag_source_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let moved = move_chosen_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let move_chosen = moved
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let for_each = rest_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [rest_inner] = for_each.effects.as_slice() else {
        return None;
    };
    let rest_conditional = rest_inner.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [rest_move] = rest_conditional.if_false.as_slice() else {
        return None;
    };
    let rest_move = rest_move.downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    if count.min != 0
        || count.max != Some(5)
        || exile.face_down
        || exile_filter.zone != Some(Zone::Graveyard)
        || exile_filter.card_types != vec![CardType::Creature]
        || tag_source.tag.as_str() != "divvy_source"
        || choose.tag.as_str() != "divvy_chosen"
        || choose.chooser != PlayerFilter::Opponent
        || choose_primary_zone(choose) != Some(Zone::Exile)
        || choose.count.min != 0
        || choose.count.max.is_some()
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == tag_source.tag
        })
        || !matches!(&move_chosen.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || move_chosen.zone != Zone::Battlefield
        || move_chosen.battlefield_controller != crate::effects::BattlefieldController::You
        || for_each.tag != tag_source.tag
        || !rest_conditional.if_true.is_empty()
        || !matches!(&rest_move.target, ChooseSpec::Iterated)
        || rest_move.zone != Zone::Graveyard
    {
        return None;
    }

    Some("Exile up to five target creature cards from graveyards. An opponent separates those cards into two piles. Put all cards from the pile of your choice onto the battlefield under your control and the rest into their owners' graveyards".to_string())
}

pub(super) fn describe_choose_x_permanents_create_x_copies(effects: &[&Effect]) -> Option<String> {
    let [choose_effect, for_each_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [copy_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let copy = unwrap_basic_tag_wrappers(copy_effect)
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    if !choose.count.dynamic_x
        || choose.count.up_to_x
        || choose.count_value.is_some()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.filter.card_types
            != vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ]
        || for_each.tag != choose.tag
        || !matches!(copy.target.unhinted(), ChooseSpec::Iterated)
        || copy.count != Value::X
        || copy.controller != PlayerFilter::You
        || copy.enters_tapped
        || copy.has_haste
        || copy.loses_soulbond
        || copy.enters_attacking
        || copy.attack_target_mode.is_some()
        || copy.exile_at_end_of_combat
        || copy.sacrifice_at_next_end_step
        || copy.exile_at_next_end_step
        || copy.pt_adjustment.is_some()
        || !copy.added_card_types.is_empty()
        || !copy.added_subtypes.is_empty()
        || !copy.removed_supertypes.is_empty()
        || copy.set_base_power_toughness.is_some()
        || copy.set_colors.is_some()
        || copy.set_card_types.is_some()
        || copy.set_subtypes.is_some()
        || !copy.granted_static_abilities.is_empty()
    {
        return None;
    }
    Some(
        "For each of X target permanents, create X tokens that are copies of that permanent"
            .to_string(),
    )
}

pub(super) fn describe_counter_artifact_ability_destroy_source(
    effects: &[&Effect],
) -> Option<String> {
    if let [sequence_effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
    {
        let nested = sequence.effects.iter().collect::<Vec<_>>();
        return describe_counter_artifact_ability_destroy_source(&nested);
    }

    let [counter_effect, conditional_effect] = effects else {
        return None;
    };
    let counter = structural_unwrap_render_wrappers(counter_effect)
        .downcast_ref::<crate::effects::CounterEffect>()?;
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let ChooseSpec::Target(target) = &counter.target else {
        return None;
    };
    let ChooseSpec::Object(counter_filter) = target.as_ref() else {
        return None;
    };
    let [destroy_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::Object(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    if counter_filter.stack_kind != Some(StackObjectKind::ActivatedAbility)
        || counter_filter.card_types != vec![CardType::Artifact]
        || counter_filter.zone != Some(Zone::Stack)
        || !conditional.if_false.is_empty()
        || destroy_filter.zone != Some(Zone::Battlefield)
        || destroy_filter.card_types != vec![CardType::Artifact]
        || destroy_filter.tagged_constraints.is_empty()
    {
        return None;
    }
    Some(
        "Counter target activated ability from an artifact source and destroy that artifact if it's on the battlefield"
            .to_string(),
    )
}

pub(super) fn greatest_commander_mana_value_owned_by(
    filter: &ObjectFilter,
    owner: PlayerFilter,
) -> bool {
    filter.zone.is_none()
        && filter.owner.is_none()
        && filter.any_of.len() == 2
        && [Zone::Battlefield, Zone::Command].iter().all(|zone| {
            filter.any_of.iter().any(|candidate| {
                candidate.zone == Some(*zone)
                    && candidate.owner == Some(owner.clone())
                    && candidate.is_commander
            })
        })
}

pub(super) fn greatest_commander_mana_value_owned_by_iterated(filter: &ObjectFilter) -> bool {
    greatest_commander_mana_value_owned_by(filter, PlayerFilter::IteratedPlayer)
}

pub(super) fn describe_each_player_may_discard_hand_draw_commander_value(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let [discard_effect, draw_effect] = may.effects.as_slice() else {
        return None;
    };
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardHandEffect>()?;
    if discard.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let Value::GreatestManaValue(filter) = draw.count.unhinted() else {
        return None;
    };
    greatest_commander_mana_value_owned_by_iterated(filter).then(|| {
        "Each player may discard their hand and draw cards equal to the greatest mana value of a commander they own on the battlefield or in the command zone"
            .to_string()
    })
}

pub(super) fn describe_each_player_may_discard_hand_draw(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let [discard_effect, draw_effect] = may.effects.as_slice() else {
        return None;
    };
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardHandEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if discard.player != PlayerFilter::IteratedPlayer || draw.player != PlayerFilter::IteratedPlayer
    {
        return None;
    }
    Some(format!(
        "Each player may discard their hand and draw {}",
        describe_card_count(&draw.count)
    ))
}

pub(super) fn describe_each_player_may_discard_card_then_draw(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let subject = describe_for_players_subject(&for_players.filter)?;
    let [may_effect, draw_if_discarded_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may_with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = may_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.is_some() {
        return None;
    }
    let [discard_effect] = may.effects.as_slice() else {
        return None;
    };
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count != Value::Fixed(1)
        || discard.player != PlayerFilter::IteratedPlayer
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }
    let draw_if_discarded = draw_if_discarded_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if draw_if_discarded.condition != may_with_id.id
        || draw_if_discarded.predicate != EffectPredicate::Happened
        || !draw_if_discarded.else_.is_empty()
    {
        return None;
    }
    let [draw_effect] = draw_if_discarded.then.as_slice() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.count != Value::Fixed(1) || draw.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    Some(format!(
        "{subject} may discard a card, then each player who discarded a card this way draws a card"
    ))
}

pub(super) fn describe_iterated_player_search_effects(effects: &[Effect]) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(effects.first()?)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose_search_zones(choose).is_none_or(|zones| !zones.contains(&Zone::Library))
    {
        return None;
    }

    let mut render_choose = choose.clone();
    let mut next = 1;
    if let Some(reveal) = effects
        .get(next)
        .map(structural_unwrap_render_wrappers)
        .and_then(|effect| effect.downcast_ref::<crate::effects::RevealTaggedEffect>())
    {
        if reveal.tag != choose.tag {
            return None;
        }
        render_choose.reveal = true;
        next += 1;
    }

    let for_each = structural_unwrap_render_wrappers(effects.get(next)?)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    next += 1;

    let shuffle = if let Some(effect) = effects.get(next) {
        let shuffle = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
        next += 1;
        Some(shuffle)
    } else {
        None
    };
    if next != effects.len() {
        return None;
    }

    let mut search_text =
        describe_search_choose_for_each(&render_choose, for_each, shuffle, false)?;
    search_text = search_text
        .replace("that player's library", "their library")
        .replace("that player's hand", "their hand")
        .replace(", then that player shuffles", ", then shuffle")
        .replace(". Then that player shuffles", ", then shuffle")
        .replace(", then they shuffle", ", then shuffle")
        .replace(". Then they shuffle", ", then shuffle")
        .replace(", then they shuffles", ", then shuffle")
        .replace(". Then they shuffles", ", then shuffle");
    Some(search_text)
}

pub(super) fn describe_iterated_player_search_sequence(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    describe_iterated_player_search_effects(&sequence.effects)
}

pub(super) fn describe_for_players_may_search_library_then_shuffle(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let subject = describe_for_players_subject(&for_players.filter)?;
    if let [may_effect] = for_players.effects.as_slice() {
        let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
        if may.decider != Some(PlayerFilter::IteratedPlayer) {
            return None;
        }
        let search_text = if let [sequence_effect] = may.effects.as_slice() {
            let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
            describe_iterated_player_search_sequence(sequence)?
        } else {
            describe_iterated_player_search_effects(&may.effects)?
        };
        let rest = search_text.strip_prefix("Search ")?;
        let rendered = format!("{subject} may search {}", lowercase_first(rest));
        return Some(
            rendered
                .replace(". Then they shuffle", ", then shuffle")
                .replace(". Then they shuffles", ", then shuffle"),
        );
    }

    let [search_effect, shuffle_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let [sequence_effect] = may.effects.as_slice() else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let choose = sequence
        .effects
        .first()?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose_search_zones(choose).is_none_or(|zones| !zones.contains(&Zone::Library))
    {
        return None;
    }

    let conditional = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle] = conditional.then.as_slice() else {
        return None;
    };
    let shuffle = shuffle.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if conditional.condition != with_id.id
        || conditional.predicate != EffectPredicate::Happened
        || !conditional.else_.is_empty()
        || shuffle.player != PlayerFilter::IteratedPlayer
    {
        return None;
    }

    let mut search_text = describe_search_sequence(sequence)?;
    search_text = search_text
        .replacen("Search ", "search ", 1)
        .replace("that player's library", "their library")
        .replace("that player's hand", "their hand")
        .replace(
            ", put it into their hand",
            " and put that card into their hand",
        )
        .replace(
            ", put them into their hand",
            " and put those cards into their hand",
        );
    Some(format!(
        "{subject} may {search_text}. Then {} who searched their library this way shuffles",
        lowercase_first(subject)
    ))
}

fn tagged_library_partition_filter(
    filter: &ObjectFilter,
    included_tag: &crate::TagKey,
    excluded_tag: Option<&crate::TagKey>,
    owner_tag: Option<&crate::TagKey>,
) -> bool {
    let owner_matches = filter.owner.as_ref().is_some_and(|owner| {
        owner == &PlayerFilter::IteratedPlayer
            || owner_tag.is_some_and(|expected_tag| {
                matches!(
                    owner,
                    PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag))
                        | PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(tag))
                        if tag == expected_tag
                )
            })
    });
    if filter.zone != Some(Zone::Library)
        || !owner_matches
        || filter.tagged_constraints.len() != 1 + usize::from(excluded_tag.is_some())
        || !filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == *included_tag
        })
        || excluded_tag.is_some_and(|tag| {
            !filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                    && constraint.tag == *tag
            })
        })
    {
        return false;
    }

    let mut remainder = filter.clone();
    remainder.zone = None;
    remainder.owner = None;
    remainder.tagged_constraints.clear();
    remainder == ObjectFilter::default()
}

fn iterated_player_or_owner_of_tag(player: &PlayerFilter, expected_tag: &crate::TagKey) -> bool {
    player == &PlayerFilter::IteratedPlayer
        || matches!(
            player,
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag))
                | PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(tag))
                if tag == expected_tag
        )
}

/// Render an optional per-player library search whose result is partitioned
/// between two battlefield controllers. Keeping this as one structural bundle
/// mirrors the runtime program: each iterated player chooses, partitions only
/// the cards they found, and shuffles only after accepting the search.
fn describe_for_players_optional_search_battlefield_partition_with_followups(
    for_players: &crate::effects::ForPlayersEffect,
    sibling_followups: &[Effect],
) -> Option<String> {
    let subject = describe_for_players_subject(&for_players.filter)?;
    let (may_effect, nested_followups) = for_players.effects.split_first()?;
    let may = unwrap_basic_tag_wrappers(may_effect).downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }

    let mut effects = Vec::new();
    for effect in &may.effects {
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            effects.extend(sequence.effects.iter());
        } else {
            effects.push(effect);
        }
    }
    // Sentence-level lowering may keep the optional search inside `May` while
    // leaving its correlated partition and shuffle as sibling effects in the
    // same iterated-player program. They still share explicit collection tags,
    // so treating those siblings as one rendering bundle does not infer any
    // relationship that the runtime model has not preserved.
    effects.extend(nested_followups.iter());
    effects.extend(sibling_followups.iter());
    let [
        search_effect,
        capture_effect,
        choose_effect,
        chosen_move_effect,
        remainder_move_effect,
        shuffle_effect,
    ] = effects.as_slice()
    else {
        return None;
    };

    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let capture = capture_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let chosen_move = unwrap_basic_tag_wrappers(chosen_move_effect)
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    let remainder_move = unwrap_basic_tag_wrappers(remainder_move_effect)
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    let shuffle = if let Some(shuffle) =
        shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
    {
        shuffle
    } else {
        let conditional = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
        let [shuffle] = conditional.then.as_slice() else {
            return None;
        };
        let shuffle = shuffle.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
        let correlated = remainder_move_effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .is_some_and(|with_id| with_id.id == conditional.condition);
        if !correlated
            || conditional.predicate != EffectPredicate::Happened
            || !conditional.else_.is_empty()
        {
            return None;
        }
        shuffle
    };

    if !search.is_search
        || search.chooser != PlayerFilter::IteratedPlayer
        || choose_search_zones(search).as_deref() != Some(&[Zone::Library])
        || search.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || capture.zone != Some(Zone::Library)
        || !capture.additional_zones.is_empty()
        || !tagged_library_partition_filter(&capture.filter, &search.tag, None, None)
        || !iterated_player_or_owner_of_tag(&choose.chooser, &search.tag)
        || choose_search_zones(choose).as_deref() != Some(&[Zone::Library])
        || !exact_count(&choose.count, 1)
        || !tagged_library_partition_filter(&choose.filter, &capture.tag, None, Some(&search.tag))
        || !chosen_move.tapped
        || chosen_move.face_down
        || chosen_move.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Put
        || chosen_move.battlefield_controller != crate::effects::BattlefieldController::You
        || !tagged_library_partition_filter(
            &chosen_move.filter,
            &choose.tag,
            None,
            Some(&search.tag),
        )
        || !remainder_move.tapped
        || remainder_move.face_down
        || remainder_move.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Put
        || remainder_move.battlefield_controller != crate::effects::BattlefieldController::Owner
        || !tagged_library_partition_filter(
            &remainder_move.filter,
            &capture.tag,
            Some(&choose.tag),
            Some(&choose.tag),
        )
        || shuffle.player != PlayerFilter::IteratedPlayer
    {
        return None;
    }

    let mut selection_filter = search.filter.clone();
    selection_filter.zone = None;
    selection_filter.owner = None;
    let selection_noun = if selection_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        describe_nonbattlefield_card_filter_without_zone(&selection_filter, Zone::Library)
    };
    let selection = if search.count.is_single() {
        with_indefinite_article(&selection_noun)
    } else {
        format!(
            "{} {}",
            describe_choice_count(&search.count),
            pluralize_noun_phrase(&selection_noun)
        )
    };

    Some(format!(
        "{subject} may search their library for {selection}. They each put one of those cards onto the battlefield tapped under your control and the rest onto the battlefield tapped under their control. Then each player who searched their library this way shuffles"
    ))
}

pub(super) fn describe_for_players_optional_search_battlefield_partition(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    describe_for_players_optional_search_battlefield_partition_with_followups(for_players, &[])
}

/// Sentence lowering can place only the optional search inside the player
/// loop while keeping its tag-correlated partition and shuffle as siblings.
pub(super) fn describe_optional_search_battlefield_partition_effects(
    effects: &[Effect],
) -> Option<String> {
    let (for_players_effect, followups) = effects.split_first()?;
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    describe_for_players_optional_search_battlefield_partition_with_followups(
        for_players,
        followups,
    )
}

pub(super) fn describe_for_players_search_library_then_shuffle(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let subject = describe_for_players_subject(&for_players.filter)?;
    let search_text = if let [sequence_effect] = for_players.effects.as_slice() {
        let sequence = structural_unwrap_render_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()?;
        describe_iterated_player_search_sequence(sequence)?
    } else {
        describe_iterated_player_search_effects(&for_players.effects)?
    };

    let rest = search_text.strip_prefix("Search ")?;
    let mut rest = lowercase_first(rest);
    if subject != "You" {
        rest = rest
            .replace(", reveal ", ", reveals ")
            .replace(". Reveal ", ". Reveals ")
            .replace(", put ", ", puts ")
            .replace(". Put ", ". Puts ")
            .replace(", then shuffle", ", then shuffles")
            .replace(". Then shuffle", ", then shuffles");
    }
    let verb = if subject == "You" {
        "search"
    } else {
        "searches"
    };
    Some(format!("{subject} {verb} {rest}"))
}

pub(in crate::compiled_text) fn describe_each_player_return_from_graveyard_to_hand(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let [return_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let return_from_gy = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if return_from_gy.random {
        return None;
    }
    let ChooseSpec::Object(filter) = return_from_gy.target.base() else {
        return None;
    };
    if filter.zone != Some(Zone::Graveyard) || filter.owner != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let target_text = describe_choose_spec_without_graveyard_zone(&return_from_gy.target);
    Some(format!(
        "Each player returns {target_text} from their graveyard to their hand"
    ))
}

pub(super) fn describe_each_player_choose_type_return_from_graveyard_to_hand(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    // The choose/return pair may arrive as two sibling effects or folded
    // into a single per-player SequenceEffect by sentence lowering.
    let effects: &[Effect] = match for_players.effects.as_slice() {
        [single] => {
            if let Some(sequence) = single.downcast_ref::<crate::effects::SequenceEffect>() {
                &sequence.effects
            } else {
                return None;
            }
        }
        effects => effects,
    };
    let [choose_effect, return_effect] = effects else {
        return None;
    };
    let choose_type = choose_effect.downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()?;
    if choose_type.chooser != PlayerFilter::IteratedPlayer
        || !choose_type.excluded_subtypes.is_empty()
    {
        return None;
    }
    let return_from_gy = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if return_from_gy.random {
        return None;
    }
    let ChooseSpec::Object(filter) = return_from_gy.target.base() else {
        return None;
    };
    if filter.zone != Some(Zone::Graveyard)
        || filter.owner != Some(PlayerFilter::IteratedPlayer)
        || !filter.chosen_creature_type
    {
        return None;
    }
    if !matches!(&return_from_gy.target, ChooseSpec::WithCount(_, count) if count.is_any_number()) {
        return None;
    }
    let target_text = describe_choose_spec_without_graveyard_zone(&return_from_gy.target)
        .replace("the chosen type", "that type");
    Some(format!(
        "Each player chooses a creature type and returns {target_text} from their graveyard to their hand"
    ))
}

#[derive(Clone, Copy)]
pub(crate) struct SacrificeView<'a> {
    pub(super) filter: &'a ObjectFilter,
    pub(super) count: &'a Value,
    pub(super) player: &'a PlayerFilter,
}

pub(super) fn sacrifice_view(effect: &Effect) -> Option<SacrificeView<'_>> {
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        return Some(SacrificeView {
            filter: &sacrifice.filter,
            count: &sacrifice.count,
            player: &sacrifice.player,
        });
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::zones::SacrificePlayerEffect>() {
        return Some(SacrificeView {
            filter: &sacrifice.filter,
            count: &sacrifice.count,
            player: &sacrifice.player,
        });
    }
    None
}

pub(super) fn sacrifice_view_unwrapped(effect: &Effect) -> Option<SacrificeView<'_>> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        sacrifice_view(&with_id.effect)
    } else {
        sacrifice_view(effect)
    }
}

pub(super) fn filter_is_exactly_tagged(filter: &ObjectFilter, tag: &crate::TagKey) -> bool {
    filter.tagged_constraints.len() == 1
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == *tag
        })
        && filter.zone.is_none()
        && filter.controller.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
}

pub(super) fn filter_is_exactly_one_tagged_object(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.len() == 1
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        && filter.zone.is_none()
        && filter.controller.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
}

pub(super) fn choose_spec_is_tagged_object(spec: &ChooseSpec, tag: &crate::TagKey) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter_is_exactly_tagged(filter, tag) || object_filter_has_tag(filter, tag)
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_is_tagged_object(inner, tag),
        ChooseSpec::SurfaceHinted { spec, .. } => choose_spec_is_tagged_object(spec, tag),
        _ => false,
    }
}

pub(super) fn exact_count(count: &ChoiceCount, expected: usize) -> bool {
    count.min == expected && count.max == Some(expected) && !count.dynamic_x && !count.random
}

pub(super) fn effect_outer_tag(effect: &Effect) -> Option<&crate::TagKey> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_outer_tag(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tag_all.tag);
    }
    None
}

pub(super) fn tagged_apply_continuous_effect(
    effect: &Effect,
) -> Option<&crate::effects::ApplyContinuousEffect> {
    unwrap_basic_tag_wrappers(effect).downcast_ref()
}

pub(super) fn apply_continuous_targets_tag(
    apply: &crate::effects::ApplyContinuousEffect,
    tag: &crate::TagKey,
) -> bool {
    matches!(apply.target_spec.as_ref(), Some(ChooseSpec::Tagged(candidate)) if candidate == tag)
}

pub(super) fn describe_return_then_color_subtype_addition_compact(
    effects: &[&Effect],
) -> Option<String> {
    let (return_effect, color_effect, subtype_effect, followup_effect) = match effects {
        [return_effect, color_effect, subtype_effect] => {
            (*return_effect, *color_effect, *subtype_effect, None)
        }
        [return_effect, color_effect, subtype_effect, followup_effect] => (
            *return_effect,
            *color_effect,
            *subtype_effect,
            Some(*followup_effect),
        ),
        _ => return None,
    };
    let returned_tag = effect_outer_tag(return_effect)?;
    let return_to_battlefield = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
    )?;

    let first_apply = tagged_apply_continuous_effect(color_effect)?;
    let second_apply = tagged_apply_continuous_effect(subtype_effect)?;
    if first_apply.until != second_apply.until
        || first_apply.condition != second_apply.condition
        || !apply_continuous_targets_tag(first_apply, returned_tag)
        || !apply_continuous_targets_tag(second_apply, returned_tag)
        || !first_apply.additional_modifications.is_empty()
        || !second_apply.additional_modifications.is_empty()
        || !first_apply.runtime_modifications.is_empty()
        || !second_apply.runtime_modifications.is_empty()
    {
        return None;
    }

    let (colors, subtypes) = match (&first_apply.modification, &second_apply.modification) {
        (
            Some(crate::continuous::Modification::AddColors(colors)),
            Some(crate::continuous::Modification::AddSubtypes(subtypes)),
        ) => (*colors, subtypes),
        (
            Some(crate::continuous::Modification::AddSubtypes(subtypes)),
            Some(crate::continuous::Modification::AddColors(colors)),
        ) => (*colors, subtypes),
        _ => return None,
    };
    if colors.is_empty() || subtypes.is_empty() {
        return None;
    }

    let mut text = describe_effect(return_effect)
        .trim_end_matches('.')
        .to_string();
    let subtype_words = subtypes
        .iter()
        .map(|subtype| subtype.display_name())
        .collect::<Vec<_>>()
        .join(" ");
    let descriptor = with_indefinite_article(&format!(
        "{} {subtype_words}",
        describe_token_color_words(colors, false)
    ));
    let followup_subject = if return_to_battlefield.target.count().is_single() {
        "That creature"
    } else {
        "Each of those creatures"
    };
    text.push_str(&format!(
        ". {followup_subject} is {descriptor} in addition to its other colors and types"
    ));
    if !matches!(first_apply.until, Until::Forever) {
        text.push(' ');
        text.push_str(&describe_until(&first_apply.until));
    }
    if let Some(followup) = followup_effect {
        let followup_text = describe_effect(followup);
        if !followup_text.trim().is_empty() {
            text.push_str(". ");
            text.push_str(followup_text.trim_end_matches('.'));
        }
    }
    Some(text)
}

pub(super) fn exact_random_count(count: &ChoiceCount, expected: usize) -> bool {
    count.min == expected
        && count.max == Some(expected)
        && !count.dynamic_x
        && !count.up_to_x
        && count.random
}

pub(super) fn single_you_graveyard_subtype_return(effect: &Effect) -> Option<Subtype> {
    let return_effect = unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if return_effect.random || !exact_count(&return_effect.target.count(), 1) {
        return None;
    }
    let ChooseSpec::Object(filter) = return_effect.target.base() else {
        return None;
    };
    let [subtype] = filter.subtypes.as_slice() else {
        return None;
    };
    let expected = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .with_subtype(*subtype);
    if filter != &expected {
        return None;
    }
    Some(*subtype)
}

pub(super) fn describe_return_each_subtype_card_from_your_graveyard(
    effects: &[Effect],
) -> Option<String> {
    if effects.len() < 2 {
        return None;
    }
    let subtypes = effects
        .iter()
        .map(single_you_graveyard_subtype_return)
        .collect::<Option<Vec<_>>>()?;
    let subtype_names = subtypes.iter().map(ToString::to_string).collect::<Vec<_>>();
    let [first, rest @ ..] = subtype_names.as_slice() else {
        return None;
    };
    let rest_text = join_with_and(rest);
    Some(format!(
        "Return a {first} card from your graveyard to your hand, then do the same for {rest_text}"
    ))
}

pub(super) fn describe_random_choose_then_destroy_rest(effects: &[Effect]) -> Option<String> {
    let [choose_effect, destroy_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let random_one = exact_random_count(&choose.count, 1);
    let up_to_one = choose.count.min == 0 && choose.count.max == Some(1) && !choose.count.random;
    if choose.chooser != PlayerFilter::You
        || !(random_one || up_to_one)
        || choose.count_value.is_some()
        || choose.is_search
        || choose.filter.zone != Some(Zone::Battlefield)
    {
        return None;
    }
    let destroy = unwrap_basic_tag_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    let excluded_count = destroy_filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.tag == choose.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        })
        .count();
    if excluded_count != 1 {
        return None;
    }
    let mut remaining_filter = destroy_filter.clone();
    remaining_filter.tagged_constraints.retain(|constraint| {
        constraint.tag != choose.tag
            || constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    });
    if remaining_filter != choose.filter {
        return None;
    }
    let mut selection = strip_indefinite_article(&choose.filter.description())
        .trim()
        .to_string();
    if let Some(rest) = selection.strip_suffix(" on the battlefield") {
        selection = rest.trim().to_string();
    }
    if selection.is_empty() {
        return None;
    }
    if random_one {
        Some(format!(
            "Choose {} at random, then destroy the rest",
            with_indefinite_article(&selection)
        ))
    } else {
        Some(format!("Choose up to one {selection}. Destroy the rest"))
    }
}

pub(super) fn simple_filter_plural_noun(filter: &ObjectFilter) -> Option<String> {
    if filter.card_types.len() == 1
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
        && filter.zone == Some(Zone::Battlefield)
        && filter.any_of.is_empty()
        && filter.tagged_constraints.is_empty()
    {
        let noun = filter.card_types[0].plural_name().to_ascii_lowercase();
        // A single color adjective keeps the compact noun ("green creatures",
        // "nonwhite enchantments"); anything richer needs the full describer.
        return match (filter.colors, filter.excluded_colors.count()) {
            (None, 0) => Some(noun),
            (Some(colors), 0) if colors.count() == 1 => {
                let color = crate::color::Color::ALL
                    .into_iter()
                    .find(|color| colors.contains(*color))?;
                Some(format!("{} {noun}", color.name().to_ascii_lowercase()))
            }
            (None, 1) => {
                let color = crate::color::Color::ALL
                    .into_iter()
                    .find(|color| filter.excluded_colors.contains(*color))?;
                Some(format!("non{} {noun}", color.name().to_ascii_lowercase()))
            }
            _ => None,
        };
    }
    None
}

pub(super) fn simple_filter_singular_noun(filter: &ObjectFilter) -> Option<String> {
    if filter.card_types.len() == 1
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.colors.is_none()
        && filter.any_of.is_empty()
        && filter.tagged_constraints.is_empty()
    {
        return Some(describe_card_type_word_local(filter.card_types[0]).to_string());
    }
    None
}

pub(super) fn value_is_power_of_tag(value: &Value, tag: &crate::TagKey) -> bool {
    matches!(
        value,
        Value::PowerOf(spec) if choose_spec_is_tagged_object(spec, tag)
    )
}

pub(super) fn described_counter_put_on_the_other(
    put: &crate::effects::PutCountersEffect,
) -> Option<String> {
    if put.target != ChooseSpec::AnyOtherTarget || put.target_count.is_some() || put.distributed {
        return None;
    }
    Some(format!(
        "put {} on the other",
        describe_put_counter_phrase(&put.amount, put.counter_type)
    ))
}

pub(super) fn describe_choose_two_move_one_put_counters_on_other(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, move_effect, put_effect] = effects else {
        return None;
    };
    let tagged = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCount(target, target_count) = &target_only.target else {
        return None;
    };
    if !exact_count(target_count, 2) {
        return None;
    }
    let ChooseSpec::Target(target_inner) = target.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = target_inner.as_ref() else {
        return None;
    };

    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Exile
        || !choose_spec_is_tagged_object(&move_to_zone.target, &tagged.tag)
    {
        return None;
    }
    if let ChooseSpec::WithCount(_, move_count) = &move_to_zone.target
        && !move_count.is_single()
    {
        return None;
    }

    let put = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    let put_text = described_counter_put_on_the_other(put)?;
    let plural_noun = simple_filter_plural_noun(target_filter)?;
    Some(format!(
        "Choose two target {plural_noun}. Exile one of those {plural_noun} and {put_text}"
    ))
}

pub(super) fn describe_choose_two_sacrifice_one_return_other(effects: &[Effect]) -> Option<String> {
    let [target_effect, sacrifice_effect, return_effect] = effects else {
        return None;
    };
    let tagged = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCount(target, target_count) = &target_only.target else {
        return None;
    };
    if !exact_count(target_count, 2) {
        return None;
    }
    let ChooseSpec::Target(target_inner) = target.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = target_inner.as_ref() else {
        return None;
    };
    let plural_noun = simple_filter_plural_noun(target_filter)?;

    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !choose_spec_is_tagged_object(&sacrifice.target, &tagged.tag) {
        return None;
    }
    let return_to_hand = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if return_to_hand.spec != ChooseSpec::AnyOtherTarget {
        return None;
    }

    Some(format!(
        "Choose two target {plural_noun}. Their controller sacrifices one of them. Return the other to its owner's hand"
    ))
}

pub(super) fn describe_choose_same_controller_sacrifice_one_return_other(
    effects: &[Effect],
) -> Option<String> {
    let [
        target_effect,
        choose_effect,
        sacrifice_effect,
        return_effect,
    ] = effects
    else {
        return None;
    };
    let tagged = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let target_count = target_only.target.count();
    if !exact_count(&target_count, 2) {
        return None;
    }
    let ChooseSpec::Object(target_filter) = target_only.target.base() else {
        return None;
    };
    if !target_filter.target_set_same_controller {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.is_search
        || choose_primary_zone(choose).is_some_and(|zone| zone != Zone::Battlefield)
        || !is_tagged_only_filter(&choose.filter, &tagged.tag)
        || !matches!(
            &choose.chooser,
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag == &tagged.tag
        )
    {
        return None;
    }

    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !choose_spec_is_tagged_object(&sacrifice.target, &choose.tag) {
        return None;
    }

    let return_to_hand = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    let ChooseSpec::Object(return_filter) = return_to_hand.spec.base() else {
        return None;
    };
    if !is_tagged_only_filter_except_tag(return_filter, &tagged.tag, &choose.tag) {
        return None;
    }

    let plural_noun = simple_filter_plural_noun(target_filter)?;
    Some(format!(
        "Choose two target {plural_noun} controlled by the same player. Their controller chooses and sacrifices one of them. Return the other to its owner's hand"
    ))
}

pub(super) fn is_face_up_exiled_cards_you_own_filter(filter: &ObjectFilter) -> bool {
    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized.owner = None;
    normalized.face_down = None;
    filter.zone == Some(Zone::Exile)
        && filter.owner == Some(PlayerFilter::You)
        && filter.face_down == Some(false)
        && normalized == ObjectFilter::default()
}

pub(super) fn is_all_cards_from_your_library_filter(filter: &ObjectFilter) -> bool {
    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized.owner = None;
    filter.zone == Some(Zone::Library)
        && filter.owner == Some(PlayerFilter::You)
        && normalized == ObjectFilter::default()
}

pub(crate) fn describe_choose_exiled_cards_exile_library_put_chosen_on_top(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, exile_effect, move_effect] = effects else {
        return None;
    };

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let max = choose.count.max?;
    if choose.count != ChoiceCount::up_to(max)
        || choose.count_value.is_some()
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.zone != Some(Zone::Exile)
        || !choose.additional_zones.is_empty()
        || !is_face_up_exiled_cards_you_own_filter(&choose.filter)
    {
        return None;
    }

    let exile =
        unwrap_basic_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(library_filter) = exile.spec.unhinted() else {
        return None;
    };
    if exile.face_down || !is_all_cards_from_your_library_filter(library_filter) {
        return None;
    }

    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Library
        || !move_to_zone.to_top
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.transfer_exiled_with_source_links
        || !choose_spec_is_tagged_object(&move_to_zone.target, &choose.tag)
    {
        return None;
    }

    let count = number_word(max as i32).unwrap_or_else(|| max.to_string());
    Some(format!(
        "Choose up to {count} face-up exiled cards you own. Exile all the cards from your library, then put the chosen cards on top of your library"
    ))
}

pub(super) fn is_tagged_only_filter_except_tag(
    filter: &ObjectFilter,
    included_tag: &TagKey,
    excluded_tag: &TagKey,
) -> bool {
    let mut without_tag_constraints = filter.clone();
    without_tag_constraints.tagged_constraints.clear();
    without_tag_constraints == ObjectFilter::default()
        && filter.tagged_constraints.len() == 2
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *included_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *excluded_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        })
}

pub(super) fn is_tagged_power_damage_to_iterated_object(
    effect: &Effect,
    tag: &crate::TagKey,
) -> bool {
    let Some(damage) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return false;
    };
    matches!(damage.target, ChooseSpec::Iterated) && value_is_power_of_tag(&damage.amount, tag)
}

pub(super) fn is_tagged_power_damage_to_iterated_player(
    effect: &Effect,
    tag: &crate::TagKey,
) -> bool {
    let Some(damage) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::DealDamageEffect>()
    else {
        return false;
    };
    matches!(
        damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) && value_is_power_of_tag(&damage.amount, tag)
}

pub(super) fn describe_choose_sacrifice_power_damage_each(effects: &[Effect]) -> Option<String> {
    let [
        choose_effect,
        sacrifice_effect,
        for_each_effect,
        for_players_effect,
    ] = effects
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
    {
        return None;
    }
    let sacrificed_noun = simple_filter_singular_noun(&choose.filter)?;

    let sacrifice = sacrifice_view_unwrapped(sacrifice_effect)?;
    if sacrifice.player != &PlayerFilter::You
        || !matches!(sacrifice.count, Value::Fixed(1))
        || !object_filter_has_tag(sacrifice.filter, &choose.tag)
    {
        return None;
    }

    let for_each = unwrap_basic_tag_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachObject>()?;
    let [object_damage_effect] = for_each.effects.as_slice() else {
        return None;
    };
    if !is_tagged_power_damage_to_iterated_object(object_damage_effect, &choose.tag) {
        return None;
    }
    let object_text = damage_each_creature_filter_text(&for_each.filter)?;

    let for_players = unwrap_basic_tag_wrappers(for_players_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let [player_damage_effect] = for_players.effects.as_slice() else {
        return None;
    };
    if !is_tagged_power_damage_to_iterated_player(player_damage_effect, &choose.tag) {
        return None;
    }

    Some(format!(
        "Sacrifice a {sacrificed_noun}. This deals damage equal to that {sacrificed_noun}'s power to {object_text} and each player"
    ))
}

pub(super) fn describe_choose_sacrifice_then_draw_for_sacrificed(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, draw_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let with_id = sacrifice_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sacrifice = sacrifice_view(&with_id.effect)?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;

    if choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose.count.dynamic_x
        || choose.count.up_to_x
        || choose.count.random
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || sacrifice.player != &PlayerFilter::You
        || draw.player != PlayerFilter::You
        || !filter_is_exactly_tagged(sacrifice.filter, &choose.tag)
    {
        return None;
    }

    let Value::Count(count_filter) = sacrifice.count else {
        return None;
    };
    if !filter_is_exactly_tagged(count_filter, &choose.tag) {
        return None;
    }
    if !is_effect_count_reference(&draw.count, Some(with_id.id)) {
        return None;
    }

    let selection = describe_sacrifice_choice_selection(choose);

    if draw.count.has_surface_hint(ValueSurfaceHint::ThatManyCards) {
        return Some(format!("Sacrifice {selection}, then draw that many cards"));
    }

    Some(format!(
        "Sacrifice {selection}. Draw a card for each permanent sacrificed this way"
    ))
}

pub(super) fn normalized_without_zone_controller(filter: &ObjectFilter) -> ObjectFilter {
    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized.controller = None;
    normalized
}

pub(super) fn filter_matches_ignoring_zone_controller(
    filter: &ObjectFilter,
    expected: ObjectFilter,
) -> bool {
    normalized_without_zone_controller(filter) == normalized_without_zone_controller(&expected)
}

pub(super) fn filter_is_artifact_enchantment_or_token(filter: &ObjectFilter) -> bool {
    let mut base = normalized_without_zone_controller(filter);
    let branches = std::mem::take(&mut base.any_of);
    base == ObjectFilter::default()
        && branches.len() == 3
        && branches
            .iter()
            .any(|branch| filter_matches_ignoring_zone_controller(branch, ObjectFilter::artifact()))
        && branches.iter().any(|branch| {
            filter_matches_ignoring_zone_controller(branch, ObjectFilter::enchantment())
        })
        && branches.iter().any(|branch| {
            filter_matches_ignoring_zone_controller(branch, ObjectFilter::default().token())
        })
}

pub(super) fn describe_sacrifice_choice_selection(
    choose: &crate::effects::ChooseObjectsEffect,
) -> String {
    if choose.count.is_any_number() && filter_is_artifact_enchantment_or_token(&choose.filter) {
        return "any number of artifacts, enchantments, and/or tokens".to_string();
    }

    let mut selection = describe_choose_selection(choose);
    if let Some(rest) = selection.strip_prefix("any number ") {
        selection = format!("any number of {rest}");
    }
    if let Some(rest) = selection.strip_suffix(" you control") {
        selection = rest.to_string();
    }
    if choose.filter.card_types.len() > 1 {
        selection = selection.replace(", or ", ", and/or ");
    }
    selection
}

pub(super) fn is_creature_card_filter_from_your_graveyard(filter: &ObjectFilter) -> bool {
    if filter.owner != Some(PlayerFilter::You) {
        return false;
    }
    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized.owner = None;
    filter_matches_ignoring_zone_controller(&normalized, ObjectFilter::creature())
}

pub(super) fn describe_choose_sacrifice_then_return_from_graveyard(
    effects: &[&Effect],
) -> Option<String> {
    let [
        choose_effect,
        sacrifice_effect,
        return_choose_effect,
        return_effect,
    ] = effects
    else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let with_id = sacrifice_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sacrifice = sacrifice_view(&with_id.effect)?;
    let return_choose =
        return_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let return_to_battlefield = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
    )?;

    if choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::You
        || !choose.count.is_any_number()
        || choose.count_value.is_some()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || sacrifice.player != &PlayerFilter::You
        || !filter_is_exactly_tagged(sacrifice.filter, &choose.tag)
    {
        return None;
    }

    let Value::Count(count_filter) = sacrifice.count else {
        return None;
    };
    if !filter_is_exactly_tagged(count_filter, &choose.tag) {
        return None;
    }

    if return_choose.is_search
        || return_choose.reveal
        || return_choose.chooser != PlayerFilter::You
        || !return_choose.count.is_dynamic_x()
        || return_choose.count.is_up_to_dynamic_x()
        || return_choose.count.is_random()
        || choose_primary_zone(return_choose) != Some(Zone::Graveyard)
        || !return_choose
            .count_value
            .as_ref()
            .is_some_and(|value| is_effect_count_reference(value, Some(with_id.id)))
        || !is_creature_card_filter_from_your_graveyard(&return_choose.filter)
        || return_to_battlefield.tapped
        || return_to_battlefield.as_aura.is_some()
        || !choose_spec_is_tagged_object(&return_to_battlefield.target, &return_choose.tag)
    {
        return None;
    }

    let selection = describe_sacrifice_choice_selection(choose);
    Some(format!(
        "Sacrifice {selection}. Return that many creature cards from your graveyard to the battlefield"
    ))
}

pub(super) fn describe_sacrifice_return_from_graveyard_then_exile_source_bundle(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, return_effect, exile_effect] = effects else {
        return None;
    };
    let choose = unwrap_basic_tag_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(unwrap_basic_tag_wrappers(sacrifice_effect))?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || sacrifice.player != &PlayerFilter::You
        || !matches!(sacrifice.count, Value::Fixed(1))
        || !filter_is_exactly_tagged(sacrifice.filter, &choose.tag)
    {
        return None;
    }

    unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    let exile = unwrap_basic_tag_wrappers(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile.zone != Zone::Exile || !matches!(exile.target.base(), ChooseSpec::Source) {
        return None;
    }

    let sacrifice_text = format!("Sacrifice {}", describe_sacrifice_choice_selection(choose));
    let return_text = describe_effect(return_effect);
    let exile_text = describe_effect(exile_effect);
    Some(format!(
        "{sacrifice_text}. {}. {}",
        return_text.trim_end_matches('.'),
        exile_text.trim_end_matches('.')
    ))
}

pub(super) fn value_is_affected_count_for_effect(
    value: &Value,
    id: crate::effect::EffectId,
) -> bool {
    let value = value.unhinted();
    matches!(
        value,
        Value::EffectMetric {
            effect_id,
            source: crate::effect::EffectMetricSource::AffectedObjects,
            metric:
                crate::effect::EffectMetric::Count | crate::effect::EffectMetric::AffectedCount,
        } if *effect_id == id
    )
}

pub(super) fn value_is_twice_affected_count_for_effect(
    value: &Value,
    id: crate::effect::EffectId,
) -> bool {
    let Value::Add(left, right) = value else {
        return false;
    };
    value_is_affected_count_for_effect(left, id) && value_is_affected_count_for_effect(right, id)
}

pub(super) fn describe_choose_sacrifice_then_gain_life_for_sacrificed(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, sacrifice_effect, gain_life_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let with_id = sacrifice_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sacrifice = sacrifice_view(&with_id.effect)?;
    let gain_life = gain_life_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;

    if choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose.count.dynamic_x
        || choose.count.up_to_x
        || choose.count.random
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || sacrifice.player != &PlayerFilter::You
        || gain_life.player != ChooseSpec::Player(PlayerFilter::You)
        || !filter_is_exactly_tagged(sacrifice.filter, &choose.tag)
        || !value_is_twice_affected_count_for_effect(&gain_life.amount, with_id.id)
    {
        return None;
    }

    let Value::Count(count_filter) = sacrifice.count else {
        return None;
    };
    if !filter_is_exactly_tagged(count_filter, &choose.tag) {
        return None;
    }

    let selection = describe_sacrifice_choice_selection(choose);

    Some(format!(
        "Sacrifice {selection}. You gain 2 life for each permanent sacrificed this way"
    ))
}

pub(super) fn tagged_target_only_effect(
    effect: &Effect,
) -> Option<(&crate::TagKey, &crate::effects::TargetOnlyEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if let Some(target_only) = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    {
        return Some((&tagged.tag, target_only));
    }

    // Shared collection tags may wrap an independently tagged target slot.
    // Render relationships through the innermost tag attached directly to
    // the declaration, not through the collection tag shared by its peers.
    tagged_target_only_effect(&tagged.effect)
}

pub(super) fn plural_tagged_target_reference(target: &ChooseSpec) -> Option<String> {
    if !choose_spec_is_plural(target) {
        return None;
    }

    let base = target.base();
    if let ChooseSpec::Object(filter) = base {
        let description = filter.description();
        let noun = strip_leading_article(&description);
        return Some(format!("each of those {}", pluralize_noun_phrase(noun)));
    }

    Some("each of those targets".to_string())
}

pub(super) fn plural_tagged_target_collection_reference(target: &ChooseSpec) -> Option<String> {
    if !choose_spec_is_plural(target) {
        return None;
    }

    let base = target.base();
    if let ChooseSpec::Object(filter) = base {
        let description = filter.description();
        let noun = strip_leading_article(&description);
        return Some(format!("those {}", pluralize_noun_phrase(noun)));
    }

    Some("those targets".to_string())
}

pub(super) fn describe_target_set_then_apply_continuous(effects: &[&Effect]) -> Option<String> {
    let [target_effect, apply_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let apply = unwrap_basic_tag_wrappers(apply_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(apply.target_spec.as_ref(), Some(ChooseSpec::Tagged(tag)) if tag == target_tag) {
        return None;
    }

    let target_reference = plural_tagged_target_reference(&target_only.target)?;
    let clauses = describe_apply_continuous_clauses(apply, false);
    if clauses.is_empty() {
        return None;
    }

    let mut followup = format!("{target_reference} {}", join_with_and(&clauses));
    if let Some(tail) = describe_apply_continuous_tail(apply) {
        followup.push(' ');
        followup.push_str(&tail);
    }

    Some(format!(
        "Choose {}. {}",
        describe_choose_spec(&target_only.target),
        capitalize_first(&followup)
    ))
}

pub(super) fn tagged_for_each_object_effect(
    effect: &Effect,
) -> Option<(&crate::TagKey, &crate::effects::ForEachObject)> {
    let tag = wrapped_effect_tag(effect)?;
    let for_each = unwrap_basic_tag_wrappers(effect).downcast_ref()?;
    Some((tag, for_each))
}

pub(super) fn tagged_set_subject_for_filter(filter: &ObjectFilter) -> &'static str {
    if filter.card_types.contains(&CardType::Creature) {
        "Those creatures"
    } else if filter.card_types.contains(&CardType::Artifact) {
        "Those artifacts"
    } else if filter.card_types.contains(&CardType::Enchantment) {
        "Those enchantments"
    } else if filter.card_types.contains(&CardType::Land) {
        "Those lands"
    } else if filter.card_types.contains(&CardType::Planeswalker) {
        "Those planeswalkers"
    } else {
        "Those objects"
    }
}

pub(super) fn describe_tagged_for_each_put_counters(
    effect: &Effect,
) -> Option<(&crate::TagKey, String, &'static str)> {
    let (tag, for_each) = tagged_for_each_object_effect(effect)?;
    let [put_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let put = unwrap_basic_tag_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() || !matches!(put.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let first_clause = if this_way_back_reference_filter(&for_each.filter) {
        format!(
            "Put {} on each of them",
            describe_put_counter_phrase(&put.amount, put.counter_type)
        )
    } else {
        let description = for_each.filter.description();
        let filter_text = strip_indefinite_article(&description);
        format!(
            "Put {} on each {filter_text}",
            describe_put_counter_phrase(&put.amount, put.counter_type)
        )
    };
    Some((
        tag,
        first_clause,
        tagged_set_subject_for_filter(&for_each.filter),
    ))
}

pub(super) fn describe_for_each_double_stat(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    let [apply_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let apply = unwrap_basic_tag_wrappers(apply_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || !matches!(apply.target_spec.as_ref(), Some(ChooseSpec::Iterated))
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = apply.runtime_modifications.as_slice()
    else {
        return None;
    };
    let stat = match (power.unhinted(), toughness.unhinted()) {
        (Value::PowerOf(spec), Value::Fixed(0))
            if matches!(spec.unhinted(), ChooseSpec::Iterated) =>
        {
            "power"
        }
        (Value::Fixed(0), Value::ToughnessOf(spec))
            if matches!(spec.unhinted(), ChooseSpec::Iterated) =>
        {
            "toughness"
        }
        (Value::PowerOf(power_spec), Value::ToughnessOf(toughness_spec))
            if matches!(power_spec.unhinted(), ChooseSpec::Iterated)
                && matches!(toughness_spec.unhinted(), ChooseSpec::Iterated) =>
        {
            "power and toughness"
        }
        _ => return None,
    };

    let filter_text = describe_for_each_filter(&for_each.filter);
    Some(format!(
        "Double the {stat} of each {filter_text} until end of turn"
    ))
}

pub(super) fn describe_tagged_for_each_double_stat(
    effect: &Effect,
) -> Option<(&crate::TagKey, String, &'static str)> {
    let (tag, for_each) = tagged_for_each_object_effect(effect)?;
    let first_clause = describe_for_each_double_stat(for_each)?;
    Some((
        tag,
        first_clause,
        tagged_set_subject_for_filter(&for_each.filter),
    ))
}

pub(super) fn describe_tagged_for_each_then_apply_continuous(
    effects: &[&Effect],
) -> Option<String> {
    let [set_effect, apply_effect] = effects else {
        return None;
    };
    let (set_tag, first_clause, subject) = describe_tagged_for_each_put_counters(set_effect)
        .or_else(|| describe_tagged_for_each_double_stat(set_effect))?;
    let apply = tagged_apply_continuous_effect(apply_effect)?;
    if !apply_continuous_targets_tag(apply, set_tag) {
        return None;
    }

    let clauses = describe_apply_continuous_clauses(apply, true);
    if clauses.is_empty() {
        return None;
    }
    let mut followup = format!("{subject} {}", join_with_and(&clauses));
    if let Some(tail) = describe_apply_continuous_tail(apply) {
        followup.push(' ');
        followup.push_str(&tail);
    }

    Some(format!("{first_clause}. {followup}"))
}

pub(super) fn describe_target_set_then_return_to_hand(effects: &[&Effect]) -> Option<String> {
    let [target_effect, return_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let return_to_hand = return_effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .or_else(|| {
            return_effect
                .downcast_ref::<crate::effects::TaggedEffect>()?
                .effect
                .downcast_ref::<crate::effects::ReturnToHandEffect>()
        })?;
    if !return_to_hand_uses_chosen_tag(return_to_hand, target_tag.as_str()) {
        return None;
    }

    let target_reference = plural_tagged_target_collection_reference(&target_only.target)?;
    Some(format!(
        "Choose {}. Return {target_reference} to their owners' hands",
        describe_choose_spec(&target_only.target)
    ))
}

pub(super) fn destroy_random_one_of_tagged_groups(
    destroy: &crate::effects::DestroyEffect,
    tags: &[&crate::TagKey],
) -> bool {
    let ChooseSpec::WithCount(inner, count) = &destroy.spec else {
        return false;
    };
    if !count.is_single() || !count.is_random() {
        return false;
    }
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return false;
    };
    filter.any_of.len() == tags.len()
        && tags.iter().all(|tag| {
            filter
                .any_of
                .iter()
                .any(|candidate| is_tagged_only_filter(candidate, tag))
        })
}

pub(super) fn describe_target_groups_then_random_destroy(effects: &[&Effect]) -> Option<String> {
    let [first_target, second_target, destroy_effect] = effects else {
        return None;
    };
    let (first_tag, first_target) = tagged_target_only_effect(first_target)?;
    let (second_tag, second_target) = tagged_target_only_effect(second_target)?;
    if first_tag == second_tag {
        return None;
    }
    let destroy = destroy_effect.downcast_ref::<crate::effects::DestroyEffect>()?;
    if !destroy_random_one_of_tagged_groups(destroy, &[first_tag, second_tag]) {
        return None;
    }

    Some(format!(
        "Choose {} and {}. Destroy one of them at random",
        describe_choose_spec(&first_target.target),
        describe_choose_spec(&second_target.target)
    ))
}

pub(super) fn is_tagged_only_filter(filter: &ObjectFilter, tag: &crate::TagKey) -> bool {
    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized == ObjectFilter::tagged(tag.clone())
}

pub(super) fn describe_target_then_look_at_tagged_object(effects: &[&Effect]) -> Option<String> {
    let [target_effect, look_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let look = look_effect.downcast_ref::<crate::effects::LookAtObjectsEffect>()?;
    if look.viewer != PlayerFilter::You || !is_tagged_only_filter(&look.filter, target_tag) {
        return None;
    }
    Some(format!(
        "Look at {}",
        describe_choose_spec(&target_only.target)
    ))
}

pub(super) fn describe_choose_same_controller_targets_then_sacrifice_one(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, choose_effect, sacrifice_effect] = effects else {
        return None;
    };
    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    let target_count = target_only.target.count();
    if target_count.min != 2
        || target_count.max != Some(2)
        || target_count.dynamic_x
        || target_count.random
    {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.is_search
        || choose_primary_zone(choose).is_some_and(|zone| zone != Zone::Battlefield)
        || !is_tagged_only_filter(&choose.filter, target_tag)
        || !matches!(
            &choose.chooser,
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag == target_tag
        )
    {
        return None;
    }

    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(&sacrifice.target, ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }

    let target_text = describe_choose_spec(&target_only.target);
    let target_text = if target_text.contains("controlled by the same player") {
        target_text
    } else {
        format!("{target_text} controlled by the same player")
    };
    Some(format!(
        "Choose {target_text}. That player sacrifices one of them of their choice"
    ))
}

pub(super) fn describe_search_name_conditional_put_then_shuffle(
    effects: &[&Effect],
) -> Option<String> {
    fn is_creature_condition(condition: &Condition, tag: &TagKey) -> bool {
        let Condition::TaggedObjectMatches(found_tag, filter) = condition else {
            return false;
        };
        found_tag == tag
            && filter.card_types == vec![CardType::Creature]
            && filter.tagged_constraints.is_empty()
            && filter.name.is_none()
    }

    fn is_not_named_condition(condition: &Condition, tag: &TagKey, name_tag: &TagKey) -> bool {
        let Condition::Not(inner) = condition else {
            return false;
        };
        let Condition::TaggedObjectMatches(found_tag, filter) = inner.as_ref() else {
            return false;
        };
        found_tag == tag
            && filter.tagged_constraints.len() == 1
            && filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *name_tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
            })
    }

    let [
        search_effect,
        name_effect,
        conditional_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !search.is_search
        || !search.count.is_single()
        || search.chooser != PlayerFilter::You
        || choose_primary_zone(search) != Some(Zone::Library)
        || search.filter.zone != Some(Zone::Library)
        || search.filter.owner != Some(PlayerFilter::DamagedPlayer)
    {
        return None;
    }

    let name = name_effect.downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    if name.chooser != PlayerFilter::DamagedPlayer || name.filter.is_some() {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::And(left, right) = &conditional.condition else {
        return None;
    };
    let condition_matches = (is_creature_condition(left, &search.tag)
        && is_not_named_condition(right, &search.tag, &name.tag))
        || (is_creature_condition(right, &search.tag)
            && is_not_named_condition(left, &search.tag, &name.tag));
    if !condition_matches || !conditional.if_false.is_empty() {
        return None;
    }

    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &search.tag)
    {
        return None;
    }

    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::DamagedPlayer {
        return None;
    }

    Some("Search that player's library for a card, then that player chooses a card name. If you searched for a creature card that doesn't have that name, you may put it onto the battlefield under your control. Then that player shuffles".to_string())
}

pub(super) fn describe_may_cast_target_graveyard_spell_then_exile_replacement(
    effects: &[&Effect],
) -> Option<String> {
    if let Some((prefix, consumed)) =
        describe_duration_scoped_targeted_graveyard_cast_replacement(effects)
    {
        if consumed == effects.len() {
            return Some(prefix);
        }
        let [suffix] = &effects[consumed..] else {
            return None;
        };
        let rendered_suffix = describe_effect(suffix);
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(rendered_suffix.trim_end_matches('.'))
        ));
    }

    if let [_, may_effect, _, followup_effect] = effects {
        let prefix =
            describe_may_cast_target_graveyard_spell_then_exile_replacement(&effects[..3])?;
        let cast_with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        let followup = followup_effect.downcast_ref::<crate::effects::IfEffect>()?;
        let crate::effect::EffectPredicate::PriorEffectResult(surface) = &followup.predicate else {
            return None;
        };
        if followup.condition != cast_with_id.id
            || surface.action != crate::effect::PriorEffectAction::Cast
            || surface.actor != crate::effect::PriorEffectResultActor::You
            || surface.quantifier != crate::effect::PriorEffectResultQuantifier::One
            || surface.filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
            || !followup.else_.is_empty()
        {
            return None;
        }
        let rendered_followup = describe_effect(followup_effect);
        return Some(format!(
            "{}. {}",
            prefix.trim_end_matches('.'),
            capitalize_first(rendered_followup.trim_end_matches('.'))
        ));
    }

    if let Some(text) = describe_targeted_graveyard_cast_with_gated_replacement(effects) {
        return Some(text);
    }

    let [choose_effect, may_effect, replacement_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.is_search
        || !choose.count.is_single()
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Graveyard)
        || choose.filter.zone != Some(Zone::Graveyard)
        || !graveyard_cast_card_types_are_supported(&choose.filter.card_types)
    {
        return None;
    }
    let graveyard_text = describe_targeted_cast_graveyard(&choose.filter.owner)?;

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != choose.tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    if !is_chosen_spell_graveyard_exile_replacement(replacement_effect, &choose.tag) {
        return None;
    }

    let mana_value_text = match &choose.filter.mana_value {
        Some(crate::filter::Comparison::LessThanOrEqual(limit)) => {
            format!(" with mana value {limit} or less")
        }
        Some(crate::filter::Comparison::LessThanOrEqualExpr(limit)) => format!(
            " with mana value less than or equal to {}",
            describe_value(limit)
        ),
        None => String::new(),
        _ => return None,
    };
    let free_cast_text = if cast.without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };

    let card_types = describe_graveyard_cast_card_types(&choose.filter.card_types)?;
    Some(format!(
        "You may cast target {card_types} card{mana_value_text} from {graveyard_text}{free_cast_text}. If that spell would be put into {graveyard_text}, exile it instead"
    ))
}

fn describe_duration_scoped_targeted_graveyard_cast_replacement(
    effects: &[&Effect],
) -> Option<(String, usize)> {
    let [target_effect, grant_effect, rest @ ..] = effects else {
        return None;
    };
    let target_tag = wrapped_effect_tag(target_effect)?;
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target_only.target.is_target() {
        return None;
    }
    let ChooseSpec::Object(filter) = target_only.target.base() else {
        return None;
    };
    let mut plain_filter = filter.clone();
    let zone = plain_filter.zone.take();
    let owner = plain_filter.owner.take();
    let card_types = std::mem::take(&mut plain_filter.card_types);
    let mana_value = plain_filter.mana_value.take();
    let colors = plain_filter.colors.take();
    if zone != Some(Zone::Graveyard) || plain_filter != ObjectFilter::default() {
        return None;
    }
    let graveyard_text = describe_targeted_cast_graveyard(&owner)?;
    let card_types_text = describe_graveyard_cast_card_types(&card_types)?;

    let grant = structural_unwrap_render_wrappers(grant_effect)
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let surface = grant.surface.as_ref()?;
    if &grant.tag != target_tag
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || grant.allow_land
        || grant.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
        || grant.while_on_top_of_library
        || grant.filter.is_some()
        || grant.during_turns_counter_put_on_source.is_some()
        || grant.cast_pool_is_plural
        || !surface.leading_duration
        || surface.object != Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard)
        || surface.mana_reference.is_some()
    {
        return None;
    }

    let mut replacement_idx = 0;
    let without_paying_mana_cost = rest.first().is_some_and(|effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()
            .is_some_and(|free_cast| {
                free_cast.tag == grant.tag
                    && free_cast.player == grant.player
                    && free_cast.duration == grant.duration
                    && !free_cast.while_on_top_of_library
                    && free_cast.zone.is_none()
            })
    });
    if without_paying_mana_cost {
        replacement_idx += 1;
    }
    let replacement = structural_unwrap_render_wrappers(*rest.get(replacement_idx)?)
        .downcast_ref::<crate::effects::RegisterFutureZoneReplacementEffect>()?;
    if replacement.filter != ObjectFilter::tagged(target_tag.clone()).in_zone(Zone::Stack)
        || replacement.from_zone != Some(Zone::Stack)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || replacement.mode != crate::effects::ReplacementApplyMode::UntilEndOfTurn
        || replacement.cause_filter.is_some()
        || replacement.require_cause_source_match
        || replacement.link_exiled_to_source
    {
        return None;
    }

    let mana_value_text = match mana_value {
        Some(crate::filter::Comparison::LessThanOrEqual(limit)) => {
            format!(" with mana value {limit} or less")
        }
        Some(crate::filter::Comparison::LessThanOrEqualExpr(limit)) => format!(
            " with mana value less than or equal to {}",
            describe_value(&limit)
        ),
        None => String::new(),
        _ => return None,
    };
    let color_text = colors
        .map(describe_filter_color_alternatives)
        .filter(|colors| !colors.is_empty())
        .map(|colors| format!("{colors} "))
        .unwrap_or_default();
    let free_cast_text = if without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };
    Some((
        format!(
            "Until end of turn, you may cast target {color_text}{card_types_text} card{mana_value_text} from {graveyard_text}{free_cast_text}. If that spell would be put into {graveyard_text}, exile it instead"
        ),
        3 + usize::from(without_paying_mana_cost),
    ))
}

fn graveyard_cast_card_types_are_supported(card_types: &[CardType]) -> bool {
    !card_types.is_empty()
        && (card_types.contains(&CardType::Instant) || card_types.contains(&CardType::Sorcery))
        && card_types.iter().all(|card_type| {
            matches!(
                card_type,
                CardType::Artifact | CardType::Instant | CardType::Sorcery
            )
        })
}

fn describe_targeted_cast_graveyard(owner: &Option<PlayerFilter>) -> Option<&'static str> {
    match owner {
        Some(PlayerFilter::You) => Some("your graveyard"),
        None => Some("a graveyard"),
        _ => None,
    }
}

fn describe_graveyard_cast_card_types(card_types: &[CardType]) -> Option<String> {
    if !graveyard_cast_card_types_are_supported(card_types) {
        return None;
    }
    Some(join_with_or(
        &card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>(),
    ))
}

fn describe_targeted_graveyard_cast_with_gated_replacement(effects: &[&Effect]) -> Option<String> {
    let [target_effect, may_effect, if_effect] = effects else {
        return None;
    };
    let target_tag = wrapped_effect_tag(target_effect)?;
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target_only.target.is_target() {
        return None;
    }
    let ChooseSpec::Object(filter) = target_only.target.base() else {
        return None;
    };
    let mut plain_filter = filter.clone();
    let zone = plain_filter.zone.take();
    let owner = plain_filter.owner.take();
    let card_types = std::mem::take(&mut plain_filter.card_types);
    let mana_value = plain_filter.mana_value.take();
    let colors = plain_filter.colors.take();
    if zone != Some(Zone::Graveyard) || plain_filter != ObjectFilter::default() {
        return None;
    }
    let graveyard_text = describe_targeted_cast_graveyard(&owner)?;
    let card_types_text = describe_graveyard_cast_card_types(&card_types)?;

    let may_with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = may_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.is_some() || may.effects.len() != 1 {
        return None;
    }
    let cast_effect = &may.effects[0];
    let cast_spell_tag = wrapped_effect_tag(cast_effect)?;
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if &cast.tag != target_tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != may_with_id.id
        || if_effect.predicate != crate::effect::EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [replacement_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let replacement = structural_unwrap_render_wrappers(replacement_effect)
        .downcast_ref::<crate::effects::RegisterFutureZoneReplacementEffect>()?;
    if replacement.filter != ObjectFilter::tagged(cast_spell_tag.clone()).in_zone(Zone::Stack)
        || replacement.from_zone != Some(Zone::Stack)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || replacement.mode != crate::effects::ReplacementApplyMode::OneShot
        || replacement.cause_filter.is_some()
        || replacement.require_cause_source_match
        || replacement.link_exiled_to_source
    {
        return None;
    }

    let mana_value_text = match mana_value {
        Some(crate::filter::Comparison::LessThanOrEqual(limit)) => {
            format!(" with mana value {limit} or less")
        }
        Some(crate::filter::Comparison::LessThanOrEqualExpr(limit)) => format!(
            " with mana value less than or equal to {}",
            describe_value(&limit)
        ),
        None => String::new(),
        _ => return None,
    };
    let free_cast_text = if cast.without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };
    let color_text = colors
        .map(describe_filter_color_alternatives)
        .filter(|colors| !colors.is_empty())
        .map(|colors| format!("{colors} "))
        .unwrap_or_default();
    Some(format!(
        "You may cast target {color_text}{card_types_text} card{mana_value_text} from {graveyard_text}{free_cast_text}. If that spell would be put into {graveyard_text}, exile it instead"
    ))
}

pub(super) fn is_chosen_spell_graveyard_exile_replacement(effect: &Effect, tag: &TagKey) -> bool {
    if let Some(replacement) =
        effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()
    {
        return matches!(replacement.target.base(), ChooseSpec::Tagged(candidate) if candidate == tag)
            && replacement.from_zone == Some(Zone::Stack)
            && replacement.to_zone == Some(Zone::Graveyard)
            && replacement.replacement_zone == Zone::Exile
            && replacement.mode == crate::effects::ReplacementApplyMode::OneShot;
    }

    if let Some(replacement) =
        effect.downcast_ref::<crate::effects::RegisterFutureZoneReplacementEffect>()
    {
        return replacement.filter.zone == Some(Zone::Stack)
            && object_filter_has_tag(&replacement.filter, tag)
            && replacement.from_zone == Some(Zone::Stack)
            && replacement.to_zone == Some(Zone::Graveyard)
            && replacement.replacement_zone == Zone::Exile
            && replacement.mode == crate::effects::ReplacementApplyMode::OneShot;
    }

    false
}

pub(super) fn is_nonland_permanent_filter(filter: &ObjectFilter) -> bool {
    filter.zone == Some(Zone::Battlefield)
        && filter.excluded_card_types == vec![CardType::Land]
        && (filter.card_types.is_empty()
            || filter.card_types.iter().any(|card_type| {
                matches!(
                    card_type,
                    CardType::Artifact
                        | CardType::Creature
                        | CardType::Enchantment
                        | CardType::Land
                        | CardType::Planeswalker
                        | CardType::Battle
                )
            }))
}

pub(super) fn is_nonland_permanent_filter_in_zone(filter: &ObjectFilter, zone: Zone) -> bool {
    filter.zone == Some(zone)
        && filter.excluded_card_types == vec![CardType::Land]
        && filter.card_types.iter().any(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
                    | CardType::Battle
            )
        })
}

pub(super) fn is_permanent_filter_in_zone(filter: &ObjectFilter, zone: Zone) -> bool {
    filter.zone == Some(zone)
        && filter.excluded_card_types.is_empty()
        && (filter.card_types.is_empty()
            || [
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ]
            .iter()
            .all(|card_type| filter.card_types.contains(card_type)))
}

pub(super) fn choose_filter_is_iterated_hand_card_or_permanent(
    choose: &crate::effects::ChooseObjectsEffect,
) -> bool {
    if choose.filter.any_of.len() != 2 {
        return false;
    }
    let mut has_hand_card = false;
    let mut has_permanent = false;
    for branch in &choose.filter.any_of {
        if branch.zone == Some(Zone::Hand)
            && branch.card_types.is_empty()
            && branch.any_of.is_empty()
            && matches!(branch.owner, None | Some(PlayerFilter::IteratedPlayer))
            && matches!(branch.controller, None | Some(PlayerFilter::IteratedPlayer))
        {
            has_hand_card = true;
            continue;
        }
        if is_permanent_filter_in_zone(branch, Zone::Battlefield)
            && branch.any_of.is_empty()
            && matches!(branch.controller, None | Some(PlayerFilter::IteratedPlayer))
        {
            has_permanent = true;
            continue;
        }
        return false;
    }
    has_hand_card && has_permanent
}

pub(super) fn describe_tagged_condition_card_selection(
    condition: &Condition,
    tag: &str,
) -> Option<String> {
    match condition {
        Condition::TaggedObjectMatches(condition_tag, filter) if condition_tag.as_str() == tag => {
            Some(describe_search_selection_with_cards(&filter.description()))
        }
        Condition::Or(left, right) => {
            let left = describe_tagged_condition_card_selection(left, tag)?;
            let right = describe_tagged_condition_card_selection(right, tag)?;
            if let (Some(left_type), true) = (left.strip_suffix(" card"), right.ends_with(" card"))
            {
                Some(format!("{left_type} or {right}"))
            } else {
                Some(format!("{left} or {right}"))
            }
        }
        _ => None,
    }
}

pub(super) fn describe_look_top_card_if_matching_may_reveal_put_hand(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if look_at_top.player != PlayerFilter::You || look_at_top.count != Value::Fixed(1) {
        return None;
    }
    let selection =
        describe_tagged_condition_card_selection(&conditional.condition, look_at_top.tag.as_str())?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You) || may.effects.len() != 2 {
        return None;
    }
    let reveal = may.effects[0].downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let move_to_hand = may.effects[1].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if reveal.tag != look_at_top.tag
        || move_to_hand.zone != Zone::Hand
        || move_to_hand.to_top
        || move_to_hand.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || move_to_hand.enters_tapped
        || !matches!(move_to_hand.target.base(), ChooseSpec::Tagged(tag) if tag == &look_at_top.tag)
    {
        return None;
    }

    Some(format!(
        "Look at the top card of your library. If it's {}, you may reveal it and put it into your hand",
        with_indefinite_article(&selection)
    ))
}

pub(super) fn describe_look_top_card_if_matching_may_reveal_put_hand_else_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    conditional: &crate::effects::ConditionalEffect,
    bottom_conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    let first = describe_look_top_card_if_matching_may_reveal_put_hand(look_at_top, conditional)?;
    let Condition::Not(inner) = &bottom_conditional.condition else {
        return None;
    };
    let Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
        mode,
    } = inner.as_ref()
    else {
        return None;
    };
    let hand_filter = ObjectFilter {
        zone: Some(Zone::Hand),
        ..Default::default()
    };
    if player != &PlayerFilter::You
        || *mode != crate::effect::TaggedObjectMatchMode::CurrentOrLastKnown
        || tag.as_str() != look_at_top.tag.as_str()
        || filter != &hand_filter
        || !bottom_conditional.if_false.is_empty()
    {
        return None;
    }
    let [may_effect] = bottom_conditional.if_true.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You) || may.effects.len() != 1 {
        return None;
    }
    let move_to_bottom = may.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_bottom.zone != Zone::Library
        || move_to_bottom.to_top
        || !matches!(move_to_bottom.target.base(), ChooseSpec::Tagged(tag) if tag.as_str() == look_at_top.tag.as_str())
    {
        return None;
    }

    Some(format!(
        "{first}. If you don't put it into your hand, you may put it on the bottom of your library"
    ))
}

/// Render the equivalent nested form produced when the declined optional hand
/// move is represented by a `WithId`/`If(DidNotHappen)` inside the matching
/// branch and the nonmatching branch repeats the same bottom placement.
pub(in crate::compiled_text) fn describe_nested_look_top_card_matching_hand_else_bottom(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if look_at_top.player != PlayerFilter::You || look_at_top.count != Value::Fixed(1) {
        return None;
    }
    let selection =
        describe_tagged_condition_card_selection(&conditional.condition, look_at_top.tag.as_str())?;
    let [with_id_effect, declined_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let [nonmatching_bottom] = conditional.if_false.as_slice() else {
        return None;
    };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let hand_effects = match may.effects.as_slice() {
        [effect] => effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .map(|sequence| sequence.effects.as_slice())
            .unwrap_or(may.effects.as_slice()),
        effects => effects,
    };
    let [reveal_effect, move_effect] = hand_effects else {
        return None;
    };
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let move_to_hand = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if reveal.tag != look_at_top.tag
        || move_to_hand.zone != Zone::Hand
        || move_to_hand.to_top
        || !matches!(move_to_hand.target.base(), ChooseSpec::Tagged(tag) if tag == &look_at_top.tag)
    {
        return None;
    }

    let declined = declined_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if declined.condition != with_id.id
        || declined.predicate != EffectPredicate::DidNotHappen
        || !declined.else_.is_empty()
    {
        return None;
    }
    let [declined_bottom] = declined.then.as_slice() else {
        return None;
    };
    let is_matching_bottom = |effect: &Effect| {
        let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() else {
            return false;
        };
        let [move_effect] = may.effects.as_slice() else {
            return false;
        };
        let Some(move_to_bottom) = structural_unwrap_render_wrappers(move_effect)
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
        else {
            return false;
        };
        may.decider == Some(PlayerFilter::You)
            && may.fallback == crate::decision::FallbackStrategy::Decline
            && move_to_bottom.zone == Zone::Library
            && !move_to_bottom.to_top
            && matches!(move_to_bottom.target.base(), ChooseSpec::Tagged(tag) if tag == &look_at_top.tag)
    };
    if !is_matching_bottom(declined_bottom) || !is_matching_bottom(nonmatching_bottom) {
        return None;
    }

    let (matching, antecedent) = if let Some((left, right)) = selection.split_once(" or ") {
        (
            format!(
                "{} or {}",
                with_indefinite_article(left),
                with_indefinite_article(right)
            ),
            "the card",
        )
    } else {
        (with_indefinite_article(&selection), "it")
    };
    Some(format!(
        "Look at the top card of your library. If it's {matching}, you may reveal it and put it into your hand. If you don't put {antecedent} into your hand, you may put it on the bottom of your library"
    ))
}

pub(super) fn describe_counter_constraint(counter: crate::filter::CounterConstraint) -> String {
    match counter {
        crate::filter::CounterConstraint::Any => "a counter".to_string(),
        crate::filter::CounterConstraint::Typed(counter_type) => {
            format!("a {} counter", describe_counter_type(counter_type))
        }
        crate::filter::CounterConstraint::AtLeast {
            counter_type,
            count,
        } => {
            let count = ironsmith_core::cardinal_word(count).unwrap_or_else(|| count.to_string());
            match counter_type {
                Some(counter_type) => {
                    format!(
                        "{count} or more {} counters",
                        describe_counter_type(counter_type)
                    )
                }
                None => format!("{count} or more counters"),
            }
        }
    }
}

pub(super) fn describe_attack_block_if_able_grant(
    abilities: &[crate::static_abilities::StaticAbility],
    duration: &Until,
    subject: &str,
) -> Option<String> {
    if duration != &Until::EndOfTurn || abilities.is_empty() {
        return None;
    }

    let mut has_must_attack = false;
    let mut has_must_block = false;
    for ability in abilities {
        match ability.id() {
            crate::static_abilities::StaticAbilityId::MustAttack => has_must_attack = true,
            crate::static_abilities::StaticAbilityId::MustBlock => has_must_block = true,
            _ => return None,
        }
    }

    match (has_must_attack, has_must_block) {
        (true, true) => Some(format!("{subject} attacks or blocks this turn if able")),
        (true, false) => Some(format!("{subject} attacks this turn if able")),
        (false, true) => Some(format!("{subject} blocks this turn if able")),
        (false, false) => None,
    }
}

pub(super) fn describe_for_players_choose_nonland_put_counter(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 2 {
        return None;
    }
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.chooser != PlayerFilter::IteratedPlayer
        || !matches!(
            choose.filter.controller,
            None | Some(PlayerFilter::IteratedPlayer)
        )
        || !is_nonland_permanent_filter(&choose.filter)
    {
        return None;
    }
    let put = for_players.effects[1].downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.amount != Value::Fixed(1)
        || put.distributed
        || put.target_count.is_some()
        || !matches!(
            put.target.base(),
            ChooseSpec::Iterated | ChooseSpec::Tagged(_)
        )
    {
        return None;
    }

    Some(format!(
        "Each player chooses a nonland permanent and puts a {} counter on it",
        describe_counter_type(put.counter_type)
    ))
}

pub(super) fn describe_simple_create_token_bundle(effects: &[&Effect]) -> Option<String> {
    if let [effect] = effects {
        let sequence =
            unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::SequenceEffect>()?;
        if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
            return None;
        }
        let coordinated = sequence.effects.iter().collect::<Vec<_>>();
        return describe_simple_create_token_bundle(&coordinated);
    }
    if effects.len() < 2 {
        return None;
    }
    let mut tokens = Vec::new();
    for effect in effects {
        let effect = unwrap_basic_tag_wrappers(effect);
        let create = effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;
        if !matches!(create.count.unhinted(), Value::Fixed(_) | Value::X)
            || !matches!(create.controller, PlayerFilter::You)
            || create.controller_target.is_some()
            || create.enters_tapped
            || create.enters_attacking
            || create.exile_at_end_of_combat
            || create.sacrifice_at_end_of_combat
            || create.sacrifice_at_next_end_step
            || create.exile_at_next_end_step
        {
            return None;
        }
        let rendered = describe_effect(effect);
        tokens.push(rendered.strip_prefix("Create ")?.to_string());
    }

    Some(format!("Create {}", join_with_and(&tokens)))
}

pub(super) fn is_creature_filter_for_you(filter: &ObjectFilter) -> bool {
    filter.zone == Some(Zone::Battlefield)
        && filter.controller == Some(PlayerFilter::You)
        && filter.card_types == vec![CardType::Creature]
        && filter.excluded_card_types.is_empty()
}

pub(super) fn is_creature_card_match_filter(filter: &ObjectFilter) -> bool {
    filter.zone.is_none()
        && filter.controller.is_none()
        && filter.card_types == vec![CardType::Creature]
        && filter.excluded_card_types.is_empty()
}

fn consult_match_move_to_zone<'a>(
    effect: &'a Effect,
    consult: &crate::effects::ConsultTopOfLibraryEffect,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    let effect = unwrap_basic_tag_wrappers(effect);
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
        .then_some(move_to_zone);
    }

    let for_each = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag != consult.match_tag {
        return None;
    }
    let nested = if let [sequence] = for_each.effects.as_slice()
        && let Some(sequence) =
            unwrap_basic_tag_wrappers(sequence).downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        for_each.effects.as_slice()
    };
    let [move_effect] = nested else {
        return None;
    };
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    (matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
        || matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"
        ))
    .then_some(move_to_zone)
}

/// Render reveal-until programs as collection moves rather than exposing the
/// implementation's per-object loop. The all/match/remainder tags prove which
/// revealed cards move to the primary destination and which go to the bottom.
pub(super) fn describe_consult_reveal_move_matches_then_bottom(
    effects: &[&Effect],
) -> Option<String> {
    if let [consult_effect, sequence_effect] = effects
        && let Some(sequence) = unwrap_basic_tag_wrappers(sequence_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && let [move_effect, remainder_effect] = sequence.effects.as_slice()
    {
        return describe_consult_reveal_move_matches_then_bottom(&[
            *consult_effect,
            move_effect,
            remainder_effect,
        ]);
    }

    let [consult_effect, move_effect, remainder_effect] = effects else {
        return None;
    };
    let consult = unwrap_basic_tag_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let move_uses_iteration = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
        .is_some();
    let consult_produces_multiple_matches = matches!(
        &consult.stop_rule,
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count)
            if count != &Value::Fixed(1)
    );
    if !move_uses_iteration && !consult_produces_multiple_matches {
        // Direct singular moves already have established renderers that retain
        // their surrounding optionality, pronouns, and sentence boundaries.
        // This compactor exists for an actual matched collection or for the
        // runtime's per-match iteration scaffolding; claiming an ordinary
        // singular move here would discard useful surface information.
        return None;
    }
    let move_to_zone = consult_match_move_to_zone(move_effect, consult)?;
    let remainder = unwrap_basic_tag_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;

    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.max_exposed.is_some()
        || !matches!(move_to_zone.zone, Zone::Hand | Zone::Battlefield)
        || move_to_zone.to_top
        || move_to_zone.library_order.is_some()
        || move_to_zone
            .actor_surface
            .as_ref()
            .is_some_and(|actor| actor != &PlayerFilter::You)
        || move_to_zone
            .destination_player_surface
            .as_ref()
            .is_some_and(|player| player != &PlayerFilter::You)
        || move_to_zone.destination_player_reference_surface.is_some()
        || move_to_zone.exiled_with_source_surface.is_some()
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.attack_target_mode.is_some()
        || move_to_zone.enters_face_down
        || move_to_zone.transfer_exiled_with_source_links
        || remainder.tag != consult.all_tag
        || remainder.keep_tagged.as_ref() != Some(&consult.match_tag)
        || remainder.player != consult.player
    {
        return None;
    }

    let mut selection =
        describe_search_selection_with_cards(&consult.filter.description()).to_string();
    if let Some(shorter) = selection.strip_suffix(" than it") {
        selection = shorter.to_string();
    }
    let (stop_text, matched_reference, singular) = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {
            (with_indefinite_article(&selection), "it".to_string(), true)
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(count))
            if *count > 1 =>
        {
            let count_text = number_word(*count).unwrap_or_else(|| count.to_string());
            let plural_selection = pluralize_noun_phrase(strip_leading_article(&selection));
            (
                format!("{count_text} {plural_selection}"),
                format!("the {plural_selection} revealed this way"),
                false,
            )
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => (
            describe_counted_consult_stop(count, &selection),
            "those cards".to_string(),
            false,
        ),
    };
    let order_text = match remainder.order {
        LibraryBottomOrder::Random => " in a random order",
        LibraryBottomOrder::ChooserChooses => " in any order",
    };

    match move_to_zone.zone {
        Zone::Hand if !singular => Some(format!(
            "Reveal cards from the top of your library until you reveal {stop_text}. Put {matched_reference} into your hand, then put the rest of the revealed cards on the bottom of your library{order_text}"
        )),
        Zone::Hand => Some(format!(
            "Reveal cards from the top of your library until you reveal {stop_text}. Put {matched_reference} into your hand and the rest on the bottom of your library{order_text}"
        )),
        Zone::Battlefield => Some(format!(
            "Reveal cards from the top of your library until you reveal {stop_text}, put {matched_reference} onto the battlefield, then put the rest on the bottom of your library{order_text}"
        )),
        _ => None,
    }
}

pub(super) fn describe_exile_creatures_consult_that_many_battlefield_shuffle(
    effects: &[&Effect],
) -> Option<String> {
    if let Some(partition) = describe_consult_reveal_move_matches_then_bottom(effects) {
        return Some(partition);
    }

    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_effect(&tagged.effect);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return unwrap_effect(&tag_all.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap_effect(with_id.effect.as_ref());
        }
        effect
    }

    fn wrapped_effect_id(effect: &Effect) -> Option<crate::effect::EffectId> {
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return Some(with_id.id);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return wrapped_effect_id(&tagged.effect);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return wrapped_effect_id(&tag_all.effect);
        }
        None
    }

    let (exile_effect, consult_effect, move_effect, shuffle_effect, at_next_end_step): (
        &Effect,
        &Effect,
        &Effect,
        &Effect,
        bool,
    ) = match effects {
        [exile_effect, consult_effect, move_effect, shuffle_effect] => (
            exile_effect,
            consult_effect,
            move_effect,
            shuffle_effect,
            false,
        ),
        [exile_effect, consult_effect, sequence_effect] => {
            let sequence =
                unwrap_effect(sequence_effect).downcast_ref::<crate::effects::SequenceEffect>()?;
            let [move_effect, shuffle_effect] = sequence.effects.as_slice() else {
                return None;
            };
            (
                exile_effect,
                consult_effect,
                move_effect,
                shuffle_effect,
                false,
            )
        }
        [exile_effect, schedule_effect] => {
            let schedule = unwrap_effect(schedule_effect)
                .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
            let end_step = schedule
                .trigger
                .downcast_ref::<crate::triggers::BeginningOfEndStepTrigger>()?;
            if !schedule.one_shot
                || schedule.start_next_turn
                || schedule.until_end_of_turn
                || schedule.until_end_of_combat
                || schedule.watch_ability_source
                || !schedule.target_objects.is_empty()
                || schedule.target_tag.is_some()
                || schedule.target_filter.is_some()
                || schedule.controller != PlayerFilter::You
                || end_step.player != PlayerFilter::Any
            {
                return None;
            }
            let flattened = schedule.effects.flattened_default_effects();
            let (consult_effect, move_effect, shuffle_effect) = match flattened {
                [consult_effect, move_effect, shuffle_effect] => {
                    (consult_effect, move_effect, shuffle_effect)
                }
                [sequence_effect] => {
                    let sequence = unwrap_effect(sequence_effect)
                        .downcast_ref::<crate::effects::SequenceEffect>()?;
                    let [consult_effect, move_effect, shuffle_effect] = sequence.effects.as_slice()
                    else {
                        return None;
                    };
                    (consult_effect, move_effect, shuffle_effect)
                }
                _ => return None,
            };
            (
                *exile_effect,
                consult_effect,
                move_effect,
                shuffle_effect,
                true,
            )
        }
        _ => return None,
    };
    let exile = unwrap_effect(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down
        || !matches!(
            exile.spec.base(),
            ChooseSpec::All(filter) if is_creature_filter_for_you(filter)
        )
    {
        return None;
    }
    let exile_effect_id = wrapped_effect_id(exile_effect)?;

    let consult = unwrap_effect(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !is_creature_card_match_filter(&consult.filter)
        || !matches!(
            &consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count)
                if is_effect_count_reference(count, Some(exile_effect_id))
        )
    {
        return None;
    }

    let (move_to_battlefield, target_is_consult_match) = if let Some(move_to_zone) =
        unwrap_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        (
            move_to_zone,
            matches!(
                move_to_zone.target.base(),
                ChooseSpec::Tagged(tag) if tag == &consult.match_tag
            ),
        )
    } else {
        let for_each =
            unwrap_effect(move_effect).downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
        if for_each.tag != consult.match_tag {
            return None;
        }
        let nested = if let [sequence] = for_each.effects.as_slice()
            && let Some(sequence) =
                unwrap_effect(sequence).downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            for_each.effects.as_slice()
        };
        let [move_effect] = nested else {
            return None;
        };
        let move_to_zone =
            unwrap_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        (
            move_to_zone,
            matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
                || matches!(
                    move_to_zone.target.base(),
                    ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"
                ),
        )
    };
    if move_to_battlefield.zone != Zone::Battlefield
        || move_to_battlefield.to_top
        || move_to_battlefield.enters_tapped
        || !target_is_consult_match
        || !matches!(
            move_to_battlefield.battlefield_controller,
            crate::effects::BattlefieldController::Preserve
        )
    {
        return None;
    }

    let shuffle =
        unwrap_effect(shuffle_effect).downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return None;
    }

    if at_next_end_step {
        Some("Exile all creatures you control. At the beginning of the next end step, reveal cards from the top of your library until you reveal that many creature cards, put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library".to_string())
    } else {
        Some("Exile all creatures you control, then reveal cards from the top of your library until you reveal that many creature cards. Put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library".to_string())
    }
}

pub(super) fn move_revealed_remainder_to_hand(
    effect: &Effect,
    looked_tag: &TagKey,
    chosen_tag: &TagKey,
) -> bool {
    let Some(move_to_zone) =
        unwrap_tag_wrapped_effect(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    if move_to_zone.zone != Zone::Hand || move_to_zone.to_top {
        return false;
    }
    let ChooseSpec::Object(filter) = move_to_zone.target.base() else {
        return false;
    };
    if filter.zone != Some(Zone::Library) {
        return false;
    }
    let references_looked = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *looked_tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    });
    let excludes_chosen = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *chosen_tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            )
    });
    if !references_looked || !excludes_chosen {
        return false;
    }

    let mut bare = filter.clone();
    bare.zone = None;
    bare.tagged_constraints.retain(|constraint| {
        !(constraint.tag == *looked_tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            ))
            && !(constraint.tag == *chosen_tag
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                ))
    });
    bare == ObjectFilter::default()
}

pub(super) fn describe_reveal_top_opponent_exiles_rest_hand_then_may_cast(
    effects: &[Effect],
) -> Option<String> {
    let [
        look_effect,
        choose_effect,
        exile_effect,
        hand_effect,
        may_effect,
    ] = effects
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if !look.reveal || look.player != PlayerFilter::You {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::Opponent
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.is_search
    {
        return None;
    }
    let chosen = describe_choose_filter_from_looked_cards(look, choose)?;

    let exile = unwrap_tag_wrapped_effect(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_exile_uses_chosen_tag(exile, choose.tag.as_str()) {
        return None;
    }
    if !move_revealed_remainder_to_hand(hand_effect, &look.tag, &choose.tag) {
        return None;
    }

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != choose.chooser)
        || may.effects.len() != 1
    {
        return None;
    }
    let cast = may.effects[0].downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != choose.tag
        || cast.player != choose.chooser
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
    {
        return None;
    }

    let (count_text, noun, _) = describe_look_count_and_noun(&look.count);
    Some(format!(
        "Reveal the top {count_text} {noun} of your library. An opponent exiles {chosen} from among them, then you put the rest into your hand. That opponent may cast the exiled card without paying its mana cost"
    ))
}

pub(super) fn describe_destroy_land_then_controller_reveals_until_land_graveyard(
    effects: &[Effect],
) -> Option<String> {
    let [destroy_effect, consult_effect, move_effect] = effects else {
        return None;
    };
    let tagged_destroy = destroy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let destroy = tagged_destroy
        .effect
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::Object(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    if destroy_filter.zone != Some(Zone::Battlefield)
        || destroy_filter.card_types != vec![CardType::Land]
        || !destroy_filter.all_card_types.is_empty()
        || !destroy_filter.subtypes.is_empty()
    {
        return None;
    }

    let consult = unwrap_basic_tag_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !matches!(
            &consult.player,
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))
                if tag == &tagged_destroy.tag
        )
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
        || consult.filter.zone.is_some()
        || consult.filter.card_types != vec![CardType::Land]
        || !consult.filter.all_card_types.is_empty()
        || !consult.filter.subtypes.is_empty()
    {
        return None;
    }

    let move_to_zone = unwrap_tag_wrapped_effect(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard
        || move_to_zone.to_top
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.all_tag
        )
    {
        return None;
    }

    Some("Destroy target land. Its controller reveals cards from the top of their library until they reveal a land card, then puts those cards into their graveyard".to_string())
}

pub(super) fn value_is_total_power_of_tagged_exiled_cards(value: &Value, tag: &TagKey) -> bool {
    let Value::TotalPower(filter) = value.unhinted() else {
        return false;
    };
    filter.zone == Some(Zone::Exile)
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *tag
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject
                )
        })
}

pub(super) fn is_zero_zero_blue_zombie_token(create: &crate::effects::CreateTokenEffect) -> bool {
    let card = &create.token.card;
    let Some(power_toughness) = card.power_toughness else {
        return false;
    };

    card.name == "Zombie"
        && card.color_indicator == Some(crate::color::ColorSet::BLUE)
        && card.card_types == vec![CardType::Creature]
        && card.subtypes.contains(&crate::types::Subtype::Zombie)
        && power_toughness.power == PtValue::Fixed(0)
        && power_toughness.toughness == PtValue::Fixed(0)
}

pub(super) fn describe_each_player_mill_exile_milled_creatures_create_power_token(
    effects: &[Effect],
) -> Option<String> {
    let [
        mill_effect,
        choose_effect,
        exile_effect,
        create_effect,
        set_pt_effect,
    ] = effects
    else {
        return None;
    };

    let for_players = mill_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 1 {
        return None;
    }
    let tagged_mill = &for_players.effects[0];
    let milled_tag = structural_effect_tag(tagged_mill)?;
    let mill =
        unwrap_structural_effect_tag(tagged_mill).downcast_ref::<crate::effects::MillEffect>()?;
    if mill.player != PlayerFilter::IteratedPlayer || mill.count != Value::Fixed(3) {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max != Some(2)
        || choose.filter.zone != Some(Zone::Graveyard)
        || choose.filter.card_types != vec![CardType::Creature]
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *milled_tag
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject
                )
        })
    {
        return None;
    }

    let exile = exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile.zone != Zone::Exile
        || !matches!(exile.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let (created_tag, create) = tagged_create_token_effect(create_effect)?;
    if create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::You
        || create.enters_tapped
        || create.enters_attacking
        || !is_zero_zero_blue_zombie_token(create)
    {
        return None;
    }

    let set_pt = set_pt_effect.downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()?;
    if !matches!(set_pt.target.base(), ChooseSpec::Tagged(tag) if tag == created_tag)
        || !value_is_total_power_of_tagged_exiled_cards(&set_pt.power, &choose.tag)
        || !value_is_total_power_of_tagged_exiled_cards(&set_pt.toughness, &choose.tag)
    {
        return None;
    }

    Some("Each player mills three cards. Exile up to two creature cards put into graveyards this way. Create an X/X blue Zombie creature token, where X is the total power of the cards exiled this way".to_string())
}

pub(super) fn is_plain_land_card_filter(filter: &ObjectFilter) -> bool {
    *filter
        == ObjectFilter {
            card_types: vec![CardType::Land],
            ..Default::default()
        }
}

pub(super) fn is_tagged_revealed_hand_card_filter(filter: &ObjectFilter, tag: &TagKey) -> bool {
    if filter.zone != Some(Zone::Hand) || filter.owner != Some(PlayerFilter::You) {
        return false;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    }) {
        return false;
    }

    let mut remainder = filter.clone();
    remainder.zone = None;
    remainder.owner = None;
    remainder.tagged_constraints.clear();
    remainder == ObjectFilter::default()
}

pub(super) fn condition_is_not_tagged_land_card(condition: &Condition, tag: &TagKey) -> bool {
    let Condition::Not(inner) = condition else {
        return false;
    };
    matches!(
        inner.as_ref(),
        Condition::TaggedObjectMatches(condition_tag, filter)
            if condition_tag == tag && is_plain_land_card_filter(filter)
    )
}

pub(super) fn describe_draw_reveal_discard_nonland(effects: &[Effect]) -> Option<String> {
    let [draw_effect, reveal_effect, conditional_effect] = effects else {
        return None;
    };

    let draw =
        unwrap_basic_tag_wrappers(draw_effect).downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.count != Value::Fixed(1) || draw.player != PlayerFilter::You {
        return None;
    }

    let reveal = unwrap_basic_tag_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let conditional = unwrap_basic_tag_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
        || !condition_is_not_tagged_land_card(&conditional.condition, &reveal.tag)
    {
        return None;
    }

    let discard = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count != Value::Fixed(1)
        || discard.player != PlayerFilter::You
        || discard.random
        || discard.any_number
    {
        return None;
    }
    let filter = discard.card_filter.as_ref()?;
    if !is_tagged_revealed_hand_card_filter(filter, &reveal.tag) {
        return None;
    }

    Some("Draw a card and reveal it. If it isn't a land card, discard it".to_string())
}

pub(super) fn exiled_all_creatures_effect_id(effect: &Effect) -> Option<crate::effect::EffectId> {
    let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let tagged = with_id
        .effect
        .downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = tagged
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if exile.face_down
        || !matches!(
            &exile.spec,
            ChooseSpec::All(filter) if *filter == ObjectFilter::creature().in_zone(Zone::Battlefield)
        )
    {
        return None;
    }
    Some(with_id.id)
}

pub(super) fn is_zero_zero_green_blue_fractal_token(
    create: &crate::effects::CreateTokenEffect,
) -> bool {
    let card = &create.token.card;
    card.card_types == vec![CardType::Creature]
        && card.subtypes == vec![Subtype::Fractal]
        && card.color_indicator
            == Some(crate::color::ColorSet::GREEN.union(crate::color::ColorSet::BLUE))
        && card
            .power_toughness
            .is_some_and(|pt| pt.power == PtValue::Fixed(0) && pt.toughness == PtValue::Fixed(0))
}

pub(super) fn value_is_total_power_of_effect_affected_objects(
    value: &Value,
    effect_id: crate::effect::EffectId,
) -> bool {
    matches!(
        value.unhinted(),
        Value::EffectMetric {
            effect_id: candidate,
            source: crate::effect::EffectMetricSource::AffectedObjects,
            metric: crate::effect::EffectMetric::TotalPower,
        } if *candidate == effect_id
    )
}

pub(super) fn describe_exile_all_creatures_each_player_fractal_power_counters(
    effects: &[Effect],
) -> Option<String> {
    let [exile_effect, create_effect, counters_effect] = effects else {
        return None;
    };
    let exile_id = exiled_all_creatures_effect_id(exile_effect)?;

    let for_players = create_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 1 {
        return None;
    }
    let (created_tag, create) = tagged_create_token_effect(&for_players.effects[0])?;
    if create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::IteratedPlayer
        || create.enters_tapped
        || create.enters_attacking
        || !is_zero_zero_green_blue_fractal_token(create)
    {
        return None;
    }

    let put = unwrap_structural_effect_tag(counters_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != crate::object::CounterType::PlusOnePlusOne
        || put.distributed
        || put.target_count.is_some()
        || !matches!(put.target, ChooseSpec::Tagged(ref tag) if tag == created_tag)
        || !value_is_total_power_of_effect_affected_objects(&put.amount, exile_id)
    {
        return None;
    }

    let token_blueprint = describe_create_token_blueprint(create);
    let token = if token_blueprint.starts_with("0/0 ") {
        format!("a {token_blueprint}")
    } else {
        with_indefinite_article(&token_blueprint)
    };
    Some(format!(
        "Exile all creatures. Each player creates {token} and puts a number of +1/+1 counters on it equal to the total power of creatures they controlled that were exiled this way"
    ))
}

pub(super) fn describe_choose_any_target_players_then_investigate_total_creatures(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, investigate_effect] = effects else {
        return None;
    };
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let count = target_only.target.count();
    if !count.is_any_number()
        || !matches!(
            target_only.target.base(),
            ChooseSpec::Player(PlayerFilter::Any)
        )
        || !target_only.target.is_target()
    {
        return None;
    }

    let investigate = investigate_effect.downcast_ref::<crate::effects::InvestigateEffect>()?;
    let counts_targeted_players_creatures = matches!(
        &investigate.count,
        Value::Count(filter)
            if filter.zone == Some(Zone::Battlefield)
                && filter.card_types == vec![CardType::Creature]
                && matches!(&filter.controller, Some(PlayerFilter::Target(inner)) if **inner == PlayerFilter::Any)
    );
    if investigate.player != PlayerFilter::You
        || !(investigate.count == Value::X || counts_targeted_players_creatures)
    {
        return None;
    }

    Some(
        "Choose any number of target players. Investigate X times, where X is the total number of creatures those players control"
            .to_string(),
    )
}

pub(super) fn is_creature_target_opponent_controls_filter(filter: &ObjectFilter) -> bool {
    filter.zone == Some(Zone::Battlefield)
        && matches!(&filter.controller, Some(PlayerFilter::Target(inner)) if **inner == PlayerFilter::Opponent)
        && filter.card_types == vec![CardType::Creature]
        && filter.excluded_card_types.is_empty()
}

pub(super) fn describe_divided_evenly_x_damage_to_target_opponent_creatures(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_effect(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap_effect(with_id.effect.as_ref());
        }
        effect
    }

    if !is_creature_target_opponent_controls_filter(&for_each.filter) {
        return None;
    }
    let [damage_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let damage = unwrap_effect(damage_effect).downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.amount != Value::X
        || !matches!(damage.target, ChooseSpec::Iterated)
        || damage.source_is_combat
    {
        return None;
    }

    Some(
        "Deal X damage divided evenly, rounded down, among all creatures target opponent controls"
            .to_string(),
    )
}

pub(super) fn describe_each_player_gain_life_and_draw_pair(effects: &[&Effect]) -> Option<String> {
    let [first_effect, second_effect] = effects else {
        return None;
    };
    let first = first_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let second = second_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if first.filter != PlayerFilter::Any || second.filter != PlayerFilter::Any {
        return None;
    }

    let [gain_effect] = first.effects.as_slice() else {
        return None;
    };
    let [draw_effect] = second.effects.as_slice() else {
        return None;
    };
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if !matches!(
        gain.player.base(),
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) || draw.player != PlayerFilter::IteratedPlayer
    {
        return None;
    }

    Some(format!(
        "Each player gains {} life and draws {}",
        describe_value(&gain.amount),
        describe_card_count(&draw.count)
    ))
}

pub(super) fn describe_player_loses_life_and_discards_pair(effects: &[&Effect]) -> Option<String> {
    let [lose_effect, discard_effect] = effects else {
        return None;
    };
    let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    let ChooseSpec::Player(lose_player) = lose.player.base() else {
        return None;
    };
    if lose_player != &discard.player || discard.any_number {
        return None;
    }

    let player = describe_choose_spec(&lose.player);
    let random_suffix = if discard.random { " at random" } else { "" };
    Some(format!(
        "{} {} {} and {} {}{}",
        player,
        player_verb(&player, "lose", "loses"),
        describe_life_amount_phrase(&lose.amount),
        player_verb(&player, "discard", "discards"),
        describe_discard_count(&discard.count, discard.card_filter.as_ref()),
        random_suffix
    ))
}

pub(super) fn describe_for_players_subject(filter: &PlayerFilter) -> Option<&'static str> {
    match filter {
        PlayerFilter::Any => Some("Each player"),
        PlayerFilter::Opponent => Some("Each opponent"),
        PlayerFilter::NotYou => Some("Each other player"),
        PlayerFilter::You => Some("You"),
        PlayerFilter::Excluding { base, excluded }
            if matches!(base.as_ref(), PlayerFilter::Any)
                && matches!(excluded.as_ref(), PlayerFilter::Opponent) =>
        {
            Some("Each player on your team")
        }
        PlayerFilter::Excluding { base, excluded }
            if matches!(base.as_ref(), PlayerFilter::Any)
                && matches!(excluded.as_ref(), PlayerFilter::ControllerOf(_)) =>
        {
            Some("Each player other than its controller")
        }
        PlayerFilter::Excluding { base, excluded }
            if matches!(base.as_ref(), PlayerFilter::Any)
                && matches!(excluded.as_ref(), PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Any)) =>
        {
            Some("Each player other than target player")
        }
        _ => None,
    }
}

pub(in crate::compiled_text) fn describe_life_amount_phrase(amount: &Value) -> String {
    if amount.has_surface_hint(ValueSurfaceHint::WhereXIs)
        && let Some(basis) = describe_where_x_basis(amount)
    {
        return format!("X life, where X is {basis}");
    }
    if let Some(backref) = describe_scalar_life_backref(amount) {
        return format!("{backref} life");
    }
    if let Some(additive) = describe_additive_for_each_life_amount(amount) {
        return additive;
    }
    // Match through any surface hint: a hinted `ManaValueOf` is still a
    // characteristic basis and takes oracle's "life equal to ..." tail rather
    // than an inline determiner ("the sacrificed permanent's mana value life").
    if matches!(
        amount.unhinted(),
        Value::SourcePower
            | Value::SourceToughness
            | Value::PowerOf(_)
            | Value::ToughnessOf(_)
            | Value::ManaValueOf(_)
            | Value::Speed(_)
            | Value::LifeGainedThisTurn(_)
            | Value::LifeLostThisTurn(_)
            | Value::DamageDealtToPlayersThisTurn(_)
    ) {
        return format!("life equal to {}", describe_value(amount));
    }
    let desc = describe_value(amount);
    // A counting phrase reads as oracle's "life equal to ..." tail, not as an
    // inline determiner.
    for prefix in ["the number of ", "the amount of ", "the total "] {
        if desc.starts_with(prefix) {
            return format!("life equal to {desc}");
        }
    }
    format!("{desc} life")
}

fn describe_scalar_life_backref(amount: &Value) -> Option<String> {
    match amount.unhinted() {
        Value::EffectValue(_) | Value::EventValue(EventValueSpec::Amount) => {
            Some("that much".to_string())
        }
        Value::EffectValueOffset(_, offset)
        | Value::EventValueOffset(EventValueSpec::Amount, offset) => match offset {
            0 => Some("that much".to_string()),
            offset if *offset > 0 => Some(format!("that much plus {offset}")),
            -1 => Some("that much minus one".to_string()),
            offset => Some(format!("that much minus {}", -offset)),
        },
        _ => None,
    }
}

fn describe_additive_for_each_life_amount(amount: &Value) -> Option<String> {
    let Value::Add(base, addend) = amount.unhinted() else {
        return None;
    };
    if !addend.has_surface_hint(ValueSurfaceHint::ForEach) {
        return None;
    }
    let (basis, multiplier) = match addend.unhinted() {
        Value::Scaled(basis, multiplier) if *multiplier > 0 => (basis.as_ref(), *multiplier),
        basis => (basis, 1),
    };
    let counted = describe_create_for_each_count(basis)?;
    Some(format!(
        "{} life plus {multiplier} life for each {counted}",
        describe_value(base)
    ))
}

pub(super) fn describe_half_life_amount_for_same_player(
    amount: &Value,
    player_filter: &PlayerFilter,
) -> Option<&'static str> {
    match amount {
        Value::HalfLifeTotalRoundedUp(filter) if filter == player_filter => {
            Some(if matches!(player_filter, PlayerFilter::You) {
                "half your life, rounded up"
            } else {
                "half their life, rounded up"
            })
        }
        Value::HalfLifeTotalRoundedDown(filter) if filter == player_filter => {
            Some(if matches!(player_filter, PlayerFilter::You) {
                "half your life, rounded down"
            } else {
                "half their life, rounded down"
            })
        }
        Value::HalfStartingLifeTotalRoundedUp(filter) if filter == player_filter => {
            Some(if matches!(player_filter, PlayerFilter::You) {
                "half your starting life total, rounded up"
            } else {
                "half their starting life total, rounded up"
            })
        }
        Value::HalfStartingLifeTotalRoundedDown(filter) if filter == player_filter => {
            Some(if matches!(player_filter, PlayerFilter::You) {
                "half your starting life total, rounded down"
            } else {
                "half their starting life total, rounded down"
            })
        }
        _ => None,
    }
}

fn describe_iterated_player_for_each_life_amount(amount: &Value, subject: &str) -> Option<String> {
    if !amount.has_surface_hint(ValueSurfaceHint::ForEach) {
        return None;
    }

    let (multiplier, mut counted) = match amount.unhinted() {
        Value::Count(filter) => (1, describe_for_each_count_filter(filter)),
        Value::CountScaled(filter, multiplier) => {
            (*multiplier, describe_for_each_count_filter(filter))
        }
        Value::Add(left, right) if left == right => {
            (2, describe_create_for_each_count(left.as_ref())?)
        }
        basis => (1, describe_create_for_each_count(basis)?),
    };

    let (possessive, personal) = if subject == "You" {
        ("your", "you")
    } else {
        ("their", "they")
    };
    counted = counted
        .replace("that player's", possessive)
        .replace("that player", personal);
    if subject != "You" {
        counted = rewrite_iterated_player_references(&counted);
    }
    Some(format!("{multiplier} life for each {counted}"))
}

pub(super) fn describe_for_players_simple_iterated_action(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    // Dedicated surfaces first — the generic "that player ..." rewrite below
    // can't conjugate coordinated verb chains ("chooses ... and returns ...").
    if let Some(compact) =
        describe_each_player_choose_type_return_from_graveyard_to_hand(for_players)
    {
        return Some(compact);
    }
    let [effect] = for_players.effects.as_slice() else {
        return None;
    };
    // An authored comma-then chain is still a multi-action quantified-player
    // sequence. Let the structural sequence compactor retain the shared
    // "Each player" subject instead of treating the wrapper as one generic
    // action and rewriting each nested reference to singular "they".
    if structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::SequenceEffect>()
        .is_some_and(|sequence| {
            sequence.surface == ironsmith_core::SequenceSurface::CommaThen
                && sequence.effects.len() > 1
        })
    {
        return None;
    }
    let subject = describe_for_players_subject(&for_players.filter)?;
    let subject_lower = lowercase_first(subject);
    let verb = |you: &'static str, other: &'static str| {
        if subject == "You" { you } else { other }
    };

    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>()
        && matches!(
            cant.start,
            crate::effect::RestrictionStart::NextTurn(PlayerFilter::IteratedPlayer)
        )
    {
        let inner = describe_effect(effect);
        let rest = inner
            .strip_prefix("that player ")
            .or_else(|| inner.strip_prefix("That player "))?;
        return Some(format!("{subject} {rest}"));
    }

    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>()
        && matches!(
            lose.player,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
    {
        if let Some(amount) = describe_iterated_player_for_each_life_amount(&lose.amount, subject) {
            return Some(format!("{subject} {} {amount}", verb("lose", "loses")));
        }
        if let Some(where_x) = describe_where_x_basis(&lose.amount) {
            return Some(format!(
                "{subject} {} X life, where X is {where_x}",
                verb("lose", "loses")
            ));
        }
        return Some(format!(
            "{subject} {} {}",
            verb("lose", "loses"),
            describe_life_amount_phrase(&lose.amount)
        ));
    }
    if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>()
        && matches!(
            gain.player,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
    {
        return Some(format!(
            "{subject} {} {}",
            verb("gain", "gains"),
            describe_life_amount_phrase(&gain.amount)
        ));
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && draw.player == PlayerFilter::IteratedPlayer
    {
        if let Some(dynamic_for_each) = describe_draw_count_for_each_phrase(&draw.count) {
            return Some(format!(
                "{subject} {} {dynamic_for_each}",
                verb("draw", "draws")
            ));
        }
        return Some(format!(
            "{subject} {} {}",
            verb("draw", "draws"),
            describe_card_count(&draw.count)
        ));
    }
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>()
        && discard.player == PlayerFilter::IteratedPlayer
        && !discard.any_number
    {
        let random_suffix = if discard.random { " at random" } else { "" };
        return Some(format!(
            "{subject} {} {}{}",
            verb("discard", "discards"),
            describe_discard_count(&discard.count, discard.card_filter.as_ref()),
            random_suffix
        ));
    }
    if let Some(discard_hand) = effect.downcast_ref::<crate::effects::DiscardHandEffect>()
        && discard_hand.player == PlayerFilter::IteratedPlayer
    {
        return Some(format!(
            "{subject} {} {} hand",
            verb("discard", "discards"),
            if subject == "You" { "your" } else { "their" }
        ));
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(
            damage.target,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        && matches!(damage.amount, Value::Fixed(_))
    {
        let mut text = describe_effect(effect)
            .replace(" to that player", &format!(" to {}", subject_lower))
            .replace(" to That player", &format!(" to {subject_lower}"));
        if subject != "You" {
            text = text.replace(" to each player", " to each player");
        }
        return Some(text);
    }
    if let Some(exile_top) = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        && exile_top.player == PlayerFilter::IteratedPlayer
    {
        let count_text = match exile_top.count {
            Value::Fixed(1) => "the top card".to_string(),
            Value::Fixed(count) => format!("the top {count} cards"),
            _ => format!("the top {} cards", describe_value(&exile_top.count)),
        };
        return Some(format!(
            "{subject} {} {count_text} of {} library",
            verb("exile", "exiles"),
            if subject == "You" { "your" } else { "their" }
        ));
    }
    if let Some(shuffle) =
        effect.downcast_ref::<crate::effects::ShuffleGraveyardIntoLibraryEffect>()
        && shuffle.player == PlayerFilter::IteratedPlayer
    {
        return Some(format!(
            "{subject} {} {} graveyard into {} library",
            verb("shuffle", "shuffles"),
            if subject == "You" { "your" } else { "their" },
            if subject == "You" { "your" } else { "their" }
        ));
    }
    if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        let targets_iterated_player = match move_to_zone.target.base() {
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
                filter.owner == Some(PlayerFilter::IteratedPlayer)
                    || filter.controller == Some(PlayerFilter::IteratedPlayer)
            }
            _ => false,
        };
        if move_to_zone.actor_surface == Some(PlayerFilter::IteratedPlayer)
            || targets_iterated_player
        {
            let mut inner = lowercase_first(&describe_effect(effect));
            if let Some(rest) = inner.strip_prefix("that player ") {
                inner = rest.to_string();
            }
            inner = if subject == "You" {
                normalize_you_verb_phrase(&inner)
            } else {
                normalize_third_person_verb_phrase(&inner)
                    .replace("that player's", "their")
                    .replace("that player", &subject_lower)
            };
            inner = inner
                .replace(" in your graveyard", " from your graveyard")
                .replace(" in their graveyard", " from their graveyard");
            return Some(format!("{subject} {inner}"));
        }
    }
    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
        && apply.modification.is_none()
        && apply.additional_modifications.is_empty()
        && matches!(
            apply.runtime_modifications.as_slice(),
            [
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    PlayerFilter::IteratedPlayer
                )
            ]
        )
    {
        if let crate::continuous::EffectTarget::Filter(filter) = &apply.target
            && filter.owner == Some(PlayerFilter::IteratedPlayer)
            && filter.controller == Some(PlayerFilter::You)
        {
            let mut object_filter = filter.clone();
            object_filter.owner = None;
            object_filter.controller = None;
            let object = strip_indefinite_article(&object_filter.description()).to_string();
            return Some(format!(
                "{subject} {} control of each {object} they own that you control",
                verb("gain", "gains")
            ));
        }
    }

    let inner = describe_effect_list(&for_players.effects);
    let rest = inner
        .strip_prefix("that player ")
        .or_else(|| inner.strip_prefix("That player "))?;
    Some(format!(
        "{subject} {}",
        if subject == "You" {
            normalize_you_verb_phrase(rest)
        } else {
            rewrite_iterated_player_references(&normalize_third_person_verb_phrase(rest))
        }
    ))
}

pub(super) fn describe_for_players_iterated_action_sequence(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let effects = if let [effect] = for_players.effects.as_slice()
        && let Some(sequence) = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::Coordinated
                | ironsmith_core::SequenceSurface::CommaThen
        ) {
        sequence.effects.as_slice()
    } else {
        for_players.effects.as_slice()
    };
    if effects.len() < 2 {
        return None;
    }
    let subject = describe_for_players_subject(&for_players.filter)?;
    let subject_lower = lowercase_first(subject);
    let mut phrases = Vec::with_capacity(effects.len());
    let mut effect_idx = 0usize;

    while effect_idx < effects.len() {
        if effect_idx + 1 < effects.len()
            && let Some(choose) =
                effects[effect_idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) = sacrifice_view(&effects[effect_idx + 1])
            && let Some(inner) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            phrases.push(iterated_player_action_phrase(
                &inner,
                subject,
                &subject_lower,
            )?);
            effect_idx += 2;
            continue;
        }

        if let Some(phrase) =
            iterated_player_structural_action_phrase(&effects[effect_idx], subject)
        {
            phrases.push(phrase);
            effect_idx += 1;
            continue;
        }

        let inner = describe_effect(&effects[effect_idx]);
        if inner.contains(". ")
            || inner.starts_with("For each ")
            || inner.starts_with("Choose ")
            || inner.starts_with("If ")
            || inner.starts_with("When ")
            || inner.starts_with("Whenever ")
            || inner.starts_with("At ")
        {
            return None;
        }
        phrases.push(iterated_player_action_phrase(
            &inner,
            subject,
            &subject_lower,
        )?);
        effect_idx += 1;
    }

    let last = phrases.pop()?;
    let body = if phrases.is_empty() {
        last
    } else if phrases.len() == 1
        && phrases[0].starts_with("exiles all cards from ")
        && last.starts_with("draws ")
    {
        format!("{} and {last}", phrases[0])
    } else if phrases.len() == 1
        && phrases[0].starts_with("gains ")
        && phrases[0].contains(" life")
        && last.starts_with("draws ")
    {
        format!("{} and {last}", phrases[0])
    } else {
        format!("{}, then {last}", phrases.join(", "))
    };
    Some(format!("{subject} {body}"))
}

pub(super) fn iterated_player_action_phrase(
    inner: &str,
    subject: &str,
    _subject_lower: &str,
) -> Option<String> {
    let rest = inner
        .strip_prefix("that player ")
        .or_else(|| inner.strip_prefix("That player "))?;
    if subject == "You" {
        Some(normalize_you_verb_phrase(rest))
    } else {
        Some(rewrite_iterated_player_references(
            &normalize_third_person_verb_phrase(rest),
        ))
    }
}

pub(super) fn rewrite_iterated_player_references(text: &str) -> String {
    text.replace("that player's", "their")
        .replace("to that player", "to them")
        .replace("for that player", "for them")
        .replace("from that player", "from them")
        .replace("by that player", "by them")
        .replace("that player", "they")
        .replace("they controls", "they control")
        .replace("they owns", "they own")
        .replace("they has", "they have")
        .replace("they is", "they are")
        .replace("they draws", "they draw")
        .replace("they discards", "they discard")
        .replace("they sacrifices", "they sacrifice")
        .replace("they shuffles", "they shuffle")
        .replace("they searches", "they search")
        .replace("they reveals", "they reveal")
        .replace("they puts", "they put")
        .replace("they returns", "they return")
        .replace("they exiles", "they exile")
        .replace("they pays", "they pay")
        .replace("they loses", "they lose")
        .replace("they gains", "they gain")
        .replace("they chooses", "they choose")
        .replace("they mills", "they mill")
        .replace("they scries", "they scry")
}

pub(super) fn iterated_player_structural_action_phrase(
    effect: &Effect,
    subject: &str,
) -> Option<String> {
    let verb = |you: &'static str, other: &'static str| {
        if subject == "You" { you } else { other }
    };
    let possessive = if subject == "You" { "your" } else { "their" };
    if let Some(exile) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::ExileEffect>()
        && exile.face_down
        && choose_spec_is_all_iterated_player_hand_cards(&exile.spec)
    {
        return Some(format!(
            "{} all cards from {possessive} hand face down",
            verb("exile", "exiles")
        ));
    }
    None
}

pub(super) fn choose_spec_is_all_iterated_player_hand_cards(spec: &ChooseSpec) -> bool {
    let ChooseSpec::All(filter) = spec.base() else {
        return false;
    };
    filter.zone == Some(Zone::Hand)
        && filter.owner == Some(PlayerFilter::IteratedPlayer)
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.any_of.is_empty()
        && filter.tagged_constraints.is_empty()
}

#[cfg(test)]
mod simple_create_token_bundle_tests {
    use super::*;

    #[test]
    fn looked_face_down_partition_preserves_chooser_order_surface() {
        let looked_tag = TagKey::from("looked");
        let chosen_tag = TagKey::from("chosen");
        let effects = vec![
            Effect::look_at_top_cards(PlayerFilter::You, 4, looked_tag.clone()),
            Effect::choose_objects(
                ObjectFilter::tagged(looked_tag.clone()).in_zone(Zone::Library),
                crate::effect::ChoiceCount::exactly(1),
                PlayerFilter::You,
                chosen_tag.clone(),
            ),
            Effect::new(
                crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(chosen_tag.clone()))
                    .with_face_down(true),
            ),
            Effect::put_tagged_remainder_on_library_bottom(
                looked_tag,
                Some(chosen_tag),
                LibraryBottomOrder::ChooserChooses,
                PlayerFilter::You,
            ),
        ];
        let refs = effects.iter().collect::<Vec<_>>();

        assert_eq!(
            describe_hideaway_effects(&refs).as_deref(),
            Some(
                "Look at the top four cards of your library, exile one face down, then put the rest on the bottom of your library in any order"
            )
        );
    }

    #[test]
    fn reveal_top_opponent_exiles_rest_hand_bundle_accepts_tagged_exile_move() {
        let looked_tag = TagKey::from("looked");
        let chosen_tag = TagKey::from("chosen");

        let mut chosen_filter = ObjectFilter::tagged(looked_tag.clone()).in_zone(Zone::Library);
        chosen_filter.excluded_card_types.push(CardType::Land);
        let remainder_filter = ObjectFilter::tagged(looked_tag.clone())
            .not_tagged(chosen_tag.clone())
            .in_zone(Zone::Library);
        let effects = vec![
            Effect::reveal_top_cards(PlayerFilter::You, 6, looked_tag),
            Effect::choose_objects(
                chosen_filter,
                crate::effect::ChoiceCount::exactly(1),
                PlayerFilter::Opponent,
                chosen_tag.clone(),
            ),
            Effect::move_to_zone(ChooseSpec::tagged(chosen_tag.clone()), Zone::Exile, true)
                .tag(chosen_tag.clone()),
            Effect::move_to_zone(ChooseSpec::Object(remainder_filter), Zone::Hand, false)
                .tag("moved"),
            Effect::may_player(
                PlayerFilter::Opponent,
                vec![Effect::cast_tagged(
                    chosen_tag,
                    PlayerFilter::Opponent,
                    false,
                    false,
                    true,
                    None,
                )],
            ),
        ];

        assert_eq!(
            describe_effect_list(&effects),
            "Reveal the top six cards of your library. An opponent exiles a nonland card from among them, then you put the rest into your hand. That opponent may cast the exiled card without paying its mana cost"
        );
    }

    #[test]
    fn compacts_direct_and_coordinated_full_typed_blueprint_lists() {
        let snake = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Snake")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Snake])
            .color_indicator(crate::color::ColorSet::GREEN)
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build();
        let wolf = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Wolf")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Wolf])
            .color_indicator(crate::color::ColorSet::GREEN)
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let effects = vec![
            Effect::with_id(
                0,
                Effect::new(crate::effects::CreateTokenEffect::new(
                    snake,
                    Value::Fixed(1),
                    PlayerFilter::You,
                ))
                .tag("created_snake"),
            ),
            Effect::new(crate::effects::CreateTokenEffect::new(
                wolf,
                Value::Fixed(1),
                PlayerFilter::You,
            ))
            .tag_all("created_wolf"),
        ];

        assert_eq!(
            describe_effect_list(&effects),
            "Create a 1/1 green Snake creature token and a 2/2 green Wolf creature token"
        );

        let coordinated = vec![Effect::new(crate::effects::SequenceEffect::coordinated(
            effects.clone(),
        ))];
        assert_eq!(
            describe_effect_list(&coordinated),
            "Create a 1/1 green Snake creature token and a 2/2 green Wolf creature token"
        );

        let halfling =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Halfling")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Halfling])
                .color_indicator(crate::color::ColorSet::WHITE)
                .power_toughness(crate::card::PowerToughness::fixed(1, 1))
                .build();
        let food = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Food")
            .token()
            .card_types(vec![CardType::Artifact])
            .build();
        let dynamic = vec![Effect::new(crate::effects::SequenceEffect::coordinated(
            vec![
                Effect::new(crate::effects::CreateTokenEffect::new(
                    halfling,
                    Value::X,
                    PlayerFilter::You,
                )),
                Effect::new(crate::effects::CreateTokenEffect::new(
                    food,
                    Value::X,
                    PlayerFilter::You,
                )),
            ],
        ))];
        assert_eq!(
            describe_effect_list(&dynamic),
            "Create X 1/1 white Halfling creature tokens and X Food tokens"
        );

        let draw = Effect::draw(Value::Fixed(1));
        let mixed = vec![&effects[0], &draw, &effects[1]];
        assert_eq!(describe_simple_create_token_bundle(&mixed), None);

        let sequential = Effect::new(crate::effects::SequenceEffect::new(effects));
        assert_eq!(describe_simple_create_token_bundle(&[&sequential]), None);
    }
}
