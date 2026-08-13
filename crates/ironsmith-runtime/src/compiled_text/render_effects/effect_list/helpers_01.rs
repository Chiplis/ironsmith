use super::helpers_00::*;
use super::*;

pub(crate) fn downcast_destroy_no_regeneration<'a>(
    effect: &'a Effect,
) -> Option<&'a crate::effects::DestroyNoRegenerationEffect> {
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        return Some(destroy);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return tagged
            .effect
            .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>();
    }
    effect
        .downcast_ref::<crate::effects::WithIdEffect>()?
        .effect
        .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
}

pub(crate) fn downcast_return_to_hand<'a>(
    effect: &'a Effect,
) -> Option<&'a crate::effects::ReturnToHandEffect> {
    unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::ReturnToHandEffect>()
}

pub(crate) fn filter_has_most_common_permanent_color_constraint(
    filter: &crate::target::ObjectFilter,
) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::target::TaggedOpbjectRelation::SharesMostCommonPermanentColor
    })
}

pub(crate) fn describe_targeted_most_common_color_conditional_destroy(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, conditional_effect] = effects else {
        return None;
    };
    let target_tag = effect_tag(target_effect)?;
    let target_only = downcast_target_only(target_effect)?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, filter) =
        &conditional.condition
    else {
        return None;
    };
    if condition_tag != target_tag || !filter_has_most_common_permanent_color_constraint(filter) {
        return None;
    }

    let target_text = describe_choose_spec(&target_only.target);
    let condition_text = "it shares a color with the most common color among all permanents or a color tied for most common";
    let true_effect = conditional.if_true.first()?;
    if downcast_destroy_no_regeneration(true_effect).is_some() {
        return Some(format!(
            "Destroy {target_text} if {condition_text}. A creature destroyed this way can't be regenerated"
        ));
    }
    if downcast_destroy(true_effect).is_some() {
        return Some(format!("Destroy {target_text} if {condition_text}"));
    }
    None
}

pub(crate) fn describe_targeted_most_common_color_conditional_return_to_hand(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, conditional_effect] = effects else {
        return None;
    };
    let target_tag = effect_tag(target_effect)?;
    let target_only = downcast_target_only(target_effect)?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, filter) =
        &conditional.condition
    else {
        return None;
    };
    if condition_tag != target_tag || !filter_has_most_common_permanent_color_constraint(filter) {
        return None;
    }

    let return_to_hand = downcast_return_to_hand(conditional.if_true.first()?)?;
    if return_to_hand.spec != target_only.target {
        return None;
    }

    let target_text = describe_choose_spec(&target_only.target);
    Some(format!(
        "Return {target_text} to its owner's hand if that permanent shares a color with the most common color among all permanents or a color tied for most common"
    ))
}

pub(crate) fn describe_targeted_conditional_destroy(effects: &[&Effect]) -> Option<String> {
    let [target_effect, conditional_effect] = effects else {
        return None;
    };
    let target_tag = effect_tag(target_effect)?;
    let target_only = downcast_target_only(target_effect)?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, _) = &conditional.condition
    else {
        return None;
    };
    if condition_tag != target_tag {
        return None;
    }

    // A branch that restates the outer target declaration adds nothing to
    // the surface; skip it before matching the destroy payload.
    let mut branch: &[Effect] = &conditional.if_true;
    if let [first, rest @ ..] = branch
        && let Some(inner_target) = first.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && inner_target.target == target_only.target
        && !rest.is_empty()
    {
        branch = rest;
    }
    let [true_effect] = branch else {
        return None;
    };
    let (may, true_effect) =
        if let Some(may) = true_effect.downcast_ref::<crate::effects::MayEffect>() {
            if may.effects.len() != 1 || !matches!(may.decider, None | Some(PlayerFilter::You)) {
                return None;
            }
            (true, &may.effects[0])
        } else {
            (false, true_effect)
        };
    let destroy_verb = if may { "You may destroy" } else { "Destroy" };

    let target_text = describe_choose_spec(&target_only.target);
    let condition_text = describe_condition(&conditional.condition);
    if let Some(destroy) = downcast_destroy_no_regeneration(true_effect)
        && destroy.spec == target_only.target
    {
        return Some(format!(
            "{destroy_verb} {target_text} if {condition_text}. A creature destroyed this way can't be regenerated"
        ));
    }
    if let Some(destroy) = downcast_destroy(true_effect)
        && destroy.spec == target_only.target
    {
        return Some(format!("{destroy_verb} {target_text} if {condition_text}"));
    }
    None
}

pub(crate) fn downcast_return_all_to_battlefield<'a>(
    effect: &'a Effect,
) -> Option<&'a crate::effects::ReturnAllToBattlefieldEffect> {
    if let Some(return_all) = effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
    {
        return Some(return_all);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return tagged
            .effect
            .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>();
    }
    effect
        .downcast_ref::<crate::effects::WithIdEffect>()?
        .effect
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
}

pub(crate) fn describe_filtered_future_exile_delayed_return_bundle(
    effects: &[&Effect],
) -> Option<String> {
    let [replacement_effect, schedule_effect] = effects else {
        return None;
    };
    let replacement = unwrap_render_wrappers(replacement_effect)
        .downcast_ref::<crate::effects::RegisterFutureZoneReplacementEffect>()?;
    if replacement.filter != ObjectFilter::permanent().controlled_by(PlayerFilter::You)
        || replacement.from_zone != Some(Zone::Battlefield)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || replacement.mode != crate::effects::ReplacementApplyMode::UntilEndOfTurn
        || replacement.cause_filter.is_some()
        || replacement.require_cause_source_match
        || !replacement.link_exiled_to_source
    {
        return None;
    }

    let schedule = unwrap_render_wrappers(schedule_effect)
        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    let end_step = schedule
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfEndStepTrigger>()?;
    if !schedule.one_shot
        || schedule.start_next_turn
        || schedule.until_end_of_turn
        || end_step.player != PlayerFilter::Any
    {
        return None;
    }
    let [return_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let return_all = downcast_return_all_to_battlefield(return_effect)?;
    if return_all.filter != ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile)
        || return_all.tapped
        || return_all.face_down
        || return_all.battlefield_controller != crate::effects::BattlefieldController::Owner
    {
        return None;
    }

    Some("If a permanent you control would be put into a graveyard from the battlefield this turn, exile it instead. Return it to the battlefield under its owner's control at the beginning of the next end step".to_string())
}

pub(crate) fn describe_mass_creature_change_graveyard_exile_future_replacement_bundle(
    effects: &[&Effect],
) -> Option<String> {
    let [continuous_effect, exile_effect, replacement_effect] = effects else {
        return None;
    };
    let continuous = unwrap_render_wrappers(continuous_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let affects_all_creatures = matches!(
        &continuous.target,
        crate::continuous::EffectTarget::AllCreatures
    ) || matches!(
        &continuous.target,
        crate::continuous::EffectTarget::Filter(filter) if filter == &ObjectFilter::creature()
    );
    if continuous.until != Until::EndOfTurn || !affects_all_creatures {
        return None;
    }

    let exile =
        unwrap_render_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(exile_filter) = &exile.spec else {
        return None;
    };
    if exile.face_down
        || exile_filter.zone != Some(Zone::Graveyard)
        || exile_filter.owner.is_some()
        || exile_filter.single_graveyard
        || exile_filter.card_types != vec![CardType::Creature]
        || !exile_filter.entered_graveyard_from_battlefield_this_turn
    {
        return None;
    }

    let replacement = unwrap_render_wrappers(replacement_effect)
        .downcast_ref::<crate::effects::RegisterFutureZoneReplacementEffect>()?;
    if replacement.filter != ObjectFilter::creature()
        || replacement.from_zone != Some(Zone::Battlefield)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || replacement.mode != crate::effects::ReplacementApplyMode::UntilEndOfTurn
        || replacement.cause_filter.is_some()
        || replacement.require_cause_source_match
        || replacement.link_exiled_to_source
    {
        return None;
    }

    let continuous = describe_effect(continuous_effect);
    let exile = describe_effect(exile_effect);
    let replacement = describe_effect(replacement_effect);
    Some(format!(
        "{}. {}. {}",
        continuous.trim_end_matches('.'),
        exile.trim_end_matches('.'),
        replacement.trim_end_matches('.'),
    ))
}

pub(crate) fn downcast_return_from_graveyard_to_battlefield<'a>(
    effect: &'a Effect,
) -> Option<&'a crate::effects::ReturnFromGraveyardToBattlefieldEffect> {
    if let Some(return_effect) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        return Some(return_effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return downcast_return_from_graveyard_to_battlefield(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return downcast_return_from_graveyard_to_battlefield(&with_id.effect);
    }
    None
}

pub(crate) fn downcast_mill<'a>(effect: &'a Effect) -> Option<&'a crate::effects::MillEffect> {
    unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::MillEffect>()
}

pub(crate) fn apply_removes_all_abilities(apply: &crate::effects::ApplyContinuousEffect) -> bool {
    apply.modification.as_ref().is_some_and(|modification| {
        *modification == crate::continuous::Modification::RemoveAllAbilities
    }) || apply
        .additional_modifications
        .iter()
        .any(|modification| *modification == crate::continuous::Modification::RemoveAllAbilities)
        || apply.runtime_modifications.iter().any(|modification| {
            matches!(
                modification,
                crate::effects::continuous::RuntimeModification::RemoveAllAbilities
            )
        })
}

pub(crate) fn each_subject_from_filter(filter: &crate::filter::ObjectFilter) -> String {
    let description = filter.description();
    description
        .strip_prefix("creatures ")
        .map(|rest| format!("creature {rest}"))
        .unwrap_or(description)
}

pub(crate) fn render_remove_abilities_then_destroy_matching_creatures(
    apply: &crate::effects::ApplyContinuousEffect,
    destroy: &crate::effects::DestroyEffect,
) -> Option<String> {
    let crate::continuous::EffectTarget::Filter(filter) = &apply.target else {
        return None;
    };
    let ChooseSpec::All(destroy_filter) = &destroy.spec else {
        return None;
    };
    if filter != destroy_filter
        || !filter
            .card_types
            .contains(&crate::types::CardType::Creature)
        || !apply_removes_all_abilities(apply)
    {
        return None;
    }
    let until = if matches!(apply.until, Until::Forever) {
        String::new()
    } else {
        format!(" {}", describe_until(&apply.until))
    };
    Some(format!(
        "Each {} loses all abilities{until}. Destroy those creatures",
        each_subject_from_filter(filter)
    ))
}

pub(crate) fn move_to_zone_uses_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
    zone: Zone,
) -> bool {
    move_to_zone.zone == zone
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found.as_str() == tag)
}

pub(crate) fn exile_uses_tag(exile: &crate::effects::ExileEffect, tag: &str) -> bool {
    match exile.spec.base() {
        ChooseSpec::Tagged(found) => found.as_str() == tag,
        ChooseSpec::Iterated => true,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter_is_tagged_as(filter, tag),
        _ => false,
    }
}

pub(crate) fn move_to_exile_uses_chosen_tag_or_iterated(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    if move_to_zone.zone != Zone::Exile {
        return false;
    }
    match move_to_zone.target.base() {
        ChooseSpec::Iterated => true,
        ChooseSpec::Tagged(effect_tag) => effect_tag.as_str() == tag,
        _ => false,
    }
}

pub(crate) fn is_all_cards_in_player_graveyard(
    filter: &ObjectFilter,
    player: &PlayerFilter,
) -> bool {
    let mut expected = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(player.clone());
    expected.single_graveyard = filter.single_graveyard;
    filter == &expected
}

pub(crate) fn describe_player_exile_controlled_creature_and_graveyard_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [choose_effect, exile_chosen_effect, exile_graveyard_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile_chosen = downcast_move_to_zone(exile_chosen_effect)?;
    let exile_graveyard = downcast_exile(exile_graveyard_effect)?;
    let player = &choose.chooser;

    let expected_creature = ObjectFilter::creature().controlled_by(player.clone());
    let ChooseSpec::All(graveyard_filter) = &exile_graveyard.spec else {
        return None;
    };
    if choose.is_search
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.filter != expected_creature
        || !move_to_exile_uses_chosen_tag_or_iterated(exile_chosen, choose.tag.as_str())
        || exile_graveyard.face_down
        || !is_all_cards_in_player_graveyard(graveyard_filter, player)
    {
        return None;
    }

    let subject_lower = describe_player_filter(player);
    let subject = capitalize_first(&subject_lower);
    let verb = player_verb(&subject_lower, "exile", "exiles");
    let controls = if subject_lower == "you" {
        "you control"
    } else {
        "they control"
    };
    let possessive = if subject_lower == "you" {
        "your"
    } else {
        "their"
    };
    Some(format!(
        "{subject} {verb} a creature {controls} and {possessive} graveyard"
    ))
}

pub(crate) fn describe_exile_split_pile_opponent_choice_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [exile_effect, tag_effect, choose_effect, unless_effect] = filtered else {
        return None;
    };

    let _exile = downcast_exile(exile_effect)?;
    let tag_matching = tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let unless_action = unless_effect.downcast_ref::<crate::effects::UnlessActionEffect>()?;

    if tag_matching.tag.as_str() != "divvy_source"
        || choose.tag.as_str() != "divvy_pile"
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Exile)
        || choose.is_search
        || !choose.count.is_any_number()
        || unless_action.player != PlayerFilter::Opponent
    {
        return None;
    }

    let [main_selected, main_rest] = unless_action.effects.as_slice() else {
        return None;
    };
    let [alt_selected, alt_rest] = unless_action.alternative.as_slice() else {
        return None;
    };

    let main_selected = downcast_move_to_zone(main_selected)?;
    let alt_selected =
        unwrap_tag_wrappers(alt_selected).downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    let (_, main_rest) = for_each_tagged_for_compaction(main_rest)?;
    let (_, alt_rest) = for_each_tagged_for_compaction(alt_rest)?;

    if !move_to_zone_uses_tag(main_selected, choose.tag.as_str(), Zone::Graveyard)
        || !return_to_hand_uses_chosen_tag(alt_selected, choose.tag.as_str())
        || !for_each_moves_unselected_to_zone(
            main_rest,
            tag_matching.tag.as_str(),
            choose.tag.as_str(),
            Zone::Hand,
        )
        || !for_each_moves_unselected_to_zone(
            alt_rest,
            tag_matching.tag.as_str(),
            choose.tag.as_str(),
            Zone::Graveyard,
        )
    {
        return None;
    }

    let exile_text =
        describe_effect(exile_effect).replace(" in your graveyard", " from your graveyard");
    Some(format!(
        "{exile_text} and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard."
    ))
}

pub(crate) fn describe_reveal_top_opponent_split_you_choose_pile_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let filtered = if let [first, rest @ ..] = filtered {
        if first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
        {
            rest
        } else {
            filtered
        }
    } else {
        filtered
    };
    let (reveal_effect, tag_effect, choose_player_effect, choose_effect, unless_effect) =
        match filtered {
            [reveal, tag, choose, unless] => (*reveal, *tag, None, *choose, *unless),
            [reveal, tag, choose_player, choose, unless] => {
                (*reveal, *tag, Some(*choose_player), *choose, *unless)
            }
            _ => return None,
        };

    let reveal = reveal_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let tag_matching = tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let unless_action = unless_effect.downcast_ref::<crate::effects::UnlessActionEffect>()?;

    let delegated_opponent = choose_player_effect
        .and_then(|effect| effect.downcast_ref::<crate::effects::ChoosePlayerEffect>())
        .is_some_and(|player_choice| {
            player_choice.chooser == PlayerFilter::You
                && player_choice.filter == PlayerFilter::Opponent
                && !player_choice.random
                && player_choice.excluded_tags.is_empty()
                && choose.chooser == PlayerFilter::TaggedPlayer(player_choice.tag.clone())
        });
    if choose_player_effect.is_some() != delegated_opponent
        || !reveal.reveal
        || reveal.player != PlayerFilter::You
        || tag_matching.tag.as_str() != "divvy_source"
        || choose.tag.as_str() != "divvy_pile"
        || (!delegated_opponent && choose.chooser != PlayerFilter::Opponent)
        || choose_primary_zone(choose) != Some(Zone::Library)
        || choose.is_search
        || !choose.count.is_any_number()
        || unless_action.player != PlayerFilter::You
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == tag_matching.tag
        })
    {
        return None;
    }

    let [main_selected, main_rest] = unless_action.effects.as_slice() else {
        return None;
    };
    let [alt_selected, alt_rest] = unless_action.alternative.as_slice() else {
        return None;
    };

    let main_selected = downcast_move_to_zone(main_selected)?;
    let alt_selected = downcast_move_to_zone(alt_selected)?;
    let (_, main_rest) = for_each_tagged_for_compaction(main_rest)?;
    let (_, alt_rest) = for_each_tagged_for_compaction(alt_rest)?;

    if !move_to_zone_uses_tag(main_selected, choose.tag.as_str(), Zone::Hand)
        || !move_to_zone_uses_tag(alt_selected, choose.tag.as_str(), Zone::Graveyard)
        || !for_each_moves_unselected_to_zone(
            main_rest,
            tag_matching.tag.as_str(),
            choose.tag.as_str(),
            Zone::Graveyard,
        )
        || !for_each_moves_unselected_to_zone(
            alt_rest,
            tag_matching.tag.as_str(),
            choose.tag.as_str(),
            Zone::Hand,
        )
    {
        return None;
    }

    let reveal_text = describe_effect(reveal_effect);
    Some(format!(
        "{reveal_text}. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard."
    ))
}

/// Render an exact exiled collection divided by a correlated opponent, where
/// the controller chooses which pile returns and the complement is disposed.
/// Every collection relationship is proven by tags; unrelated exile objects
/// and unrelated player choices deliberately fall through.
pub(crate) fn describe_exiled_collection_opponent_split_you_choose_pile_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [
        exile_effect,
        tag_effect,
        choose_player_effect,
        choose_effect,
        selected_effect,
        rest_effect,
    ] = filtered
    else {
        return None;
    };
    let exile = downcast_exile(exile_effect)?;
    if exile.face_down {
        return None;
    }
    let exile_result_tag =
        if let Some(tagged) = exile_effect.downcast_ref::<crate::effects::TaggedEffect>() {
            &tagged.tag
        } else {
            return None;
        };
    let tag_source = tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let choose_player =
        choose_player_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_player.chooser != PlayerFilter::You
        || choose_player.filter != PlayerFilter::Opponent
        || choose_player.random
        || !choose_player.excluded_tags.is_empty()
        || choose.chooser != PlayerFilter::TaggedPlayer(choose_player.tag.clone())
        || choose.is_search
        || !choose.count.is_any_number()
        || choose_primary_zone(choose) != Some(Zone::Exile)
        || tag_matching_zones(tag_source)? != vec![Zone::Exile]
        || !filter_is_tagged_as(&tag_source.filter, exile_result_tag.as_str())
        || !filter_is_tagged_as(&choose.filter, tag_source.tag.as_str())
    {
        return None;
    }
    let selected = downcast_move_to_zone(selected_effect)?;
    if !move_to_zone_uses_tag(selected, choose.tag.as_str(), Zone::Battlefield)
        || selected.battlefield_controller != crate::effects::BattlefieldController::You
        || effect_moves_unselected_to_zone(
            rest_effect,
            tag_source.tag.as_str(),
            choose.tag.as_str(),
        ) != Some(Zone::Graveyard)
    {
        return None;
    }

    let exile_text = describe_effect(exile_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{exile_text}. An opponent separates those cards into two piles. Put all cards from the pile of your choice onto the battlefield under your control and the rest into their owners' graveyards."
    ))
}

pub(crate) fn filter_has_not_tagged_constraint(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            && constraint.tag.as_str() == tag
    })
}

pub(crate) fn is_creature_filter_in_zone(
    filter: &ObjectFilter,
    zone: Zone,
    owner: Option<PlayerFilter>,
) -> bool {
    filter.zone == Some(zone)
        && filter.card_types == vec![CardType::Creature]
        && filter.subtypes.is_empty()
        && filter.colors.is_none()
        && filter.owner == owner
}

pub(crate) fn describe_creature_pile_destroy_bundle(filtered: &[&Effect]) -> Option<String> {
    let [choose_effect, destroy_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let destroy = downcast_destroy_no_regeneration(destroy_effect)?;

    if choose.tag.as_str() != "divvy_chosen"
        || choose.is_search
        || !choose.count.is_any_number()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.filter.card_types != vec![CardType::Creature]
        || !matches!(
            choose.filter.controller.as_ref(),
            Some(PlayerFilter::Target(_))
        )
        || !matches!(&destroy.spec, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    Some(
        "Separate all creatures target player controls into two piles. Destroy all creatures in the pile of that player's choice. They can't be regenerated."
            .to_string(),
    )
}

pub(crate) fn describe_graveyard_creature_pile_exile_return_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let (choose_player_effect, choose_effect, exile_effect, return_effect) = match filtered {
        [choose, exile, returned] => (None, *choose, *exile, *returned),
        [choose_player, choose, exile, returned] => {
            (Some(*choose_player), *choose, *exile, *returned)
        }
        _ => return None,
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = downcast_move_to_zone(exile_effect)?;
    let return_all = downcast_return_all_to_battlefield(return_effect)?;

    let delegated_opponent = choose_player_effect
        .and_then(|effect| effect.downcast_ref::<crate::effects::ChoosePlayerEffect>())
        .is_some_and(|player_choice| {
            player_choice.chooser == PlayerFilter::You
                && player_choice.filter == PlayerFilter::Opponent
                && !player_choice.random
                && player_choice.excluded_tags.is_empty()
                && choose.chooser == PlayerFilter::TaggedPlayer(player_choice.tag.clone())
        });
    if choose_player_effect.is_some() != delegated_opponent
        || choose.tag.as_str() != "divvy_chosen"
        || (!delegated_opponent && choose.chooser != PlayerFilter::Opponent)
        || choose.is_search
        || !choose.count.is_any_number()
        || choose_primary_zone(choose) != Some(Zone::Graveyard)
        || !is_creature_filter_in_zone(&choose.filter, Zone::Graveyard, Some(PlayerFilter::You))
        || !(move_to_zone_uses_tag(exile, choose.tag.as_str(), Zone::Exile)
            || (exile.zone == Zone::Exile && matches!(exile.target, ChooseSpec::Iterated)))
        || return_all.tapped
        || !is_creature_filter_in_zone(&return_all.filter, Zone::Graveyard, Some(PlayerFilter::You))
        || !filter_has_not_tagged_constraint(&return_all.filter, choose.tag.as_str())
    {
        return None;
    }

    Some(
        "Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield."
            .to_string(),
    )
}

pub(crate) fn describe_damage_and_die_replacement_bundle(filtered: &[&Effect]) -> Option<String> {
    let [first_effect, second_effect, replacement_effect] = filtered else {
        return None;
    };
    let first_tagged = first_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let first = first_tagged
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let second = second_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    let replacement =
        replacement_effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?;

    if first.amount != Value::Fixed(5)
        || second.amount != Value::Fixed(2)
        || !matches!(first.target.base(), ChooseSpec::Object(filter) if filter.card_types == vec![CardType::Creature])
        || !matches!(second.target.base(), ChooseSpec::Player(PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag))) if tag == &first_tagged.tag)
        || !matches!(&replacement.target, ChooseSpec::Tagged(tag) if tag == &first_tagged.tag)
        || replacement.from_zone != Some(Zone::Battlefield)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || !matches!(
            replacement.mode,
            crate::effects::ReplacementApplyMode::OneShot
                | crate::effects::ReplacementApplyMode::UntilEndOfTurn
        )
    {
        return None;
    }

    Some(
        "Deal 5 damage to target creature and 2 damage to that creature's controller. If that creature would die this turn, exile it instead."
            .to_string(),
    )
}

pub(crate) fn describe_compound_damage_regeneration_exile_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [damage_effect, conditional_effect] = filtered else {
        return None;
    };
    let damage_tag = wrapped_effect_tag(damage_effect)?;
    unwrap_render_wrappers(damage_effect).downcast_ref::<crate::effects::DealDamageEffect>()?;

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [cant_effect, replacement_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    if !conditional.if_false.is_empty() {
        return None;
    }

    let cant = unwrap_render_wrappers(cant_effect).downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::BeRegenerated(regeneration_filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn
        || regeneration_filter.card_types != vec![CardType::Creature]
        || !filter_is_tagged_as(regeneration_filter, damage_tag.as_str())
    {
        return None;
    }

    let replacement = unwrap_render_wrappers(replacement_effect)
        .downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?;
    if !matches!(replacement.target.base(), ChooseSpec::Tagged(tag) if tag == damage_tag)
        || replacement.from_zone != Some(Zone::Battlefield)
        || replacement.to_zone != Some(Zone::Graveyard)
        || replacement.replacement_zone != Zone::Exile
        || !matches!(
            replacement.mode,
            crate::effects::ReplacementApplyMode::OneShot
                | crate::effects::ReplacementApplyMode::UntilEndOfTurn
        )
        || replacement.optional
        || !replacement.counters.is_empty()
    {
        return None;
    }

    let followup = match &conditional.condition {
        Condition::TaggedObjectMatches(tag, filter)
            if tag == damage_tag && filter.card_types == vec![CardType::Creature] =>
        {
            "If it's a creature, it can't be regenerated this turn, and if it would die this turn, exile it instead"
        }
        Condition::ThisSpellWasKicked => {
            "If this spell was kicked, that creature can't be regenerated this turn and if it would die this turn, exile it instead"
        }
        _ => return None,
    };

    let damage = describe_effect(damage_effect);
    Some(format!("{}. {followup}", damage.trim_end_matches('.')))
}

fn effect_structurally_counters_spell(effect: &Effect) -> bool {
    let effect = unwrap_render_wrappers(effect);
    if let Some(counter) = effect.downcast_ref::<crate::effects::CounterEffect>() {
        return matches!(
            counter.target.base(),
            ChooseSpec::Object(filter)
                if filter.zone == Some(Zone::Stack)
                    && filter.stack_kind == Some(crate::filter::StackObjectKind::Spell)
        );
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        return unless_pays
            .effects
            .iter()
            .any(effect_structurally_counters_spell);
    }
    if let Some(local) = effect.downcast_ref::<crate::effects::LocalRewriteEffect>() {
        return effect_structurally_counters_spell(&local.effect);
    }
    false
}

pub(crate) trait ZoneReplacementSurface {
    fn target(&self) -> &ChooseSpec;
    fn from_zone(&self) -> Option<Zone>;
    fn to_zone(&self) -> Option<Zone>;
    fn replacement_zone(&self) -> Zone;
    fn library_placement(&self) -> Option<ironsmith_core::ZoneReplacementLibraryPlacement>;
    fn mode(&self) -> crate::effects::ReplacementApplyMode;
    fn optional(&self) -> bool;
    fn has_counters(&self) -> bool;
}

macro_rules! impl_zone_replacement_surface {
    ($ty:ty) => {
        impl ZoneReplacementSurface for $ty {
            fn target(&self) -> &ChooseSpec {
                &self.target
            }
            fn from_zone(&self) -> Option<Zone> {
                self.from_zone
            }
            fn to_zone(&self) -> Option<Zone> {
                self.to_zone
            }
            fn replacement_zone(&self) -> Zone {
                self.replacement_zone
            }
            fn library_placement(&self) -> Option<ironsmith_core::ZoneReplacementLibraryPlacement> {
                self.library_placement
            }
            fn mode(&self) -> crate::effects::ReplacementApplyMode {
                self.mode
            }
            fn optional(&self) -> bool {
                self.optional
            }
            fn has_counters(&self) -> bool {
                !self.counters.is_empty()
            }
        }
    };
}

impl_zone_replacement_surface!(crate::effects::RegisterZoneReplacementEffect);
impl_zone_replacement_surface!(ironsmith_core::RegisterZoneReplacementEffect);

fn is_countered_spell_zone_replacement(replacement: &impl ZoneReplacementSurface) -> bool {
    replacement.from_zone() == Some(Zone::Stack)
        && replacement.to_zone() == Some(Zone::Graveyard)
        && matches!(
            replacement.mode(),
            crate::effects::ReplacementApplyMode::OneShot
        )
        && !replacement.optional()
        && !replacement.has_counters()
        && match replacement.replacement_zone() {
            Zone::Exile | Zone::Hand => replacement.library_placement().is_none(),
            Zone::Library => replacement.library_placement().is_some(),
            _ => false,
        }
}

pub(crate) fn describe_countered_spell_exile_replacement_followup<R>(
    producer: &Effect,
    replacement: &R,
) -> Option<String>
where
    R: ZoneReplacementSurface,
{
    if !is_countered_spell_zone_replacement(replacement)
        || !effect_structurally_counters_spell(producer)
    {
        return None;
    }
    let ChooseSpec::Tagged(replacement_tag) = replacement.target() else {
        return None;
    };
    if wrapped_effect_tag(producer) != Some(replacement_tag) {
        return None;
    }
    match (
        replacement.replacement_zone(),
        replacement.library_placement(),
    ) {
        (Zone::Exile, None) => Some(
            "If that spell is countered this way, exile it instead of putting it into its owner's graveyard"
                .to_string(),
        ),
        (Zone::Hand, None) => Some(
            "If that spell is countered this way, put it into its owner's hand instead of into that player's graveyard"
                .to_string(),
        ),
        (
            Zone::Library,
            Some(ironsmith_core::ZoneReplacementLibraryPlacement::Top),
        ) => Some(
            "If that spell is countered this way, put it on top of its owner's library instead of into that player's graveyard"
                .to_string(),
        ),
        (
            Zone::Library,
            Some(ironsmith_core::ZoneReplacementLibraryPlacement::Bottom),
        ) => Some(
            "If that spell is countered this way, put it on the bottom of its owner's library instead of into that player's graveyard"
                .to_string(),
        ),
        (
            Zone::Library,
            Some(ironsmith_core::ZoneReplacementLibraryPlacement::TopOrBottom),
        ) => Some(
            "If that spell is countered this way, put that card on your choice of the top or bottom of its owner's library instead of into that player's graveyard"
                .to_string(),
        ),
        _ => None,
    }
}

pub(crate) fn describe_countered_spell_exile_replacement_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let (replacement_effect, prefix) = filtered.split_last()?;
    let replacement =
        replacement_effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?;
    let followup = prefix.iter().rev().find_map(|producer| {
        describe_countered_spell_exile_replacement_followup(producer, replacement)
    })?;
    let prefix = prefix
        .iter()
        .map(|effect| (*effect).clone())
        .collect::<Vec<_>>();
    let base = describe_effect_list(&prefix);
    if base.trim().is_empty() {
        return None;
    }
    Some(format!("{}. {followup}", base.trim_end_matches('.')))
}

fn describe_death_filter_subject(filter: &ObjectFilter, demonstrative: bool) -> Option<String> {
    if !matches!(filter.zone, None | Some(Zone::Battlefield)) {
        return None;
    }
    let description = filter.description();
    let noun = strip_leading_article(&description).trim();
    if noun.is_empty() || noun.contains("tagged '") {
        return None;
    }
    if demonstrative {
        Some(format!("that {noun}"))
    } else {
        Some(with_indefinite_article(noun))
    }
}

fn describe_death_choose_referent(
    target: &ChooseSpec,
    qualify_non_target_as_damaged: bool,
) -> Option<String> {
    let filter = match target.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return None,
    };
    let targeted = target.is_target();
    let mut referent = describe_death_filter_subject(filter, targeted)?;
    if !targeted && qualify_non_target_as_damaged {
        referent.push_str(" dealt damage this way");
    }
    Some(referent)
}

fn describe_prior_action_death_referent(producer: &Effect) -> Option<String> {
    let producer = unwrap_render_wrappers(producer);
    if let Some(damage) = producer.downcast_ref::<crate::effects::DealDamageEffect>() {
        return describe_death_choose_referent(&damage.target, true);
    }
    let apply = producer.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if let Some(target_spec) = &apply.target_spec {
        return describe_death_choose_referent(target_spec, false);
    }
    match &apply.target {
        crate::continuous::EffectTarget::Filter(filter) => {
            describe_death_filter_subject(filter, false)
        }
        crate::continuous::EffectTarget::AllCreatures => Some("a creature".to_string()),
        crate::continuous::EffectTarget::AllPermanents => Some("a permanent".to_string()),
        _ => None,
    }
}

pub(crate) fn describe_tagged_die_exile_replacement_followup<R>(
    producer: &Effect,
    replacement: &R,
) -> Option<String>
where
    R: ZoneReplacementSurface,
{
    if replacement.from_zone() != Some(Zone::Battlefield)
        || replacement.to_zone() != Some(Zone::Graveyard)
        || replacement.replacement_zone() != Zone::Exile
        || !matches!(
            replacement.mode(),
            crate::effects::ReplacementApplyMode::OneShot
                | crate::effects::ReplacementApplyMode::UntilEndOfTurn
        )
        || replacement.optional()
        || replacement.has_counters()
    {
        return None;
    }
    let referent = if let Some(fight) =
        unwrap_render_wrappers(producer).downcast_ref::<crate::effects::FightEffect>()
    {
        if !target_specs_select_same_objects(&fight.creature2, replacement.target()) {
            return None;
        }
        if !fight.creature1.is_target() {
            "that creature".to_string()
        } else {
            let second_fighter = describe_choose_spec(&fight.creature2);
            let second_fighter = second_fighter
                .strip_prefix("target ")
                .unwrap_or(second_fighter.as_str());
            format!("the {second_fighter}")
        }
    } else {
        let (replacement_tag, narrowed_filter) = match replacement.target().base() {
            ChooseSpec::Tagged(tag) => (tag, None),
            ChooseSpec::Object(filter) => {
                let mut tagged = filter.tagged_constraints.iter().filter(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                });
                let tag = &tagged.next()?.tag;
                if tagged.next().is_some() {
                    return None;
                }
                (tag, Some(filter))
            }
            _ => return None,
        };
        if wrapped_effect_tag(producer) != Some(replacement_tag) {
            return None;
        }
        if let Some(filter) = narrowed_filter {
            describe_death_filter_subject(filter, false)?
        } else {
            describe_prior_action_death_referent(producer)?
        }
    };
    Some(format!(
        "If {referent} would die this turn, exile it instead"
    ))
}

pub(crate) fn describe_tagged_die_exile_replacement_bundle(filtered: &[&Effect]) -> Option<String> {
    let (replacement_effect, prefix) = filtered.split_last()?;
    let replacement =
        replacement_effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?;
    let followup = prefix.iter().rev().find_map(|producer| {
        describe_tagged_die_exile_replacement_followup(producer, replacement)
    })?;
    let prefix = prefix
        .iter()
        .map(|effect| (*effect).clone())
        .collect::<Vec<_>>();
    let base = describe_effect_list(&prefix);
    if base.trim().is_empty() {
        return None;
    }
    Some(format!("{}. {followup}", base.trim_end_matches('.')))
}

pub(crate) fn describe_filtered_mill_then_draw_bundle(filtered: &[&Effect]) -> Option<String> {
    let (draw_effect, prefix) = filtered.split_last()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You {
        return None;
    }
    let Value::Count(filter) = &draw.count else {
        return None;
    };
    let counted = describe_milled_graveyard_count_filter(filter)?;

    let mill_effect = prefix.last()?;
    let mill_tag = wrapped_effect_tag(mill_effect)?;
    unwrap_render_wrappers(mill_effect).downcast_ref::<crate::effects::MillEffect>()?;
    if !filter_is_tagged_as(filter, mill_tag.as_str()) {
        return None;
    }

    let prefix = prefix
        .iter()
        .map(|effect| (*effect).clone())
        .collect::<Vec<_>>();
    let mill = describe_effect_list(&prefix);
    if mill.trim().is_empty() {
        return None;
    }
    Some(format!(
        "{}. You draw a card for each {counted}",
        mill.trim_end_matches('.')
    ))
}

pub(crate) fn describe_reveal_hand_exile_same_name_search_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [
        look_effect,
        choose_effect,
        exile_effect,
        for_each_search_effect,
        trailing @ ..,
    ] = filtered
    else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let exile = downcast_move_to_zone(exile_effect)?;
    let for_each_search_effects = if let Some(for_each) =
        for_each_search_effect.downcast_ref::<crate::effects::ForEachObject>()
    {
        if for_each.filter.zone != Some(Zone::Exile)
            || !(filter_is_tagged_as(&for_each.filter, choose.tag.as_str())
                || filter_is_tagged_as(&for_each.filter, crate::tag::SOURCE_EXILED_TAG))
        {
            return None;
        }
        for_each.effects.as_slice()
    } else if let Some(for_each) =
        for_each_search_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        if for_each.tag != choose.tag && for_each.tag.as_str() != crate::tag::SOURCE_EXILED_TAG {
            return None;
        }
        for_each.effects.as_slice()
    } else {
        return None;
    };

    let (search_effect, for_each_move_effect, shuffle_effect) =
        match (for_each_search_effects, trailing) {
            ([search_effect], [for_each_move_effect, shuffle_effect]) => {
                (search_effect, *for_each_move_effect, *shuffle_effect)
            }
            ([search_effect, for_each_move_effect], [shuffle_effect]) => {
                (search_effect, for_each_move_effect, *shuffle_effect)
            }
            _ => return None,
        };

    if !look.reveal
        || !matches!(
            look.target.base(),
            ChooseSpec::Player(PlayerFilter::Opponent)
        )
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || !choose.count.is_up_to_dynamic_x()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || !choose.filter.excluded_card_types.contains(&CardType::Land)
        || !(move_to_zone_uses_tag(exile, choose.tag.as_str(), Zone::Exile)
            || (exile.zone == Zone::Exile && matches!(exile.target, ChooseSpec::Iterated)))
    {
        return None;
    }

    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each_move =
        for_each_move_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [move_effect] = for_each_move.effects.as_slice() else {
        return None;
    };
    let move_to_exile = downcast_move_to_zone(move_effect)?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !search.is_search
        || choose_search_zones(search)? != vec![Zone::Graveyard, Zone::Hand, Zone::Library]
        || search.chooser != PlayerFilter::You
        || search.count.min != 0
        || search.count.max.is_some()
        || search.search_mode != SearchSelectionMode::Optional
        || !search.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                && constraint.tag.as_str() == "__it__"
        })
        || for_each_move.tag != search.tag
        || !move_to_zone_uses_tag(move_to_exile, search.tag.as_str(), Zone::Exile)
        || shuffle.player != PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
    {
        return None;
    }

    Some(
        "Target opponent reveals their hand. Choose up to X nonland cards from it and exile them. Search that player's graveyard, hand, and library for any number of cards with the same name as those cards and exile them. Then that player shuffles."
            .to_string(),
    )
}

pub(crate) fn describe_reveal_hand_choose_shuffle_into_library_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [look_effect, choose_effect, shuffle_effect] = filtered else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let shuffle = unwrap_tag_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;

    if !look.reveal
        || !matches!(
            look.target.base(),
            ChooseSpec::Player(PlayerFilter::Opponent)
        )
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.filter.owner != Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        || !matches!(shuffle.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || shuffle.player != PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
    {
        return None;
    }

    Some(
        "Target opponent reveals their hand. You choose a card from it. That player shuffles that card into their library."
            .to_string(),
    )
}

pub(crate) fn describe_tempting_offer_creature_return_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [choose_self_effect, return_self_effect, for_players_effect] = filtered else {
        return None;
    };
    let choose_self = choose_self_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let return_self = downcast_return_from_graveyard_to_battlefield(return_self_effect)?;
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;

    if choose_self.chooser != PlayerFilter::You
        || choose_exact_count(choose_self) != Some(1)
        || !is_creature_filter_in_zone(
            &choose_self.filter,
            Zone::Graveyard,
            Some(PlayerFilter::You),
        )
        || !matches!(&return_self.target, ChooseSpec::Tagged(tag) if tag == &choose_self.tag)
        || return_self.tapped
        || for_players.filter != PlayerFilter::Opponent
        || for_players.effects.len() != 2
    {
        return None;
    }

    let may_with_id = for_players.effects[0].downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = may_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let [choose_opp_effect, return_opp_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose_opp = choose_opp_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let return_opp = downcast_return_from_graveyard_to_battlefield(return_opp_effect)?;
    if may.decider.as_ref() != Some(&PlayerFilter::IteratedPlayer)
        || choose_opp.chooser != PlayerFilter::IteratedPlayer
        || choose_exact_count(choose_opp) != Some(1)
        || !is_creature_filter_in_zone(
            &choose_opp.filter,
            Zone::Graveyard,
            Some(PlayerFilter::IteratedPlayer),
        )
        || !matches!(&return_opp.target, ChooseSpec::Tagged(tag) if tag == &choose_opp.tag)
        || return_opp.tapped
    {
        return None;
    }
    let if_effect = for_players.effects[1].downcast_ref::<crate::effects::IfEffect>()?;
    let [choose_bonus_effect, return_bonus_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let choose_bonus = choose_bonus_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let return_bonus = downcast_return_from_graveyard_to_battlefield(return_bonus_effect)?;
    if if_effect.condition != may_with_id.id
        || !matches!(
            if_effect.predicate,
            crate::effect::EffectPredicate::Happened | crate::effect::EffectPredicate::Chosen
        )
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    if choose_bonus.chooser != PlayerFilter::You
        || choose_exact_count(choose_bonus) != Some(1)
        || !is_creature_filter_in_zone(
            &choose_bonus.filter,
            Zone::Graveyard,
            Some(PlayerFilter::You),
        )
        || !matches!(&return_bonus.target, ChooseSpec::Tagged(tag) if tag == &choose_bonus.tag)
        || return_bonus.tapped
    {
        return None;
    }

    Some(
        "Return a creature card from your graveyard to the battlefield. Each opponent may return a creature card from their graveyard to the battlefield. For each opponent who does, return a creature card from your graveyard to the battlefield."
            .to_string(),
    )
}

pub(crate) fn describe_mill_return_land_else_counter_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    if let [mill_effect, choose_effect, for_each_effect, if_effect] = filtered {
        let mill = downcast_mill(mill_effect)?;
        let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
        if mill.player != PlayerFilter::You
            || mill.count != Value::Fixed(3)
            || choose.chooser != PlayerFilter::You
            || choose.count.min != 0
            || choose.count.max != Some(1)
            || choose.filter.card_types != vec![CardType::Land]
            || choose_primary_zone(choose) != Some(Zone::Graveyard)
            || if_effect.then.len() != 1
            || !if_effect.else_.is_empty()
        {
            return None;
        }
        let (_, for_each) = for_each_tagged_for_compaction(for_each_effect)?;
        if for_each.tag != choose.tag || for_each.effects.len() != 1 {
            return None;
        }
        let returned_effect = unwrap_tag_wrappers(&for_each.effects[0]);
        let returns_iterated_to_hand = returned_effect
            .downcast_ref::<crate::effects::ReturnToHandEffect>()
            .is_some_and(|return_to_hand| {
                matches!(return_to_hand.spec.base(), ChooseSpec::Iterated)
                    || matches!(return_to_hand.spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "__it__")
            })
            || returned_effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .is_some_and(|move_to_zone| {
                    move_to_zone.zone == Zone::Hand
                        && matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
                });
        if !returns_iterated_to_hand {
            return None;
        }
        let put_counter = if_effect.then[0].downcast_ref::<crate::effects::PutCountersEffect>()?;
        if put_counter.counter_type != crate::object::CounterType::PlusOnePlusOne
            || put_counter.amount != Value::Fixed(1)
            || !matches!(put_counter.target, ChooseSpec::Source)
        {
            return None;
        }
        return Some(
            "You mill three cards. You may return a land card from a graveyard to your hand. If you don't, put a +1/+1 counter on this creature"
                .to_string(),
        );
    }

    let [mill_effect, with_id_effect, if_effect] = filtered else {
        return None;
    };
    let mill = downcast_mill(mill_effect)?;
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;

    if mill.player != PlayerFilter::You
        || mill.count != Value::Fixed(3)
        || may.effects.len() != 2
        || if_effect.condition != with_id.id
        || if_effect.predicate != crate::effect::EffectPredicate::Failed
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let choose = may.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let return_to_hand = may.effects[1].downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    let [put_counter_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let put_counter = put_counter_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max != Some(1)
        || choose.filter.card_types != vec![CardType::Land]
        || choose_primary_zone(choose) != Some(Zone::Graveyard)
        || !matches!(&return_to_hand.spec, ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || put_counter.counter_type != crate::object::CounterType::PlusOnePlusOne
        || put_counter.amount != Value::Fixed(1)
        || !matches!(put_counter.target, ChooseSpec::Source)
    {
        return None;
    }

    Some(
        "You mill three cards. You may return a land card from a graveyard to your hand. If you don't, put a +1/+1 counter on this creature"
            .to_string(),
    )
}

pub(crate) fn describe_dynamic_pt_token_bundle(filtered: &[&Effect]) -> Option<String> {
    if let [create_effect, set_pt_effect] = filtered {
        let create = downcast_create_token(create_effect)?;
        let set_pt = downcast_set_base_power_toughness(set_pt_effect)?;
        let created_tag = wrapped_effect_tag(create_effect)?;
        if create.enters_tapped
            || create.enters_attacking
            || create.exile_at_end_of_combat
            || create.sacrifice_at_end_of_combat
            || create.sacrifice_at_next_end_step
            || create.exile_at_next_end_step
            || set_pt.duration != Until::Forever
            || !matches!(&set_pt.target, ChooseSpec::Tagged(tag) if tag == created_tag)
        {
            return None;
        }

        let pt_text = format!(
            "{}/{}",
            describe_value(&set_pt.power),
            describe_value(&set_pt.toughness)
        );
        let create_text = describe_effect(unwrap_tag_wrappers(create_effect));
        if set_pt.power.unhinted() == set_pt.toughness.unhinted()
            && !matches!(set_pt.power.unhinted(), Value::Fixed(_))
        {
            let compact = create_text
                .replacen("a 0/0 ", "an X/X ", 1)
                .replacen("an 0/0 ", "an X/X ", 1);
            if compact != create_text {
                return Some(format!(
                    "{compact}, where X is {}",
                    describe_dynamic_token_pt_value(&set_pt.power)
                ));
            }
        }
        let compact = create_text.replacen("0/0 ", &format!("{pt_text} "), 1);
        if compact != create_text {
            return normalize_dynamic_equal_pt_create_text(&compact).or(Some(compact));
        }
        return None;
    }

    let [first_effect, create_effect, set_pt_effect] = filtered else {
        return None;
    };
    let create = downcast_create_token(create_effect)?;
    let set_pt = downcast_set_base_power_toughness(set_pt_effect)?;
    let created_tag = wrapped_effect_tag(create_effect);
    if create.count != Value::Fixed(1)
        || create.enters_tapped
        || create.enters_attacking
        || set_pt.duration != Until::Forever
    {
        return None;
    }
    if let Some(created_tag) = created_tag {
        if !matches!(&set_pt.target, ChooseSpec::Tagged(tag) if tag == created_tag) {
            return None;
        }
    } else if !matches!(set_pt.target, ChooseSpec::Source) {
        return None;
    }
    let (Value::PowerOf(power_spec), Value::ToughnessOf(toughness_spec)) =
        (&set_pt.power, &set_pt.toughness)
    else {
        return None;
    };

    let exiled_creature_from_graveyard =
        downcast_exile(first_effect).is_some_and(|exile| {
            matches!(exile.spec.base(), ChooseSpec::Object(filter) if filter.zone == Some(Zone::Graveyard) && filter.card_types == vec![CardType::Creature])
        }) || downcast_move_to_zone(first_effect).is_some_and(|move_to_zone| {
            move_to_zone.zone == Zone::Exile
                && matches!(move_to_zone.target.base(), ChooseSpec::Object(filter) if filter.zone == Some(Zone::Graveyard) && filter.card_types == vec![CardType::Creature])
        });
    if exiled_creature_from_graveyard {
        if !matches!(
            (power_spec.as_ref(), toughness_spec.as_ref()),
            (ChooseSpec::Tagged(_), ChooseSpec::Tagged(_))
        ) {
            return None;
        }
        let mut token_phrase = describe_create_token_blueprint(create);
        token_phrase = token_phrase.replace("0/0 ", "");
        return Some(format!(
            "{}. Create {} with base power and toughness equal to that card's power and toughness.",
            describe_effect(first_effect).replace(" in your graveyard", " from your graveyard"),
            with_indefinite_article(&token_phrase)
        ));
    }

    downcast_destroy_no_regeneration(first_effect)?;
    if !matches!(
        (power_spec.as_ref(), toughness_spec.as_ref()),
        (ChooseSpec::Tagged(_), ChooseSpec::Tagged(_))
    ) {
        return None;
    }
    let mut token_phrase = describe_create_token_blueprint(create);
    token_phrase = token_phrase.replace("0/0 ", "");
    Some(format!(
        "{}. Create {} with base power and toughness equal to that creature's power and toughness{}.",
        describe_effect(first_effect),
        with_indefinite_article(&token_phrase),
        if create.sacrifice_at_next_end_step {
            ". Sacrifice it at the beginning of the next end step"
        } else {
            ""
        }
    ))
}

pub(crate) fn prior_effect_count_metric(value: &Value) -> Option<crate::effect::EffectId> {
    match value.unhinted() {
        Value::EffectValue(id)
        | Value::EffectMetric {
            effect_id: id,
            metric: crate::effect::EffectMetric::Count | crate::effect::EffectMetric::AffectedCount,
            ..
        } => Some(*id),
        _ => None,
    }
}

pub(crate) fn prior_effect_count_subject(effect: &Effect) -> Option<(String, &'static str)> {
    fn clean_subject(filter: &ObjectFilter) -> String {
        let mut subject = describe_count_filter_value_subject(filter);
        for suffix in [
            " in exile",
            " in all graveyards",
            " in a graveyard",
            " in graveyard",
            " on the battlefield",
        ] {
            if let Some(stripped) = subject.strip_suffix(suffix) {
                subject = stripped.to_string();
                break;
            }
        }
        subject
    }

    fn choose_subject(spec: &ChooseSpec) -> Option<String> {
        match spec.base() {
            ChooseSpec::All(filter) | ChooseSpec::Object(filter) => Some(clean_subject(filter)),
            ChooseSpec::WithCount(inner, _) => choose_subject(inner),
            _ => None,
        }
    }

    let effect = unwrap_render_wrappers(effect);
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return choose_subject(&destroy.spec).map(|subject| (subject, "destroyed"));
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return choose_subject(&exile.spec).map(|subject| (subject, "exiled"));
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        return Some((clean_subject(&sacrifice.filter), "sacrificed"));
    }
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>()
        && let Some(filter) = discard.card_filter.as_ref()
    {
        return Some((clean_subject(filter), "discarded"));
    }
    None
}

pub(crate) fn describe_prior_effect_count_create_token_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [prior_effect, create_effect] = filtered else {
        return None;
    };
    let with_id = prior_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let create = downcast_create_token(create_effect)?;
    if prior_effect_count_metric(&create.count) != Some(with_id.id)
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
    let (subject, action) = prior_effect_count_subject(&with_id.effect)?;
    let subject = if action == "exiled"
        && value_has_surface_hint(&create.count, ValueSurfaceHint::CardsExiledThisWay)
    {
        "card".to_string()
    } else {
        subject
    };
    let prior_text = describe_effect(prior_effect)
        .replace(
            " in target player's graveyard",
            " from target player's graveyard",
        )
        .replace(" in your graveyard", " from your graveyard")
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{prior_text}, then create a {} for each {subject} {action} this way",
        describe_create_token_blueprint(create)
    ))
}

pub(crate) fn describe_prior_effect_dynamic_count_token_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [prior_effect, create_effect, set_pt_effect] = filtered else {
        return None;
    };
    let create = downcast_create_token(create_effect)?;
    let set_pt = downcast_set_base_power_toughness(set_pt_effect)?;
    let created_tag = wrapped_effect_tag(create_effect)?;
    if create.count != Value::Fixed(1)
        || create.enters_tapped
        || create.enters_attacking
        || set_pt.duration != Until::Forever
        || set_pt.power.unhinted() != set_pt.toughness.unhinted()
        || matches!(set_pt.power.unhinted(), Value::Fixed(_))
        || !matches!(&set_pt.target, ChooseSpec::Tagged(tag) if tag == created_tag)
    {
        return None;
    }
    let (producer, linked_by_tag) =
        if let Some(with_id) = prior_effect.downcast_ref::<crate::effects::WithIdEffect>() {
            if prior_effect_count_metric(&set_pt.power) != Some(with_id.id) {
                return None;
            }
            (with_id.effect.as_ref(), false)
        } else {
            let producer_tag = wrapped_effect_tag(prior_effect)?;
            let Value::Count(filter) = set_pt.power.unhinted() else {
                return None;
            };
            let references_producer = filter.tagged_constraints.iter().any(|constraint| {
                &constraint.tag == producer_tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            });
            if !references_producer
                || !matches!(create.controller, PlayerFilter::You)
                || create.controller_target.is_some()
                || create.exile_at_end_of_combat
                || create.sacrifice_at_end_of_combat
                || create.sacrifice_at_next_end_step
                || create.exile_at_next_end_step
            {
                return None;
            }
            (*prior_effect, true)
        };
    let (subject, action) = prior_effect_count_subject(producer)?;
    if linked_by_tag {
        let producer_tag = wrapped_effect_tag(prior_effect)?;
        if tagged_this_way_action(producer_tag.as_str()) != Some(action) {
            return None;
        }
    }
    let dynamic_pt_prefix = if create.enters_tapped {
        "tapped X/X "
    } else {
        "X/X "
    };
    let token_phrase =
        describe_create_token_blueprint(create).replacen("0/0 ", dynamic_pt_prefix, 1);
    let controller_suffix = if matches!(create.controller, PlayerFilter::You) {
        String::new()
    } else {
        format!(
            " under {} control",
            describe_possessive_player_filter(&create.controller)
        )
    };
    let producer_text = describe_effect(prior_effect);
    if linked_by_tag {
        Some(format!(
            "{}, then create {}{}, where X is the number of {subject} {action} this way",
            producer_text.trim_end_matches('.'),
            with_indefinite_article(&token_phrase),
            controller_suffix
        ))
    } else {
        Some(format!(
            "{producer_text}. Create {}{}, where X is the number of {subject} {action} this way",
            with_indefinite_article(&token_phrase),
            controller_suffix
        ))
    }
}

pub(crate) fn describe_create_token_then_set_base_pt_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    fn append_token_cleanup(
        mut text: String,
        create: &crate::effects::CreateTokenEffect,
    ) -> String {
        if create.sacrifice_at_end_of_combat {
            text.push_str(". Sacrifice it at end of combat");
        }
        if create.sacrifice_at_next_end_step {
            let timing = describe_next_end_step_cleanup_timing(&create.next_end_step_player);
            text.push_str(&format!(". Sacrifice it at the beginning of {timing}"));
        }
        if create.exile_at_end_of_combat {
            text.push_str(". Exile it at end of combat");
        }
        if create.exile_at_next_end_step {
            let timing = describe_next_end_step_cleanup_timing(&create.next_end_step_player);
            text.push_str(&format!(". Exile it at the beginning of {timing}"));
        }
        text
    }

    fn append_token_entry_and_cleanup(
        mut text: String,
        create: &crate::effects::CreateTokenEffect,
    ) -> String {
        if create.enters_tapped && create.enters_attacking {
            text.push_str(" that's tapped and attacking");
        } else if create.enters_attacking {
            text.push_str(" that's attacking");
        } else if create.enters_tapped {
            text.push_str(" tapped");
        }
        append_token_cleanup(text, create)
    }

    let [create_effect, set_pt_effect] = filtered else {
        return None;
    };
    let create = downcast_create_token(create_effect)?;
    let set_pt = downcast_set_base_power_toughness(set_pt_effect)?;
    let created_tag = wrapped_effect_tag(create_effect)?;
    if create.count != Value::Fixed(1) || set_pt.duration != Until::Forever {
        return None;
    }
    if !matches!(&set_pt.target, ChooseSpec::Tagged(tag) if tag == created_tag) {
        return None;
    }

    let token_blueprint = describe_create_token_blueprint(create);
    if matches!(set_pt.power, Value::SourcePower | Value::PowerOf(_))
        && matches!(
            set_pt.toughness,
            Value::SourceToughness | Value::ToughnessOf(_)
        )
    {
        let token_phrase = token_blueprint.replacen("0/0 ", "", 1);
        let controller_suffix = if matches!(create.controller, PlayerFilter::You) {
            String::new()
        } else {
            format!(
                " under {} control",
                describe_possessive_player_filter(&create.controller)
            )
        };
        let mut text = format!(
            "Create {}{}",
            with_indefinite_article(&token_phrase),
            controller_suffix
        );
        text = append_token_entry_and_cleanup(text, create);
        text.push_str(&format!(
            ". Its power is equal to {} and its toughness is equal to {}",
            describe_value(&set_pt.power),
            describe_value(&set_pt.toughness)
        ));
        return Some(text);
    }

    let power_fixed = matches!(set_pt.power.unhinted(), Value::Fixed(_));
    let toughness_fixed = matches!(set_pt.toughness.unhinted(), Value::Fixed(_));
    let (dynamic_pt, basis) = match (power_fixed, toughness_fixed) {
        (false, false) if set_pt.power.unhinted() == set_pt.toughness.unhinted() => {
            ("X/X".to_string(), &set_pt.power)
        }
        (false, true) => (
            format!("X/{}", describe_value(&set_pt.toughness)),
            &set_pt.power,
        ),
        (true, false) => (
            format!("{}/X", describe_value(&set_pt.power)),
            &set_pt.toughness,
        ),
        _ => return None,
    };

    let rest = token_blueprint
        .strip_prefix("0/0 ")
        .or_else(|| token_blueprint.strip_prefix("X/X "))?;
    let token_phrase = if create.enters_tapped {
        format!("tapped {dynamic_pt} {rest}")
    } else {
        format!("{dynamic_pt} {rest}")
    };
    let controller_suffix = if matches!(create.controller, PlayerFilter::You) {
        String::new()
    } else {
        format!(
            " under {} control",
            describe_possessive_player_filter(&create.controller)
        )
    };

    Some(append_token_cleanup(
        format!(
            "Create {}{}, where X is {}",
            with_indefinite_article(&token_phrase),
            controller_suffix,
            describe_dynamic_token_pt_value(basis)
        ),
        create,
    ))
}

pub(crate) fn describe_dynamic_token_pt_value(value: &Value) -> String {
    match value.unhinted() {
        Value::Count(filter) => {
            describe_dynamic_token_count_value(filter, 1).unwrap_or_else(|| describe_value(value))
        }
        Value::CountScaled(filter, multiplier) => {
            describe_dynamic_token_count_value(filter, *multiplier)
                .unwrap_or_else(|| describe_value(value))
        }
        _ => describe_value(value),
    }
}

pub(crate) fn describe_dynamic_token_count_value(
    filter: &ObjectFilter,
    multiplier: i32,
) -> Option<String> {
    let subject = describe_filter_subject_this_way(filter)?;
    Some(if multiplier == 1 {
        format!("the number of {subject}")
    } else if multiplier == 2 {
        format!("twice the number of {subject}")
    } else {
        format!("{multiplier} times the number of {subject}")
    })
}

pub(crate) fn describe_filter_subject_this_way(filter: &ObjectFilter) -> Option<String> {
    let action = filter.tagged_constraints.iter().find_map(|constraint| {
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        tagged_this_way_action(constraint.tag.as_str())
    })?;
    let mut subject = describe_count_filter_value_subject(filter);
    for suffix in [
        " in exile",
        " in all graveyards",
        " in a graveyard",
        " in graveyard",
        " on the battlefield",
    ] {
        if let Some(stripped) = subject.strip_suffix(suffix) {
            subject = stripped.to_string();
            break;
        }
    }
    Some(format!("{subject} {action} this way"))
}

pub(crate) fn tagged_this_way_action(tag: &str) -> Option<&'static str> {
    if let Some(action) = tag_action_from_name(tag) {
        return Some(action);
    }
    let base = tag.split('_').next().unwrap_or(tag);
    match base {
        "exile" => Some("exiled"),
        "discard" => Some("discarded"),
        "sacrifice" => Some("sacrificed"),
        _ => None,
    }
}

pub(crate) fn describe_reveal_power_cards_for_mana_bundle(filtered: &[&Effect]) -> Option<String> {
    let [choose_effect, reveal_effect, mana_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let mana = mana_effect.downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || reveal.tag != choose.tag
        || mana.player != PlayerFilter::You
        || mana.mana != vec![crate::mana::ManaSymbol::Green]
        || !matches!(&mana.amount, Value::Count(filter) if filter_is_tagged_as(filter, choose.tag.as_str()))
    {
        return None;
    }
    let mut selection = choose.filter.clone();
    selection.zone = None;
    selection.owner = None;
    selection.controller = None;
    selection.tagged_constraints.clear();
    let mut selection = pluralize_noun_phrase(&selection.description());
    if let Some(rest) = selection.strip_prefix("creatures ") {
        selection = format!("creature cards {rest}");
    } else if selection == "creatures" {
        selection = "creature cards".to_string();
    } else if !selection.contains("card") {
        selection.push_str(" cards");
    }
    Some(format!(
        "Reveal any number of {selection} from your hand. Add {{G}} for each card revealed this way"
    ))
}

pub(crate) fn describe_reveal_top_hand_or_graveyard_bundle(filtered: &[&Effect]) -> Option<String> {
    let [reveal_effect, choose_effect, if_effect] = filtered else {
        return None;
    };
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTopEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if reveal.player != PlayerFilter::You
        || reveal.tag.is_none()
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Library)
        || if_effect.else_.len() != 1
        || if_effect.then.len() != 1
    {
        return None;
    }
    let to_hand = if_effect.then[0].downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    let to_graveyard = downcast_move_to_zone(&if_effect.else_[0])?;
    if !matches!(&to_hand.spec, ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || !move_to_zone_uses_tag(to_graveyard, choose.tag.as_str(), Zone::Graveyard)
    {
        return None;
    }
    Some(
        "Reveal the top card of your library. Put it into your hand or into your graveyard"
            .to_string(),
    )
}

pub(crate) fn describe_each_player_choose_unselected_bounce_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [for_players_effect, return_effect, rest @ ..] = filtered else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let return_to_hand = return_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if for_players.filter != PlayerFilter::Any || for_players.effects.len() != 1 {
        return None;
    }
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let ChooseSpec::All(filter) = &return_to_hand.spec else {
        return None;
    };
    if choose.chooser != PlayerFilter::IteratedPlayer
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !choose.filter.excluded_card_types.contains(&CardType::Land)
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || !filter.excluded_card_types.contains(&CardType::Land)
        || !filter_has_not_tagged_constraint(filter, choose.tag.as_str())
    {
        return None;
    }
    let mut rendered = "Each player chooses a nonland permanent they control. Return all nonland permanents not chosen this way to their owners' hands.".to_string();
    if !rest.is_empty() {
        rendered.push(' ');
        rendered.push_str(&describe_effect_list(
            &rest.iter().copied().cloned().collect::<Vec<_>>(),
        ));
    }
    Some(rendered)
}

pub(crate) fn describe_grant_keyword_and_unblockable_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [apply_effect, choose_effect, cant_effect] = filtered else {
        return None;
    };
    let apply = apply_continuous_for_compaction(apply_effect)?;
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let apply_filter = match &apply.target {
        crate::continuous::EffectTarget::Filter(filter) => filter,
        crate::continuous::EffectTarget::Source => match apply.target_spec.as_ref()?.base() {
            ChooseSpec::Object(filter) => filter,
            ChooseSpec::Target(inner) => match inner.base() {
                ChooseSpec::Object(filter) => filter,
                _ => return None,
            },
            _ => return None,
        },
        _ => return None,
    };
    let chosen_tag =
        if let Some(choose) = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            if choose.filter != *apply_filter
                || choose.chooser != PlayerFilter::You
                || choose_exact_count(choose) != Some(1)
            {
                return None;
            }
            choose.tag.as_str()
        } else {
            target_only_tag(choose_effect)?
        };
    if apply.until != Until::EndOfTurn
        || apply_filter.card_types != vec![CardType::Creature]
        || !matches!(
            apply_filter.power,
            None | Some(crate::filter::Comparison::LessThanOrEqual(2))
        )
    {
        return None;
    }
    let mut keywords = Vec::new();
    for modification in apply
        .modification
        .iter()
        .chain(apply.additional_modifications.iter())
    {
        let crate::continuous::Modification::AddAbility(ability) = modification else {
            return None;
        };
        keywords.push(keyword_label_from_static_ability_id(ability.id())?);
    }
    let [keyword] = keywords.as_slice() else {
        return None;
    };
    let crate::effect::Restriction::BeBlocked(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn || !filter_is_tagged_as(filter, chosen_tag) {
        return None;
    }
    Some(format!(
        "{} gains {keyword} until end of turn and can't be blocked this turn",
        capitalize_first(&describe_choose_spec(apply.target_spec.as_ref()?))
    ))
}

pub(crate) fn describe_return_creature_mana_value_scry_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [return_effect, conditional_effect] = filtered else {
        return None;
    };
    let return_to_hand = return_effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .or_else(|| {
            return_effect
                .downcast_ref::<crate::effects::TaggedEffect>()?
                .effect
                .downcast_ref::<crate::effects::ReturnToHandEffect>()
        })?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !matches!(return_to_hand.spec.base(), ChooseSpec::Object(filter) if filter.card_types == vec![CardType::Creature])
        || !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
    {
        return None;
    }
    let scry = conditional.if_true[0].downcast_ref::<crate::effects::ScryEffect>()?;
    if scry.player != PlayerFilter::You || scry.count != Value::Fixed(1) {
        return None;
    }
    Some(
        "Return target creature to its owner's hand. If that creature's mana value was 3 or less, scry 1"
            .to_string(),
    )
}

pub(crate) fn describe_exchange_control_bundle(filtered: &[&Effect]) -> Option<String> {
    let (choose_player_one, choose_player_two, choose_one, choose_two, swap_one, swap_two) =
        match filtered {
            [
                target_one,
                target_two,
                choose_one,
                choose_two,
                swap_one,
                swap_two,
            ] if target_one
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some()
                && target_two
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some() =>
            {
                (None, None, *choose_one, *choose_two, *swap_one, *swap_two)
            }
            [
                choose_player_one,
                choose_player_two,
                choose_one,
                choose_two,
                swap_one,
                swap_two,
            ] => (
                choose_player_one.downcast_ref::<crate::effects::ChoosePlayerEffect>(),
                choose_player_two.downcast_ref::<crate::effects::ChoosePlayerEffect>(),
                *choose_one,
                *choose_two,
                *swap_one,
                *swap_two,
            ),
            _ => return None,
        };
    let choose_one = choose_one.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let choose_two = choose_two.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_one.count.min != 0
        || choose_one.count.max.is_some()
        || choose_two.count_value.is_none()
        || choose_one.filter.card_types != vec![CardType::Creature]
        || choose_two.filter.card_types != vec![CardType::Creature]
    {
        return None;
    }
    if let (Some(player_one), Some(player_two)) = (choose_player_one, choose_player_two) {
        if choose_one.filter.controller != Some(PlayerFilter::TaggedPlayer(player_one.tag.clone()))
            || choose_two.filter.controller
                != Some(PlayerFilter::TaggedPlayer(player_two.tag.clone()))
        {
            return None;
        }
    }
    let first = describe_effect(swap_one).to_ascii_lowercase();
    let second = describe_effect(swap_two).to_ascii_lowercase();
    if !first.contains("gains control") || !second.contains("gains control") {
        return None;
    }
    Some(
        "Choose any number of creatures target player controls. Choose the same number of creatures another target player controls. Those players exchange control of those creatures."
            .to_string(),
    )
}

pub(crate) fn describe_graveyard_mana_ladder_return_bundle(filtered: &[&Effect]) -> Option<String> {
    let [first_choose, second_choose, third_choose, return_effect] = filtered else {
        return None;
    };
    let chooses = [
        first_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        second_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
        third_choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?,
    ];
    let card_types = chooses[0].filter.card_types.as_slice();
    let supported_card_types = card_types == [CardType::Creature]
        || card_types == [CardType::Artifact, CardType::Creature];
    if !supported_card_types {
        return None;
    }
    for (idx, choose) in chooses.iter().enumerate() {
        let chooses_zero_or_one = choose.count.min == 0 && choose.count.max == Some(1);
        if choose.chooser != PlayerFilter::You
            || (!chooses_zero_or_one && choose_exact_count(choose) != Some(1))
            || choose.filter.zone != Some(Zone::Graveyard)
            || choose.filter.owner != Some(PlayerFilter::You)
            || choose.filter.card_types.as_slice() != card_types
            || choose.filter.mana_value != Some(crate::filter::Comparison::Equal((idx + 1) as i32))
        {
            return None;
        }
    }
    let returned_target = if let Some(return_to_battlefield) =
        unwrap_tag_wrappers(return_effect)
            .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        if return_to_battlefield.tapped {
            return None;
        }
        &return_to_battlefield.target
    } else {
        let move_to_zone = unwrap_tag_wrappers(return_effect)
            .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        if move_to_zone.zone != Zone::Battlefield
            || move_to_zone.enters_tapped
            || move_to_zone.enters_attacking
            || move_to_zone.enters_face_down
        {
            return None;
        }
        &move_to_zone.target
    };
    if !matches!(returned_target, ChooseSpec::Tagged(tag) if tag == &chooses[0].tag)
        && !matches!(returned_target, ChooseSpec::Iterated)
    {
        return None;
    }
    let discarded_tag = chooses[0]
        .filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str().contains("discarded_mana_ladder")
        })
        .map(|constraint| &constraint.tag);
    if let Some(discarded_tag) = discarded_tag
        && !chooses.iter().all(|choose| {
            choose.filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && &constraint.tag == discarded_tag
            })
        })
    {
        return None;
    }

    if card_types == [CardType::Artifact, CardType::Creature] && discarded_tag.is_some() {
        Some(
            "You may choose an artifact or creature card with mana value 1 you discarded this way, then do the same for artifact or creature cards with mana values 2 and 3. Return those cards to the battlefield."
                .to_string(),
        )
    } else {
        Some(
            "Choose a creature card with mana value 1 in your graveyard, then do the same for creature cards with mana value 2 and 3. Return those cards to the battlefield."
                .to_string(),
        )
    }
}

pub(crate) fn describe_linked_graveyard_choices_then_may_return_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [first_choose_effect, second_choose_effect, may_effect] = filtered else {
        return None;
    };
    let first_choose = first_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let second_choose =
        second_choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_to_zone = downcast_move_to_zone(move_effect)?;

    if first_choose.is_search
        || second_choose.is_search
        || first_choose.replace_tagged_objects
        || second_choose.replace_tagged_objects
        || first_choose.tag != second_choose.tag
        || choose_exact_count(first_choose) != Some(1)
        || choose_exact_count(second_choose) != Some(1)
        || choose_primary_zone(first_choose) != Some(Zone::Graveyard)
        || choose_primary_zone(second_choose) != Some(Zone::Graveyard)
        || !matches!(
            &second_choose.chooser,
            PlayerFilter::AliasedOwnerOf(crate::filter::ObjectRef::Tagged(tag))
                | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag))
                if tag == &first_choose.tag
        )
        || !move_to_battlefield_uses_chosen_tag(move_to_zone, first_choose.tag.as_str())
    {
        return None;
    }

    let describe_choose_clause = |choose: &crate::effects::ChooseObjectsEffect,
                                  capitalize_subject: bool| {
        let chooser = describe_player_filter(&choose.chooser);
        let chosen = describe_choose_selection(choose);
        let location = describe_choose_zone_location(choose, "graveyard");
        if chooser == "you" {
            return format!("Choose {chosen} {location}");
        }
        let subject = if capitalize_subject {
            capitalize_first(&chooser)
        } else {
            chooser.clone()
        };
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        format!("{subject} {choose_verb} {chosen} {location}")
    };

    let first_clause = describe_choose_clause(first_choose, true);
    let second_clause = describe_choose_clause(second_choose, false);
    let tapped_suffix = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_to_zone.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => " under their owners' control",
        crate::effects::BattlefieldController::You => " under your control",
    };
    let decider = may
        .decider
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    let may_clause = if decider == "you" {
        format!("You may return those cards to the battlefield{tapped_suffix}{controller_suffix}")
    } else {
        let may_verb = player_verb(&decider, "may", "may");
        format!(
            "{} {may_verb} return those cards to the battlefield{tapped_suffix}{controller_suffix}",
            capitalize_first(&decider)
        )
    };

    Some(format!(
        "{first_clause}, then {second_clause}. {may_clause}"
    ))
}

pub(crate) fn describe_random_hand_reveal_damage_bundle(filtered: &[&Effect]) -> Option<String> {
    let [choose_effect, reveal_effect, damage_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !choose.count.random
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || reveal.tag != choose.tag
    {
        return None;
    }
    let Value::ManaValueOf(spec) = &damage.amount else {
        return None;
    };
    if !matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }
    let chooser = describe_player_filter(&choose.chooser);
    let reveal_subject = match &choose.chooser {
        PlayerFilter::You => "You reveal a card at random from your hand",
        PlayerFilter::Opponent => "Target opponent reveals a card at random from their hand",
        player if is_target_opponent_player_filter(player) => {
            "Target opponent reveals a card at random from their hand"
        }
        PlayerFilter::Target(_) => "Target player reveals a card at random from their hand",
        PlayerFilter::IteratedPlayer => "That player reveals a card at random from their hand",
        _ if chooser == "the damaged player" => {
            "That player reveals a card at random from their hand"
        }
        _ => "That player reveals a card at random from their hand",
    };
    let mut target = describe_choose_spec(&damage.target);
    if matches!(choose.chooser, PlayerFilter::Target(_)) && target == "target player" {
        target = "that player".to_string();
    }
    Some(format!(
        "{reveal_subject}. Deal damage to {target} equal to that card's mana value"
    ))
}

pub(crate) fn describe_random_hand_reveal_life_loss_bundle(filtered: &[&Effect]) -> Option<String> {
    let [choose_effect, reveal_effect, lose_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if !choose.count.random
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || reveal.tag != choose.tag
    {
        return None;
    }
    let Value::ManaValueOf(spec) = &lose.amount else {
        return None;
    };
    if !matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }
    let player = match &choose.chooser {
        PlayerFilter::You => "You",
        PlayerFilter::Opponent => "Target opponent",
        player if is_target_opponent_player_filter(player) => "Target opponent",
        PlayerFilter::Target(_) => "Target player",
        PlayerFilter::IteratedPlayer => "That player",
        _ => "That player",
    };
    let reveal_verb = player_verb(player, "reveal", "reveals");
    let hand = describe_possessive_player_filter(&choose.chooser);
    let mut loser = describe_choose_spec(&lose.player);
    if matches!(choose.chooser, PlayerFilter::Target(_)) && loser == "target player" {
        loser = "that player".to_string();
    }
    let lose_verb = player_verb(&loser, "lose", "loses");
    let same_revealing_player = loser.eq_ignore_ascii_case(player)
        || (matches!(choose.chooser, PlayerFilter::Target(_)) && loser == "that player");
    let loss_clause = if same_revealing_player {
        format!("{lose_verb} life equal to that card's mana value")
    } else {
        format!("{loser} {lose_verb} life equal to that card's mana value")
    };
    Some(format!(
        "{player} {reveal_verb} a card at random from {hand} hand, then {loss_clause}"
    ))
}

pub(crate) fn describe_random_hand_reveal_bundle(filtered: &[&Effect]) -> Option<String> {
    let [choose_effect, reveal_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if !choose.count.random
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || reveal.tag != choose.tag
    {
        return None;
    }
    let subject = match &choose.chooser {
        PlayerFilter::You => "You reveal a card at random from your hand",
        PlayerFilter::Opponent => "Target opponent reveals a card at random from their hand",
        player if is_target_opponent_player_filter(player) => {
            "Target opponent reveals a card at random from their hand"
        }
        PlayerFilter::Target(_) => "Target player reveals a card at random from their hand",
        PlayerFilter::IteratedPlayer => "That player reveals a card at random from their hand",
        _ => "That player reveals a card at random from their hand",
    };
    Some(subject.to_string())
}

pub(crate) fn describe_choose_then_reveal_from_hand_bundle(filtered: &[&Effect]) -> Option<String> {
    let [choose_effect, reveal_effect] = filtered else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect
        .downcast_ref::<crate::effects::RevealTaggedEffect>()
        .or_else(|| {
            reveal_effect
                .downcast_ref::<crate::effects::WithIdEffect>()
                .and_then(|with_id| {
                    with_id
                        .effect
                        .downcast_ref::<crate::effects::RevealTaggedEffect>()
                })
        })?;
    if reveal.tag != choose.tag
        || choose.is_search
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose.count.dynamic_x
        || choose.count.random
        || choose_primary_zone(choose) != Some(Zone::Hand)
    {
        return None;
    }

    let mut selection = describe_choose_selection(choose);
    if let Some(rest) = selection.strip_prefix("any number ") {
        selection = format!("any number of {}", rest.trim());
    }
    let hand_owner = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    let hand = format!("{} hand", describe_possessive_player_filter(hand_owner));
    Some(format!("Reveal {selection} in {hand}"))
}

pub(crate) fn describe_choose_reveal_from_hand_then_reflexive_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [choose_effect, reveal_effect, reflexive_effect] = filtered else {
        return None;
    };
    let with_id = reveal_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let reflexive = reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()?;
    if reflexive.condition != with_id.id {
        return None;
    }

    let reveal_text =
        describe_choose_then_reveal_from_hand_bundle(&[*choose_effect, *reveal_effect])?;
    let condition = match reflexive.predicate {
        EffectPredicate::Happened => "When you do".to_string(),
        EffectPredicate::DidNotHappen => "When you don't".to_string(),
        EffectPredicate::HappenedNotReplaced => "When you do and it isn't replaced".to_string(),
        _ => format!("When {}", describe_effect_predicate(&reflexive.predicate)),
    };
    let triggered = lowercase_first(&describe_result_branch_effect_list(&reflexive.effects));
    Some(format!("{reveal_text}. {condition}, {triggered}"))
}

pub(crate) fn describe_self_unblockable_bundle(filtered: &[&Effect]) -> Option<String> {
    let (apply_effect, chosen_tag, cant_effect) = match filtered {
        [apply_effect, cant_effect] => (*apply_effect, None, *cant_effect),
        [apply_effect, choose_effect, cant_effect] => {
            let chosen_tag = if let Some(choose) =
                choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            {
                if choose.chooser != PlayerFilter::You
                    || choose_exact_count(choose) != Some(1)
                    || !choose.filter.source
                {
                    return None;
                }
                choose.tag.as_str()
            } else {
                target_only_tag(choose_effect)?
            };
            (*apply_effect, Some(chosen_tag), *cant_effect)
        }
        _ => return None,
    };
    let apply = apply_continuous_for_compaction(apply_effect)?;
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if apply.until != Until::EndOfTurn
        || !matches!(apply.target, crate::continuous::EffectTarget::Source)
    {
        return None;
    }
    let has_shroud = apply.modification.as_ref().is_some_and(|modification| {
        matches!(modification, crate::continuous::Modification::AddAbility(ability) if ability.id() == crate::static_abilities::StaticAbilityId::Shroud)
    }) || apply.additional_modifications.iter().any(|modification| {
        matches!(modification, crate::continuous::Modification::AddAbility(ability) if ability.id() == crate::static_abilities::StaticAbilityId::Shroud)
    });
    let crate::effect::Restriction::BeBlocked(filter) = &cant.restriction else {
        return None;
    };
    let same_object = if let Some(chosen_tag) = chosen_tag {
        filter_is_tagged_as(filter, chosen_tag)
    } else {
        object_filters_equivalent_ignoring_source_surface(filter, &ObjectFilter::source())
    };
    if !has_shroud || cant.duration != Until::EndOfTurn || !same_object {
        return None;
    }
    Some("This creature gains shroud and can't be blocked this turn".to_string())
}

pub(crate) fn describe_source_pump_unblockable_bundle(filtered: &[&Effect]) -> Option<String> {
    let [apply_effect, cant_effect] = filtered else {
        return None;
    };
    let apply = apply_continuous_for_compaction(apply_effect)?;
    if apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || !matches!(apply.target, crate::continuous::EffectTarget::Source)
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = apply.runtime_modifications.as_slice()
    else {
        return None;
    };

    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::BeBlocked(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn || !filter.source {
        return None;
    }

    Some(format!(
        "This creature gets {}/{} until end of turn and can't be blocked this turn",
        describe_signed_value(power),
        describe_toughness_delta_with_power_context(power, toughness),
    ))
}

pub(crate) fn describe_target_pump_unblockable_bundle(filtered: &[&Effect]) -> Option<String> {
    let [apply_effect, cant_effect] = filtered else {
        return None;
    };
    // The result tag is semantically meaningful here: the unblockable filter
    // consumes the exact set modified by the pump. It need not be one of the
    // parser's implicit-reference tag names for this structural compaction.
    let apply = tagged_apply_continuous(apply_effect)?;
    if apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = apply.runtime_modifications.as_slice()
    else {
        return None;
    };
    let target_spec = apply.target_spec.as_ref()?;
    let target_text = describe_choose_spec(target_spec);
    if !target_text.to_ascii_lowercase().starts_with("target ") {
        return None;
    }

    let cant = cant_be_blocked_view(cant_effect)?;
    let crate::effect::Restriction::BeBlocked(filter) = &cant.restriction else {
        return None;
    };
    let pumped_tag = effect_tag(apply_effect)?;
    if cant.duration != Until::EndOfTurn || !filter_is_tagged_as(filter, pumped_tag.as_str()) {
        return None;
    }

    Some(format!(
        "{target_text} gets {}/{} until end of turn and can't be blocked this turn",
        describe_signed_value(power),
        describe_toughness_delta_with_power_context(power, toughness),
    ))
}

pub(crate) fn describe_tap_freeze_bundle(filtered: &[&Effect]) -> Option<String> {
    let [tap_effect, cant_effect] = filtered else {
        return None;
    };
    let (tap, tagged_tap) =
        if let Some(tagged) = tap_effect.downcast_ref::<crate::effects::TaggedEffect>() {
            (
                tagged.effect.downcast_ref::<crate::effects::TapEffect>()?,
                Some(tagged.tag.as_str()),
            )
        } else {
            (
                tap_effect.downcast_ref::<crate::effects::TapEffect>()?,
                None,
            )
        };
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    let target_text = match tap.target.base() {
        ChooseSpec::Tagged(tag) if tag.as_str() == "damaged" => "that creature".to_string(),
        _ => describe_choose_spec(&tap.target),
    };
    let same_target = match tap.target.base() {
        ChooseSpec::Object(tap_filter) => tap_filter == filter,
        ChooseSpec::Tagged(tag) => filter_is_tagged_as(filter, tag.as_str()),
        ChooseSpec::Target(inner) => {
            matches!(inner.base(), ChooseSpec::Object(tap_filter) if tap_filter == filter)
        }
        _ => false,
    } || tagged_tap.is_some_and(|tag| filter_is_tagged_as(filter, tag));
    if !same_target {
        return None;
    }

    let singular_target = !tap.target.is_all()
        && !tap.target.count().is_dynamic_x()
        && tap.target.count().max.is_some_and(|max| max <= 1);
    let followup = if tap.target.is_all() {
        describe_untap_restriction_oracle(cant)?
    } else {
        let subject_text = if singular_target {
            if target_text == "it" {
                "It".to_string()
            } else if target_text.starts_with("that ") {
                capitalize_first(&target_text)
            } else {
                format!("That {}", untap_restriction_filter_noun(filter))
            }
        } else {
            format!(
                "Those {}",
                pluralize_noun_phrase(untap_restriction_filter_noun(filter))
            )
        };
        let subject = if filter.controller == Some(PlayerFilter::You) {
            UntapRestrictionSubject::controlled_by_you(subject_text, !singular_target)
        } else if !singular_target {
            UntapRestrictionSubject::plural(subject_text, false)
        } else {
            UntapRestrictionSubject::singular(subject_text)
        };
        describe_untap_restriction_for_subject(cant, subject)?
    };
    Some(format!("Tap {target_text}. {followup}"))
}

pub(crate) fn describe_target_freeze_bundle(filtered: &[&Effect]) -> Option<String> {
    let [target_effect, cant_effect] = filtered else {
        return None;
    };
    let tagged = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    if !filter_is_tagged_as(filter, tagged.tag.as_str()) {
        return None;
    }

    let subject_text = capitalize_first(&describe_choose_spec(&target_only.target));
    let plural = target_only.target.is_all()
        || target_only.target.count().is_dynamic_x()
        || target_only.target.count().max.is_none_or(|max| max > 1);
    let subject = if filter.controller == Some(PlayerFilter::You) {
        UntapRestrictionSubject::controlled_by_you(subject_text, plural)
    } else if plural {
        UntapRestrictionSubject::plural(subject_text, false)
    } else {
        UntapRestrictionSubject::singular(subject_text)
    };
    describe_untap_restriction_for_subject(cant, subject)
}

pub(crate) fn describe_reveal_top_to_hand_bundle(filtered: &[&Effect]) -> Option<String> {
    let [reveal_effect, move_effect] = filtered else {
        return None;
    };
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTopEffect>()?;
    if reveal.player != PlayerFilter::You {
        return None;
    }
    if let Some(return_to_hand) = move_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()
        && let Some(tag) = &reveal.tag
        && matches!(&return_to_hand.spec, ChooseSpec::Tagged(found) if found == tag)
    {
        return Some(
            "Reveal the top card of your library and put that card into your hand".to_string(),
        );
    }
    let move_to_zone = downcast_move_to_zone(move_effect)?;
    if move_to_zone.zone == Zone::Hand
        && let Some(tag) = &reveal.tag
        && matches!(&move_to_zone.target, ChooseSpec::Tagged(found) if found == tag)
    {
        return Some(
            "Reveal the top card of your library and put that card into your hand".to_string(),
        );
    }
    None
}

pub(crate) fn filter_is_tagged_as(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

pub(crate) fn tag_matching_zones(
    tag_matching: &crate::effects::TagMatchingObjectsEffect,
) -> Option<Vec<Zone>> {
    let primary_zone = tag_matching.filter.zone.or(tag_matching.zone)?;
    let mut zones = vec![primary_zone];
    for zone in &tag_matching.additional_zones {
        if !zones.contains(zone) {
            zones.push(*zone);
        }
    }
    Some(zones)
}

pub(crate) fn effect_moves_chosen_to_zone(
    effect: &Effect,
    chosen_tag: &str,
) -> Option<(Zone, bool)> {
    if let Some(move_to_zone) = downcast_move_to_zone(effect)
        && move_to_zone_uses_tag(move_to_zone, chosen_tag, move_to_zone.zone)
    {
        return Some((move_to_zone.zone, move_to_zone.enters_tapped));
    }

    let (_, for_each) = for_each_tagged_for_compaction(effect)?;
    if for_each.tag.as_str() != chosen_tag || for_each.effects.len() != 1 {
        return None;
    }
    let move_to_zone = downcast_move_to_zone(&for_each.effects[0])?;
    matches!(move_to_zone.target, ChooseSpec::Iterated)
        .then_some((move_to_zone.zone, move_to_zone.enters_tapped))
}

pub(crate) fn effect_moves_unselected_to_zone(
    effect: &Effect,
    source_tag: &str,
    chosen_tag: &str,
) -> Option<Zone> {
    effect_moves_unselected_to_zone_and_tapped(effect, source_tag, chosen_tag).map(|(zone, _)| zone)
}

pub(crate) fn effect_moves_unselected_to_zone_and_tapped(
    effect: &Effect,
    source_tag: &str,
    chosen_tag: &str,
) -> Option<(Zone, bool)> {
    let (_, for_each) = for_each_tagged_for_compaction(effect)?;
    let [effect] = for_each.effects.as_slice() else {
        return None;
    };
    let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [rest_move] = conditional.if_false.as_slice() else {
        return None;
    };
    let move_to_zone = downcast_move_to_zone(rest_move)?;
    for_each_moves_unselected_to_zone(for_each, source_tag, chosen_tag, move_to_zone.zone)
        .then_some((move_to_zone.zone, move_to_zone.enters_tapped))
}

pub(crate) fn divvy_search_selection(search: &crate::effects::ChooseObjectsEffect) -> String {
    if search.filter.card_types == vec![CardType::Creature]
        && search.filter.subtypes.is_empty()
        && search.filter.colors.is_none()
        && let Some(max) = search.count.max
        && search.count.min == 0
    {
        let count = number_word(max as i32).unwrap_or_else(|| max.to_string());
        let mut selection = format!("up to {count} creature cards");
        if search.filter.distinct_names {
            selection.push_str(" with different names");
        }
        if let Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) =
            &search.filter.mana_value
        {
            selection.push_str(&format!(
                " that each have mana value {} or less",
                describe_value(value)
            ));
        }
        return selection;
    }
    if search.filter.card_types.is_empty()
        && search.filter.subtypes.is_empty()
        && search.filter.colors.is_none()
        && search.filter.name.is_none()
        && search.filter.tagged_constraints.is_empty()
        && search.filter.mana_value.is_none()
        && let Some(excluded_name) = search
            .filter
            .excluded_name_surface()
            .or(search.filter.excluded_name.as_deref())
        && let Some(exact) = choose_exact_count(search)
    {
        let count = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
        let noun = if exact == 1 { "card" } else { "cards" };
        let different_names = if search.filter.distinct_names {
            " that have different names"
        } else {
            ""
        };
        return format!("exactly {count} {noun} not named {excluded_name}{different_names}");
    }

    describe_search_selection_with_cards_preserving_where(&describe_choose_selection(search))
}

pub(crate) fn search_origin_for_divvy(
    search: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    Some(describe_search_origin_zones(search)?.replace(" and/or ", " and "))
}

pub(crate) fn render_search_reveal_opponent_choose_rest_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let filtered = if let [first, rest @ ..] = filtered {
        if first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
        {
            rest
        } else {
            filtered
        }
    } else {
        filtered
    };

    fn search_and_reveal_for_divvy<'a>(
        search_effect: &'a Effect,
        reveal_effect: Option<&'a Effect>,
    ) -> Option<(&'a crate::effects::ChooseObjectsEffect, bool)> {
        if let Some(search) = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            match reveal_effect {
                Some(reveal_effect) => {
                    let reveal =
                        reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
                    if search.reveal || reveal.tag != search.tag {
                        return None;
                    }
                }
                None if !search.reveal => return None,
                None => {}
            }
            return Some((search, false));
        }

        if reveal_effect.is_none()
            && let Some(sequence) = search_effect.downcast_ref::<crate::effects::SequenceEffect>()
            && let [search_effect, reveal_effect] = sequence.effects.as_slice()
        {
            return search_and_reveal_for_divvy(search_effect, Some(reveal_effect));
        }

        let with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
        if may.effects.len() != 1 {
            return None;
        }
        let search = may.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        if may
            .decider
            .as_ref()
            .is_some_and(|decider| decider != &search.chooser)
        {
            return None;
        }
        if let Some(reveal_effect) = reveal_effect {
            let if_effect = reveal_effect.downcast_ref::<crate::effects::IfEffect>()?;
            if search.reveal
                || if_effect.condition != with_id.id
                || if_effect.predicate != crate::effect::EffectPredicate::Happened
                || !if_effect.else_.is_empty()
                || if_effect.then.len() != 1
            {
                return None;
            }
            let reveal = if_effect.then[0].downcast_ref::<crate::effects::RevealTaggedEffect>()?;
            if reveal.tag != search.tag {
                return None;
            }
        } else if !search.reveal {
            return None;
        }
        Some((search, true))
    }

    let mut normalized = filtered.to_vec();
    let mut delegated_opponent = false;
    if let Some(index) = normalized.windows(2).position(|window| {
        let Some(player_choice) = window[0].downcast_ref::<crate::effects::ChoosePlayerEffect>()
        else {
            return false;
        };
        let Some(object_choice) = window[1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        else {
            return false;
        };
        player_choice.chooser == PlayerFilter::You
            && player_choice.filter == PlayerFilter::Opponent
            && !player_choice.random
            && player_choice.excluded_tags.is_empty()
            && object_choice.chooser == PlayerFilter::TaggedPlayer(player_choice.tag.clone())
    }) {
        normalized.remove(index);
        delegated_opponent = true;
    }
    let filtered = normalized.as_slice();

    let (
        search_effect,
        reveal_effect,
        tag_source_effect,
        choose_effect,
        first_move_effect,
        second_move_effect,
        shuffle_effect,
        source_exile,
    ) = match filtered {
        [
            search_effect,
            reveal_effect,
            tag_source_effect,
            choose_effect,
            first_move_effect,
            second_move_effect,
            shuffle_effect,
            source_exile_effect,
        ] => (
            *search_effect,
            Some(*reveal_effect),
            *tag_source_effect,
            *choose_effect,
            *first_move_effect,
            *second_move_effect,
            *shuffle_effect,
            Some(*source_exile_effect),
        ),
        [
            search_effect,
            second_effect,
            third_effect,
            fourth_effect,
            fifth_effect,
            sixth_effect,
            seventh_effect,
        ] => {
            if second_effect
                .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
                .is_some()
            {
                (
                    *search_effect,
                    None,
                    *second_effect,
                    *third_effect,
                    *fourth_effect,
                    *fifth_effect,
                    *sixth_effect,
                    Some(*seventh_effect),
                )
            } else {
                (
                    *search_effect,
                    Some(*second_effect),
                    *third_effect,
                    *fourth_effect,
                    *fifth_effect,
                    *sixth_effect,
                    *seventh_effect,
                    None,
                )
            }
        }
        [
            search_effect,
            tag_source_effect,
            choose_effect,
            first_move_effect,
            second_move_effect,
            shuffle_effect,
        ] => (
            *search_effect,
            None,
            *tag_source_effect,
            *choose_effect,
            *first_move_effect,
            *second_move_effect,
            *shuffle_effect,
            None,
        ),
        _ => return None,
    };

    let (search, optional_search) = search_and_reveal_for_divvy(search_effect, reveal_effect)?;
    let tag_source =
        tag_source_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let search_zones = choose_search_zones(search)?;

    if !search.is_search
        || !search_zones.contains(&Zone::Library)
        || tag_source.tag.as_str() != "divvy_source"
        || !filter_is_tagged_as(&tag_source.filter, search.tag.as_str())
        || tag_matching_zones(tag_source)? != search_zones
        || choose.is_search
        || choose.tag.as_str() != "divvy_chosen"
        || choose_search_zones(choose)? != search_zones
        || !filter_is_tagged_as(&choose.filter, tag_source.tag.as_str())
        || shuffle.player != search.chooser
    {
        return None;
    }

    let first_chosen_zone = effect_moves_chosen_to_zone(first_move_effect, choose.tag.as_str());
    let second_chosen_zone = effect_moves_chosen_to_zone(second_move_effect, choose.tag.as_str());
    let first_rest_zone = effect_moves_unselected_to_zone_and_tapped(
        first_move_effect,
        tag_source.tag.as_str(),
        choose.tag.as_str(),
    );
    let second_rest_zone = effect_moves_unselected_to_zone_and_tapped(
        second_move_effect,
        tag_source.tag.as_str(),
        choose.tag.as_str(),
    );
    let (chosen_zone, rest_zone) = match (
        first_chosen_zone,
        first_rest_zone,
        second_chosen_zone,
        second_rest_zone,
    ) {
        (Some(chosen), None, None, Some(rest)) => (chosen, rest),
        (None, Some(rest), Some(chosen), None) => (chosen, rest),
        _ => return None,
    };

    if let Some(source_exile) = source_exile {
        let source_move = downcast_move_to_zone(source_exile)?;
        if source_move.zone != Zone::Exile || !matches!(source_move.target, ChooseSpec::Source) {
            return None;
        }
    }

    let searched_owner = describe_possessive_player_filter(&search.chooser);
    let describe_searched_owner_zone = |zone: Zone, tapped: bool| -> Option<String> {
        let tapped_suffix = if tapped { " tapped" } else { "" };
        match zone {
            Zone::Hand => Some(format!("into {searched_owner} hand")),
            Zone::Graveyard => Some(format!("into {searched_owner} graveyard")),
            Zone::Library => Some(format!("into {searched_owner} library")),
            Zone::Battlefield => Some(format!("onto the battlefield{tapped_suffix}")),
            _ => None,
        }
    };

    let search_selection = divvy_search_selection(search);
    let search_origin = search_origin_for_divvy(search)?;
    let (search_line, reveal_line) = if optional_search {
        let may_search = if search.chooser == PlayerFilter::You {
            format!("You may search {search_origin} for {search_selection}")
        } else {
            let search_player = describe_player_filter(&search.chooser);
            format!(
                "{} may search {search_origin} for {search_selection}",
                capitalize_first(&search_player)
            )
        };
        let reveal_object = if search.count.is_single() {
            "it"
        } else {
            "those cards"
        };
        let reveal_line = if search.chooser == PlayerFilter::You {
            format!("If you do, reveal {reveal_object}")
        } else {
            let search_player = describe_player_filter(&search.chooser);
            format!("If {search_player} does, reveal {reveal_object}")
        };
        (may_search, Some(reveal_line))
    } else {
        let reveal_object = if search.count.is_single() {
            "it"
        } else {
            "them"
        };
        let search_line = if search.chooser == PlayerFilter::You {
            format!("Search {search_origin} for {search_selection} and reveal {reveal_object}")
        } else {
            let search_player = describe_player_filter(&search.chooser);
            let verb = player_verb(&search_player, "search", "searches");
            format!(
                "{} {verb} {search_origin} for {search_selection} and reveals {reveal_object}",
                capitalize_first(&search_player)
            )
        };
        (search_line, None)
    };

    let chooser = if delegated_opponent {
        "an opponent".to_string()
    } else {
        describe_player_filter(&choose.chooser)
    };
    let chosen_count = if choose.count.is_single() {
        if search.count.is_single() {
            "that card".to_string()
        } else {
            "one of them".to_string()
        }
    } else if let Some(exact) = choose_exact_count(choose) {
        let count_text = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
        format!("{count_text} of those cards")
    } else {
        format!("{} of those cards", describe_choice_count(&choose.count))
    };
    let choice_line = format!("{} chooses {chosen_count}", capitalize_first(&chooser));

    let selected_object = if choose.count.is_single() {
        if search.count.is_single() {
            "that card"
        } else {
            "the chosen card"
        }
    } else {
        "the chosen cards"
    };
    let rest_object = if choose.count.is_single() && choose_exact_count(search) == Some(2) {
        "the other"
    } else {
        "the rest"
    };
    let (chosen_zone, chosen_tapped) = chosen_zone;
    let (rest_zone, rest_tapped) = rest_zone;
    let rest_destination = describe_searched_owner_zone(rest_zone, rest_tapped)?;
    let (move_line, shuffle_line) = if chosen_zone == Zone::Library {
        let shuffle_verb = if shuffle.player == PlayerFilter::You {
            "Shuffle".to_string()
        } else {
            let player = describe_player_filter(&shuffle.player);
            format!("{} shuffles", capitalize_first(&player))
        };
        (
            format!(
                "{shuffle_verb} {selected_object} into {searched_owner} library and put {rest_object} {rest_destination}"
            ),
            None,
        )
    } else {
        let selected_destination = describe_searched_owner_zone(chosen_zone, chosen_tapped)?;
        let shuffle_line = if shuffle.player == PlayerFilter::You {
            "Then shuffle".to_string()
        } else {
            let player = describe_player_filter(&shuffle.player);
            format!("Then {player} shuffles")
        };
        (
            format!(
                "Put {selected_object} {selected_destination} and {rest_object} {rest_destination}"
            ),
            Some(shuffle_line),
        )
    };

    let mut rendered = format!("{search_line}.");
    if let Some(reveal_line) = reveal_line {
        rendered.push(' ');
        rendered.push_str(&reveal_line);
        rendered.push('.');
    }
    rendered.push(' ');
    rendered.push_str(&choice_line);
    rendered.push('.');
    rendered.push(' ');
    rendered.push_str(&move_line);
    rendered.push('.');
    if let Some(shuffle_line) = shuffle_line {
        rendered.push(' ');
        rendered.push_str(&shuffle_line);
        rendered.push('.');
    }
    if let Some(source_exile) = source_exile {
        rendered.push(' ');
        rendered.push_str(&describe_effect(source_exile));
        rendered.push('.');
    }
    Some(rendered)
}

pub(crate) fn describe_discard_then_for_each_discarded(
    discard: &crate::effects::DiscardEffect,
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    let references_discard = for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "triggering"
            || discard
                .tag
                .as_ref()
                .is_some_and(|tag| constraint.tag == *tag)
    });
    if !references_discard {
        return None;
    }
    let [counter_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let counters = counter_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(counters.target, ChooseSpec::Source)
        || counters.target_count.is_some()
        || counters.distributed
    {
        return None;
    }

    let first = describe_effect_impl(&Effect::new(discard.clone()));
    let second = format!(
        "For each card discarded this way, put {} on this creature",
        describe_put_counter_phrase(&counters.amount, counters.counter_type)
    );
    Some(format!("{first}. {second}"))
}

pub(crate) fn describe_ticket_then_may_put_sticker(
    ticket: &crate::effects::TicketCountersEffect,
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if ticket.player != PlayerFilter::You || may.decider != Some(PlayerFilter::You) {
        return None;
    }
    let [choose_effect, sticker_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_sticker = sticker_effect.downcast_ref::<crate::effects::PutStickerEffect>()?;
    if !matches!(&put_sticker.target, ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }
    let Value::Fixed(amount) = ticket.count else {
        return None;
    };
    if amount <= 0 {
        return None;
    }
    let tickets = "{TK}".repeat(amount as usize);
    Some(format!(
        "you get {tickets}, then you may put {} on {}",
        sticker_phrase(put_sticker.action),
        describe_choose_selection(choose)
    ))
}

pub(crate) fn tagged_untap_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::UntapEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let untap = tagged
        .effect
        .downcast_ref::<crate::effects::UntapEffect>()?;
    Some((&tagged.tag, untap))
}

pub(crate) fn tagged_apply_view(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::ApplyContinuousEffect)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let apply = tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    Some((&tagged.tag, apply))
}

pub(crate) fn describe_untap_gain_control_then_haste(effects: &[&Effect]) -> Option<String> {
    let [untap_effect, control_effect, haste_effect] = effects else {
        return None;
    };
    let (target_tag, untap) = tagged_untap_view(untap_effect)?;
    let (_, control) = tagged_apply_view(control_effect)?;
    let (_, haste) = tagged_apply_view(haste_effect)?;
    if control.target != crate::continuous::EffectTarget::Source
        || haste.target != crate::continuous::EffectTarget::Source
        || !matches!(
            control.target_spec.as_ref().map(ChooseSpec::unhinted),
            Some(ChooseSpec::Tagged(tag)) if tag == target_tag
        )
        || !matches!(
            haste.target_spec.as_ref().map(ChooseSpec::unhinted),
            Some(ChooseSpec::Tagged(tag)) if tag == target_tag
        )
        || control.until != Until::EndOfTurn
        || haste.until != Until::EndOfTurn
        || control.condition.is_some()
        || haste.condition.is_some()
        || control.modification.is_some()
        || !control.additional_modifications.is_empty()
        || !haste.additional_modifications.is_empty()
        || !matches!(
            control.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
        || !haste.runtime_modifications.is_empty()
        || !matches!(
            &haste.modification,
            Some(crate::continuous::Modification::AddAbility(ability))
                if ability.id() == crate::static_abilities::StaticAbilityId::Haste
        )
    {
        return None;
    }
    let target = describe_choose_spec(&untap.target);
    let followup_subject = if gain_control_followup_untap_target_text(&target) == "that creature" {
        "That creature"
    } else {
        "It"
    };
    Some(format!(
        "Untap {target} and gain control of it until end of turn. {followup_subject} gains haste until end of turn"
    ))
}

pub(crate) fn describe_exile_source_and_unless_pays_target(effects: &[&Effect]) -> Option<String> {
    let [maybe_target_only, unless_effect] = effects else {
        return None;
    };
    maybe_target_only.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let unless_pays = unless_effect.downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    let [source_move_effect, target_move_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    let source_move = source_move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if source_move.zone != Zone::Exile
        || !matches!(source_move.target.unhinted(), ChooseSpec::Source)
    {
        return None;
    }
    let target_move = unwrap_tag_wrappers(target_move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if target_move.zone != Zone::Exile {
        return None;
    }
    let target = describe_choose_spec(&target_move.target);
    if !target.starts_with("target ") {
        return None;
    }
    let display = describe_total_cost_payment(&unless_pays.cost);
    let payment_text = display.strip_prefix("Pay ").unwrap_or(&display);
    let controller = if let Some(kind) = target
        .strip_prefix("target ")
        .and_then(|rest| rest.split_whitespace().next())
    {
        format!("that {kind}'s controller")
    } else {
        "its controller".to_string()
    };
    Some(format!(
        "Exile this card and {target} unless {controller} pays {payment_text}"
    ))
}

#[cfg(test)]
mod random_hand_reveal_surface_tests {
    use super::*;

    #[test]
    fn nested_target_opponent_keeps_opponent_surface() {
        let tag = TagKey::from("random_reveal");
        let opponent = PlayerFilter::target_opponent();
        let choose = Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .owned_by(opponent.clone()),
                ChoiceCount::exactly(1).at_random(),
                opponent,
                tag.clone(),
            )
            .in_zone(Zone::Hand),
        );
        let reveal = Effect::new(crate::effects::RevealTaggedEffect::new(tag));

        assert_eq!(
            describe_random_hand_reveal_bundle(&[&choose, &reveal]).as_deref(),
            Some("Target opponent reveals a card at random from their hand")
        );
    }
}
