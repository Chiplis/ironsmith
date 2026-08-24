use super::*;

/// Compact a source-exile followed by an authored target declaration and a
/// delayed trigger watching that exact target:
///
/// `Exile it and choose target creature. When that creature leaves ...`
///
/// The tag relationships make this structural rather than card-specific.
pub(crate) fn describe_exile_then_choose_delayed_leaves(effects: &[Effect]) -> Option<String> {
    let [
        tag_triggering_effect,
        exile_effect,
        target_effect,
        schedule_effect,
    ] = effects
    else {
        return None;
    };
    let tag_triggering =
        tag_triggering_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let exile = unwrap_basic_tag_wrappers(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile.zone != Zone::Exile
        || !choose_spec_is_tagged_object(&exile.target, &tag_triggering.tag)
    {
        return None;
    }

    let (target_tag, target_only) = tagged_target_only_effect(target_effect)?;
    if !target_only.explicit_declaration {
        return None;
    }

    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    let leaves = schedule
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if schedule.target_tag.as_ref() != Some(target_tag)
        || !schedule.one_shot
        || schedule.start_next_turn
        || schedule.until_end_of_turn
        || schedule.until_end_of_combat
        || !leaves.this_object
        || !matches!(
            leaves.from,
            crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        )
        || !matches!(leaves.to, crate::triggers::ZonePattern::Any)
        || !schedule
            .target_filter
            .as_ref()
            .is_some_and(|filter| filter_references_exact_tag(filter, target_tag))
    {
        return None;
    }

    let exile_text = describe_effect(exile_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let choose_text = describe_effect(target_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if !choose_text
        .to_ascii_lowercase()
        .starts_with("choose target")
    {
        return None;
    }
    let delayed_text = capitalize_first(
        describe_effect(schedule_effect)
            .trim()
            .trim_end_matches('.'),
    );
    Some(format!(
        "{exile_text} and {}. {delayed_text}",
        lowercase_first(&choose_text)
    ))
}

pub(crate) fn describe_tag_attached_tap_then_become_monarch(effects: &[Effect]) -> Option<String> {
    fn choose_spec_references_attached_tag(spec: &ChooseSpec, tag: &str) -> bool {
        match spec {
            ChooseSpec::Tagged(candidate) => candidate.as_str() == tag,
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
                filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == tag
                        && matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                })
            }
            ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
                choose_spec_references_attached_tag(inner, tag)
            }
            _ => false,
        }
    }

    let [tag_effect, tap_effect, monarch_effect] = effects else {
        return None;
    };
    let tag_attached = tag_effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()?;
    let tag = tag_attached.tag.as_str();
    if !matches!(tag, "enchanted" | "equipped") {
        return None;
    }
    let tap = tap_effect.downcast_ref::<crate::effects::TapEffect>()?;
    if !choose_spec_references_attached_tag(&tap.target, tag) {
        return None;
    }
    let become_monarch = monarch_effect.downcast_ref::<crate::effects::BecomeMonarchEffect>()?;
    if become_monarch.player != PlayerFilter::You {
        return None;
    }
    let attached_object = describe_attached_object_for_tag(tag, Some(&tap.target));
    Some(format!("Tap {attached_object} and you become the monarch"))
}

pub(crate) fn describe_optional_sticker_aura_return_attach_sequence(
    effects: &[&Effect],
) -> Option<String> {
    let effects = if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        rest
    } else {
        effects
    };
    let [may_effect, choose_effect, return_effect, attach_effect] = effects else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You) {
        return None;
    }
    let may_effects = if let [effect] = may.effects.as_slice()
        && let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(sequence.surface, ironsmith_core::SequenceSurface::CommaThen)
    {
        sequence.effects.as_slice()
    } else {
        may.effects.as_slice()
    };
    let [put_sticker_effect, aura_effect] = may_effects else {
        return None;
    };
    let put_sticker = unwrap_basic_tag_wrappers(put_sticker_effect)
        .downcast_ref::<crate::effects::PutStickerEffect>()?;
    if !matches!(
        put_sticker.action,
        crate::events::KeywordActionKind::NameSticker
    ) {
        return None;
    }
    let aura_text = describe_effect(aura_effect);
    if !aura_text
        .to_ascii_lowercase()
        .contains("becomes an aura with enchant")
    {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
    {
        return None;
    }
    let returned = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;
    if returned.tapped || !choose_spec_references_exact_tag(&returned.target, &choose.tag) {
        return None;
    }
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(attach.objects, ChooseSpec::Source) {
        return None;
    }

    let returned_text = describe_chosen_graveyard_card_for_return(choose)?;
    let aura_text = lowercase_first(aura_text.trim_end_matches('.'));
    let attach_target = describe_choose_spec(&attach.target);
    Some(format!(
        "You may put a name sticker on {}, then {aura_text}. Return {returned_text} to the battlefield and attach this Aura to {attach_target}",
        describe_choose_spec(&put_sticker.target)
    ))
}

pub(crate) fn describe_chosen_graveyard_card_for_return(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if choose.filter.zone != Some(Zone::Graveyard) {
        return None;
    }
    let mut filter = choose.filter.clone();
    let owner = filter.owner.take();
    filter.zone = None;
    let mut selection = strip_leading_article(&filter.description()).to_string();
    if !selection.to_ascii_lowercase().contains("card") {
        selection.push_str(" card");
    }
    let graveyard = match owner {
        Some(owner) => format!("{} graveyard", describe_possessive_player_filter(&owner)),
        None => "a graveyard".to_string(),
    };
    Some(format!(
        "{} from {graveyard}",
        with_indefinite_article(&selection)
    ))
}

pub(crate) fn filter_references_exact_tag(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == tag.as_str()
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    })
}

pub(crate) fn described_filter_without_tag(filter: &ObjectFilter) -> String {
    let mut base = filter.clone();
    base.tagged_constraints.clear();
    let description = strip_leading_article(&base.description()).to_string();
    if description.is_empty() || description == "object" {
        "permanent".to_string()
    } else {
        description
    }
}

pub(crate) fn describe_attached_to_source_sacrifice_sequence(
    effects: &[&Effect],
) -> Option<String> {
    let effects = if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        rest
    } else {
        effects
    };
    let [tag_effect, choose_effect, sacrifice_effect] = effects else {
        return None;
    };
    let attached = tag_effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()?;
    let attached_word = match attached.tag.as_str() {
        "enchanted" => "enchanted",
        "equipped" => "equipped",
        _ => return None,
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || !filter_references_exact_tag(&choose.filter, &attached.tag)
    {
        return None;
    }

    let sacrifice =
        sacrifice_effect.downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()?;
    if sacrifice.player != PlayerFilter::You
        || sacrifice.count != Value::Fixed(1)
        || !filter_references_exact_tag(&sacrifice.filter, &choose.tag)
    {
        return None;
    }

    Some(format!(
        "Sacrifice {attached_word} {}",
        described_filter_without_tag(&choose.filter)
    ))
}

/// Preserve the actor in an attached-object sacrifice. `SacrificeTargetEffect`
/// correctly makes the chosen permanent's controller perform the sacrifice;
/// rendering it as an imperative incorrectly attributes the action to the
/// ability's controller.
pub(crate) fn describe_attached_target_controller_sacrifice(effects: &[&Effect]) -> Option<String> {
    let [tag_effect, sacrifice_effect] = effects else {
        return None;
    };
    let attached = tag_effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()?;
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    let noun = match sacrifice.target.base() {
        ChooseSpec::Tagged(tag) if tag == &attached.tag => match attached.tag.as_str() {
            "enchanted" => "permanent".to_string(),
            "equipped" => "creature".to_string(),
            _ => return None,
        },
        ChooseSpec::Object(filter) | ChooseSpec::All(filter)
            if filter_references_exact_tag(filter, &attached.tag) =>
        {
            described_filter_without_tag(filter)
        }
        _ => return None,
    };
    let adjective = match attached.tag.as_str() {
        "enchanted" => "enchanted",
        "equipped" => "equipped",
        _ => return None,
    };
    Some(format!("{adjective} {noun}'s controller sacrifices it"))
}

pub(crate) fn describe_tagged_token_copy_then_sacrifice(effects: &[&Effect]) -> Option<String> {
    let effects = if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        rest
    } else {
        effects
    };
    let [create_effect, sacrifice_effect] = effects else {
        return None;
    };
    let tagged_create = create_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let create_copy = tagged_create
        .effect
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(&sacrifice.target, ChooseSpec::Tagged(tag) if tag == &tagged_create.tag) {
        return None;
    }

    let created_object = if matches!(create_copy.count.unhinted(), Value::Fixed(1)) {
        "that token"
    } else {
        "those tokens"
    };
    let create_text = describe_effect(create_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!("{create_text}. Sacrifice {created_object}"))
}

pub(crate) fn unwrap_wrapped_effect(effect: &Effect) -> &Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_wrapped_effect(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_wrapped_effect(&tag_all.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_wrapped_effect(with_id.effect.as_ref());
    }
    effect
}

pub(crate) fn for_each_moves_iterated_to_battlefield(
    effect: &Effect,
    expected_tag: &crate::TagKey,
) -> bool {
    let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() else {
        return false;
    };
    let [move_effect] = for_each.effects.as_slice() else {
        return false;
    };
    let Some(move_to_zone) =
        unwrap_wrapped_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    let mut normalized_move = move_to_zone.clone();
    if normalized_move.verb_surface == ironsmith_core::MoveToZoneVerbSurface::Put {
        normalized_move.verb_surface = ironsmith_core::MoveToZoneVerbSurface::Canonical;
    }
    for_each.tag == *expected_tag
        && for_each.controller_at_last_blocked_by.is_none()
        && normalized_move
            == crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Battlefield, false)
}

pub(crate) fn describe_consult_choose_any_number_to_battlefield_rest_bottom(
    effects: &[&Effect],
) -> Option<String> {
    let effects = if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        rest
    } else {
        effects
    };
    let [consult_effect, choose_effect, move_effect, bottom_effect] = effects else {
        return None;
    };
    let consult = unwrap_wrapped_effect(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let bottom =
        bottom_effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !choose.count.is_any_number()
        || choose.chooser != PlayerFilter::You
        || choose.is_search
        || choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == consult.match_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        || !for_each_moves_iterated_to_battlefield(move_effect, &choose.tag)
        || bottom.tag != consult.all_tag
        || bottom.keep_tagged.as_ref() != Some(&choose.tag)
        || bottom.player != PlayerFilter::You
        || bottom.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
    {
        return None;
    }

    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let stop_text = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::ColorsAmong(filter)) => {
            format!(
                "X {}, where X is the number of colors among {}",
                pluralize_noun_phrase(&selection),
                pluralize_noun_phrase(&describe_for_each_filter(filter))
            )
        }
        stop_rule => {
            describe_consult_stop_text(&selection, stop_rule, consult.max_exposed.as_ref())
        }
    };
    Some(format!(
        "Reveal cards from the top of your library until you reveal {stop_text}. Put any number of those {} onto the battlefield, then put the rest of the revealed cards on the bottom of your library in a random order",
        pluralize_noun_phrase(&selection)
    ))
}

pub(crate) fn describe_shuffle_reveal_repeated_permanent_groups_rest_bottom(
    effects: &[&Effect],
) -> Option<String> {
    if let [
        shuffle_effect,
        look_effect,
        union_tag_effect,
        move_effect,
        bottom_effect,
    ] = effects
    {
        let with_id = shuffle_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        let shuffle = unwrap_wrapped_effect(shuffle_effect)
            .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;
        let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
        let tag_matching =
            union_tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        let bottom = bottom_effect
            .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
        let shuffled_filter = match &shuffle.target {
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
            _ => return None,
        };
        let mut normalized_shuffled_filter = shuffled_filter.clone();
        normalized_shuffled_filter.set_set_quantifier_surface(None);
        let exact_shuffle = normalized_shuffled_filter
            == ObjectFilter::permanent()
                .in_zone(Zone::Battlefield)
                .owned_by(PlayerFilter::You)
            || normalized_shuffled_filter
                == ObjectFilter::permanent_card()
                    .in_zone(Zone::Battlefield)
                    .owned_by(PlayerFilter::You);
        if !exact_shuffle
            || shuffle.player != PlayerFilter::You
            || shuffle.owner_library_destination
            || shuffle.possessive_owner_subject
            || look.player != PlayerFilter::You
            || !look.reveal
            || !matches!(
                look.count.unhinted(),
                Value::EffectMetric {
                    effect_id,
                    source: crate::effect::EffectMetricSource::Outcome,
                    metric: crate::effect::EffectMetric::Count,
                } if *effect_id == with_id.id
            )
            || tag_matching.zone != Some(Zone::Library)
            || !tag_matching.additional_zones.is_empty()
            || tag_matching.filter.any_of.len() != 2
            || !for_each_moves_iterated_to_battlefield(move_effect, &tag_matching.tag)
            || bottom.tag != look.tag
            || bottom.keep_tagged.as_ref() != Some(&tag_matching.tag)
            || bottom.player != PlayerFilter::You
            || bottom.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
        {
            return None;
        }

        let tagged_revealed = |mut filter: ObjectFilter| {
            filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: look.tag.clone(),
                    relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                });
            filter
        };
        let expected_union = |mut non_aura: ObjectFilter| {
            non_aura = tagged_revealed(non_aura);
            non_aura.set_explicit_card_noun(true);
            non_aura.excluded_subtypes.push(crate::types::Subtype::Aura);
            let mut aura =
                tagged_revealed(ObjectFilter::default().with_subtype(crate::types::Subtype::Aura));
            aura.set_explicit_card_noun(true);
            let mut union = ObjectFilter::default();
            union.any_of = vec![non_aura, aura];
            union
        };
        if tag_matching.filter != expected_union(ObjectFilter::permanent())
            && tag_matching.filter != expected_union(ObjectFilter::permanent_card())
        {
            return None;
        }
        return Some("Shuffle all permanents you own into your library, then reveal that many cards from the top of your library. Put all non-Aura permanent cards revealed this way onto the battlefield, then do the same for Aura cards, then put the rest on the bottom of your library in a random order".to_string());
    }

    let [
        shuffle_effect,
        look_effect,
        union_tag_effect,
        first_tag_effect,
        first_move_effect,
        second_tag_effect,
        second_move_effect,
        bottom_effect,
    ] = effects
    else {
        return None;
    };
    let with_id = shuffle_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let shuffle = unwrap_wrapped_effect(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()?;
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let tag_matching =
        union_tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let first_matching =
        first_tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let second_matching =
        second_tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let bottom =
        bottom_effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let shuffled_filter = match &shuffle.target {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
        _ => return None,
    };
    // `ChooseSpec::All` already carries the executable quantifier. The public
    // parser additionally retains the authored `all` surface on the filter;
    // normalize only that redundant presentation bit before comparing the
    // exact permanent domain.
    let mut normalized_shuffled_filter = shuffled_filter.clone();
    normalized_shuffled_filter.set_set_quantifier_surface(None);
    let shuffled_permanents = ObjectFilter::permanent()
        .in_zone(Zone::Battlefield)
        .owned_by(PlayerFilter::You);
    let shuffled_explicit_permanent_types = ObjectFilter::permanent_card()
        .in_zone(Zone::Battlefield)
        .owned_by(PlayerFilter::You);
    if (normalized_shuffled_filter != shuffled_permanents
        && normalized_shuffled_filter != shuffled_explicit_permanent_types)
        || shuffle.player != PlayerFilter::You
        || shuffle.owner_library_destination
        || shuffle.possessive_owner_subject
        || look.player != PlayerFilter::You
        || !look.reveal
        || !matches!(
            look.count.unhinted(),
            Value::EffectMetric {
                effect_id,
                source: crate::effect::EffectMetricSource::Outcome,
                metric: crate::effect::EffectMetric::Count,
            } if *effect_id == with_id.id
        )
        || tag_matching.zone != Some(Zone::Library)
        || !tag_matching.additional_zones.is_empty()
        || tag_matching.filter.any_of.len() != 2
        || first_matching.zone != Some(Zone::Library)
        || !first_matching.additional_zones.is_empty()
        || second_matching.zone != Some(Zone::Library)
        || !second_matching.additional_zones.is_empty()
        || first_matching.filter != tag_matching.filter.any_of[0]
        || second_matching.filter != tag_matching.filter.any_of[1]
        || first_matching.tag == second_matching.tag
        || first_matching.tag == tag_matching.tag
        || second_matching.tag == tag_matching.tag
        || !for_each_moves_iterated_to_battlefield(first_move_effect, &first_matching.tag)
        || !for_each_moves_iterated_to_battlefield(second_move_effect, &second_matching.tag)
        || bottom.tag != look.tag
        || bottom.keep_tagged.as_ref() != Some(&tag_matching.tag)
        || bottom.player != PlayerFilter::You
        || bottom.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
    {
        return None;
    }

    let [first, second] = tag_matching.filter.any_of.as_slice() else {
        return None;
    };
    let tagged_revealed = |mut filter: ObjectFilter| {
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: look.tag.clone(),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        filter
    };
    let expected_non_aura = |mut filter: ObjectFilter| {
        filter = tagged_revealed(filter);
        filter.set_explicit_card_noun(true);
        filter.excluded_subtypes.push(crate::types::Subtype::Aura);
        filter
    };
    let mut expected_aura =
        tagged_revealed(ObjectFilter::default().with_subtype(crate::types::Subtype::Aura));
    expected_aura.set_explicit_card_noun(true);
    let expected_union = |non_aura| {
        let mut union = ObjectFilter::default();
        union.any_of = vec![non_aura, expected_aura.clone()];
        union
    };
    let battlefield_shorthand_union = expected_union(expected_non_aura(ObjectFilter::permanent()));
    let explicit_permanent_types_union =
        expected_union(expected_non_aura(ObjectFilter::permanent_card()));
    if (tag_matching.filter != battlefield_shorthand_union
        && tag_matching.filter != explicit_permanent_types_union)
        || first == second
    {
        return None;
    }

    Some("Shuffle all permanents you own into your library, then reveal that many cards from the top of your library. Put all non-Aura permanent cards revealed this way onto the battlefield, then do the same for Aura cards, then put the rest on the bottom of your library in a random order".to_string())
}

pub(crate) fn effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tag_all.tag);
    }
    None
}

pub(crate) fn tagged_apply_continuous(
    effect: &Effect,
) -> Option<&crate::effects::ApplyContinuousEffect> {
    unwrap_wrapped_effect(effect).downcast_ref::<crate::effects::ApplyContinuousEffect>()
}

pub(crate) fn apply_targets_tag(
    apply: &crate::effects::ApplyContinuousEffect,
    tag: &crate::TagKey,
) -> bool {
    matches!(apply.target_spec.as_ref(), Some(ChooseSpec::Tagged(candidate)) if candidate == tag)
}

pub(crate) fn describe_return_all_face_down_then_become(effects: &[&Effect]) -> Option<String> {
    let effects = if let Some(first) = effects.first()
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        &effects[1..]
    } else {
        effects
    };
    let [return_effect, become_effect] = effects else {
        return None;
    };
    let returned_tag = effect_tag(return_effect)?;
    let return_all = unwrap_wrapped_effect(return_effect)
        .downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    if !return_all.face_down {
        return None;
    }

    let apply = tagged_apply_continuous(become_effect)?;
    if !apply_targets_tag(apply, returned_tag)
        || apply.until != Until::Forever
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddCardTypes(card_types)) = &apply.modification
    else {
        return None;
    };
    let mut power = None;
    let mut toughness = None;
    let mut subtypes = Vec::new();
    for modification in &apply.additional_modifications {
        match modification {
            crate::continuous::Modification::SetPowerToughness {
                power: p,
                toughness: t,
                ..
            } => {
                power = Some(p);
                toughness = Some(t);
            }
            crate::continuous::Modification::AddSubtypes(found) => {
                subtypes = found.clone();
            }
            crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                crate::types::SubtypeFamily::Creature,
            ) => {}
            _ => return None,
        }
    }
    let (Some(power), Some(toughness)) = (power, toughness) else {
        return None;
    };
    if !card_types.contains(&CardType::Artifact) || !card_types.contains(&CardType::Creature) {
        return None;
    }

    let return_text = describe_return_all_to_battlefield_effect(return_all)
        .replacen("Return all", "Put all", 1)
        .replace(" to the battlefield", " onto the battlefield");
    let subtype_text = subtypes
        .iter()
        .map(|subtype| subtype.display_name())
        .collect::<Vec<_>>()
        .join(" ");
    let descriptor = if subtype_text.is_empty() {
        "artifact creatures".to_string()
    } else {
        format!("{subtype_text} artifact creatures")
    };
    Some(format!(
        "{return_text}. They're {}/{} {descriptor}",
        describe_value(power),
        describe_value(toughness)
    ))
}

pub(crate) fn describe_return_then_conditional_animation(effects: &[Effect]) -> Option<String> {
    let [return_effect, conditional_effect] = effects else {
        return None;
    };
    let returned_tag = effect_tag(return_effect)?;
    unwrap_wrapped_effect(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let apply = tagged_apply_continuous(&conditional.if_true[0])?;
    if !apply_targets_tag(apply, returned_tag) {
        return None;
    }

    let return_text = describe_effect(return_effect);
    let animation_text = describe_effect(&conditional.if_true[0]);
    if let crate::ConditionExpr::TaggedObjectMatches(condition_tag, condition_filter) =
        &conditional.condition
    {
        let mut semantic_filter = condition_filter.clone();
        semantic_filter.zone = None;
        semantic_filter.source_surface = None;
        semantic_filter.union_surface = Default::default();
        if condition_tag == returned_tag
            && semantic_filter
                == ObjectFilter::default().with_subtype(crate::types::Subtype::Vehicle)
            && apply.until == Until::Forever
            && apply.condition.is_none()
            && apply.runtime_modifications.is_empty()
            && apply.additional_modifications.is_empty()
            && matches!(
                &apply.modification,
                Some(
                    crate::continuous::Modification::AddCardTypes(card_types)
                        | crate::continuous::Modification::SetCardTypes(card_types)
                )
                    if card_types.as_slice() == [CardType::Artifact, CardType::Creature]
            )
        {
            return Some(format!(
                "{}. If it's a Vehicle, it becomes an artifact creature",
                return_text.trim_end_matches('.')
            ));
        }
    }
    Some(format!(
        "{}. If {}, {}",
        return_text.trim_end_matches('.'),
        describe_condition(&conditional.condition),
        lowercase_first(animation_text.trim_end_matches('.'))
    ))
}

/// Fold a targeted same-name graveyard card into the optional normal-cost
/// cast that consumes that exact target. The target tag is executable
/// provenance; unrelated graveyard choices or free casts must not match.
pub(crate) fn describe_target_same_name_graveyard_may_cast(effects: &[Effect]) -> Option<String> {
    let effects = match effects {
        [triggering, rest @ ..]
            if triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some_and(|tag| tag.tag.as_str() == "triggering") =>
        {
            rest
        }
        _ => effects,
    };
    let [target_effect, may_effect] = effects else {
        return None;
    };
    let target_tag = effect_tag(target_effect)?;
    let target_only =
        unwrap_wrapped_effect(target_effect).downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::Object(filter) = target_only.target.base() else {
        return None;
    };
    let [same_name] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    let mut semantic_filter = filter.clone();
    semantic_filter.zone = None;
    semantic_filter.owner = None;
    semantic_filter.tagged_constraints.clear();
    semantic_filter.source_surface = None;
    semantic_filter.union_surface = Default::default();
    if !target_only.explicit_declaration
        || target_only.chooser.is_some()
        || filter.zone != Some(Zone::Graveyard)
        || filter.owner != Some(PlayerFilter::You)
        || same_name.tag.as_str() != "triggering"
        || same_name.relation != crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        || semantic_filter != ObjectFilter::default()
    {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast =
        unwrap_wrapped_effect(cast_effect).downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::You)
        || cast.tag != *target_tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || cast.copy_cast_reminder_surface
        || cast.without_paying_mana_cost
        || cast.additional_mana_cost.is_some()
        || cast.cost_reduction.is_some()
        || cast.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }
    Some(
        "You may cast target card with the same name as that spell from your graveyard".to_string(),
    )
}

#[cfg(test)]
mod graveyard_target_and_animation_surface_tests {
    use super::*;

    #[test]
    fn targeted_same_name_graveyard_card_and_optional_cast_share_the_target_tag() {
        let tag = TagKey::from("targeted_same_name_spell");
        let mut filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You);
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: TagKey::from("triggering"),
                relation: crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
            });
        let target = Effect::new(crate::effects::TargetOnlyEffect::explicit(
            ChooseSpec::target(ChooseSpec::Object(filter)),
        ))
        .tag(tag.clone());
        let cast = Effect::may(vec![Effect::new(crate::effects::CastTaggedEffect::new(
            tag.clone(),
            PlayerFilter::You,
        ))]);
        assert_eq!(
            describe_target_same_name_graveyard_may_cast(&[target.clone(), cast]).as_deref(),
            Some("You may cast target card with the same name as that spell from your graveyard")
        );

        let wrong_target = Effect::may(vec![Effect::new(crate::effects::CastTaggedEffect::new(
            "other_card",
            PlayerFilter::You,
        ))]);
        assert!(
            describe_target_same_name_graveyard_may_cast(&[target, wrong_target]).is_none(),
            "an unrelated graveyard card must not inherit the targeted cast permission"
        );
    }

    #[test]
    fn returned_vehicle_condition_uses_the_same_returned_object_pronoun() {
        let tag = TagKey::from("returned_0");
        let returned = Effect::new(crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(
            ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::default()
                    .with_type(CardType::Artifact)
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            )),
            false,
        ))
        .tag(tag.clone());
        let animate = Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(
                tag.clone(),
                ObjectFilter::default().with_subtype(Subtype::Vehicle),
            ),
            vec![Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    ChooseSpec::Tagged(tag),
                    crate::continuous::Modification::AddCardTypes(vec![
                        CardType::Artifact,
                        CardType::Creature,
                    ]),
                    Until::Forever,
                ),
            )],
            vec![],
        ));
        assert_eq!(
            describe_return_then_conditional_animation(&[returned.clone(), animate]).as_deref(),
            Some(concat!(
                "Return target artifact card from your graveyard to the battlefield. ",
                "If it's a Vehicle, it becomes an artifact creature"
            ))
        );

        let wrong_tag = Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(
                TagKey::from("other_return"),
                ObjectFilter::default().with_subtype(Subtype::Vehicle),
            ),
            vec![Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    ChooseSpec::Tagged(TagKey::from("other_return")),
                    crate::continuous::Modification::AddCardTypes(vec![
                        CardType::Artifact,
                        CardType::Creature,
                    ]),
                    Until::Forever,
                ),
            )],
            vec![],
        ));
        assert_ne!(
            describe_return_then_conditional_animation(&[returned, wrong_tag]).as_deref(),
            Some(concat!(
                "Return target artifact card from your graveyard to the battlefield. ",
                "If it's a Vehicle, it becomes an artifact creature"
            ))
        );
    }
}

/// Render an authored `it` conditional against the exact object tagged by a
/// preceding counter action. The tag is executable provenance; it should not
/// expand into the verbose prior-effect set when the source used a singular
/// pronoun.
pub(crate) fn describe_put_counters_then_conditional_animation(
    effects: &[Effect],
) -> Option<String> {
    let effects = match effects {
        [sequence] => {
            let sequence = sequence.downcast_ref::<crate::effects::SequenceEffect>()?;
            if !matches!(
                sequence.surface,
                ironsmith_core::SequenceSurface::Sequential
                    | ironsmith_core::SequenceSurface::Coordinated
            ) {
                return None;
            }
            sequence.effects.as_slice()
        }
        effects => effects,
    };
    let [counter_effect, conditional_effect] = effects else {
        return None;
    };
    let counter_tag = wrapped_effect_tag(counter_effect)?;
    unwrap_wrapped_effect(counter_effect).downcast_ref::<crate::effects::PutCountersEffect>()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let crate::ConditionExpr::Not(inner) = &conditional.condition else {
        return None;
    };
    let crate::ConditionExpr::TaggedObjectMatches(condition_tag, condition_filter) = inner.as_ref()
    else {
        return None;
    };
    let mut semantic_filter = condition_filter.clone();
    semantic_filter.source_surface = None;
    semantic_filter.union_surface = Default::default();
    semantic_filter.set_explicit_card_type_noun(None);
    if condition_tag != counter_tag || semantic_filter != ObjectFilter::creature() {
        return None;
    }

    let animation = tagged_apply_continuous(&conditional.if_true[0])?;
    if !apply_targets_tag(animation, counter_tag)
        || animation.until != Until::Forever
        || !animation.runtime_modifications.is_empty()
    {
        return None;
    }
    let mut adds_creature_type = false;
    let mut power_toughness = None;
    let mut subtypes = Vec::new();
    for modification in animation
        .modification
        .iter()
        .chain(animation.additional_modifications.iter())
    {
        match modification {
            crate::continuous::Modification::AddCardTypes(card_types)
                if card_types.as_slice() == [CardType::Creature] =>
            {
                adds_creature_type = true;
            }
            crate::continuous::Modification::SetPowerToughness {
                power, toughness, ..
            } => power_toughness = Some((power, toughness)),
            crate::continuous::Modification::AddSubtypes(found) => {
                subtypes.extend(found.iter().copied());
            }
            crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                crate::types::SubtypeFamily::Creature,
            ) => {}
            _ => return None,
        }
    }
    let (power, toughness) = power_toughness?;
    if !adds_creature_type {
        return None;
    }
    let subtype_prefix = subtypes
        .iter()
        .map(|subtype| subtype.display_name())
        .collect::<Vec<_>>()
        .join(" ");
    let creature_descriptor = if subtype_prefix.is_empty() {
        "creature".to_string()
    } else {
        format!("{subtype_prefix} creature")
    };
    Some(format!(
        "{}. If it isn't a creature, it becomes a {}/{} {creature_descriptor} in addition to its other types",
        describe_effect(counter_effect).trim_end_matches('.'),
        describe_value(power),
        describe_value(toughness),
    ))
}

pub(crate) fn describe_immediate_life_gain_then_delayed_source_return(
    effects: &[&Effect],
) -> Option<String> {
    let [gain_effect, schedule_effect] = effects else {
        return None;
    };
    let gain =
        unwrap_basic_tag_wrappers(gain_effect).downcast_ref::<crate::effects::GainLifeEffect>()?;
    if !matches!(gain.player.base(), ChooseSpec::Player(PlayerFilter::You)) {
        return None;
    }

    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    let trigger = schedule.trigger.display().to_ascii_lowercase();
    if !schedule.one_shot
        || schedule.start_next_turn
        || schedule.until_end_of_turn
        || !trigger.contains("beginning of")
        || !trigger.contains("end step")
    {
        return None;
    }
    let [return_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    let returned = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if !matches!(returned.target.base(), ChooseSpec::Source) {
        return None;
    }

    let gain_text = describe_effect(gain_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let return_text = describe_effect(return_effect);
    let return_object = return_text
        .trim()
        .trim_end_matches('.')
        .strip_prefix("Return ")?;
    Some(format!(
        "{gain_text}, and you return {return_object} at the beginning of the next end step"
    ))
}

pub(crate) fn describe_return_with_counter_and_static_followups_text(
    triggering_tag: Option<&crate::TagKey>,
    return_effect: &Effect,
    counter_effect: &Effect,
    followups: &[&Effect],
    timing_suffix: Option<&str>,
    followup_subject: &str,
) -> Option<String> {
    if followups.is_empty() && timing_suffix.is_none() {
        return None;
    }

    let move_to_zone =
        unwrap_wrapped_effect(return_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
    {
        return None;
    }
    let moved_tag = choose_spec_tag(&move_to_zone.target)?;
    let returned_tag = effect_tag(return_effect);

    let put_counters = unwrap_wrapped_effect(counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put_counters.distributed || put_counters.target_count.is_some() {
        return None;
    }
    let continuation_tag = returned_tag
        .filter(|tag| choose_spec_references_exact_tag(&put_counters.target, tag))
        .or_else(|| {
            choose_spec_references_exact_tag(&put_counters.target, moved_tag).then_some(moved_tag)
        })?;

    let followup_clauses = describe_permanent_static_followup_clauses(followups, continuation_tag)?;

    let mut text = describe_effect(return_effect)
        .trim_end_matches('.')
        .to_string();
    if triggering_tag.is_some_and(|tag| tag == moved_tag) {
        text = text.replacen("Return it ", "Return that card ", 1);
    }
    text.push_str(" with ");
    text.push_str(&describe_put_counter_phrase(
        &put_counters.amount,
        put_counters.counter_type,
    ));
    text.push_str(" on it");
    if let Some(timing_suffix) = timing_suffix {
        text.push(' ');
        text.push_str(timing_suffix);
    }

    if followup_clauses.is_empty() {
        return timing_suffix.map(|_| text);
    }
    text.push_str(". ");
    text.push_str(followup_subject);
    text.push(' ');
    text.push_str(&join_with_and(&followup_clauses));
    Some(text)
}

fn describe_permanent_static_followup_clauses(
    followups: &[&Effect],
    continuation_tag: &crate::TagKey,
) -> Option<Vec<String>> {
    let mut ability_labels = Vec::new();
    let mut color_words = Vec::new();
    let mut subtype_words = Vec::new();
    for followup in followups {
        let apply = tagged_apply_continuous(followup)?;
        if apply.until != Until::Forever
            || apply.condition.is_some()
            || !apply.runtime_modifications.is_empty()
            || !matches!(
                apply.target_spec.as_ref(),
                Some(spec) if choose_spec_references_exact_tag(spec, continuation_tag)
            )
        {
            return None;
        }
        let mut found_modification = false;
        for modification in apply
            .modification
            .iter()
            .chain(apply.additional_modifications.iter())
        {
            match modification {
                crate::continuous::Modification::AddAbility(ability) => {
                    ability_labels
                        .push(keyword_label_from_static_ability_id(ability.id())?.to_string());
                    found_modification = true;
                }
                crate::continuous::Modification::AddColors(colors) => {
                    if !colors.is_empty() {
                        color_words.push(describe_token_color_words(*colors, false));
                        found_modification = true;
                    }
                }
                crate::continuous::Modification::AddSubtypes(subtypes) => {
                    subtype_words.extend(
                        subtypes
                            .iter()
                            .map(|subtype| subtype.display_name())
                            .collect::<Vec<_>>(),
                    );
                    found_modification = true;
                }
                _ => return None,
            }
        }
        if !found_modification {
            return None;
        }
    }

    let mut followup_clauses = Vec::new();
    if !ability_labels.is_empty() {
        followup_clauses.push(format!("has {}", join_with_and(&ability_labels)));
    }
    if !color_words.is_empty() && !subtype_words.is_empty() {
        let descriptor = with_indefinite_article(&format!(
            "{} {}",
            color_words.join(" "),
            subtype_words.join(" ")
        ));
        followup_clauses.push(format!(
            "is {descriptor} in addition to its other colors and types"
        ));
    } else if !color_words.is_empty() {
        followup_clauses.push(format!(
            "is {} in addition to its other colors",
            color_words.join(" and ")
        ));
    } else if !subtype_words.is_empty() {
        followup_clauses.push(format!(
            "is {} in addition to its other types",
            with_indefinite_article(&subtype_words.join(" "))
        ));
    }
    Some(followup_clauses)
}

/// Preserve authored follow-up predicates when lowering has already fused an
/// inline entry counter into the battlefield move. The result tag proves that
/// every permanent characteristic change applies to the object returned by
/// that move rather than to its old graveyard snapshot.
pub(crate) fn describe_return_with_inline_counter_and_static_followups(
    effects: &[&Effect],
) -> Option<String> {
    let (effects, triggering_tag) = if let Some(first) = effects.first()
        && let Some(tag_triggering) =
            first.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    {
        (&effects[1..], Some(&tag_triggering.tag))
    } else {
        (effects, None)
    };
    let [return_effect, followups @ ..] = effects else {
        return None;
    };
    if followups.is_empty() {
        return None;
    }

    let move_to_zone =
        unwrap_wrapped_effect(return_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.enters_with_counters.is_empty()
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
    {
        return None;
    }
    let moved_tag = choose_spec_tag(&move_to_zone.target)?;
    let returned_tag = effect_tag(return_effect);
    let followup_clauses = returned_tag
        .and_then(|tag| describe_permanent_static_followup_clauses(followups, tag))
        .or_else(|| describe_permanent_static_followup_clauses(followups, moved_tag))?;
    if followup_clauses.is_empty() {
        return None;
    }

    let mut text = describe_effect(return_effect)
        .trim_end_matches('.')
        .to_string();
    if triggering_tag.is_some_and(|tag| tag == moved_tag) {
        text = text.replacen("Return it ", "Return that card ", 1);
    }
    text.push_str(". It ");
    text.push_str(&join_with_and(&followup_clauses));
    Some(text)
}

pub(crate) fn describe_return_with_counter_and_static_followups(
    effects: &[&Effect],
) -> Option<String> {
    let (effects, triggering_tag) = if let Some(first) = effects.first()
        && let Some(tag_triggering) =
            first.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    {
        (&effects[1..], Some(&tag_triggering.tag))
    } else {
        (effects, None)
    };
    let [return_effect, counter_effect, followups @ ..] = effects else {
        return None;
    };
    describe_return_with_counter_and_static_followups_text(
        triggering_tag,
        return_effect,
        counter_effect,
        followups,
        None,
        "It",
    )
}

pub(crate) fn describe_delayed_return_with_counter_and_static_followups(
    effects: &[&Effect],
) -> Option<String> {
    let (effects, triggering_tag) = if let Some(first) = effects.first()
        && let Some(tag_triggering) =
            first.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    {
        (&effects[1..], Some(&tag_triggering.tag))
    } else {
        (effects, None)
    };
    let [schedule_effect, followups @ ..] = effects else {
        return None;
    };
    let schedule =
        schedule_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if !schedule.one_shot || schedule.start_next_turn || schedule.until_end_of_turn {
        return None;
    }
    let trigger_lower = schedule.trigger.display().to_ascii_lowercase();
    if !trigger_lower.contains("beginning of") || !trigger_lower.contains("end step") {
        return None;
    }
    let [return_effect, counter_effect] = schedule.effects.flattened_default_effects() else {
        return None;
    };
    describe_return_with_counter_and_static_followups_text(
        triggering_tag,
        return_effect,
        counter_effect,
        followups,
        Some("at the beginning of the next end step"),
        "That creature",
    )
}

pub(crate) fn filter_is_tagged_object(filter: &ObjectFilter, tag: &crate::TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    })
}

pub(crate) fn describe_artifact_enchantment_condition(condition: &Condition) -> Option<String> {
    fn controlled_type(condition: &Condition) -> Option<CardType> {
        let Condition::PlayerControls {
            player: PlayerFilter::You,
            filter,
        } = condition
        else {
            return None;
        };
        if filter.controller == Some(PlayerFilter::You)
            && filter.zone == Some(Zone::Battlefield)
            && filter.card_types.len() == 1
            && filter.subtypes.is_empty()
        {
            return filter.card_types.first().copied();
        }
        None
    }

    let Condition::And(left, right) = condition else {
        return None;
    };
    let left_type = controlled_type(left)?;
    let right_type = controlled_type(right)?;
    if matches!(
        (left_type, right_type),
        (CardType::Artifact, CardType::Enchantment) | (CardType::Enchantment, CardType::Artifact)
    ) {
        return Some("you control an artifact and an enchantment".to_string());
    }
    None
}

fn describe_counter_presence_on_tagged_object(filter: &ObjectFilter) -> Option<String> {
    filter.with_counter.as_ref()?;

    // The condition must be only a counter-presence test.  Keeping this
    // structural guard prevents a compact surface from dropping an unrelated
    // type, controller, zone, or characteristic predicate.
    let mut without_counter = filter.clone();
    without_counter.with_counter = None;
    if without_counter != ObjectFilter::default() {
        return None;
    }

    let described = filter.description();
    described
        .strip_prefix("permanent with ")
        .or_else(|| described.strip_prefix("a permanent with "))
        .map(str::to_string)
}

/// A continuous effect can introduce a target tag which a following
/// conditional draw tests for counters.  Render the second instruction as a
/// trailing condition on that exact target instead of inventing a new generic
/// permanent subject ("Then if it's a permanent ...").
pub(crate) fn describe_tagged_continuous_then_counter_conditional_draw(
    effects: &[&Effect],
) -> Option<String> {
    let [leading_effect, conditional_effect] = effects else {
        return None;
    };
    let tag = effect_tag(leading_effect)?;
    let leading = unwrap_wrapped_effect(leading_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if leading.condition.is_some() {
        return None;
    }
    let target_spec = leading.target_spec.as_ref()?;
    let target_reference = tagged_reference_noun_from_target(target_spec)?;

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let [draw_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition else {
        return None;
    };
    if condition_tag != tag {
        return None;
    }
    let counter_presence = describe_counter_presence_on_tagged_object(filter)?;
    let draw =
        unwrap_wrapped_effect(draw_effect).downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You {
        return None;
    }

    let leading_text = describe_effect(leading_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let draw_text =
        normalize_imperative_you_clause(describe_effect(draw_effect).trim().trim_end_matches('.'));
    Some(format!(
        "{leading_text}. {} if {target_reference} has {counter_presence}",
        capitalize_first(&draw_text)
    ))
}

pub(crate) fn describe_tagged_pump_then_conditional_keyword(effects: &[&Effect]) -> Option<String> {
    let [pump_effect, followup_effect] = effects else {
        return None;
    };
    let pump = unwrap_wrapped_effect(pump_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if pump.until != Until::EndOfTurn
        || pump.condition.is_some()
        || pump.modification.is_some()
        || !pump.additional_modifications.is_empty()
    {
        return None;
    }
    let [
        crate::effects::continuous::RuntimeModification::ModifyPowerToughness { power, toughness },
    ] = pump.runtime_modifications.as_slice()
    else {
        return None;
    };
    let target_spec = pump.target_spec.as_ref()?;

    // A same-target power/toughness modification and color choice are sibling
    // continuous instructions with one shared duration.  Preserve their
    // conjoined surface rather than rendering a synthetic temporal "then" and
    // repeating the source subject.
    if let Some(become_color) = unwrap_wrapped_effect(followup_effect)
        .downcast_ref::<crate::effects::BecomeColorChoiceEffect>()
    {
        if become_color.duration != pump.until
            || !target_specs_select_same_objects(target_spec, &become_color.target)
        {
            return None;
        }
        let duration = describe_until(&pump.until);
        if duration.is_empty() {
            return None;
        }
        let subject = if matches!(target_spec.unhinted(), ChooseSpec::Source)
            && pump.require_creature_target
        {
            "This creature".to_string()
        } else {
            capitalize_first(&describe_choose_spec(target_spec))
        };
        let choice = if become_color.allow_multiple {
            "color or colors"
        } else {
            "color"
        };
        let color_action = format!(
            "becomes the {choice} of {} choice",
            describe_possessive_player_filter(&become_color.chooser)
        );
        return Some(format!(
            "{subject} gets {}/{} and {color_action} {duration}",
            describe_signed_value(power),
            describe_toughness_delta_with_power_context(power, toughness),
        ));
    }

    let pumped_tag = effect_tag(pump_effect)?;
    let target_text = describe_choose_spec(target_spec);
    let conditional = followup_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [grant_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    if !conditional.if_false.is_empty() {
        return None;
    }
    let grant = unwrap_wrapped_effect(grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if grant.until != Until::EndOfTurn
        || grant.condition.is_some()
        || !grant.runtime_modifications.is_empty()
    {
        return None;
    }
    match grant.target_spec.as_ref() {
        Some(ChooseSpec::Tagged(tag)) if tag == pumped_tag => {}
        Some(ChooseSpec::Object(filter)) if filter_is_tagged_object(filter, pumped_tag) => {}
        _ => return None,
    }

    // A counter test on the same tagged target is the structural form of
    // "If it has a counter on it, it also gains ...".  Support any set of
    // ordinary keyword grants carried by the continuous-effect model.
    if let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition
        && condition_tag == pumped_tag
        && tagged_reference_noun_from_target(target_spec).is_some()
        && let Some(counter_presence) = describe_counter_presence_on_tagged_object(filter)
    {
        let mut keywords = Vec::new();
        let collect_keyword = |modification: &crate::continuous::Modification| {
            let crate::continuous::Modification::AddAbility(ability) = modification else {
                return None;
            };
            keyword_label_from_static_ability_id(ability.id()).map(str::to_string)
        };
        keywords.push(collect_keyword(grant.modification.as_ref()?)?);
        for modification in &grant.additional_modifications {
            keywords.push(collect_keyword(modification)?);
        }
        if keywords.is_empty() {
            return None;
        }
        let pump_text = describe_effect(pump_effect)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return Some(format!(
            "{pump_text}. If it has {counter_presence}, it also gains {} until end of turn",
            join_with_and(&keywords)
        ));
    }

    if !target_text.contains("target") || !target_text.ends_with("creatures") {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &grant.modification else {
        return None;
    };
    if ability.id() != crate::static_abilities::StaticAbilityId::Lifelink
        || !grant.additional_modifications.is_empty()
    {
        return None;
    }

    let condition = describe_artifact_enchantment_condition(&conditional.condition)
        .unwrap_or_else(|| describe_condition(&conditional.condition));
    Some(format!(
        "{} each get {}/{} until end of turn. If {condition}, those creatures also gain lifelink until end of turn",
        capitalize_first(&target_text),
        describe_signed_value(power),
        describe_toughness_delta_with_power_context(power, toughness),
    ))
}

fn player_filter_is_reference_to_target(candidate: &PlayerFilter, target: &PlayerFilter) -> bool {
    matches!(
        candidate,
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner)
            if inner.as_ref() == target
    )
}

/// Target declaration plus two restrictions which share that player.  The
/// creature restriction quantifies the whole controlled set; it must not be
/// rendered as a new singular target.
pub(crate) fn describe_target_player_cast_and_creatures_attack_restrictions(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, cast_effect, attack_effect] = effects else {
        return None;
    };
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::Target(target) = &target_only.target else {
        return None;
    };
    let ChooseSpec::Player(target_player) = target.as_ref() else {
        return None;
    };

    let cast = cast_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let attack = attack_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if cast.duration != Until::EndOfTurn || attack.duration != Until::EndOfTurn {
        return None;
    }
    let crate::effect::Restriction::CastSpellsMatching(cast_player, spell_filter) =
        &cast.restriction
    else {
        return None;
    };
    if !player_filter_is_reference_to_target(cast_player, target_player)
        || *spell_filter != ObjectFilter::default()
    {
        return None;
    }
    let crate::effect::Restriction::Attack(attack_filter) = &attack.restriction else {
        return None;
    };
    let controller = attack_filter.controller.as_ref()?;
    if !player_filter_is_reference_to_target(controller, target_player) {
        return None;
    }
    let mut creature_filter = attack_filter.clone();
    creature_filter.controller = None;
    // These record only the authored plural/type noun surface. The
    // restriction's executable shape is still the unqualified creature set.
    creature_filter.set_plural_object_noun_surface(false);
    creature_filter.set_explicit_card_type_noun(None);
    if creature_filter != ObjectFilter::creature() {
        return None;
    }

    Some(format!(
        "{} can't cast spells this turn, and creatures that player controls can't attack this turn",
        capitalize_first(&describe_choose_spec(&target_only.target))
    ))
}

pub(crate) fn tagged_reference_noun_from_target(spec: &ChooseSpec) -> Option<&'static str> {
    let ChooseSpec::Target(inner) = spec else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    if filter.zone == Some(Zone::Stack)
        && matches!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        )
    {
        Some("that spell")
    } else if filter.card_types.contains(&CardType::Creature) {
        Some("that creature")
    } else if filter.card_types.contains(&CardType::Artifact) {
        Some("that artifact")
    } else if filter.card_types.contains(&CardType::Enchantment) {
        Some("that enchantment")
    } else if filter.card_types.contains(&CardType::Planeswalker) {
        Some("that planeswalker")
    } else if filter.card_types.contains(&CardType::Battle) {
        Some("that battle")
    } else if filter.card_types.contains(&CardType::Land) {
        Some("that land")
    } else {
        Some("that permanent")
    }
}

pub(crate) fn describe_tagged_effect_then_remove_all_counters(
    effects: &[&Effect],
) -> Option<String> {
    let effects = if let Some(first) = effects.first()
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        &effects[1..]
    } else {
        effects
    };
    let [target_effect, remove_effect] = effects else {
        return None;
    };
    let target_tag = effect_tag(target_effect)?;
    let apply = tagged_apply_continuous(target_effect)?;
    let target_reference = tagged_reference_noun_from_target(apply.target_spec.as_ref()?)?;
    let remove = unwrap_wrapped_effect(remove_effect)
        .downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()?;
    let removes_all_counters_from_tagged_target = match &remove.max_count {
        Value::CountersOn(spec, None) => {
            matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag == target_tag)
        }
        _ => false,
    };
    if !matches!(&remove.target, ChooseSpec::Tagged(tag) if tag == target_tag)
        || !removes_all_counters_from_tagged_target
    {
        return None;
    }

    Some(format!(
        "{}. Remove all counters from {target_reference}",
        describe_effect(target_effect).trim_end_matches('.')
    ))
}

pub(crate) fn conditional_tagged_subtype(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<(&crate::TagKey, Subtype)> {
    let Condition::TaggedObjectMatches(tag, filter) = &conditional.condition else {
        return None;
    };
    if filter.zone.is_none()
        && filter.controller.is_none()
        && filter.card_types.is_empty()
        && filter.subtypes.len() == 1
    {
        return Some((tag, filter.subtypes[0]));
    }
    None
}

pub(crate) fn describe_choose_then_mount_vehicle_become(effects: &[&Effect]) -> Option<String> {
    let [target_effect, first_conditional, second_conditional] = effects else {
        return None;
    };
    let target_tag = effect_tag(target_effect)?;
    let target_only =
        unwrap_wrapped_effect(target_effect).downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCount(target, count) = target_only.target.unhinted() else {
        return None;
    };
    let ChooseSpec::Target(target) = target.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = target.as_ref() else {
        return None;
    };
    let mut expected_target_filter = ObjectFilter::default();
    expected_target_filter.zone = Some(Zone::Battlefield);
    expected_target_filter.controller = Some(PlayerFilter::You);
    expected_target_filter.subtypes = vec![Subtype::Mount, Subtype::Vehicle];
    let mut semantic_target_filter = target_filter.clone();
    semantic_target_filter.source_surface = None;
    semantic_target_filter.union_surface = crate::filter::ObjectFilterUnionSurface::default();
    if count != &crate::effect::ChoiceCount::up_to(1)
        || semantic_target_filter != expected_target_filter
    {
        return None;
    }

    let first = first_conditional.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let second = second_conditional.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !first.if_false.is_empty()
        || !second.if_false.is_empty()
        || first.if_true.len() != 1
        || second.if_true.len() != 1
    {
        return None;
    }
    let (first_tag, first_subtype) = conditional_tagged_subtype(first)?;
    let (second_tag, second_subtype) = conditional_tagged_subtype(second)?;
    if first_tag != target_tag || second_tag != target_tag {
        return None;
    }

    let saddled_source = unwrap_wrapped_effect(&first.if_true[0])
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(saddled_source.source.base(), ChooseSpec::Tagged(tag) if tag == target_tag)
        || saddled_source
            .effect
            .downcast_ref::<crate::effects::BecomeSaddledUntilEotEffect>()
            .is_none()
    {
        return None;
    }

    let vehicle_apply = tagged_apply_continuous(&second.if_true[0])?;
    if vehicle_apply.until != Until::EndOfTurn
        || vehicle_apply.condition.is_some()
        || !vehicle_apply.additional_modifications.is_empty()
        || !vehicle_apply.runtime_modifications.is_empty()
        || !matches!(vehicle_apply.target_spec.as_ref().map(ChooseSpec::base), Some(ChooseSpec::Tagged(tag)) if tag == target_tag)
    {
        return None;
    }
    let card_types = match &vehicle_apply.modification {
        Some(
            crate::continuous::Modification::AddCardTypes(card_types)
            | crate::continuous::Modification::SetCardTypes(card_types),
        ) => card_types,
        _ => return None,
    };
    if first_subtype != Subtype::Mount
        || second_subtype != Subtype::Vehicle
        || card_types.len() != 2
        || !card_types.contains(&CardType::Artifact)
        || !card_types.contains(&CardType::Creature)
    {
        return None;
    }

    Some(format!(
        "{}. Until end of turn, that permanent becomes saddled if it's a Mount and becomes an artifact creature if it's a Vehicle",
        describe_effect(target_effect).trim_end_matches('.')
    ))
}

#[cfg(test)]
mod mount_vehicle_become_tests {
    use super::*;

    fn effects(vehicle_types: Vec<CardType>, vehicle_tag: &str) -> Vec<Effect> {
        let target_tag = TagKey::from("targeted_0");
        let mut target_filter = ObjectFilter::default();
        target_filter.zone = Some(Zone::Battlefield);
        target_filter.controller = Some(PlayerFilter::You);
        target_filter.subtypes = vec![Subtype::Mount, Subtype::Vehicle];
        let target = Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target(ChooseSpec::Object(target_filter))
                .with_count(crate::effect::ChoiceCount::up_to(1)),
        ))
        .tag(target_tag.clone());

        let mut mount_filter = ObjectFilter::default();
        mount_filter.subtypes.push(Subtype::Mount);
        let saddle = Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(target_tag.clone(), mount_filter),
            vec![Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Tagged(target_tag.clone()),
                Effect::new(crate::effects::BecomeSaddledUntilEotEffect::new()),
            ))],
            vec![],
        ));

        let vehicle_tag = TagKey::from(vehicle_tag);
        let mut vehicle_filter = ObjectFilter::default();
        vehicle_filter.subtypes.push(Subtype::Vehicle);
        let animate = Effect::new(crate::effects::ConditionalEffect::new(
            Condition::TaggedObjectMatches(vehicle_tag.clone(), vehicle_filter),
            vec![Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    ChooseSpec::Tagged(vehicle_tag),
                    crate::continuous::Modification::SetCardTypes(vehicle_types),
                    Until::EndOfTurn,
                ),
            )],
            vec![],
        ));
        vec![target, saddle, animate]
    }

    #[test]
    fn set_card_types_mount_vehicle_pair_rejoins_the_shared_duration() {
        let effects = effects(vec![CardType::Artifact, CardType::Creature], "targeted_0");
        let refs = effects.iter().collect::<Vec<_>>();
        assert_eq!(
            describe_choose_then_mount_vehicle_become(&refs).as_deref(),
            Some(concat!(
                "Choose up to one target Mount or Vehicle you control. Until end of turn, ",
                "that permanent becomes saddled if it's a Mount and becomes an artifact creature if it's a Vehicle"
            ))
        );
    }

    #[test]
    fn mount_vehicle_pair_rejects_changed_target_or_card_types() {
        for effects in [
            effects(
                vec![CardType::Artifact, CardType::Creature],
                "different_target",
            ),
            effects(vec![CardType::Artifact], "targeted_0"),
        ] {
            let refs = effects.iter().collect::<Vec<_>>();
            assert!(describe_choose_then_mount_vehicle_become(&refs).is_none());
        }
    }
}

pub(crate) fn describe_tagged_counter_then_color_subtype_keyword(
    effects: &[&Effect],
) -> Option<String> {
    let effects = if let Some(first) = effects.first()
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        &effects[1..]
    } else {
        effects
    };
    let [counter_effect, color_effect, subtype_effect, ability_effect] = effects else {
        return None;
    };
    let counter_tag = wrapped_effect_tag(counter_effect)?;
    unwrap_wrapped_effect(counter_effect).downcast_ref::<crate::effects::PutCountersEffect>()?;

    let color_apply = tagged_apply_continuous(color_effect)?;
    let subtype_apply = tagged_apply_continuous(subtype_effect)?;
    let ability_apply = tagged_apply_continuous(ability_effect)?;
    if color_apply.until != subtype_apply.until
        || color_apply.until != ability_apply.until
        || color_apply.condition.is_some()
        || subtype_apply.condition.is_some()
        || ability_apply.condition.is_some()
        || !apply_targets_tag(color_apply, counter_tag)
        || !apply_targets_tag(subtype_apply, counter_tag)
        || !apply_targets_tag(ability_apply, counter_tag)
        || !color_apply.additional_modifications.is_empty()
        || !subtype_apply.additional_modifications.is_empty()
        || !ability_apply.additional_modifications.is_empty()
        || !color_apply.runtime_modifications.is_empty()
        || !subtype_apply.runtime_modifications.is_empty()
        || !ability_apply.runtime_modifications.is_empty()
    {
        return None;
    }

    let Some(crate::continuous::Modification::SetColors(colors)) = &color_apply.modification else {
        return None;
    };
    let Some(crate::continuous::Modification::AddSubtypes(subtypes)) = &subtype_apply.modification
    else {
        return None;
    };
    let Some(crate::continuous::Modification::AddAbility(ability)) = &ability_apply.modification
    else {
        return None;
    };
    let keyword = keyword_label_from_static_ability_id(ability.id())?;
    if colors.is_empty() || subtypes.is_empty() {
        return None;
    }

    let subtype_words = subtypes
        .iter()
        .map(|subtype| subtype.to_string().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let descriptor = with_indefinite_article(&format!(
        "{} {subtype_words}",
        describe_token_color_words(*colors, false)
    ));
    let mut become_text = format!(
        "That creature becomes {descriptor} in addition to its other types and gains {keyword}"
    );
    if !matches!(color_apply.until, Until::Forever) {
        become_text.push(' ');
        become_text.push_str(&describe_until(&color_apply.until));
    }
    Some(format!(
        "{}. {become_text}",
        describe_effect(counter_effect).trim_end_matches('.')
    ))
}

fn coordinated_color_subtype_effects(effect: &Effect) -> Option<(&Effect, &Effect)> {
    let sequence =
        unwrap_wrapped_effect(effect).downcast_ref::<crate::effects::SequenceEffect>()?;
    let [color_effect, subtype_effect] = sequence.effects.as_slice() else {
        return None;
    };
    Some((color_effect, subtype_effect))
}

pub(crate) fn describe_return_then_color_subtype_addition(effects: &[&Effect]) -> Option<String> {
    let (return_effect, color_effect, subtype_effect, followup_effect) = match effects {
        [return_effect, sequence_effect] => {
            let (color_effect, subtype_effect) =
                coordinated_color_subtype_effects(sequence_effect)?;
            (*return_effect, color_effect, subtype_effect, None)
        }
        [return_effect, sequence_effect, followup_effect]
            if coordinated_color_subtype_effects(sequence_effect).is_some() =>
        {
            let (color_effect, subtype_effect) =
                coordinated_color_subtype_effects(sequence_effect)?;
            (
                *return_effect,
                color_effect,
                subtype_effect,
                Some(*followup_effect),
            )
        }
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
    let returned_tag = effect_tag(return_effect)?;
    let return_to_battlefield = unwrap_wrapped_effect(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
    )?;

    let first_apply = tagged_apply_continuous(color_effect)?;
    let second_apply = tagged_apply_continuous(subtype_effect)?;
    if first_apply.until != second_apply.until
        || first_apply.condition != second_apply.condition
        || !apply_targets_tag(first_apply, returned_tag)
        || !apply_targets_tag(second_apply, returned_tag)
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
        text.push_str(". ");
        text.push_str(describe_effect(followup).trim_end_matches('.'));
    }
    Some(text)
}

pub(crate) fn describe_move_then_color_subtype_addition(effects: &[&Effect]) -> Option<String> {
    let (move_effect, color_effect, subtype_effect) = match effects {
        [move_effect, sequence_effect] => {
            let (color_effect, subtype_effect) =
                coordinated_color_subtype_effects(sequence_effect)?;
            (*move_effect, color_effect, subtype_effect)
        }
        [move_effect, color_effect, subtype_effect] => {
            (*move_effect, *color_effect, *subtype_effect)
        }
        _ => return None,
    };
    let moved_tag = effect_tag(move_effect)?;
    let move_to_zone =
        unwrap_wrapped_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield {
        return None;
    }

    let first_apply = tagged_apply_continuous(color_effect)?;
    let second_apply = tagged_apply_continuous(subtype_effect)?;
    if first_apply.until != second_apply.until
        || first_apply.condition != second_apply.condition
        || !apply_targets_tag(first_apply, moved_tag)
        || !apply_targets_tag(second_apply, moved_tag)
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

    let mut move_text =
        describe_effect(move_effect).replace(" in a graveyard", " from a graveyard");
    move_text = move_text.replace(" in your graveyard", " from your graveyard");
    move_text = move_text.replace(
        " in an opponent's graveyard",
        " from an opponent's graveyard",
    );
    let lower_move = move_text.to_ascii_lowercase();
    let followup_subject = if lower_move.contains("creature card") {
        "That creature"
    } else if lower_move.contains("permanent card") {
        "That permanent"
    } else {
        "That card"
    };
    let subtype_words = subtypes
        .iter()
        .map(|subtype| subtype.to_string().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let descriptor = with_indefinite_article(&format!(
        "{} {subtype_words}",
        describe_token_color_words(colors, false)
    ));
    let mut followup =
        format!("{followup_subject} is {descriptor} in addition to its other colors and types");
    if !matches!(first_apply.until, Until::Forever) {
        followup.push(' ');
        followup.push_str(&describe_until(&first_apply.until));
    }
    Some(format!("{move_text}. {followup}"))
}

pub(crate) fn describe_consult_reveal_put_battlefield_then_bottom(
    effects: &[&Effect],
) -> Option<String> {
    let [consult_effect, move_effect, bottom_effect] = effects else {
        return None;
    };

    let consult = unwrap_wrapped_effect(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;

    let move_to_zone =
        unwrap_wrapped_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return None;
    }

    let bottom = unwrap_wrapped_effect(bottom_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if bottom.tag != consult.all_tag || bottom.keep_tagged.as_ref() != Some(&consult.match_tag) {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let (consult_verb, pronoun_consult_verb) = match consult.mode {
        crate::effects::consult_helpers::LibraryConsultMode::Reveal => {
            (player_verb(&player, "reveal", "reveals"), "reveal")
        }
        crate::effects::consult_helpers::LibraryConsultMode::Exile => {
            (player_verb(&player, "exile", "exiles"), "exile")
        }
    };
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let stop_text =
        describe_consult_stop_text(&selection, &consult.stop_rule, consult.max_exposed.as_ref());
    let stop_text = if matches!(
        &consult.stop_rule,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
            | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
    ) {
        with_indefinite_article(&stop_text)
    } else {
        stop_text
    };
    let moved_phrase = match &consult.stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {
            "that card".to_string()
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(_) => {
            format!("those {}", pluralize_noun_phrase(&selection))
        }
    };
    let tapped_suffix = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let order_text = match bottom.order {
        crate::effects::consult_helpers::LibraryBottomOrder::Random => {
            " in a random order".to_string()
        }
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => format!(
            " in an order chosen by {}",
            describe_player_filter(&bottom.player)
        ),
    };

    if player == "you" {
        Some(format!(
            "{} cards from the top of {library_owner} library until {pronoun} {pronoun_consult_verb} {stop_text}. Put {moved_phrase} onto the battlefield{tapped_suffix} and the rest on the bottom of {library_owner} library{order_text}",
            capitalize_first(consult_verb)
        ))
    } else {
        Some(format!(
            "{player} {consult_verb} cards from the top of {library_owner} library until {pronoun} {pronoun_consult_verb} {stop_text}, then {player} {put_verb} {moved_phrase} onto the battlefield{tapped_suffix} and {put_verb} the rest on the bottom of {library_owner} library{order_text}"
        ))
    }
}

pub(crate) fn doubled_affected_count_for_effect(
    value: &Value,
    id: crate::effect::EffectId,
) -> bool {
    let Value::Add(left, right) = value.unhinted() else {
        return false;
    };
    let is_matching_metric = |value: &Value| {
        matches!(
            value.unhinted(),
            Value::EffectMetric {
                effect_id,
                source: crate::effect::EffectMetricSource::AffectedObjects,
                metric: crate::effect::EffectMetric::Count
                    | crate::effect::EffectMetric::AffectedCount,
            } if *effect_id == id
        )
    };
    is_matching_metric(left) && is_matching_metric(right)
}

pub(crate) fn describe_destroy_then_doubled_life_loss(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let with_id = first.downcast_ref::<crate::effects::WithIdEffect>()?;
    let destroy =
        unwrap_wrapped_effect(&with_id.effect).downcast_ref::<crate::effects::DestroyEffect>()?;
    let lose = second.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if !matches!(lose.player, ChooseSpec::Player(PlayerFilter::You))
        || !doubled_affected_count_for_effect(&lose.amount, with_id.id)
    {
        return None;
    }

    let (destroy_text, counted) = match &destroy.spec {
        ChooseSpec::All(filter)
            if filter.card_types.as_slice() == [CardType::Creature]
                && matches!(
                    filter.controller,
                    Some(PlayerFilter::Target(ref inner))
                        if matches!(inner.as_ref(), PlayerFilter::Opponent)
                ) =>
        {
            (
                "Destroy all creatures target opponent controls".to_string(),
                "creature".to_string(),
            )
        }
        ChooseSpec::All(filter) if filter.card_types.as_slice() == [CardType::Creature] => {
            (describe_effect(first), "creature".to_string())
        }
        _ => (describe_effect(first), "object".to_string()),
    };

    Some(format!(
        "{destroy_text}. You lose 2 life for each {counted} destroyed this way"
    ))
}

pub(crate) fn describe_put_onto_battlefield_attached(effects: &[&Effect]) -> Option<String> {
    let effects = if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            .is_some()
    {
        rest
    } else {
        effects
    };
    let [move_effect, attach_effect] = effects else {
        return None;
    };

    let moved_tag = effect_tag(move_effect)?;
    let moved_effect = unwrap_basic_tag_wrappers(move_effect);
    if moved_effect
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        .is_some()
    {
        let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
        if !matches!(attach.objects.base(), ChooseSpec::Source)
            || !choose_spec_references_exact_tag(&attach.target, moved_tag)
        {
            return None;
        }
        let return_text = describe_effect(move_effect)
            .trim_end_matches('.')
            .to_string();
        let attach_text = lowercase_first(describe_effect(attach_effect).trim_end_matches('.'));
        return Some(format!("{return_text} and {attach_text}"));
    }
    let move_to_zone = moved_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || !choose_spec_references_exact_tag(&attach.objects, moved_tag)
    {
        return None;
    }

    let move_text = describe_attached_battlefield_move_text(move_to_zone).unwrap_or_else(|| {
        describe_effect(move_effect)
            .trim_end_matches('.')
            .to_string()
    });

    let attachment_target = describe_return_attachment_target(&attach.target);
    let attachment_target = if attach.individual_targets {
        pluralize_noun_phrase(&attachment_target)
    } else {
        attachment_target
    };
    Some(format!("{move_text} attached to {attachment_target}"))
}

pub(crate) fn describe_become_aura_manifest_then_attach(effects: &[&Effect]) -> Option<String> {
    let [become_aura_effect, manifest_effect, attach_effect] = effects else {
        return None;
    };
    let become_aura = describe_effect(become_aura_effect)
        .trim_end_matches('.')
        .to_string();
    if !become_aura
        .to_ascii_lowercase()
        .contains("becomes an aura with enchant")
    {
        return None;
    }
    let tagged_manifest = manifest_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if tagged_manifest
        .effect
        .downcast_ref::<crate::effects::ManifestTopCardOfLibraryEffect>()
        .is_none()
        && tagged_manifest
            .effect
            .downcast_ref::<crate::effects::ManifestCardFromHandEffect>()
            .is_none()
    {
        return None;
    }
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(attach.objects.base(), ChooseSpec::Source)
        || !choose_spec_references_exact_tag(&attach.target, &tagged_manifest.tag)
    {
        return None;
    }
    let manifest = describe_effect(manifest_effect)
        .trim_end_matches('.')
        .to_string();
    let attach = lowercase_first(describe_effect(attach_effect).trim_end_matches('.'));
    Some(format!("{become_aura}. {manifest} and {attach}"))
}

pub(crate) fn describe_attached_battlefield_move_text(
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || !matches!(
            move_to_zone.battlefield_controller,
            crate::effects::BattlefieldController::Preserve
                | crate::effects::BattlefieldController::Owner
        )
    {
        return None;
    }

    if matches!(move_to_zone.target.base(), ChooseSpec::Source) {
        return Some("Return this card to the battlefield".to_string());
    }

    if let Some(owner) = graveyard_owner_from_spec(&move_to_zone.target) {
        let moved_text = describe_choose_spec_without_graveyard_zone(&move_to_zone.target)
            .replace("Auras or Equipment cards", "Aura and/or Equipment cards");
        let from_graveyard = match owner {
            Some(owner) => format!(
                "from {} graveyard",
                describe_possessive_player_filter(&owner)
            ),
            None => "from a graveyard".to_string(),
        };
        return Some(format!(
            "Return {moved_text} {from_graveyard} to the battlefield"
        ));
    }

    None
}

pub(crate) fn describe_return_attachment_target(target: &ChooseSpec) -> String {
    if let Some(surface) = target.source_reference_surface() {
        return surface.display_text();
    }
    let enchanted = crate::TagKey::from("enchanted");
    if choose_spec_is_tagged_object(target, &enchanted) {
        return "enchanted creature".to_string();
    }
    match target {
        ChooseSpec::Tagged(tag) if tag.as_str() == "enchanted" => "enchanted creature".to_string(),
        ChooseSpec::Tagged(tag)
            if tag.as_str() == "__it__"
                || tag.as_str() == "triggering"
                || tag.as_str().starts_with("targeted_") =>
        {
            "that creature".to_string()
        }
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            describe_return_attachment_target(inner)
        }
        ChooseSpec::SurfaceHinted { spec, .. } => describe_return_attachment_target(spec),
        _ => describe_choose_spec(target),
    }
}

pub(crate) fn describe_return_then_return_attached(effects: &[&Effect]) -> Option<String> {
    let [return_effect, move_effect, attach_effect] = effects else {
        return None;
    };

    let returned_tag = effect_tag(return_effect)?;
    unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;
    let tagged_move = move_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let move_to_zone = tagged_move
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let attach = attach_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || !matches!(
            move_to_zone.battlefield_controller,
            crate::effects::BattlefieldController::Preserve
        )
        || !choose_spec_references_exact_tag(&attach.objects, &tagged_move.tag)
        || !choose_spec_is_tagged_object(&attach.target, returned_tag)
    {
        return None;
    }

    let graveyard_owner = graveyard_owner_from_spec(&move_to_zone.target)?;
    let from_graveyard = match graveyard_owner {
        Some(owner) => format!(
            "from {} graveyard",
            describe_possessive_player_filter(&owner)
        ),
        None => "from a graveyard".to_string(),
    };
    let return_text = describe_effect(return_effect)
        .trim_end_matches('.')
        .to_string();
    let moved_text = describe_choose_spec_without_graveyard_zone(&move_to_zone.target)
        .replace("Auras or Equipment cards", "Aura and/or Equipment cards");
    Some(format!(
        "{return_text}, then return {moved_text} {from_graveyard} to the battlefield attached to that creature"
    ))
}

pub(crate) fn describe_exile_then_incubate_count(effects: &[&Effect]) -> Option<String> {
    let [exile_effect, incubate_effect] = effects else {
        return None;
    };
    let with_id = exile_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let exile =
        unwrap_wrapped_effect(&with_id.effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let incubate =
        unwrap_wrapped_effect(incubate_effect).downcast_ref::<crate::effects::IncubateEffect>()?;
    if incubate.controller != PlayerFilter::You || incubate.count != Value::Fixed(1) {
        return None;
    }
    let Value::EffectMetric {
        effect_id,
        source: crate::effect::EffectMetricSource::AffectedObjects,
        metric: crate::effect::EffectMetric::Count | crate::effect::EffectMetric::AffectedCount,
    } = incubate.amount.unhinted()
    else {
        return None;
    };
    if *effect_id != with_id.id {
        return None;
    }
    let basis = match &exile.spec {
        ChooseSpec::All(filter) => format!(
            "the number of {} exiled this way",
            pluralize_noun_phrase(&describe_for_each_count_filter(filter))
        ),
        ChooseSpec::Object(filter) => format!(
            "the number of {} exiled this way",
            pluralize_noun_phrase(&describe_for_each_count_filter(filter))
        ),
        _ => "the number of objects exiled this way".to_string(),
    };
    Some(format!(
        "{}. Incubate X, where X is {basis}",
        describe_effect(exile_effect)
    ))
}

pub(crate) fn describe_clash_win_optional_top_replacement(effects: &[&Effect]) -> Option<String> {
    let [clash_effect, conditional_effect] = effects else {
        return None;
    };
    let clash_with_id = wrapped_with_id(clash_effect)?;
    clash_with_id
        .effect
        .downcast_ref::<crate::effects::ClashEffect>()?;
    let conditional = conditional_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != clash_with_id.id
        || conditional.predicate
            != crate::effect::EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
        || conditional.then.len() != 1
        || conditional.else_.len() != 1
    {
        return None;
    }
    let local = conditional.then[0].downcast_ref::<crate::effects::LocalRewriteEffect>()?;
    let replacement = local.zone_replacements.first()?;
    if !replacement.optional
        || replacement.from_zone != Some(Zone::Battlefield)
        || replacement.to_zone != Some(Zone::Hand)
        || replacement.replacement_zone != Zone::Library
    {
        return None;
    }
    let return_text = describe_effect(&local.effect);
    if return_text != describe_effect(&conditional.else_[0])
        || !return_text
            .to_ascii_lowercase()
            .starts_with("return target creature")
    {
        return None;
    }
    Some(format!(
        "Clash with an opponent, then {}. If you win, you may put that creature on top of its owner's library instead",
        lowercase_first(&return_text)
    ))
}

pub(crate) fn describe_energy_then_pay_any_then_destroy(effects: &[&Effect]) -> Option<String> {
    let [energy_effect, may_effect, destroy_effect] = effects else {
        return None;
    };
    let energy = unwrap_wrapped_effect(energy_effect)
        .downcast_ref::<crate::effects::EnergyCountersEffect>()?;
    if energy.player != PlayerFilter::You {
        return None;
    }
    let may_effect = unwrap_singleton_sequence_member(may_effect);
    let may = unwrap_wrapped_effect(may_effect).downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You)) || may.effects.len() != 1 {
        return None;
    }
    let pay_any = unwrap_wrapped_effect(&may.effects[0])
        .downcast_ref::<crate::effects::PayAnyEnergyEffect>()?;
    if !matches!(pay_any.player, ChooseSpec::Player(PlayerFilter::You)) {
        return None;
    }
    let destroy = describe_effect(destroy_effect);
    if !destroy.contains("the amount of {E} paid this way") {
        return None;
    }
    let may_text = describe_effect(may_effect);
    let may_tail = may_text
        .strip_prefix("You may ")
        .or_else(|| may_text.strip_prefix("you may "))?;
    Some(format!(
        "{}, then you may {}. {destroy}",
        describe_effect(energy_effect),
        may_tail
    ))
}

pub(crate) fn describe_energy_then_pay_any_then_create_paid_x_token(
    effects: &[&Effect],
) -> Option<String> {
    let [energy_effect, may_effect, conditional_effect] = effects else {
        return None;
    };
    let energy = unwrap_wrapped_effect(energy_effect)
        .downcast_ref::<crate::effects::EnergyCountersEffect>()?;
    if energy.player != PlayerFilter::You {
        return None;
    }
    let may_effect = unwrap_singleton_sequence_member(may_effect);
    let may_with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = may_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You)) || may.effects.len() != 1 {
        return None;
    }
    let pay_any = unwrap_wrapped_effect(&may.effects[0])
        .downcast_ref::<crate::effects::PayAnyEnergyEffect>()?;
    if !matches!(pay_any.player, ChooseSpec::Player(PlayerFilter::You)) {
        return None;
    }
    let if_effect = conditional_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != may_with_id.id
        || if_effect.predicate != crate::effect::EffectPredicate::Happened
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 2
    {
        return None;
    }
    let create = unwrap_tag_wrappers(&if_effect.then[0])
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create.count != Value::Fixed(1) || !create.token.card.is_token {
        return None;
    }
    let set_pt = if_effect.then[1].downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()?;
    if !is_effect_count_reference(&set_pt.power, Some(may_with_id.id))
        || !is_effect_count_reference(&set_pt.toughness, Some(may_with_id.id))
    {
        return None;
    }

    let payment = describe_pay_any_energy_amount(pay_any)?;
    let if_text = describe_effect(conditional_effect)
        .replace(
            &format!("If effect #{} happened", may_with_id.id.0),
            "If you do",
        )
        .replace("where X is X", "where X is the amount of {E} paid this way");
    Some(format!(
        "{}, then you may pay {payment}. {if_text}",
        describe_effect(energy_effect)
    ))
}

pub(crate) fn describe_energy_then_pay_any_then_put_paid_counters(
    effects: &[&Effect],
) -> Option<String> {
    let [energy_effect, may_effect, if_effect] = effects else {
        return None;
    };
    let energy = unwrap_wrapped_effect(energy_effect)
        .downcast_ref::<crate::effects::EnergyCountersEffect>()?;
    if energy.player != PlayerFilter::You {
        return None;
    }
    let may_effect = unwrap_singleton_sequence_member(may_effect);
    let may_with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = may_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You)) || may.effects.len() != 1 {
        return None;
    }
    let pay_any = unwrap_wrapped_effect(&may.effects[0])
        .downcast_ref::<crate::effects::PayAnyEnergyEffect>()?;
    if !matches!(pay_any.player, ChooseSpec::Player(PlayerFilter::You)) {
        return None;
    }
    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != may_with_id.id
        || if_effect.predicate != crate::effect::EffectPredicate::Happened
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 1
    {
        return None;
    }
    let put = if_effect.then[0].downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed
        || put.target_count.is_some()
        || !is_effect_count_reference(&put.amount, Some(may_with_id.id))
    {
        return None;
    }

    let target = describe_choose_spec(&put.target);
    let counter = describe_counter_type(put.counter_type);
    let payment = describe_pay_any_energy_amount(pay_any)?;
    Some(format!(
        "{}. Then you may pay {payment}. If you do, put that many {counter} counters on {target}",
        describe_effect(energy_effect)
    ))
}

pub(crate) fn describe_copy_then_may_cast_copy(effects: &[&Effect]) -> Option<String> {
    let [copy_effect, may_effect] = effects else {
        return None;
    };
    let copy_spell = copy_spell_from_effect(copy_effect)?;
    if copy_spell.count != Value::Fixed(1)
        || !copy_spell.removed_supertypes.is_empty()
        || copy_spell.has_characteristic_modifiers()
    {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if !cast.as_copy {
        return None;
    }

    let copied = match &copy_spell.target {
        ChooseSpec::Tagged(tag) if tag.as_str().starts_with("__sentence_helper_exiled") => {
            "it".to_string()
        }
        ChooseSpec::Tagged(tag)
            if tag.as_str() == "triggering" && cast.cost_reduction.is_some() =>
        {
            "that card".to_string()
        }
        ChooseSpec::Tagged(tag) if tag.as_str().starts_with("exiled_") => {
            "the exiled card".to_string()
        }
        ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::PRIOR_EXILED_CARD_TAG => {
            "the exiled card".to_string()
        }
        _ => describe_choose_spec(&copy_spell.target),
    };
    let mut cast_text = "You may cast the copy".to_string();
    if cast.without_paying_mana_cost {
        cast_text.push_str(" without paying its mana cost");
    }
    if let Some(reduction) = cast.cost_reduction.as_ref() {
        return Some(format!(
            "Copy {copied} and you may cast the copy. That copy costs {} less to cast",
            reduction.to_oracle()
        ));
    }
    Some(format!("Copy {copied}. {cast_text}"))
}

pub(crate) fn describe_two_distinct_targets_counter_then_fight(
    effects: &[&Effect],
) -> Option<String> {
    let [
        first_target_effect,
        second_target_effect,
        counter_effect,
        fight_effect,
    ] = effects
    else {
        return None;
    };
    let (first_tag, first_target) = tagged_target_only_effect(first_target_effect)?;
    let (second_tag, second_target) = tagged_target_only_effect(second_target_effect)?;

    let conditional = counter_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let counter_tagged = conditional.if_true[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let counters = counter_tagged
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(&counters.target, ChooseSpec::Tagged(tag) if tag == first_tag) {
        return None;
    }

    let fight = fight_effect.downcast_ref::<crate::effects::FightEffect>()?;
    if !matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == first_tag)
        || !matches!(&fight.creature2, ChooseSpec::Tagged(tag) if tag == second_tag)
    {
        return None;
    }

    let first = describe_choose_spec(&first_target.target);
    let second = describe_choose_spec(&second_target.target);
    let counter = describe_counter_type(counters.counter_type);
    let count = describe_value(&counters.amount);
    let counter_text = if count == "1" {
        format!("Put a {counter} counter on the creature you control")
    } else {
        format!("Put {count} {counter} counters on the creature you control")
    };
    Some(format!(
        "Choose {first} and {second}. {counter_text} if {}. Then those creatures fight each other",
        describe_condition(&conditional.condition)
    ))
}

fn explicit_controlled_creature_target(
    target: &crate::effects::TargetOnlyEffect,
    controller: PlayerFilter,
) -> bool {
    let ChooseSpec::Target(inner) = &target.target else {
        return false;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return false;
    };
    filter.zone == Some(Zone::Battlefield)
        && filter.card_types.as_slice() == [CardType::Creature]
        && filter.controller == Some(controller)
}

fn conditional_continuous_action_for_target(
    effects: &[Effect],
    target_tag: &crate::TagKey,
) -> Option<String> {
    fn collect_applies<'a>(
        effects: &'a [Effect],
        applies: &mut Vec<&'a crate::effects::ApplyContinuousEffect>,
    ) -> Option<()> {
        for effect in effects {
            let effect = unwrap_wrapped_effect(effect);
            if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
                applies.push(apply);
                continue;
            }
            if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
                collect_applies(&sequence.effects, applies)?;
                continue;
            }
            return None;
        }
        Some(())
    }

    let mut applies = Vec::new();
    collect_applies(effects, &mut applies)?;
    let first = *applies.first()?;
    if applies.iter().any(|apply| {
        apply.condition.is_some()
            || !matches!(
                apply.target_spec.as_ref(),
                Some(ChooseSpec::Tagged(tag)) if tag == target_tag
            )
            || apply.until != first.until
            || describe_apply_continuous_tail(apply) != describe_apply_continuous_tail(first)
    }) {
        return None;
    }

    let clauses = applies
        .iter()
        .flat_map(|apply| describe_apply_continuous_clauses(apply, false))
        .collect::<Vec<_>>();
    if clauses.is_empty() {
        return None;
    }
    let mut action = join_with_and(&clauses);
    if let Some(tail) = describe_apply_continuous_tail(first) {
        action.push(' ');
        action.push_str(&tail);
    }
    Some(action)
}

fn coven_counter_action_for_target(
    conditional: &crate::effects::ConditionalEffect,
    target_tag: &crate::TagKey,
) -> Option<String> {
    fn is_coven_condition(condition: &Condition) -> bool {
        let (player, filter, count, distinct_powers_in_filter) = match condition {
            Condition::PlayerHasAtLeastWithDifferentPowers {
                player,
                filter,
                count,
            } => (player, filter, count, false),
            Condition::PlayerHasAtLeast {
                player,
                filter,
                count,
            } if filter.distinct_powers => (player, filter, count, true),
            _ => return false,
        };
        if player != &PlayerFilter::You || *count != 3 {
            return false;
        }

        let mut normalized_filter = filter.clone();
        if normalized_filter.controller.as_ref() == Some(player) {
            normalized_filter.controller = None;
        }
        if distinct_powers_in_filter {
            normalized_filter.distinct_powers = false;
        }
        normalized_filter == ObjectFilter::creature()
    }

    fn singleton_put_counters(effect: &Effect) -> Option<&crate::effects::PutCountersEffect> {
        let effect = unwrap_wrapped_effect(effect);
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let [effect] = sequence.effects.as_slice() else {
                return None;
            };
            return singleton_put_counters(effect);
        }
        effect.downcast_ref()
    }

    if !is_coven_condition(&conditional.condition) {
        return None;
    }
    let [effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let counters = singleton_put_counters(effect)?;
    if counters.distributed
        || counters.target_count.is_some()
        || !matches!(&counters.target, ChooseSpec::Tagged(tag) if tag == target_tag)
    {
        return None;
    }
    Some(format!(
        "put {} on the chosen creature you control",
        describe_put_counter_phrase(&counters.amount, counters.counter_type)
    ))
}

pub(crate) fn describe_two_distinct_targets_conditional_then_fight(
    effects: &[&Effect],
) -> Option<String> {
    let [
        first_target_effect,
        second_target_effect,
        conditional_effect,
        fight_effect,
    ] = effects
    else {
        return None;
    };
    let (first_tag, first_target) = tagged_target_only_effect(first_target_effect)?;
    let (second_tag, second_target) = tagged_target_only_effect(second_target_effect)?;
    if first_tag == second_tag
        || !explicit_controlled_creature_target(first_target, PlayerFilter::You)
        || !explicit_controlled_creature_target(second_target, PlayerFilter::NotYou)
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.is_empty() {
        return None;
    }
    let fight = fight_effect.downcast_ref::<crate::effects::FightEffect>()?;
    if !matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == first_tag)
        || !matches!(&fight.creature2, ChooseSpec::Tagged(tag) if tag == second_tag)
    {
        return None;
    }

    let first = describe_choose_spec(&first_target.target);
    let second = describe_choose_spec(&second_target.target);
    let condition = describe_condition(&conditional.condition);
    let (conditional_text, fighter_reference, ability_word) =
        if let Some(action) = coven_counter_action_for_target(conditional, first_tag) {
            (
                format!("If {condition}, {action}"),
                "the chosen creatures",
                "Coven — ",
            )
        } else {
            let action = conditional_continuous_action_for_target(&conditional.if_true, first_tag)?;
            let subject = "the creature you control";
            if matches!(
                &conditional.condition,
                Condition::TaggedObjectMatches(tag, _) if tag == first_tag
            ) {
                (
                    format!("The creature you control {action} if {condition}"),
                    "those creatures",
                    "",
                )
            } else {
                (
                    format!("If {condition}, {subject} {action}"),
                    "those creatures",
                    "",
                )
            }
        };

    Some(format!(
        "{ability_word}Choose {first} and {second}. {conditional_text}. Then {fighter_reference} fight each other"
    ))
}

fn describe_bound_target_condition(
    condition: &Condition,
    target_tag: &crate::TagKey,
) -> Option<String> {
    let Condition::TaggedObjectMatches(tag, filter) = condition else {
        return None;
    };
    if tag != target_tag {
        return None;
    }

    let mut legendary = ObjectFilter::default();
    legendary.supertypes = vec![crate::types::Supertype::Legendary];
    if filter == &legendary {
        return Some("it's legendary".to_string());
    }
    Some(describe_condition(condition))
}

pub(crate) fn describe_targeted_conditional_action_then_fight(
    effects: &[&Effect],
) -> Option<String> {
    let [
        opposing_target_effect,
        friendly_target_effect,
        conditional_effect,
        fight_effect,
    ] = effects
    else {
        return None;
    };
    let (opposing_tag, opposing_target) = tagged_target_only_effect(opposing_target_effect)?;
    let (friendly_tag, friendly_target) = tagged_target_only_effect(friendly_target_effect)?;
    if opposing_tag == friendly_tag
        || (!explicit_controlled_creature_target(opposing_target, PlayerFilter::Opponent)
            && !explicit_controlled_creature_target(opposing_target, PlayerFilter::NotYou))
        || !explicit_controlled_creature_target(friendly_target, PlayerFilter::You)
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let [action_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let condition = describe_bound_target_condition(&conditional.condition, friendly_tag)?;
    let fight = fight_effect.downcast_ref::<crate::effects::FightEffect>()?;
    if !matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == friendly_tag)
        || !matches!(&fight.creature2, ChooseSpec::Tagged(tag) if tag == opposing_tag)
    {
        return None;
    }

    let friendly = describe_choose_spec(&friendly_target.target);
    let opposing = describe_choose_spec(&opposing_target.target);
    let action = unwrap_wrapped_effect(action_effect);
    let first_sentence =
        if let Some(counters) = action.downcast_ref::<crate::effects::PutCountersEffect>() {
            if counters.distributed
                || counters.target_count.is_some()
                || !matches!(&counters.target, ChooseSpec::Tagged(tag) if tag == friendly_tag)
            {
                return None;
            }
            format!(
                "Put {} on {friendly} if {condition}",
                describe_put_counter_phrase(&counters.amount, counters.counter_type)
            )
        } else {
            let continuous =
                conditional_continuous_action_for_target(&conditional.if_true, friendly_tag)?;
            format!(
                "{} {continuous} if {condition}",
                capitalize_first(&friendly)
            )
        };

    Some(format!("{first_sentence}. Then it fights {opposing}"))
}

pub(crate) fn describe_look_at_hand_top_and_face_down_creatures(
    effects: &[&Effect],
) -> Option<String> {
    let [hand_effect, top_effect, objects_effect] = effects else {
        return None;
    };
    let hand = hand_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let viewed_player = choose_spec_player_filter(&hand.target)?;
    if hand.reveal
        || !matches!(
            viewed_player,
            PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_)
        )
    {
        return None;
    }
    let top = top_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if top.reveal
        || top.count != Value::Fixed(1)
        || !player_filters_refer_to_same_player(&top.player, &viewed_player)
    {
        return None;
    }
    let objects = objects_effect.downcast_ref::<crate::effects::LookAtObjectsEffect>()?;
    let mut object_filter = objects.filter.clone();
    let object_controller = object_filter.controller.take()?;
    if objects.viewer != PlayerFilter::You
        || !player_filters_refer_to_same_player(&objects.subject, &viewed_player)
        || !player_filters_refer_to_same_player(&object_controller, &viewed_player)
        || object_filter != ObjectFilter::creature().face_down()
    {
        return None;
    }
    Some(
        "Look at target player's hand, the top card of that player's library, and any face-down creatures they control"
            .to_string(),
    )
}

pub(crate) fn describe_reveal_hand_choose_move(effects: &[&Effect]) -> Option<String> {
    let [look_effect, choose_effect, move_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let looked_player = choose_spec_player_filter(&look.target)?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose
            .filter
            .owner
            .as_ref()
            .is_none_or(|owner| !player_filters_refer_to_same_player(owner, &looked_player))
    {
        return None;
    }
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
        || move_to_zone.zone != Zone::Exile
        || move_to_zone.enters_tapped
    {
        return None;
    }
    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let mut selection = choose.filter.description();
    for suffix in [
        format!(" in {revealer}'s hand"),
        " in their hand".to_string(),
        " in hand".to_string(),
    ] {
        if let Some(rest) = selection.strip_suffix(&suffix) {
            selection = rest.trim().to_string();
            break;
        }
    }
    let selection = with_indefinite_article(&selection);
    Some(format!(
        "{} {} their hand. You choose {selection} from it. Exile that card",
        capitalize_first(&revealer),
        reveal_verb
    ))
}

/// "exile ... and copy it. You may cast the copy without paying its mana
/// cost." — the copy lives at the tail of the exile sentence's sequence, and
/// the standalone may-cast render would re-emit a spurious "Copy it."
pub(crate) fn describe_sequence_copy_then_may_cast(effects: &[&Effect]) -> Option<String> {
    if std::env::var("IRONSMITH_COPY_TRACE").is_ok() {
        eprintln!("seq-copy-may window: len={}", effects.len());
    }
    let [sequence_effect, may_effect] = effects else {
        return None;
    };
    if std::env::var("IRONSMITH_COPY_TRACE").is_ok() {
        eprintln!(
            "seq-copy-may pair: seq={} may={}",
            sequence_effect
                .downcast_ref::<crate::effects::SequenceEffect>()
                .is_some(),
            may_effect
                .downcast_ref::<crate::effects::MayEffect>()
                .is_some()
        );
    }
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let copy_spell = copy_spell_from_effect(sequence.effects.last()?);
    if std::env::var("IRONSMITH_COPY_TRACE").is_ok() {
        eprintln!("seq-copy-may copy: {copy_spell:?}");
    }
    let copy_spell = copy_spell?;
    if copy_spell.count.unhinted() != &Value::Fixed(1)
        || !copy_spell.removed_supertypes.is_empty()
        || copy_spell.has_characteristic_modifiers()
    {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast =
        unwrap_wrapped_effect(cast_effect).downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if std::env::var("IRONSMITH_COPY_TRACE").is_ok() {
        eprintln!(
            "seq-copy-may cast: as_copy={} tag={:?} reduction={:?}",
            cast.as_copy, cast.tag, cast.cost_reduction
        );
    }
    if !cast.as_copy
        || cast.cost_reduction.is_some()
        || !matches!(&copy_spell.target, ChooseSpec::Tagged(tag) if *tag == cast.tag)
    {
        return None;
    }
    let mut cast_text = "You may cast the copy".to_string();
    if cast.without_paying_mana_cost {
        cast_text.push_str(" without paying its mana cost");
    }
    let sequence_text = describe_effect(sequence_effect);
    Some(format!(
        "{}. {cast_text}",
        sequence_text.trim_end_matches('.')
    ))
}

/// "you may reveal a Soldier card from your hand" lowers as a hand-zone
/// choice followed by a reveal of the chosen card; recombine the pair into
/// the authored reveal-from-hand surface.
pub(crate) fn describe_choose_hand_then_reveal(effects: &[&Effect]) -> Option<String> {
    let [choose_effect, reveal_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != choose.tag {
        return None;
    }
    if choose.chooser != PlayerFilter::You
        || choose.is_search
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose
            .filter
            .owner
            .as_ref()
            .is_some_and(|owner| *owner != PlayerFilter::You)
        || choose
            .filter
            .controller
            .as_ref()
            .is_some_and(|controller| *controller != PlayerFilter::You)
    {
        return None;
    }
    let selection = hand_choice_from_it_text(choose)?;
    Some(format!("reveal {selection} from your hand"))
}

pub(crate) fn describe_reveal_hand_choose_discard(effects: &[&Effect]) -> Option<String> {
    let [look_effect, choose_effect, discard_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let looked_player = choose_spec_player_filter(&look.target)?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose
            .filter
            .owner
            .as_ref()
            .is_none_or(|owner| !player_filters_refer_to_same_player(owner, &looked_player))
    {
        return None;
    }
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.count != Value::Fixed(1)
        || discard.random
        || discard.any_number
        || !player_filters_refer_to_same_player(&discard.player, &looked_player)
        || !discard.card_filter.as_ref().is_some_and(|filter| {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag == choose.tag
            })
        })
    {
        return None;
    }
    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let selection = hand_choice_from_it_text(choose)?;
    Some(format!(
        "{} {} their hand. You choose {selection} from it. That player discards that card",
        capitalize_first(&revealer),
        reveal_verb
    ))
}

pub(crate) fn describe_reveal_hand_then_discard(effects: &[&Effect]) -> Option<String> {
    let [look_effect, discard_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !look.reveal || discard.random || discard.any_number {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    if describe_player_filter(&discard.player) != revealer {
        return None;
    }

    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let discard_verb = player_verb(&revealer, "discard", "discards");
    let hand = if revealer == "you" {
        "your hand"
    } else {
        "their hand"
    };
    Some(format!(
        "{} {} {hand} and {discard_verb} {}",
        capitalize_first(&revealer),
        reveal_verb,
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    ))
}

pub(crate) fn downcast_move_to_library_nth(
    effect: &Effect,
) -> Option<&crate::effects::MoveToLibraryNthFromTopEffect> {
    if let Some(move_to_library) =
        effect.downcast_ref::<crate::effects::MoveToLibraryNthFromTopEffect>()
    {
        return Some(move_to_library);
    }
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()?
        .effect
        .downcast_ref::<crate::effects::MoveToLibraryNthFromTopEffect>()
}

pub(crate) fn library_top_position_text(position: &crate::effect::Value) -> String {
    library_position_from_top_text(position, true)
}

pub(crate) fn look_hand_choose_then_move_to_library(effects: &[&Effect]) -> Option<String> {
    let [look_effect, choose_effect, move_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Hand)
    {
        return None;
    }

    let looked_player = describe_choose_spec(&look.target);
    let choose_owner_matches_looked_player = choose.filter.owner.as_ref().is_none_or(|owner| {
        let owner_text = describe_player_filter(owner);
        owner_text == looked_player || owner_text == format!("target {looked_player}")
    });
    if !choose_owner_matches_looked_player {
        return None;
    }

    let selection = hand_choice_selection_from_it(choose);
    if let Some(move_to_zone) =
        unwrap_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_library_uses_chosen_tag(move_to_zone, choose.tag.as_str())
        && move_to_zone.to_top
    {
        let look_verb = if look.reveal {
            player_verb(&looked_player, "reveal", "reveals")
        } else {
            "look at"
        };
        let opener = if look.reveal {
            format!(
                "{} {look_verb} their hand",
                capitalize_first(&looked_player)
            )
        } else {
            format!("Look at {looked_player}'s hand")
        };
        return Some(format!(
            "{opener} and choose {selection} from it. Put that card on top of that player's library"
        ));
    }

    if let Some(move_to_library) = downcast_move_to_library_nth(move_effect)
        && matches!(&move_to_library.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        let position = library_top_position_text(&move_to_library.position);
        let reveal_verb = player_verb(&looked_player, "reveal", "reveals");
        return Some(format!(
            "{} {reveal_verb} their hand. You choose {selection} from it. That player puts that card into their library {position}",
            capitalize_first(&looked_player)
        ));
    }

    None
}

pub(crate) fn describe_target_source_power_damage_to_controller(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, damage_effect] = effects else {
        return None;
    };
    let tagged = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let execute = damage_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !choose_spec_references_exact_tag(&execute.source, &tagged.tag) {
        return None;
    }
    let damage = execute
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat
        || !matches!(&damage.amount, Value::PowerOf(spec) if choose_spec_references_exact_tag(spec, &tagged.tag))
    {
        return None;
    }
    let damage_targets_source_controller = match &damage.target {
        ChooseSpec::Player(PlayerFilter::ControllerOf(crate::target::ObjectRef::Target)) => true,
        ChooseSpec::Target(inner) => matches!(
            inner.as_ref(),
            ChooseSpec::Player(PlayerFilter::ControllerOf(crate::target::ObjectRef::Target))
        ),
        ChooseSpec::Player(PlayerFilter::Target(inner)) => matches!(
            inner.as_ref(),
            PlayerFilter::ControllerOf(crate::target::ObjectRef::Target)
        ),
        _ => false,
    };
    if !damage_targets_source_controller {
        return None;
    }
    let source_text = match &target_only.target {
        ChooseSpec::Target(inner) => match inner.as_ref() {
            ChooseSpec::Object(filter)
                if filter.card_types == vec![CardType::Creature]
                    && matches!(
                        filter.controller,
                        Some(PlayerFilter::Opponent | PlayerFilter::NotYou)
                    ) =>
            {
                "target creature an opponent controls".to_string()
            }
            _ => describe_choose_spec(&target_only.target),
        },
        _ => describe_choose_spec(&target_only.target),
    };
    Some(format!(
        "{source_text} deals damage equal to its power to that player"
    ))
}

pub(crate) fn choose_spec_shares_card_type_with_reference(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Object(filter) = spec.base() else {
        return false;
    };
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::SharesCardType
            && (is_implicit_reference_tag(constraint.tag.as_str())
                || constraint.tag.as_str() == "triggering")
    })
}

pub(crate) fn describe_exchange_target_choice(spec: &ChooseSpec) -> String {
    let mut text = describe_choose_spec(spec);
    if choose_spec_shares_card_type_with_reference(spec) {
        text = text.replace(
            "shares a permanent type with that object",
            "shares a card type with it",
        );
        text = text.replace(
            "shares a card type with that object",
            "shares a card type with it",
        );
    }
    text
}

pub(crate) fn describe_target_only_then_exchange_control(effects: &[&Effect]) -> Option<String> {
    let [tag_triggering_effect, target_effect, exchange_effect] = effects else {
        return None;
    };
    let tag_triggering =
        tag_triggering_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;

    let (target_tag, target_only) =
        if let Some(tagged) = target_effect.downcast_ref::<crate::effects::TaggedEffect>() {
            (
                Some(&tagged.tag),
                tagged
                    .effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()?,
            )
        } else {
            (
                None,
                target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?,
            )
        };
    if !target_only.explicit_declaration
        || !target_only.target.is_target()
        || target_only.target.count() != ChoiceCount::exactly(1)
    {
        return None;
    }
    let chooser = target_only.chooser.as_ref()?;
    if !matches!(
        chooser,
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
            if tag == &tag_triggering.tag
    ) {
        return None;
    }

    let ChooseSpec::Object(target_filter) = target_only.target.base() else {
        return None;
    };
    let expected_controller = PlayerFilter::excluding(PlayerFilter::Any, chooser.clone());
    let [shares_constraint] = target_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if target_filter.controller.as_ref() != Some(&expected_controller)
        || shares_constraint.relation != crate::filter::TaggedOpbjectRelation::SharesCardType
        || shares_constraint.tag != tag_triggering.tag
        || !is_permanent_filter_in_zone(target_filter, Zone::Battlefield)
    {
        return None;
    }
    let mut plain_target_filter = target_filter.clone();
    plain_target_filter.controller = None;
    plain_target_filter.tagged_constraints.clear();
    plain_target_filter.card_types.clear();
    if plain_target_filter != ObjectFilter::permanent() {
        return None;
    }

    let exchange = unwrap_render_wrappers(exchange_effect)
        .downcast_ref::<crate::effects::ExchangeControlEffect>()?;
    if exchange.permanent1 != exchange.permanent2 {
        return None;
    }
    let ChooseSpec::WithCount(selected, count) = exchange.permanent1.unhinted() else {
        return None;
    };
    if *count != ChoiceCount::exactly(2) || !selected.is_target() {
        return None;
    }
    let ChooseSpec::Object(selected_filter) = selected.base() else {
        return None;
    };
    let [selected_constraint] = selected_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if selected_constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        || !target_tag.map_or_else(
            || is_implicit_reference_tag(selected_constraint.tag.as_str()),
            |tag| tag == &selected_constraint.tag,
        )
        || !is_permanent_filter_in_zone(selected_filter, Zone::Battlefield)
    {
        return None;
    }
    let mut plain_selected_filter = selected_filter.clone();
    plain_selected_filter.tagged_constraints.clear();
    plain_selected_filter.card_types.clear();
    if plain_selected_filter != ObjectFilter::permanent() {
        return None;
    }

    Some(format!(
        "{} chooses {}. Exchange control of those permanents",
        capitalize_first(&describe_player_filter(chooser)),
        describe_exchange_target_choice(&target_only.target)
    ))
}

pub(crate) fn describe_source_counter_and_create(effects: &[&Effect]) -> Option<String> {
    let [counter_effect, create_effect] = effects else {
        return None;
    };
    let put = counter_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != CounterType::PlusOnePlusOne
        || put.amount != Value::Fixed(1)
        || !matches!(put.target.base(), ChooseSpec::Source)
        || put.target_count.is_some()
        || put.distributed
    {
        return None;
    }
    create_effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;

    let counter_text = describe_effect(counter_effect);
    let create_text = lowercase_first(&describe_effect(create_effect));
    Some(format!(
        "{} and {}",
        counter_text.trim_end_matches('.'),
        create_text.trim_end_matches('.')
    ))
}

pub(crate) fn describe_search_two_split_hand_graveyard(effects: &[&Effect]) -> Option<String> {
    let [
        search_effect,
        choose_effect,
        hand_effect,
        graveyard_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let hand_uses_chosen_tag = if let Some(hand_move) = downcast_move_to_zone(hand_effect) {
        move_to_zone_uses_tag(hand_move, choose.tag.as_str(), Zone::Hand)
    } else if let Some(return_to_hand) =
        hand_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()
    {
        matches!(
            return_to_hand.spec.base(),
            ChooseSpec::Tagged(found) if found.as_str() == choose.tag.as_str()
        )
    } else {
        false
    };
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if !search.is_search
        || choose.is_search
        || search.count.min != 2
        || search.count.max != Some(2)
        || search.count_value.is_some()
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count_value.is_some()
        || search.chooser != choose.chooser
        || shuffle.player != search.chooser
        || choose_search_zones(search)? != vec![Zone::Library]
        || !choose_search_zones(choose)?.contains(&Zone::Library)
        || !filter_is_tagged_as(&choose.filter, search.tag.as_str())
        || !hand_uses_chosen_tag
        || effect_moves_unselected_to_zone(
            graveyard_effect,
            search.tag.as_str(),
            choose.tag.as_str(),
        ) != Some(Zone::Graveyard)
    {
        return None;
    }

    if search.chooser == PlayerFilter::You {
        return Some(
            "Search your library for two cards. Put one into your hand and the other into your graveyard. Then shuffle"
                .to_string(),
        );
    }

    let player = describe_player_filter(&search.chooser);
    let capitalized = capitalize_first(&player);
    let possessive = describe_possessive_player_filter(&search.chooser);
    let shuffle_verb = player_verb(&player, "shuffle", "shuffles");
    Some(format!(
        "{capitalized} searches {possessive} library for two cards. Put one into {possessive} hand and the other into {possessive} graveyard. Then {player} {shuffle_verb}"
    ))
}

pub(crate) fn describe_council_vote_winners_exile(effects: &[&Effect]) -> Option<String> {
    let [vote_effect, exile_effect] = effects else {
        return None;
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::Objects { filter, count } = &vote.choice else {
        return None;
    };
    if count.min != 1
        || count.max != Some(1)
        || !vote.starting_with_controller
        || filter.controller != Some(PlayerFilter::NotYou)
        || !filter.excluded_card_types.contains(&CardType::Land)
    {
        return None;
    }
    let exile = unwrap_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(exile_filter) = &exile.spec else {
        return None;
    };
    if !has_vote_winners_tag(exile_filter) || exile.face_down {
        return None;
    }
    Some(
        "Will of the council — Starting with you, each player votes for a nonland permanent you don't control. Exile each permanent with the most votes or tied for most votes"
            .to_string(),
    )
}

pub(crate) fn describe_choose_player_add_mana(effects: &[&Effect]) -> Option<String> {
    let [choose_effect, add_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    if choose.chooser != PlayerFilter::You || choose.filter != PlayerFilter::Any || choose.random {
        return None;
    }
    let add = add_effect.downcast_ref::<crate::effects::AddManaOfAnyColorEffect>()?;
    if add.amount != Value::Fixed(1)
        || add.available_colors.is_some()
        || !matches!(&add.player, PlayerFilter::TaggedPlayer(tag) if tag == &choose.tag || tag.as_str() == "__it__")
    {
        return None;
    }
    Some("Choose a player. That player adds one mana of any color they choose".to_string())
}

pub(crate) fn tagged_put_counters_view(
    effect: &Effect,
) -> Option<(Option<&TagKey>, &crate::effects::PutCountersEffect)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let put = tagged
            .effect
            .downcast_ref::<crate::effects::PutCountersEffect>()?;
        return Some((Some(&tagged.tag), put));
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_put_counters_view(&with_id.effect);
    }
    effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .map(|put| (None, put))
}

pub(crate) fn goad_view(effect: &Effect) -> Option<&crate::effects::GoadEffect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return goad_view(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return goad_view(&with_id.effect);
    }
    effect.downcast_ref::<crate::effects::GoadEffect>()
}

pub(crate) fn describe_put_counters_then_goad(effects: &[&Effect]) -> Option<String> {
    if let [tag_triggering, sequence] = effects
        && tag_triggering
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
        && let Some(sequence) = sequence.downcast_ref::<crate::effects::SequenceEffect>()
        && sequence.surface == ironsmith_core::SequenceSurface::Coordinated
    {
        let nested = sequence.effects.iter().collect::<Vec<_>>();
        return describe_put_counters_then_goad(&nested);
    }
    if let [sequence] = effects
        && let Some(sequence) = sequence.downcast_ref::<crate::effects::SequenceEffect>()
        && sequence.surface == ironsmith_core::SequenceSurface::Coordinated
    {
        let nested = sequence.effects.iter().collect::<Vec<_>>();
        return describe_put_counters_then_goad(&nested);
    }
    let (put_effect, goad_effect) = match effects {
        [put_effect, goad_effect] => (*put_effect, *goad_effect),
        [tag_triggering, put_effect, goad_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (*put_effect, *goad_effect)
        }
        _ => return None,
    };
    let (put_tag, put) = tagged_put_counters_view(put_effect)?;
    if put.distributed || put.target_count.is_some() {
        return None;
    }
    let goad = goad_view(goad_effect)?;
    let Some(put_tag) = put_tag else {
        return None;
    };
    if !choose_spec_references_exact_tag(&goad.target, put_tag) {
        return None;
    }
    Some(format!(
        "Put {} on {} and goad it",
        describe_put_counter_phrase(&put.amount, put.counter_type),
        describe_choose_spec(&put.target)
    ))
}

pub(crate) fn cant_be_blocked_view(effect: &Effect) -> Option<&crate::effects::CantEffect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return cant_be_blocked_view(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return cant_be_blocked_view(&with_id.effect);
    }
    effect.downcast_ref::<crate::effects::CantEffect>()
}

pub(crate) fn put_counter_target_matches_unblockable_filter(
    put_tag: Option<&TagKey>,
    put: &crate::effects::PutCountersEffect,
    filter: &ObjectFilter,
) -> bool {
    if matches!(put.target.base(), ChooseSpec::Source)
        && object_filters_equivalent_ignoring_source_surface(filter, &ObjectFilter::source())
    {
        return true;
    }
    if let Some(tag) = put_tag
        && filter_is_tagged_as(filter, tag.as_str())
    {
        return true;
    }
    if let ChooseSpec::Tagged(tag) = put.target.base() {
        return filter_is_tagged_as(filter, tag.as_str());
    }
    false
}

pub(crate) fn describe_put_counters_then_unblockable(effects: &[&Effect]) -> Option<String> {
    let (put_effect, target_only_effect, cant_effect) = match effects {
        [tag_triggering, put_effect, target_only_effect, cant_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (*put_effect, Some(*target_only_effect), *cant_effect)
        }
        [tag_triggering, put_effect, cant_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (*put_effect, None, *cant_effect)
        }
        [put_effect, target_only_effect, cant_effect] => {
            (*put_effect, Some(*target_only_effect), *cant_effect)
        }
        _ => return None,
    };
    let (put_tag, put) = tagged_put_counters_view(put_effect)?;
    if put.distributed || put.target_count.is_some() || !put.target.is_single() {
        return None;
    }
    let put_tag = put_tag?;
    let cant = cant_be_blocked_view(cant_effect)?;
    let crate::effect::Restriction::BeBlocked(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::EndOfTurn
        || !put_counter_target_matches_unblockable_filter(Some(put_tag), put, filter)
    {
        return None;
    }
    if let Some(target_only_effect) = target_only_effect {
        let target_only = target_only_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        if !choose_spec_references_exact_tag(&target_only.target, put_tag)
            && !(matches!(put.target.base(), ChooseSpec::Source)
                && object_filters_equivalent_ignoring_source_surface(
                    filter,
                    &ObjectFilter::source(),
                ))
        {
            return None;
        }
    }
    Some(format!(
        "Put {} on {} and it can't be blocked this turn",
        describe_put_counter_phrase(&put.amount, put.counter_type),
        describe_choose_spec(&put.target)
    ))
}

pub(crate) fn tagged_tap_view(effect: &Effect) -> Option<(&TagKey, &crate::effects::TapEffect)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let tap = tagged.effect.downcast_ref::<crate::effects::TapEffect>()?;
        return Some((&tagged.tag, tap));
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_tap_view(&with_id.effect);
    }
    None
}

pub(crate) fn target_defending_player_creature(filter: &ObjectFilter) -> bool {
    filter.controller == Some(PlayerFilter::Defending)
        && filter.card_types == [CardType::Creature]
        && filter.zone == Some(Zone::Battlefield)
}

pub(crate) fn describe_tap_defending_creature_then_goad(effects: &[&Effect]) -> Option<String> {
    let (tap_effect, goad_effect) = match effects {
        [tap_effect, goad_effect] => (*tap_effect, *goad_effect),
        [tag_triggering, tap_effect, goad_effect]
            if tag_triggering
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (*tap_effect, *goad_effect)
        }
        _ => return None,
    };
    let (tap_tag, tap) = tagged_tap_view(tap_effect)?;
    if !tap.target.is_target() {
        return None;
    }
    let ChooseSpec::Object(filter) = tap.target.base() else {
        return None;
    };
    if !target_defending_player_creature(filter) {
        return None;
    }
    let goad = goad_view(goad_effect)?;
    if !choose_spec_references_exact_tag(&goad.target, tap_tag) {
        return None;
    }
    Some("Tap target creature that player controls and goad it".to_string())
}

pub(crate) fn tagged_apply_continuous_view(
    effect: &Effect,
) -> Option<&crate::effects::ApplyContinuousEffect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return tagged_apply_continuous_view(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_apply_continuous_view(&with_id.effect);
    }
    effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
}

pub(crate) fn describe_targeted_haste_then_role(effects: &[&Effect]) -> Option<String> {
    let [haste_effect, role_effect] = effects else {
        return None;
    };
    let apply = tagged_apply_continuous_view(haste_effect)?;
    if !matches!(apply.until, Until::EndOfTurn)
        || apply.condition.is_some()
        || apply.target_spec.is_none()
        || !apply_continuous_adds_static_ability(
            apply,
            crate::static_abilities::StaticAbilityId::Haste,
        )
    {
        return None;
    }
    let target = apply.target_spec.as_ref()?;
    let ChooseSpec::WithCount(inner, count) = target else {
        return None;
    };
    if count.min != 1 || count.max != Some(2) || count.dynamic_x || count.random {
        return None;
    }
    if !matches!(
        inner.as_ref(),
        ChooseSpec::Target(target) if matches!(target.as_ref(), ChooseSpec::Object(filter) if filter.card_types == vec![CardType::Creature])
    ) {
        return None;
    }
    let for_each = role_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let role = describe_for_each_tagged_created_token_attachment(for_each)?;
    Some(format!(
        "{} each gain haste until end of turn. {role}",
        capitalize_first(&describe_choose_spec(target))
    ))
}

pub(crate) fn describe_tag_attached_sacrifice_then_create(effects: &[&Effect]) -> Option<String> {
    let [tag_effect, sacrifice_effect, create_effect] = effects else {
        return None;
    };
    let tag_attached = tag_effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()?;
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    let sacrifices_attached = match &sacrifice.target {
        ChooseSpec::Tagged(tag) => tag == &tag_attached.tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag == tag_attached.tag
            })
        }
        _ => false,
    };
    if !sacrifices_attached {
        return None;
    }
    let create = create_effect.downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::You
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
    Some(format!(
        "enchanted creature's controller sacrifices it and you create {}",
        with_indefinite_article(&describe_create_token_blueprint(create))
    ))
}

pub(crate) fn describe_target_players_each_effects(effects: &[&Effect]) -> Option<String> {
    // Trigger-object tags are execution scaffolding, not part of the authored
    // participant instruction. An enters trigger commonly prefixes the
    // target declaration with this marker in the same resolution segment.
    let participant_effects = effects
        .iter()
        .copied()
        .filter(|effect| {
            unwrap_basic_tag_wrappers(effect)
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_none()
        })
        .collect::<Vec<_>>();
    let (target_only, per_player_effects): (&crate::effects::TargetOnlyEffect, Vec<&Effect>) =
        match participant_effects.as_slice() {
            [target_effect, for_players] => {
                let target_only = unwrap_basic_tag_wrappers(target_effect)
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
                (target_only, vec![*for_players])
            }
            [first_for_players, target_effect, second_for_players] => {
                let target_only = unwrap_basic_tag_wrappers(target_effect)
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
                (target_only, vec![*first_for_players, *second_for_players])
            }
            _ => return None,
        };
    let ChooseSpec::WithCount(target, count) = &target_only.target else {
        return None;
    };
    // This surface is only valid for a synthetic plural target declaration.
    // Preserve authored standalone "Choose ..." clauses and singular choices,
    // while accepting both unbounded ("any number") and bounded plural
    // cardinalities ("two" / "up to two").
    let plural_count = count.max.map_or(!count.dynamic_x, |maximum| maximum >= 2);
    if target_only.explicit_declaration
        || target_only.chooser.is_some()
        || count.dynamic_x
        || count.random
        || !plural_count
    {
        return None;
    }
    let ChooseSpec::Target(target) = target.as_ref() else {
        return None;
    };
    let ChooseSpec::Player(target_filter) = target.as_ref() else {
        return None;
    };

    let excluded_surface = match target_filter {
        PlayerFilter::Excluding { base, excluded }
            if matches!(base.as_ref(), PlayerFilter::Any) =>
        {
            Some(describe_player_filter(excluded))
        }
        PlayerFilter::Any | PlayerFilter::Opponent => None,
        _ => return None,
    };

    let mut clauses = Vec::new();
    for effect in per_player_effects {
        let for_players =
            unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::ForPlayersEffect>()?;
        if for_players.filter != PlayerFilter::Target(Box::new(target_filter.clone())) {
            return None;
        }
        for inner in &for_players.effects {
            let text = describe_effect(inner);
            let lower = text.to_ascii_lowercase();
            let clause = lower
                .strip_prefix("that player ")
                .or_else(|| lower.strip_prefix("target player "))
                .unwrap_or(lower.as_str())
                .to_string();
            clauses.push(
                clause
                    .strip_prefix("discards ")
                    .map(|rest| format!("discard {rest}"))
                    .or_else(|| {
                        clause
                            .strip_prefix("mills ")
                            .map(|rest| format!("mill {rest}"))
                    })
                    .or_else(|| {
                        clause
                            .strip_prefix("loses ")
                            .map(|rest| format!("lose {rest}"))
                    })
                    .or_else(|| {
                        clause
                            .strip_prefix("draws ")
                            .map(|rest| format!("draw {rest}"))
                    })
                    .unwrap_or(clause)
                    .to_string(),
            );
        }
    }
    if clauses.is_empty() {
        return None;
    }
    let joined = match clauses.as_slice() {
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => join_with_and(&clauses),
    };
    let mut subject = capitalize_first(&describe_choose_spec(&target_only.target));
    let mut joined = joined;
    if let Some(excluded_surface) = excluded_surface {
        subject = subject.replace(&excluded_surface, "that player");
        joined = joined.replace(&excluded_surface, "that player");
    }
    Some(format!("{subject} each {joined}"))
}

pub(crate) fn describe_destroy_all_then_target_players_each(effects: &[&Effect]) -> Option<String> {
    let [destroy_effect, target_effect, for_players_effect] = effects else {
        return None;
    };
    let destroy = unwrap_basic_tag_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(filter) = &destroy.spec else {
        return None;
    };
    if filter.zone != Some(Zone::Battlefield) {
        return None;
    }
    let fanout = describe_target_players_each_effects(&[*target_effect, *for_players_effect])?;
    let destroy = describe_effect(destroy_effect);
    Some(format!(
        "{}, then {}",
        destroy.trim_end_matches('.'),
        lowercase_first(&fanout)
    ))
}

pub(crate) fn target_only_two_creatures(effect: &Effect) -> Option<(Vec<&crate::TagKey>, bool)> {
    let mut tags = Vec::new();
    let mut target_only_effect = effect;
    loop {
        if let Some(with_id) = target_only_effect.downcast_ref::<crate::effects::WithIdEffect>() {
            target_only_effect = &with_id.effect;
        } else if let Some(tag_all) =
            target_only_effect.downcast_ref::<crate::effects::TagAllEffect>()
        {
            tags.push(&tag_all.tag);
            target_only_effect = &tag_all.effect;
        } else if let Some(tagged) =
            target_only_effect.downcast_ref::<crate::effects::TaggedEffect>()
        {
            tags.push(&tagged.tag);
            target_only_effect = &tagged.effect;
        } else {
            break;
        }
    }
    let target_only = target_only_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCount(target, count) = &target_only.target else {
        return None;
    };
    if count.min != 2 || count.max != Some(2) || count.dynamic_x || count.random {
        return None;
    }
    let ChooseSpec::Target(inner) = target.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    (filter.zone == Some(Zone::Battlefield) && filter.card_types == vec![CardType::Creature])
        .then_some((tags, filter.distinct_creature_types))
}

pub(crate) fn choose_spec_references_tag_constraint(
    spec: &ChooseSpec,
    tag: &crate::TagKey,
) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag == *tag
            })
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::SurfaceHinted { spec: inner, .. } => {
            choose_spec_references_tag_constraint(inner, tag)
        }
        _ => false,
    }
}

pub(crate) fn describe_two_target_creature_exchange_or_fight(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, action_effect] = effects else {
        return None;
    };
    let (target_tags, distinct_creature_types) = target_only_two_creatures(target_effect)?;
    let action = unwrap_tag_wrappers(action_effect);
    if let Some(exchange) = action.downcast_ref::<crate::effects::ExchangeTextBoxesEffect>() {
        let exchange_targets_creatures = matches!(
            &exchange.target,
            ChooseSpec::Object(filter)
                if filter.zone == Some(Zone::Battlefield)
                    && filter.card_types == vec![CardType::Creature]
        );
        if !target_tags
            .iter()
            .any(|target_tag| choose_spec_references_exact_tag(&exchange.target, target_tag))
            && !exchange_targets_creatures
        {
            return None;
        }
        return Some(
            "Choose two target creatures. For as long as this enchantment remains on the battlefield, exchange the text boxes of those creatures"
                .to_string(),
        );
    }
    if let Some(fight) = action.downcast_ref::<crate::effects::FightEffect>() {
        let same_target_set = target_tags.iter().any(|target_tag| {
            choose_spec_references_tag_constraint(&fight.creature1, target_tag)
                && choose_spec_references_tag_constraint(&fight.creature2, target_tag)
        });
        if !same_target_set {
            return None;
        }
        let target_text = if distinct_creature_types {
            "Choose two target creatures that share no creature types"
        } else {
            "Choose two target creatures"
        };
        return Some(format!("{target_text}. Those creatures fight each other"));
    }
    None
}

pub(crate) fn describe_shape_anew_like_bundle(effects: &[&Effect]) -> Option<String> {
    let [
        sacrifice_effect,
        consult_effect,
        move_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let sacrifice = unwrap_tag_wrappers(sacrifice_effect)
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    let ChooseSpec::Target(target) = &sacrifice.target else {
        return None;
    };
    let ChooseSpec::Object(sac_filter) = target.as_ref() else {
        return None;
    };
    if sac_filter.zone != Some(Zone::Battlefield)
        || sac_filter.card_types != vec![CardType::Artifact]
    {
        return None;
    }
    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let controller_reference_matches = matches!(
        &consult.player,
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
            | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target)
    );
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.stop_rule != crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        || !controller_reference_matches
        || consult.filter.card_types != vec![CardType::Artifact]
    {
        return None;
    }
    let move_to_zone =
        unwrap_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
    {
        return None;
    }
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let shuffle_returns_to_consulting_player = matches!(
        (&consult.player, &shuffle.player),
        (
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
                | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target),
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
                | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target),
        )
    ) || matches!(
        &shuffle.player,
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
            | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag))
            if tag == &consult.match_tag
    );
    if !shuffle_returns_to_consulting_player {
        return None;
    }
    Some(
        "The controller of target artifact sacrifices it, then reveals cards from the top of their library until they reveal an artifact card. That player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library"
            .to_string(),
    )
}

pub(crate) fn describe_reveal_until_land_put_all_graveyard(effects: &[&Effect]) -> Option<String> {
    let (target_effect, consult_effect, move_effect) = match effects {
        [target_effect, consult_effect, move_effect] => {
            (Some(*target_effect), *consult_effect, *move_effect)
        }
        [consult_effect, move_effect] => (None, *consult_effect, *move_effect),
        _ => return None,
    };
    let explicit_player = if let Some(target_effect) = target_effect {
        let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        Some(match &target_only.target {
            ChooseSpec::Target(inner) => match inner.as_ref() {
                ChooseSpec::Player(PlayerFilter::Any) => {
                    PlayerFilter::Target(Box::new(PlayerFilter::Any))
                }
                ChooseSpec::Player(PlayerFilter::Opponent) => {
                    PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
                }
                _ => return None,
            },
            ChooseSpec::Player(PlayerFilter::Any) => {
                PlayerFilter::Target(Box::new(PlayerFilter::Any))
            }
            ChooseSpec::Player(PlayerFilter::Opponent) => {
                PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
            }
            _ => return None,
        })
    } else {
        None
    };

    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
        || consult.filter.card_types != vec![CardType::Land]
    {
        return None;
    }
    let player = explicit_player.unwrap_or_else(|| consult.player.clone());
    let move_to_zone =
        unwrap_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard
        || move_to_zone.to_top
        || !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &consult.all_tag)
    {
        return None;
    }

    let player_text = match player {
        PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Opponent) => {
            "Target opponent"
        }
        PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Any) => {
            "Target player"
        }
        PlayerFilter::Opponent => "Each opponent",
        PlayerFilter::Any => "Each player",
        PlayerFilter::Defending => "Defending player",
        PlayerFilter::IteratedPlayer => "That player",
        _ => return None,
    };
    Some(format!(
        "{player_text} reveals cards from the top of their library until they reveal a land card, then puts those cards into their graveyard"
    ))
}

pub(crate) fn describe_tap_for_mana_player_bonus_and_damage(effects: &[&Effect]) -> Option<String> {
    let [add_effect, damage_effect] = effects else {
        return None;
    };
    let add = add_effect.downcast_ref::<crate::effects::AddManaOfLandProducedTypesEffect>()?;
    if add.amount != Value::Fixed(1)
        || add.player != PlayerFilter::IteratedPlayer
        || add.land_filter.card_types != vec![CardType::Land]
        || !add.allow_colorless
        || add.same_type
        || add.mana_type_source != crate::effects::ManaTypeSource::TriggeringEventProduced
    {
        return None;
    }
    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.amount != Value::Fixed(1)
        || damage.source_is_combat
        || !matches!(
            &damage.target,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
    {
        return None;
    }
    Some(
        "that player adds one mana of any type that land produced, and this enchantment deals 1 damage to the player"
            .to_string(),
    )
}

pub(crate) fn describe_creature_secret_vote_with_default_draw(
    effects: &[&Effect],
) -> Option<String> {
    let [vote_effect, conditional_effect] = effects else {
        return None;
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::Objects { filter, count } = &vote.choice else {
        return None;
    };
    if !vote.secret
        || count.min != 0
        || count.max != Some(1)
        || filter.card_types != vec![CardType::Creature]
    {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::Not(condition) = &conditional.condition else {
        return None;
    };
    let crate::effect::Condition::TaggedObjectMatches(tag, condition_filter) = condition.as_ref()
    else {
        return None;
    };
    if tag.as_str() != crate::effects::VOTED_OBJECTS_TAG
        || condition_filter.card_types != vec![CardType::Creature]
    {
        return None;
    }
    let [draw_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let for_players = draw_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let [draw_inner] = for_players.effects.as_slice() else {
        return None;
    };
    let draw = draw_inner.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.count != Value::Fixed(1) || draw.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let [destroy_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    let destroy = destroy_effect.downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(destroy_filter) = &destroy.spec else {
        return None;
    };
    if destroy_filter.card_types != vec![CardType::Creature]
        || !has_vote_winners_tag(destroy_filter)
    {
        return None;
    }
    Some(
        "Each player secretly votes for up to one creature, then those votes are revealed. If no creature got votes, each player draws a card. Otherwise, destroy each creature with the most votes or tied for most votes"
            .to_string(),
    )
}

pub(crate) fn describe_each_player_choose_creature_destroy_others(
    effects: &[&Effect],
) -> Option<String> {
    let (producer, consumer) = match effects {
        [producer, consumer] => (*producer, *consumer),
        [tag_effect, producer, consumer]
            if tag_effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            (*producer, *consumer)
        }
        _ => return None,
    };
    describe_each_player_choose_creature_then_destroy_others_pair(producer, consumer)
}

pub(crate) fn apply_continuous_for_compaction(
    effect: &Effect,
) -> Option<&crate::effects::ApplyContinuousEffect> {
    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        return Some(apply);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>()
        && is_implicit_reference_tag(tag_all.tag.as_str())
        && let Some(apply) = tag_all
            .effect
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    {
        return Some(apply);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && is_implicit_reference_tag(tagged.tag.as_str())
        && let Some(apply) = tagged
            .effect
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    {
        return Some(apply);
    }
    None
}

pub(crate) fn unwrap_tag_wrappers(effect: &Effect) -> &Effect {
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_tag_wrappers(&tag_all.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_tag_wrappers(&tagged.effect);
    }
    effect
}

pub(crate) fn unwrap_render_wrappers(effect: &Effect) -> &Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_render_wrappers(&with_id.effect);
    }
    unwrap_tag_wrappers(effect)
}

pub(crate) fn describe_dynamic_return_from_graveyard_bundle(
    effects: &[Effect],
    filtered: &[&Effect],
) -> Option<String> {
    if filtered.len() != effects.len() || filtered.len() < 2 {
        return None;
    }
    let choose_effect = filtered[filtered.len() - 2];
    let return_effect = filtered[filtered.len() - 1];
    let choose = unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Graveyard)
        || !choose.count.dynamic_x
        || !choose
            .count_value
            .as_ref()
            .is_some_and(|value| is_effect_count_reference(value, None))
    {
        return None;
    }

    let return_to_battlefield = unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>(
    )?;
    if !matches!(&return_to_battlefield.target, ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }

    let from_text = match &choose.filter.owner {
        Some(owner) => format!(
            "from {} graveyard",
            describe_possessive_player_filter(owner)
        ),
        None => "from a graveyard".to_string(),
    };
    let return_sentence = format!(
        "Return {} {from_text} to the battlefield{}",
        describe_choose_selection(choose),
        if return_to_battlefield.tapped {
            " tapped"
        } else {
            ""
        }
    );
    if effects.len() == 2 {
        return Some(return_sentence);
    }

    let prefix = describe_effect_list(&effects[..effects.len() - 2]);
    Some(format!(
        "{}. {return_sentence}",
        prefix.trim_end_matches('.')
    ))
}

pub(crate) fn describe_exile_graveyard_reflexive_copy_artifact(
    effects: &[&Effect],
) -> Option<String> {
    let (exile_effect, choose_effect, copy_effect) = match effects {
        [exile_effect, choose_or_reflexive_effect, copy_effect] => {
            if let Some(reflexive) =
                choose_or_reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
            {
                let with_id = exile_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
                if reflexive.condition != with_id.id
                    || reflexive.predicate != EffectPredicate::Happened
                {
                    return None;
                }
                let [choose_effect] = reflexive.effects.as_slice() else {
                    return None;
                };
                (*exile_effect, choose_effect, *copy_effect)
            } else {
                (*exile_effect, *choose_or_reflexive_effect, *copy_effect)
            }
        }
        [exile_effect, reflexive_effect] => {
            let with_id = exile_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
            let reflexive =
                reflexive_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()?;
            if reflexive.condition != with_id.id || reflexive.predicate != EffectPredicate::Happened
            {
                return None;
            }
            let [choose_effect, copy_effect] = reflexive.effects.as_slice() else {
                return None;
            };
            (*exile_effect, choose_effect, copy_effect)
        }
        _ => return None,
    };
    let exile =
        unwrap_render_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(exile_filter) = &exile.spec else {
        return None;
    };
    if exile_filter.zone != Some(Zone::Graveyard)
        || exile_filter.owner != Some(PlayerFilter::Opponent)
    {
        return None;
    }

    let unwrapped_choose = unwrap_render_wrappers(choose_effect);
    let target_only =
        if let Some(choose) = unwrapped_choose.downcast_ref::<crate::effects::IfEffect>() {
            if choose.then.len() != 1 || !choose.else_.is_empty() {
                return None;
            }
            unwrap_render_wrappers(&choose.then[0])
                .downcast_ref::<crate::effects::TargetOnlyEffect>()?
        } else {
            unwrapped_choose.downcast_ref::<crate::effects::TargetOnlyEffect>()?
        };
    let ChooseSpec::WithCount(target, count) = &target_only.target else {
        return None;
    };
    if count.min != 0 || count.max != Some(1) {
        return None;
    }
    let ChooseSpec::Target(inner) = target.as_ref() else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = inner.as_ref() else {
        return None;
    };
    if target_filter.zone != Some(Zone::Exile)
        || target_filter.card_types.as_slice() != [CardType::Creature]
    {
        return None;
    }

    let copy = unwrap_render_wrappers(copy_effect)
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    if copy.controller != PlayerFilter::You
        || copy.count != Value::Fixed(1)
        || copy.set_card_types.as_deref() != Some(&[CardType::Artifact])
    {
        return None;
    }

    Some("Exile each opponent's graveyard. When you do, choose up to one target creature card exiled this way. Create a token that's a copy of that card, except it's an artifact and it loses all other card types".to_string())
}

pub(crate) fn is_target_opponent_player_filter(player: &PlayerFilter) -> bool {
    matches!(
        player,
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner)
            if matches!(inner.as_ref(), PlayerFilter::Opponent)
    )
}

pub(crate) fn is_target_opponent_spec(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Target(inner) = spec else {
        return false;
    };
    match inner.as_ref() {
        ChooseSpec::Player(PlayerFilter::Opponent) => true,
        ChooseSpec::Player(player) => is_target_opponent_player_filter(player),
        _ => false,
    }
}

pub(crate) fn is_nonbasic_land_name_filter(filter: &ObjectFilter) -> bool {
    if filter.any_of.len() != 2 {
        return false;
    }
    let excludes_land = filter.any_of.iter().any(|option| {
        option.excluded_card_types == vec![CardType::Land] && option.excluded_supertypes.is_empty()
    });
    let excludes_basic = filter.any_of.iter().any(|option| {
        option.excluded_supertypes == vec![crate::types::Supertype::Basic]
            && option.excluded_card_types.is_empty()
    });
    excludes_land && excludes_basic
}

pub(crate) fn filter_has_same_name_tag(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    })
}

pub(crate) fn for_each_exiles_search_tag(
    for_each: &crate::effects::ForEachTaggedEffect,
    tag: &TagKey,
) -> bool {
    if for_each.tag != *tag || for_each.effects.len() != 1 {
        return false;
    }
    let move_to_zone = unwrap_tag_wrappers(&for_each.effects[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>();
    move_to_zone.is_some_and(|move_to_zone| {
        move_to_zone.zone == Zone::Exile
            && matches!(&move_to_zone.target, ChooseSpec::Tagged(found) if found == tag)
    })
}

pub(crate) fn count_filter_counts_tagged_hand_cards(filter: &ObjectFilter, tag: &TagKey) -> bool {
    filter.zone == Some(Zone::Hand)
        && filter.controller.is_none()
        && filter
            .owner
            .as_ref()
            .is_none_or(is_target_opponent_player_filter)
        && filter.card_types.is_empty()
        && filter.tagged_constraints.len() == 1
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
}

pub(crate) fn render_necromentia_shape(effects: &[&Effect]) -> Option<String> {
    let (choose_name_effect, choose_effect, for_each_effect, shuffle_effect, create_effect) =
        match effects {
            [
                choose_name_effect,
                target_only_effect,
                choose_effect,
                for_each_effect,
                shuffle_effect,
                create_effect,
            ] => {
                let target_only =
                    target_only_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
                if !is_target_opponent_spec(&target_only.target) {
                    return None;
                }
                (
                    *choose_name_effect,
                    *choose_effect,
                    *for_each_effect,
                    *shuffle_effect,
                    *create_effect,
                )
            }
            [
                choose_name_effect,
                choose_effect,
                for_each_effect,
                shuffle_effect,
                create_effect,
            ] => (
                *choose_name_effect,
                *choose_effect,
                *for_each_effect,
                *shuffle_effect,
                *create_effect,
            ),
            _ => return None,
        };
    let choose_name = structural_unwrap_render_wrappers(choose_name_effect)
        .downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    if choose_name.chooser != PlayerFilter::You
        || !choose_name
            .filter
            .as_ref()
            .is_some_and(is_nonbasic_land_name_filter)
    {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let zones = choose_search_zones(choose)?;
    if !choose.is_search
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose.chooser != PlayerFilter::You
        || choose
            .filter
            .owner
            .as_ref()
            .is_none_or(|owner| !is_target_opponent_player_filter(owner))
        || !filter_has_same_name_tag(&choose.filter, &choose_name.tag)
        || !(zones.contains(&Zone::Graveyard)
            && zones.contains(&Zone::Hand)
            && zones.contains(&Zone::Library))
    {
        return None;
    }

    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if !for_each_exiles_search_tag(for_each, &choose.tag) {
        return None;
    }

    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !is_target_opponent_player_filter(&shuffle.player) {
        return None;
    }

    let create =
        unwrap_tag_wrappers(create_effect).downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if !is_target_opponent_player_filter(&create.controller)
        || create.token.card.name != "Zombie"
        || !create.token.card.card_types.contains(&CardType::Creature)
        || !create.token.card.subtypes.contains(&Subtype::Zombie)
    {
        return None;
    }
    let Value::Count(count_filter) = create.count.unhinted() else {
        return None;
    };
    if !count_filter_counts_tagged_hand_cards(count_filter, &choose.tag) {
        return None;
    }

    Some("Choose a card name other than a basic land card name. Search target opponent's graveyard, hand, and library for any number of cards with that name and exile them. That player shuffles, then creates a 2/2 black Zombie creature token for each card exiled from their hand this way".to_string())
}

pub(crate) fn same_name_extraction_selection(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    if choose.count.min != 0 || choose.count.dynamic_x || choose.count.random {
        return None;
    }
    if let Some(max) = choose.count.max {
        if max == 0 {
            return None;
        }
        let max = number_word(max as i32).unwrap_or_else(|| max.to_string());
        return Some(format!("up to {max} cards"));
    }
    match choose.search_mode {
        crate::effect::SearchSelectionMode::Optional => Some("any number of cards".to_string()),
        crate::effect::SearchSelectionMode::AllMatching
        | crate::effect::SearchSelectionMode::Exact => Some("all cards".to_string()),
    }
}

fn same_name_extraction_player(left: &PlayerFilter, right: &PlayerFilter) -> bool {
    same_search_player_filter(left, right)
        || matches!(left, PlayerFilter::IteratedPlayer)
        || matches!(right, PlayerFilter::IteratedPlayer)
}

/// Whether `player` is the controller of the object produced under `tag`.
///
/// Parser lowering can retain the authored target reference or replace it with
/// the producer's exact tag, and can mark either form as an anaphoric alias.
/// Keep all four representation variants equivalent without accepting a
/// controller reference to an unrelated tagged object.
pub(crate) fn player_is_controller_of_produced_target(player: &PlayerFilter, tag: &TagKey) -> bool {
    matches!(
        player,
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Target)
            | PlayerFilter::AliasedControllerOf(crate::target::ObjectRef::Target)
    ) || matches!(
        player,
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(found))
            | PlayerFilter::AliasedControllerOf(crate::target::ObjectRef::Tagged(found))
            if found == tag
    )
}

pub(crate) fn same_name_extraction_hand_draw_matches(
    draw_effect: &Effect,
    searched_tag: &TagKey,
    searched_player: &PlayerFilter,
) -> bool {
    let Some(draw) = unwrap_basic_tag_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawForEachTaggedMatchingEffect>()
    else {
        return false;
    };
    let mut counted = draw.filter.clone();
    let owner = counted.owner.take();
    counted.zone = None;
    counted.controller = None;
    draw.tag == *searched_tag
        && draw.filter.zone == Some(Zone::Hand)
        && counted == ObjectFilter::default()
        && owner
            .as_ref()
            .is_some_and(|owner| same_name_extraction_player(owner, searched_player))
        && same_name_extraction_player(&draw.player, searched_player)
}

pub(crate) fn render_choose_name_search_same_name_exile_shuffle(
    effects: &[&Effect],
) -> Option<String> {
    let (core, draw_effect) = match effects.len() {
        5 => (effects, None),
        6 => (&effects[..5], Some(effects[5])),
        _ => return None,
    };
    let [
        choose_name_effect,
        target_only_effect,
        choose_effect,
        for_each_effect,
        shuffle_effect,
    ] = core
    else {
        return None;
    };

    let choose_name = structural_unwrap_render_wrappers(choose_name_effect)
        .downcast_ref::<crate::effects::ChooseCardNameEffect>()?;
    if choose_name.chooser != PlayerFilter::You {
        return None;
    }

    let target_only = structural_unwrap_render_wrappers(target_only_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let searched_player = choose_spec_player_filter(&target_only.target)?;

    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let zones = choose_search_zones(choose)?;
    if !choose.is_search
        || choose.chooser != PlayerFilter::You
        || !filter_has_same_name_tag(&choose.filter, &choose_name.tag)
        || !(zones.contains(&Zone::Graveyard)
            && zones.contains(&Zone::Hand)
            && zones.contains(&Zone::Library))
    {
        return None;
    }

    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if !for_each_exiles_search_tag(for_each, &choose.tag) {
        return None;
    }

    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let search_owner = choose.filter.owner.as_ref()?;
    if !same_name_extraction_player(search_owner, &searched_player)
        || !same_name_extraction_player(&shuffle.player, search_owner)
    {
        return None;
    }

    let choose_line = describe_choose_card_name_filter(choose_name.filter.as_ref());
    let search_origin = describe_search_origin_zones(choose)?;
    let count_text = same_name_extraction_selection(choose)?;
    if let Some(draw_effect) = draw_effect {
        if !same_name_extraction_hand_draw_matches(draw_effect, &choose.tag, search_owner) {
            return None;
        }
        Some(format!(
            "{choose_line}. Search {search_origin} for {count_text} with that name and exile them. That player shuffles, then draws a card for each card exiled from their hand this way"
        ))
    } else {
        Some(format!(
            "{choose_line}. Search {search_origin} for {count_text} with that name and exile them. Then that player shuffles"
        ))
    }
}

pub(crate) fn describe_choose_card_name_filter(filter: Option<&ObjectFilter>) -> String {
    let Some(filter) = filter else {
        return "Choose a card name".to_string();
    };
    if choose_card_name_excludes_only_basic_lands(filter) {
        return "Choose a card name other than a basic land card name".to_string();
    }
    let name_kind = if filter.excluded_card_types.contains(&CardType::Artifact)
        && filter.excluded_card_types.contains(&CardType::Land)
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
    {
        "nonartifact, nonland card"
    } else if filter.excluded_card_types.contains(&CardType::Land)
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
    {
        "nonland card"
    } else if filter.card_types == vec![CardType::Artifact] {
        "artifact card"
    } else if filter.card_types == vec![CardType::Creature] {
        "creature card"
    } else {
        "card"
    };
    format!("Choose {} name", with_indefinite_article(name_kind))
}

pub(crate) fn render_reveal_hand_choose_same_name_exile_shuffle(
    effects: &[&Effect],
) -> Option<String> {
    let [
        look_effect,
        choose_effect,
        search_effect,
        for_each_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (reveal, selection, revealed_player) = describe_reveal_hand_choose_from_it(look, choose)?;

    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let search_owner = search.filter.owner.as_ref()?;
    if !search.is_search
        || search.chooser != PlayerFilter::You
        || choose_search_zones(search)? != [Zone::Graveyard, Zone::Hand, Zone::Library]
        || !filter_has_same_name_tag(&search.filter, &choose.tag)
        || !same_name_extraction_player(search_owner, &revealed_player)
    {
        return None;
    }
    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if !for_each_exiles_search_tag(for_each, &search.tag) {
        return None;
    }
    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !same_name_extraction_player(&shuffle.player, search_owner) {
        return None;
    }

    let search_origin = "that player's graveyard, hand, and library";
    let count_text = same_name_extraction_selection(search)?;
    let selection_from_it = card_choice_from_it_text(&selection);
    Some(format!(
        "{reveal}. You choose {selection_from_it}. Search {search_origin} for {count_text} with the same name as that card and exile them. Then that player shuffles"
    ))
}

/// Render the plural reveal/choose/exile form of the three-zone same-name
/// extraction family. The nested `ForEachTaggedEffect` is significant: the
/// search is repeated once for every card selected from the revealed hand,
/// rather than being one search keyed to a single chosen card.
pub(crate) fn render_reveal_hand_choose_exile_each_same_name_shuffle(
    effects: &[&Effect],
) -> Option<String> {
    let [
        look_effect,
        choose_effect,
        exile_effect,
        each_effect,
        shuffle_effect,
    ] = effects
    else {
        return None;
    };

    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !look.reveal
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.count.min != 0
        || choose.count.max.is_some()
        || !choose.count.dynamic_x
        || !choose.count.up_to_x
        || choose.count.random
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
    {
        return None;
    }

    let revealed_player = choose_spec_player_filter(&look.target)?;
    if !choose
        .filter
        .owner
        .as_ref()
        .is_some_and(|owner| player_filters_refer_to_same_player(owner, &revealed_player))
    {
        return None;
    }

    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile.zone != Zone::Exile
        || !matches!(&exile.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let each = structural_unwrap_render_wrappers(each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if each.tag != choose.tag && each.tag.as_str() != crate::tag::SOURCE_EXILED_TAG {
        return None;
    }
    let [inner_effect] = each.effects.as_slice() else {
        return None;
    };
    let inner = if let Some(sequence) = structural_unwrap_render_wrappers(inner_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence
    } else {
        let conditional = structural_unwrap_render_wrappers(inner_effect)
            .downcast_ref::<crate::effects::ConditionalEffect>()?;
        let Condition::TaggedObjectMatchedLastKnown(iterated_tag, card_filter) =
            &conditional.condition
        else {
            return None;
        };
        let mut bare_card_filter = card_filter.clone();
        bare_card_filter.set_explicit_card_noun(false);
        if iterated_tag.as_str() != "__it__"
            || !card_filter.has_explicit_card_noun()
            || bare_card_filter != ObjectFilter::default()
            || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
            || !conditional.if_false.is_empty()
        {
            return None;
        }
        let [guarded_effect] = conditional.if_true.as_slice() else {
            return None;
        };
        structural_unwrap_render_wrappers(guarded_effect)
            .downcast_ref::<crate::effects::SequenceEffect>()?
    };
    let [search_effect, exile_matches_effect] = inner.effects.as_slice() else {
        return None;
    };

    let search = structural_unwrap_render_wrappers(search_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let iterated_tag = TagKey::from("__it__");
    let search_owner = search.filter.owner.as_ref()?;
    if !search.is_search
        || search.chooser != PlayerFilter::You
        || choose_search_zones(search)? != [Zone::Graveyard, Zone::Hand, Zone::Library]
        || !filter_has_same_name_tag(&search.filter, &iterated_tag)
        || !same_name_extraction_player(search_owner, &revealed_player)
    {
        return None;
    }
    let exile_matches = structural_unwrap_render_wrappers(exile_matches_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if !for_each_exiles_search_tag(exile_matches, &search.tag) {
        return None;
    }

    let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !same_name_extraction_player(&shuffle.player, search_owner)
        || !same_name_extraction_player(&shuffle.player, &revealed_player)
    {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let reveal = format!("{} {} their hand", capitalize_first(&revealer), reveal_verb);
    let choice = hand_choice_selection_from_it(choose);
    let selection = format!(
        "up to X {}",
        pluralize_relative_object_phrase(strip_indefinite_article(&choice))
    );
    describe_search_origin_zones(search)?;
    let search_origin = "that player's graveyard, hand, and library";
    let count_text = same_name_extraction_selection(search)?;
    Some(format!(
        "{reveal}. You choose {selection} from it and exile them. For each card exiled this way, search {search_origin} for {count_text} with the same name as that card and exile them. Then that player shuffles"
    ))
}

pub(crate) fn render_optional_draw_then_sylvan_card_choice(effects: &[&Effect]) -> Option<String> {
    let [may_draw_effect, if_effect, for_each_effect] = effects else {
        return None;
    };
    let with_id = may_draw_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You) || may.effects.len() != 1 {
        return None;
    }
    let draw = may.effects[0].downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You || draw.count.unhinted() != &Value::Fixed(2) {
        return None;
    }

    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 1
    {
        return None;
    }
    let choose = if_effect.then[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.count.min != 2
        || choose.count.max != Some(2)
        || choose.filter.zone != Some(Zone::Hand)
        || choose.filter.owner != Some(PlayerFilter::You)
        || !choose.filter.drawn_this_turn
    {
        return None;
    }

    let for_each = for_each_effect
        .downcast_ref::<crate::effects::ForEachObject>()
        .or_else(|| {
            for_each_effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .and_then(|tagged| {
                    tagged
                        .effect
                        .downcast_ref::<crate::effects::ForEachObject>()
                })
        })?;
    if for_each.effects.len() != 1
        || !for_each.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == choose.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }
    let unless = for_each.effects[0].downcast_ref::<crate::effects::UnlessActionEffect>()?;
    if unless.player != PlayerFilter::You
        || unless.effects.len() != 1
        || unless.alternative.len() != 1
    {
        return None;
    }
    let (move_to_zone, lose_life) = if let (Some(move_to_zone), Some(lose_life)) = (
        unless.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>(),
        unless.alternative[0].downcast_ref::<crate::effects::LoseLifeEffect>(),
    ) {
        (move_to_zone, lose_life)
    } else {
        (
            unless.alternative[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?,
            unless.effects[0].downcast_ref::<crate::effects::LoseLifeEffect>()?,
        )
    };
    let moves_chosen_card = match move_to_zone.target.unhinted() {
        ChooseSpec::Tagged(tag) => tag == &choose.tag,
        ChooseSpec::Iterated => true,
        _ => false,
    };
    if move_to_zone.zone != Zone::Library || !move_to_zone.to_top || !moves_chosen_card {
        return None;
    }
    if lose_life.amount != Value::Fixed(4)
        || !matches!(lose_life.player, ChooseSpec::Player(PlayerFilter::You))
    {
        return None;
    }

    Some("You may draw two additional cards. If you do, choose two cards in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top of your library".to_string())
}

pub(crate) fn combat_damage_prevention_source(effect: &Effect) -> Option<(&ChooseSpec, &Until)> {
    let effect = unwrap_tag_wrappers(effect);
    if let Some(prevent_from) =
        effect.downcast_ref::<crate::effects::PreventAllCombatDamageFromEffect>()
    {
        return Some((&prevent_from.source, &prevent_from.until));
    }
    let prevent_combat = effect.downcast_ref::<crate::effects::PreventAllCombatDamageEffect>()?;
    match &prevent_combat.target {
        crate::effects::CombatDamagePreventionTarget::From(source) => {
            Some((source, &prevent_combat.until))
        }
        _ => None,
    }
}

pub(crate) fn describe_choose_then_color_matched_combat_prevention(
    choose: &crate::effects::ChooseObjectsEffect,
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if choose.is_search || !choose.count.is_single() || !conditional.if_false.is_empty() {
        return None;
    }
    let prevent_effect = match conditional.if_true.as_slice() {
        [prevent_effect] => prevent_effect,
        [target_only, prevent_effect] => {
            target_only.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            prevent_effect
        }
        _ => return None,
    };
    let (source, until) = combat_damage_prevention_source(prevent_effect)?;
    if !matches!(until, Until::EndOfTurn) {
        return None;
    }

    let crate::ConditionExpr::TargetMatches(filter) = &conditional.condition else {
        return None;
    };
    if !filter
        .card_types
        .contains(&crate::types::CardType::Creature)
    {
        return None;
    }
    let shares_chosen_color = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == choose.tag
            && constraint.relation == crate::target::TaggedOpbjectRelation::SharesColorWithTagged
    });
    if !shares_chosen_color {
        return None;
    }

    let target_text = describe_choose_spec(source);
    if !target_text.contains("target creature") {
        return None;
    }
    let chosen = describe_choose_selection(choose);
    let chosen_noun = if chosen.contains("permanent") {
        "permanent"
    } else {
        choose_reference_noun(choose)
    };
    Some(format!(
        "Choose {}. Prevent all combat damage {target_text} would deal this turn if it shares a color with that {}",
        chosen, chosen_noun
    ))
}

pub(crate) fn describe_sacrifice_source_then_return_with_counters(
    filtered: &[&Effect],
) -> Option<String> {
    let [sacrifice_effect, move_effect, put_counter_effect] = filtered else {
        return None;
    };
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target, ChooseSpec::Source) {
        return None;
    }
    let move_tag = wrapped_effect_tag(move_effect)?;
    let move_to_zone =
        unwrap_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
        || move_to_zone.enters_tapped
    {
        return None;
    }
    let put_counters = put_counter_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put_counters.distributed
        || put_counters.target_count.is_some()
        || !matches!(&put_counters.target, ChooseSpec::Tagged(tag) if tag == move_tag)
    {
        return None;
    }
    let target_filter = match move_to_zone.target.base() {
        ChooseSpec::Object(filter) => filter,
        ChooseSpec::WithCount(inner, count) if count.is_single() => {
            let ChooseSpec::Object(filter) = inner.base() else {
                return None;
            };
            filter
        }
        _ => return None,
    };
    if !target_filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }

    let mut display_filter = target_filter.clone();
    display_filter.zone = None;
    display_filter.tagged_constraints.clear();
    let target_text = format!(
        "{} exiled with it",
        with_indefinite_article(&describe_nonbattlefield_card_filter_without_zone(
            &display_filter,
            Zone::Exile,
        ))
    );
    let counter_type = describe_counter_type(put_counters.counter_type);
    let counter_suffix = match &put_counters.amount {
        Value::Fixed(1) => format!("an additional {counter_type} counter"),
        Value::Fixed(amount) => {
            let count_text = number_word(*amount).unwrap_or_else(|| amount.to_string());
            format!("{count_text} additional {counter_type} counters")
        }
        _ => return None,
    };
    Some(format!(
        "Sacrifice this enchantment, then put {target_text} onto the battlefield under your control with {counter_suffix} on it"
    ))
}

pub(crate) fn sticker_phrase(action: crate::events::KeywordActionKind) -> &'static str {
    match action {
        crate::events::KeywordActionKind::Sticker => "a sticker",
        crate::events::KeywordActionKind::NameSticker => "a name sticker",
        crate::events::KeywordActionKind::ArtSticker => "an art sticker",
        crate::events::KeywordActionKind::AbilitySticker => "an ability sticker",
        crate::events::KeywordActionKind::PowerToughnessSticker => "a power and toughness sticker",
        _ => "a sticker",
    }
}

pub(crate) fn describe_choose_then_put_sticker(
    choose: &crate::effects::ChooseObjectsEffect,
    put_sticker: &crate::effects::PutStickerEffect,
) -> Option<String> {
    if choose.is_search {
        return None;
    }
    if !matches!(&put_sticker.target, ChooseSpec::Tagged(tag) if tag == &choose.tag) {
        return None;
    }
    Some(format!(
        "Put {} on {}",
        sticker_phrase(put_sticker.action),
        describe_choose_selection(choose)
    ))
}

pub(crate) fn describe_target_only_then_create_token_count(
    target_only: &crate::effects::TargetOnlyEffect,
    create_token: &crate::effects::CreateTokenEffect,
) -> Option<String> {
    if create_token.exile_at_end_of_combat
        || create_token.sacrifice_at_end_of_combat
        || create_token.sacrifice_at_next_end_step
        || create_token.exile_at_next_end_step
    {
        return None;
    }
    let Value::Count(filter) = create_token.count.unhinted() else {
        return None;
    };
    if !matches!(create_token.controller, PlayerFilter::You) {
        return None;
    }

    let choose_text = format!("Choose {}", describe_choose_spec(&target_only.target));
    let token_blueprint = describe_create_token_blueprint(create_token);
    let token_phrase = pluralize_token_phrase(&token_blueprint);
    let target_count = target_only.target.count();
    let plural_target = target_count.max.is_none_or(|max| max > 1) || target_count.dynamic_x;
    let target_reference = if plural_target {
        if matches!(
            target_only.target.base(),
            ChooseSpec::Player(PlayerFilter::Opponent)
        ) {
            "those opponents"
        } else {
            "those players"
        }
    } else {
        "that player"
    };
    let control_reference = if plural_target {
        format!("{target_reference} control")
    } else {
        format!("{target_reference} controls")
    };
    let owner_reference = if plural_target {
        format!("{target_reference} own")
    } else {
        format!("{target_reference} owns")
    };
    let mut count_desc = pluralize_noun_phrase(strip_indefinite_article(
        &describe_for_each_count_filter(filter),
    ));
    count_desc = count_desc
        .replace("target opponent controls", &control_reference)
        .replace("target player controls", &control_reference)
        .replace("they control", &control_reference)
        .replace("target opponent owns", &owner_reference)
        .replace("target player owns", &owner_reference)
        .replace("they own", &owner_reference);
    let count_desc = count_desc.trim();
    if count_desc.is_empty() {
        return None;
    }

    Some(format!(
        "{choose_text}. Create X {token_phrase}, where X is the number of {count_desc}"
    ))
}

pub(crate) fn describe_target_only_then_damage_that_player(
    target_only: &crate::effects::TargetOnlyEffect,
    damage: &crate::effects::DealDamageEffect,
) -> Option<String> {
    let ChooseSpec::Target(target_inner) = &target_only.target else {
        return None;
    };
    let ChooseSpec::Player(chosen_filter) = target_inner.as_ref() else {
        return None;
    };
    let ChooseSpec::Player(PlayerFilter::Target(damage_filter)) = &damage.target else {
        return None;
    };
    if chosen_filter != damage_filter.as_ref() {
        return None;
    }

    let damage_text = describe_effect_impl(&Effect::new(damage.clone()));
    let damage_target = describe_choose_spec(&damage.target);
    if !damage_text.contains(&damage_target) {
        return None;
    }
    Some(format!(
        "Choose {}. {}",
        describe_choose_spec(&target_only.target),
        damage_text.replace(&damage_target, "that player")
    ))
}

pub(crate) fn downcast_exile(effect: &Effect) -> Option<&crate::effects::ExileEffect> {
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return Some(exile);
    }
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()?
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()
}

pub(crate) fn downcast_move_to_zone(effect: &Effect) -> Option<&crate::effects::MoveToZoneEffect> {
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

pub(crate) fn basic_land_exception_graveyard_owner(filter: &ObjectFilter) -> Option<PlayerFilter> {
    let [first, second] = filter.any_of.as_slice() else {
        return None;
    };
    let branch_owner = |branch: &ObjectFilter| -> Option<PlayerFilter> {
        if branch.zone != Some(Zone::Graveyard)
            || !branch.any_of.is_empty()
            || !branch.card_types.is_empty()
            || !branch.all_card_types.is_empty()
            || !branch.subtypes.is_empty()
            || !branch.supertypes.is_empty()
        {
            return None;
        }
        branch.owner.clone()
    };
    let first_owner = branch_owner(first)?;
    let second_owner = branch_owner(second)?;
    if first_owner != second_owner {
        return None;
    }

    let first_excludes_land =
        first.excluded_card_types == [CardType::Land] && first.excluded_supertypes.is_empty();
    let second_excludes_land =
        second.excluded_card_types == [CardType::Land] && second.excluded_supertypes.is_empty();
    let first_excludes_basic =
        first.excluded_card_types.is_empty() && first.excluded_supertypes == [Supertype::Basic];
    let second_excludes_basic =
        second.excluded_card_types.is_empty() && second.excluded_supertypes == [Supertype::Basic];
    if !((first_excludes_land && second_excludes_basic)
        || (first_excludes_basic && second_excludes_land))
    {
        return None;
    }

    Some(first_owner)
}

/// Render the reusable "choose a graveyard card set / use the total mana
/// value of those cards / return that same set" sequence without collapsing
/// the tagged collection to a singular pronoun.
pub(crate) fn describe_targeted_card_set_total_mana_value_then_return(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, damage_effect, move_effect] = effects else {
        return None;
    };

    let targeted = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = targeted
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::WithCount(target, count) = target_only.target.unhinted() else {
        return None;
    };
    if count.min != 0 || count.max.is_some() {
        return None;
    }
    let ChooseSpec::Target(target) = target.unhinted() else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = target.unhinted() else {
        return None;
    };
    if target_filter.zone != Some(Zone::Graveyard) || !target_filter.has_explicit_card_noun() {
        return None;
    }

    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    let Value::TotalManaValue(value_filter) = damage.amount.unhinted() else {
        return None;
    };
    if damage.target.unhinted() != &ChooseSpec::SourceController
        || !value_filter.has_explicit_card_noun()
        || !filter_references_exact_tag(value_filter, &targeted.tag)
    {
        return None;
    }

    let move_to_battlefield = downcast_move_to_zone(move_effect)?;
    if move_to_battlefield.zone != Zone::Battlefield
        || move_to_battlefield.battlefield_controller != crate::effects::BattlefieldController::You
        || !matches!(
            move_to_battlefield.target.unhinted(),
            ChooseSpec::Tagged(tag) if tag == &targeted.tag
        )
    {
        return None;
    }

    let choose = capitalize_first(describe_effect(target_effect).trim().trim_end_matches('.'))
        .replace(" in a graveyard", " in graveyards");
    let damage = capitalize_first(describe_effect(damage_effect).trim().trim_end_matches('.'));
    Some(format!(
        "{choose}. {damage}. Put them onto the battlefield under your control"
    ))
}

/// Preserve the singular object selected from an event-derived card set when
/// a successful optional exile grants a cast permission for that same tag.
pub(crate) fn describe_may_exile_one_from_triggered_set_then_cast(
    effects: &[Effect],
) -> Option<String> {
    let effects = if let [first, rest @ ..] = effects
        && first
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
    {
        rest
    } else {
        effects
    };
    let [may_effect, conditional_effect] = effects else {
        return None;
    };
    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You) || may.effects.len() != 1 {
        return None;
    }
    let move_to_exile = structural_unwrap_render_wrappers(&may.effects[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::WithCount(target, count) = move_to_exile.target.unhinted() else {
        return None;
    };
    let ChooseSpec::Object(filter) = target.unhinted() else {
        return None;
    };
    if move_to_exile.zone != Zone::Exile
        || count.min != 1
        || count.max != Some(1)
        || filter.zone != Some(Zone::Graveyard)
        || filter.owner != Some(PlayerFilter::You)
        || !filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "triggering"
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != with_id.id
        || conditional.predicate != crate::effect::EffectPredicate::Happened
        || !conditional.else_.is_empty()
        || conditional.then.len() != 1
    {
        return None;
    }
    let moved_tag = wrapped_effect_tag(&may.effects[0])
        .cloned()
        // Non-targeted direct exile actions publish their result through the
        // canonical source-exiled collection instead of adding a redundant
        // tag wrapper.
        .unwrap_or_else(|| TagKey::from(crate::tag::SOURCE_EXILED_TAG));
    let grant = structural_unwrap_render_wrappers(&conditional.then[0])
        .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    if grant.tag != moved_tag
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
        || grant.allow_land
        || grant.allow_any_color_for_cast
        || grant.while_on_top_of_library
        || grant.filter.is_some()
    {
        return None;
    }

    Some(
        "You may exile one of them from your graveyard. If you do, you may cast that card this turn"
            .to_string(),
    )
}

pub(crate) fn describes_same_name_as_iterated(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
            && constraint.tag.as_str() == "__it__"
    })
}

pub(crate) fn describe_exile_graveyard_then_same_name_library_exile_bundle(
    filtered: &[&Effect],
) -> Option<String> {
    let [exile_effect, for_each_search_effect, trailing @ ..] = filtered else {
        return None;
    };

    let tagged_exile = exile_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = tagged_exile
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(exile_filter) = &exile.spec else {
        return None;
    };
    let graveyard_owner = basic_land_exception_graveyard_owner(exile_filter)?;

    let for_each_effects = if let Some(for_each) =
        for_each_search_effect.downcast_ref::<crate::effects::ForEachObject>()
    {
        let iterates_prior_exile = for_each.filter.zone == Some(Zone::Exile)
            && (filter_is_tagged_as(&for_each.filter, tagged_exile.tag.as_str())
                || filter_is_tagged_as(&for_each.filter, crate::tag::SOURCE_EXILED_TAG)
                || for_each.filter.tagged_constraints.is_empty());
        if !iterates_prior_exile {
            return None;
        }
        for_each.effects.as_slice()
    } else if let Some(for_each) =
        for_each_search_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        if for_each.tag != tagged_exile.tag
            && for_each.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
        {
            return None;
        }
        for_each.effects.as_slice()
    } else {
        return None;
    };

    let for_each_effects = match for_each_effects {
        [sequence_effect] => sequence_effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .map_or(for_each_effects, |sequence| sequence.effects.as_slice()),
        _ => for_each_effects,
    };
    let (search_effect, for_each_move_effect, shuffle_effect) = match (for_each_effects, trailing) {
        ([search_effect], [for_each_move_effect, shuffle_effect]) => {
            (search_effect, *for_each_move_effect, *shuffle_effect)
        }
        ([search_effect, for_each_move_effect], [shuffle_effect]) => {
            (search_effect, for_each_move_effect, *shuffle_effect)
        }
        _ => return None,
    };
    let search = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !search.is_search
        || choose_search_zones(search)? != vec![Zone::Library]
        || search.chooser != PlayerFilter::You
        || search.filter.owner.as_ref() != Some(&graveyard_owner)
        || !describes_same_name_as_iterated(&search.filter)
        || search.count.min != 0
        || search.count.max.is_some()
        || search.search_mode != SearchSelectionMode::AllMatching
    {
        return None;
    }

    let for_each_tagged =
        for_each_move_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each_tagged.tag != search.tag {
        return None;
    }
    let [move_effect] = for_each_tagged.effects.as_slice() else {
        return None;
    };
    let exiles_searched = downcast_move_to_zone(move_effect).is_some_and(|move_to_exile| {
        move_to_zone_uses_tag(move_to_exile, search.tag.as_str(), Zone::Exile)
            || (move_to_exile.zone == Zone::Exile
                && matches!(move_to_exile.target.base(), ChooseSpec::Iterated))
    }) || downcast_exile(move_effect)
        .is_some_and(|exile| exile_uses_tag(exile, search.tag.as_str()));
    if !exiles_searched {
        return None;
    }

    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != graveyard_owner {
        return None;
    }

    let graveyard = format!(
        "{} graveyard",
        describe_possessive_player_filter(&graveyard_owner)
    );
    let followup_possessive = if matches!(graveyard_owner, PlayerFilter::Target(_)) {
        "that player's".to_string()
    } else {
        describe_possessive_player_filter(&graveyard_owner)
    };
    Some(format!(
        "Exile all cards from {graveyard} other than basic land cards. For each card exiled this way, search {followup_possessive} library for all cards with the same name as that card and exile them. Then that player shuffles"
    ))
}

pub(crate) fn downcast_create_token(effect: &Effect) -> Option<&crate::effects::CreateTokenEffect> {
    unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::CreateTokenEffect>()
}

pub(crate) fn downcast_set_base_power_toughness(
    effect: &Effect,
) -> Option<&crate::effects::SetBasePowerToughnessEffect> {
    unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()
}

pub(crate) fn wrapped_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return wrapped_effect_tag(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    effect
        .downcast_ref::<crate::effects::TagAllEffect>()
        .map(|tag_all| &tag_all.tag)
}

/// Recombine an amass action with damage dealt by the exact Army produced or
/// enlarged by that action. Runtime lowering uses a tag plus an
/// ExecuteWithSource fanout so the damage source and its live power are
/// correct; Oracle expresses that provenance as "the Army you amassed" and
/// then uses "its power".
pub(in crate::compiled_text) fn describe_amass_then_amassed_army_power_damage(
    producer_effect: &Effect,
    for_each_effect: &Effect,
) -> Option<String> {
    let producer_tag = wrapped_effect_tag(producer_effect)?;
    let producer = unwrap_tag_wrappers(producer_effect);
    let amass = producer.downcast_ref::<crate::effects::AmassEffect>()?;

    let consumer = unwrap_tag_wrappers(for_each_effect);
    let (recipient_filter, damage, source) =
        if let Some(for_each) = consumer.downcast_ref::<crate::effects::ForEachObject>() {
            let [inner] = for_each.effects.as_slice() else {
                return None;
            };
            let (source, damage) = damage_with_source_view(inner)?;
            if !matches!(damage.target.unhinted(), ChooseSpec::Iterated) {
                return None;
            }
            (&for_each.filter, damage, source?)
        } else {
            let with_source = consumer.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
            if let Some(for_each) = unwrap_tag_wrappers(&with_source.effect)
                .downcast_ref::<crate::effects::ForEachObject>()
            {
                let [inner] = for_each.effects.as_slice() else {
                    return None;
                };
                let (inner_source, damage) = damage_with_source_view(inner)?;
                let source = compatible_damage_sources(Some(&with_source.source), inner_source)??;
                if !matches!(damage.target.unhinted(), ChooseSpec::Iterated) {
                    return None;
                }
                (&for_each.filter, damage, source)
            } else {
                let (inner_source, damage) = damage_with_source_view(&with_source.effect)?;
                let source = compatible_damage_sources(Some(&with_source.source), inner_source)??;
                let recipient_filter = match damage.target.unhinted() {
                    ChooseSpec::Object(filter) | ChooseSpec::All(filter)
                        if filter.set_quantifier_surface()
                            == Some(ironsmith_core::SetQuantifierSurface::Each) =>
                    {
                        filter
                    }
                    _ => return None,
                };
                (recipient_filter, damage, source)
            }
        };
    let Value::PowerOf(power_source) = damage.amount.unhinted() else {
        return None;
    };
    if !choose_spec_is_tagged_object(source, producer_tag)
        || !choose_spec_is_tagged_object(power_source.as_ref(), producer_tag)
    {
        return None;
    }

    let mut recipient = describe_damage_fanout_filter(recipient_filter)?;
    if recipient_filter
        .excluded_subtypes
        .contains(&crate::types::Subtype::Army)
    {
        recipient = recipient.replace("non-army", "non-Army");
    }
    let producer_text = if let Some(subtype) = amass.subtype {
        let subtype = capitalize_first(&pluralize_word(&subtype.to_string().to_ascii_lowercase()));
        format!("Amass {subtype} {}", describe_value(&amass.amount))
    } else {
        describe_effect(producer_effect)
            .trim()
            .trim_end_matches('.')
            .to_string()
    };
    (!producer_text.is_empty()).then(|| {
        format!(
            "{producer_text}, then the Army you amassed deals damage equal to its power to each {recipient}"
        )
    })
}

/// Recombine an animation with a counter instruction that consumes the exact
/// tagged animation result. The consumer commonly retains type refinements
/// (`artifact creature`) for runtime validation, but identity comes from the
/// producer tag rather than from re-querying every matching permanent.
pub(in crate::compiled_text) fn describe_animation_then_counters_on_result(
    producer_effect: &Effect,
    consumer_effect: &Effect,
) -> Option<String> {
    fn object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
        match spec.unhinted() {
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => Some(filter),
            ChooseSpec::Target(inner)
            | ChooseSpec::WithCount(inner, _)
            | ChooseSpec::WithCountValue(inner, _, _) => object_filter(inner),
            _ => None,
        }
    }

    let producer_tag = wrapped_effect_tag(producer_effect)?;
    let animation = unwrap_tag_wrappers(producer_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let Some(crate::continuous::Modification::AddCardTypes(types)) = &animation.modification else {
        return None;
    };
    if !types.contains(&CardType::Creature) {
        return None;
    }
    let animated_filter =
        animation
            .target_spec
            .as_ref()
            .and_then(object_filter)
            .or(match &animation.target {
                crate::continuous::EffectTarget::Filter(filter) => Some(filter),
                _ => None,
            })?;

    let for_each =
        unwrap_tag_wrappers(consumer_effect).downcast_ref::<crate::effects::ForEachObject>()?;
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *producer_tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }
    let [counter_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let counters =
        unwrap_tag_wrappers(counter_effect).downcast_ref::<crate::effects::PutCountersEffect>()?;
    if counters.distributed
        || counters.target_count.is_some()
        || counters.target.unhinted() != &ChooseSpec::Iterated
    {
        return None;
    }

    let result_noun = animated_filter
        .card_types
        .iter()
        .copied()
        .find(|card_type| *card_type != CardType::Creature)
        .map(CardType::name)
        .unwrap_or("permanent");
    let counter_text = describe_effect(counter_effect);
    let counter_prefix = counter_text
        .trim()
        .trim_end_matches('.')
        .strip_suffix(" on it")
        .or_else(|| {
            counter_text
                .trim()
                .trim_end_matches('.')
                .strip_suffix(" on that object")
        })?;
    let producer_text = describe_effect(producer_effect);
    let producer_text = producer_text.trim().trim_end_matches('.');
    (!producer_text.is_empty()).then(|| {
        format!(
            "{producer_text}. {counter_prefix} on each {result_noun} that became a creature this way"
        )
    })
}

/// Render a goad consumer whose object set is the exact tagged outcome of a
/// counter-placement producer. This covers both direct counter placement and
/// counter choices nested under a per-player `may` action.
pub(in crate::compiled_text) fn describe_counters_then_goad_countered_result(
    producer_effect: &Effect,
    consumer_effect: &Effect,
) -> Option<String> {
    fn counter_action(effect: &Effect) -> Option<&crate::effects::PutCountersEffect> {
        let effect = unwrap_tag_wrappers(effect);
        if let Some(counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
            return Some(counters);
        }
        if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>()
            && let [inner] = for_players.effects.as_slice()
        {
            return counter_action(inner);
        }
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>()
            && let [inner] = may.effects.as_slice()
        {
            return counter_action(inner);
        }
        None
    }

    let producer_tag = wrapped_effect_tag(producer_effect)?;
    let counters = counter_action(producer_effect)?;
    let goad = unwrap_tag_wrappers(consumer_effect).downcast_ref::<crate::effects::GoadEffect>()?;
    let (ChooseSpec::All(filter) | ChooseSpec::Object(filter)) = goad.target.base() else {
        return None;
    };
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == *producer_tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }

    let noun = filter
        .card_types
        .as_slice()
        .first()
        .copied()
        .map(CardType::name)
        .unwrap_or("permanent");
    let counter_reference = if counters.amount.unhinted() == &Value::Fixed(1) {
        format!("a {} counter", counters.counter_type.description())
    } else {
        "counters".to_string()
    };
    let producer_text = describe_effect(producer_effect);
    let producer_text = producer_text.trim().trim_end_matches('.');
    if producer_text.is_empty() {
        return None;
    }
    let goad_text = format!("goad each {noun} that had {counter_reference} put on it this way");
    if counters
        .amount
        .has_surface_hint(ValueSurfaceHint::CounterFollowupThen)
    {
        Some(format!("{producer_text}, then {goad_text}"))
    } else {
        Some(format!("{producer_text}. {}", capitalize_first(&goad_text)))
    }
}

/// Preserve a per-player exile result across an intervening sacrifice. The
/// final battlefield move is authorized by the exact producer tag, so it must
/// not be rendered as a query over every card currently in exile.
pub(in crate::compiled_text) fn describe_each_player_exile_sacrifice_return_result(
    effect: &Effect,
) -> Option<String> {
    fn wrapper_contains_tag(effect: &Effect, expected: &TagKey) -> bool {
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return wrapper_contains_tag(&with_id.effect, expected);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return tagged.tag == *expected || wrapper_contains_tag(&tagged.effect, expected);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return tag_all.tag == *expected || wrapper_contains_tag(&tag_all.effect, expected);
        }
        false
    }

    let for_players =
        unwrap_render_wrappers(effect).downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let per_player_effects = if let [effect] = for_players.effects.as_slice()
        && let Some(sequence) =
            unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::CommaThen
                | ironsmith_core::SequenceSurface::RepeatedCommaThen
        ) {
        sequence.effects.as_slice()
    } else {
        for_players.effects.as_slice()
    };
    let [exile_effect, sacrifice_effect, return_effect] = per_player_effects else {
        return None;
    };

    let exile = unwrap_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()?;
    let (ChooseSpec::All(exile_filter) | ChooseSpec::Object(exile_filter)) = exile.spec.base()
    else {
        return None;
    };
    if exile_filter.zone != Some(Zone::Graveyard)
        || exile_filter.owner != Some(PlayerFilter::IteratedPlayer)
    {
        return None;
    }

    let sacrifice = sacrifice_view(unwrap_tag_wrappers(sacrifice_effect))?;
    if sacrifice.player != &PlayerFilter::IteratedPlayer {
        return None;
    }

    let returned = unwrap_tag_wrappers(return_effect);
    let exact_return =
        if let Some(move_to_zone) = returned.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            if move_to_zone.zone != Zone::Battlefield
                || move_to_zone.battlefield_controller
                    != crate::effects::BattlefieldController::Preserve
            {
                false
            } else {
                match move_to_zone.target.base() {
                    ChooseSpec::All(return_filter) | ChooseSpec::Object(return_filter) => {
                        return_filter.zone == Some(Zone::Exile)
                            && return_filter.tagged_constraints.iter().any(|constraint| {
                                constraint.relation
                                    == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                                    && wrapper_contains_tag(exile_effect, &constraint.tag)
                            })
                    }
                    ChooseSpec::Tagged(tag) => wrapper_contains_tag(exile_effect, tag),
                    _ => false,
                }
            }
        } else if let Some(put_onto_battlefield) =
            returned.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
        {
            !put_onto_battlefield.tapped
                && put_onto_battlefield.controller == PlayerFilter::IteratedPlayer
                && matches!(
                    put_onto_battlefield.target.base(),
                    ChooseSpec::Tagged(tag) if wrapper_contains_tag(exile_effect, tag)
                )
        } else {
            false
        };
    if !exact_return {
        return None;
    }

    let exile_text = describe_effect(exile_effect);
    let exile_tail = exile_text
        .trim()
        .trim_end_matches('.')
        .strip_prefix("Exile ")?
        .replace("that player's", "their")
        .replace(" in their graveyard", " from their graveyard");
    let sacrifice_text = lowercase_first(
        describe_effect(sacrifice_effect)
            .trim()
            .trim_end_matches('.'),
    );
    let sacrifice_tail = sacrifice_text
        .strip_prefix("that player ")
        .or_else(|| sacrifice_text.strip_prefix("each player "))
        .unwrap_or(sacrifice_text.as_str());
    let sacrifice_tail = sacrifice_tail
        .strip_prefix("sacrifice ")
        .map(|tail| format!("sacrifices {tail}"))
        .unwrap_or_else(|| sacrifice_tail.to_string());
    let sacrifice_tail = sacrifice_tail.replace("that player controls", "they control");

    Some(format!(
        "Each player exiles {exile_tail}, then {sacrifice_tail}, then puts all cards they exiled this way onto the battlefield"
    ))
}

pub(in crate::compiled_text) fn describe_result_producer_then_for_each_tagged(
    producer_effect: &Effect,
    for_each_effect: &Effect,
) -> Option<String> {
    fn object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
        match spec.unhinted() {
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => Some(filter),
            ChooseSpec::Target(inner)
            | ChooseSpec::WithCount(inner, _)
            | ChooseSpec::WithCountValue(inner, _, _) => object_filter(inner),
            _ => None,
        }
    }

    fn result_noun(filter: &ObjectFilter, action: &str, force_card_noun: bool) -> String {
        let source_zone = filter.zone;
        let mut base = filter.clone();
        base.zone = None;
        base.owner = None;
        base.controller = None;
        base.tagged_constraints.clear();
        let mut noun = strip_leading_article(&base.description()).to_string();
        if noun.is_empty() {
            noun = "object".to_string();
        }
        if action == "exiled" && (force_card_noun || source_zone != Some(Zone::Battlefield)) {
            if noun == "permanent" || noun == "object" {
                noun = "card".to_string();
            } else if !noun.ends_with(" card") && !noun.ends_with(" cards") {
                noun.push_str(" card");
            }
        }
        noun
    }

    let producer_tag = wrapped_effect_tag(producer_effect)?;
    let (consumer_tag, consumer_effects) = if let Some(for_each) =
        for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        (&for_each.tag, for_each.effects.as_slice())
    } else {
        let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachObject>()?;
        let [constraint] = for_each.filter.tagged_constraints.as_slice() else {
            return None;
        };
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        let mut base = for_each.filter.clone();
        base.tagged_constraints.clear();
        if base != ObjectFilter::default() {
            return None;
        }
        (&constraint.tag, for_each.effects.as_slice())
    };
    if consumer_tag != producer_tag || consumer_effects.is_empty() {
        return None;
    }
    let producer = unwrap_tag_wrappers(producer_effect);
    let (producer_filter, action) =
        if let Some(exile) = producer.downcast_ref::<crate::effects::ExileEffect>() {
            (object_filter(&exile.spec)?, "exiled")
        } else if let Some(destroy) = producer.downcast_ref::<crate::effects::DestroyEffect>() {
            (object_filter(&destroy.spec)?, "destroyed")
        } else {
            return None;
        };
    let producer_text = describe_effect(producer_effect)
        .trim_end_matches('.')
        .to_string();

    // A producer deliberately captures the complete result set, while a
    // last-known-information gate inside its loop can select the qualified
    // subset that receives the follow-up. Preserve that executable split, but
    // render the sole current-iterand gate as part of the Oracle noun phrase:
    // "each nontoken creature destroyed this way" rather than the mechanically
    // expanded "each creature ..., if it was a nontoken creature".
    let (noun_filter, followup_effects) = if let [gate_effect] = consumer_effects
        && let Some(gate) =
            unwrap_tag_wrappers(gate_effect).downcast_ref::<crate::effects::ConditionalEffect>()
        && gate.surface == ironsmith_core::ConditionalSurface::LeadingIf
        && gate.if_false.is_empty()
        && let Condition::TaggedObjectMatchedLastKnown(iterated_tag, filter) = &gate.condition
        && iterated_tag.as_str() == "__it__"
        && !gate.if_true.is_empty()
    {
        (filter, gate.if_true.as_slice())
    } else {
        (producer_filter, consumer_effects)
    };
    let force_card_noun = followup_effects.iter().any(|effect| {
        unwrap_tag_wrappers(effect)
            .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            .is_some()
    });
    let followup = lowercase_first(&describe_effect_list(followup_effects));
    if producer_text.is_empty() || followup.is_empty() {
        return None;
    }
    Some(format!(
        "{producer_text}. For each {} {action} this way, {followup}",
        result_noun(noun_filter, action, force_card_noun)
    ))
}

pub(crate) fn downcast_target_only(effect: &Effect) -> Option<&crate::effects::TargetOnlyEffect> {
    unwrap_tag_wrappers(effect).downcast_ref::<crate::effects::TargetOnlyEffect>()
}

pub(crate) fn target_only_tag(effect: &Effect) -> Option<&str> {
    let target_only = downcast_target_only(effect)?;
    match target_only.target.base() {
        ChooseSpec::Tagged(tag) => Some(tag.as_str()),
        _ => None,
    }
}

pub(crate) fn downcast_destroy(effect: &Effect) -> Option<&crate::effects::DestroyEffect> {
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some(destroy);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return tagged
            .effect
            .downcast_ref::<crate::effects::DestroyEffect>();
    }
    effect
        .downcast_ref::<crate::effects::WithIdEffect>()?
        .effect
        .downcast_ref::<crate::effects::DestroyEffect>()
}
