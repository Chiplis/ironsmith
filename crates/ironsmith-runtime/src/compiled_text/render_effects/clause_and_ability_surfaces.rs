use super::*;

pub(super) fn describe_leading_effect_then_shared_draw_lose_clause(
    effects: &[Effect],
) -> Option<String> {
    let [leading_effect, draw_effect, lose_effect, rest @ ..] = effects else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let draw_lose = describe_draw_then_lose_life(draw, lose)?;

    let leading = describe_effect(leading_effect);
    let leading_trimmed = leading.trim();
    if leading_trimmed.is_empty()
        || leading_trimmed.contains(". ")
        || leading_trimmed.contains(": ")
        || leading_trimmed.starts_with("If ")
        || leading_trimmed.starts_with("When ")
        || leading_trimmed.starts_with("Whenever ")
        || leading_trimmed.starts_with("At ")
        || leading_trimmed.starts_with("Choose ")
    {
        return None;
    }
    let leading_clause = normalize_imperative_you_clause(leading_trimmed.trim_end_matches('.'));
    let mut rendered = format!(
        "{}. {draw_lose}",
        cleanup_decompiled_text(&lowercase_first(&leading_clause))
    );
    if !rest.is_empty() {
        let rest_text =
            describe_effect_clause_list(rest).unwrap_or_else(|| describe_effect_list(rest));
        if !rest_text.trim().is_empty() {
            rendered.push_str(". ");
            rendered.push_str(&capitalize_first(rest_text.trim().trim_end_matches('.')));
        }
    }
    Some(rendered)
}

pub(super) fn normalize_imperative_you_clause(text: &str) -> String {
    for prefix in ["You ", "you "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            const IMPERATIVE_VERBS: &[&str] = &[
                "pay ",
                "lose ",
                "gain ",
                "draw ",
                "put ",
                "discard ",
                "sacrifice ",
                "choose ",
                "mill ",
                "reveal ",
                "scry ",
                "search ",
                "shuffle ",
                "surveil ",
            ];
            if IMPERATIVE_VERBS.iter().any(|verb| rest.starts_with(verb)) {
                return rest.to_string();
            }
            let normalized = normalize_you_verb_phrase(rest);
            if normalized != rest {
                return normalized;
            }
        }
    }
    text.to_string()
}

/// "You and that player each <verb> ..." for adjacent same-payload effects
/// whose only difference is the affected player (you + a back-reference).
pub(super) fn describe_joint_subject_pair(first: &Effect, second: &Effect) -> Option<String> {
    fn joint_other_surface(player: &PlayerFilter) -> Option<&'static str> {
        match player {
            PlayerFilter::DamagedPlayer
            | PlayerFilter::TaggedPlayer(_)
            | PlayerFilter::ChosenPlayer => Some("that player"),
            PlayerFilter::Target(inner) if **inner == PlayerFilter::Opponent => {
                Some("target opponent")
            }
            PlayerFilter::Target(inner) if **inner == PlayerFilter::Any => Some("target player"),
            _ => None,
        }
    }

    fn joined_damage_text(amount: &Value, recipients: &str) -> String {
        let (amount_text, where_x) = describe_damage_amount_clause(amount);
        let mut text = format!("this deals {amount_text} to {recipients}");
        if let Some(where_x) = where_x {
            text.push_str(&format!(", where X is {where_x}"));
        }
        text
    }

    if let Some(first_gain) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::GainLifeEffect>()
        && let Some(second_gain) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()
        && first_gain.amount == second_gain.amount
        && matches!(&first_gain.player, ChooseSpec::Player(PlayerFilter::You))
        && let ChooseSpec::Player(second_player) = &second_gain.player
        && let Some(other) = joint_other_surface(second_player)
    {
        return Some(format!(
            "You and {other} each gain {}",
            describe_life_amount_phrase(&first_gain.amount)
        ));
    }

    // "Target creature you control deals damage equal to its power to each
    // of two other target creatures" — tagged target prelude + execute-with-
    // source power damage compact to the oracle's single sentence.
    if let Some(first_tagged) = first.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(target_only) = first_tagged
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let Some(second_exec) = unwrap_basic_tag_wrappers(second)
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        && matches!(&second_exec.source, ChooseSpec::Tagged(tag) if *tag == first_tagged.tag)
        && let Some(damage) = second_exec
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(
            &damage.amount,
            Value::PowerOf(spec)
                if matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if *tag == first_tagged.tag)
        )
    {
        let source_text = describe_choose_spec(&target_only.target);
        let raw_target = describe_choose_spec(&damage.target);
        let target_text = if let Some((count_word, rest)) = raw_target.split_once(" target other ")
        {
            format!("each of {count_word} other target {rest}")
        } else {
            raw_target
        };
        return Some(format!(
            "{} deals damage equal to its power to {target_text}",
            capitalize_first(&source_text)
        ));
    }

    // "deals X damage to each creature and each player" — for-each-creature
    // damage followed by for-each-player damage with the same amount.
    if let Some(for_each) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::ForEachObject>()
        && let [inner] = for_each.effects.as_slice()
        && let Some(first_damage) =
            unwrap_basic_tag_wrappers(inner).downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(first_damage.target, ChooseSpec::Iterated)
        && for_each.filter.card_types == [CardType::Creature]
        && for_each.filter.controller.is_none()
        && for_each.filter.subtypes.is_empty()
        && for_each.filter.tagged_constraints.is_empty()
        && let Some(for_players) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::ForPlayersEffect>()
        && for_players.filter == PlayerFilter::Any
        && let [player_inner] = for_players.effects.as_slice()
        && let Some(second_damage) = unwrap_basic_tag_wrappers(player_inner)
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && second_damage.amount == first_damage.amount
        && matches!(
            &second_damage.target,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
    {
        return Some(joined_damage_text(
            &first_damage.amount,
            "each creature and each player",
        ));
    }

    // "deals X damage to each creature and each planeswalker" — same as
    // the creature/player compaction above, but both halves are object
    // fanouts.
    if let Some(for_each) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::ForEachObject>()
        && let [inner] = for_each.effects.as_slice()
        && let Some(first_damage) =
            unwrap_basic_tag_wrappers(inner).downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(first_damage.target, ChooseSpec::Iterated)
        && for_each.filter.card_types == [CardType::Creature]
        && for_each.filter.controller.is_none()
        && for_each.filter.subtypes.is_empty()
        && for_each.filter.tagged_constraints.is_empty()
        && let Some(for_each_planeswalker) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::ForEachObject>()
        && let [planeswalker_inner] = for_each_planeswalker.effects.as_slice()
        && let Some(second_damage) = unwrap_basic_tag_wrappers(planeswalker_inner)
            .downcast_ref::<crate::effects::DealDamageEffect>()
        && matches!(second_damage.target, ChooseSpec::Iterated)
        && second_damage.amount == first_damage.amount
        && for_each_planeswalker.filter.card_types == [CardType::Planeswalker]
        && for_each_planeswalker.filter.controller.is_none()
        && for_each_planeswalker.filter.subtypes.is_empty()
        && for_each_planeswalker.filter.tagged_constraints.is_empty()
    {
        return Some(joined_damage_text(
            &first_damage.amount,
            "each creature and each planeswalker",
        ));
    }

    if let Some(first_create) =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::CreateTokenEffect>()
        && let Some(second_create) =
            unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::CreateTokenEffect>()
        && first_create.count == second_create.count
        && first_create.controller == PlayerFilter::You
        && let Some(other) = joint_other_surface(&second_create.controller)
    {
        // Compare the rendered token descriptions (after the count word) so
        // instance-specific token ids don't break the pairing.
        let first_rendered = describe_effect(first);
        let first_body = first_rendered
            .trim()
            .trim_end_matches('.')
            .strip_prefix("Create ")?
            .to_string();
        let second_rendered = describe_effect(second);
        let second_lower = second_rendered.to_ascii_lowercase();
        let creates_idx = second_lower.find("creates ")?;
        let second_body = second_rendered[creates_idx + "creates ".len()..]
            .trim_end_matches('.')
            .to_string();
        let description_after_count = |body: &str| {
            body.split_once(' ')
                .map(|(_, rest)| rest.to_ascii_lowercase())
        };
        if description_after_count(&first_body)? == description_after_count(&second_body)? {
            return Some(format!("You and {other} each create {first_body}"));
        }
    }

    None
}

pub(super) fn describe_player_or_planeswalker_damage_then_controlled_creature_damage(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let first_damage = deal_damage_effect_view(first)?;
    if describe_choose_spec(&first_damage.target) != "target player or planeswalker" {
        return None;
    }

    let for_each =
        unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::ForEachObject>()?;
    let mut expected_filter = ObjectFilter::creature();
    expected_filter.controller = Some(PlayerFilter::TargetPlayerOrControllerOfTarget);
    if for_each.filter != expected_filter {
        return None;
    }

    let [inner] = for_each.effects.as_slice() else {
        return None;
    };
    let second_damage = deal_damage_effect_view(inner)?;
    if second_damage.amount != first_damage.amount
        || !matches!(second_damage.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let (amount_text, where_x) = describe_damage_amount_clause(&first_damage.amount);
    let mut text = format!(
        "Deal {amount_text} to target player or planeswalker and each creature that player or that planeswalker's controller controls"
    );
    if let Some(where_x) = where_x {
        text.push_str(&format!(", where X is {where_x}"));
    }
    Some(text)
}

pub(crate) fn describe_false_only_conditional(
    condition: &crate::effect::Condition,
    false_branch: &str,
) -> String {
    if let crate::effect::Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
    } = condition
    {
        let verb = if tag.as_str().starts_with("discarded_") {
            Some("discard")
        } else if tag.as_str().starts_with("sacrificed_") {
            Some("sacrifice")
        } else if tag.as_str().starts_with("exiled_") {
            Some("exile")
        } else if tag.as_str().starts_with("destroyed_") {
            Some("destroy")
        } else {
            None
        };
        if let Some(verb) = verb {
            let object_text = if (tag.as_str().starts_with("discarded_")
                || tag.as_str().starts_with("exiled_")
                || tag.as_str().starts_with("revealed_"))
                && !filter.card_types.is_empty()
                && filter.zone.is_none()
                && filter.controller.is_none()
                && filter.owner.is_none()
                && filter.subtypes.is_empty()
                && filter.any_of.is_empty()
                && filter.tagged_constraints.is_empty()
            {
                let words = filter
                    .card_types
                    .iter()
                    .map(|card_type| describe_card_type_word_local(*card_type).to_string())
                    .collect::<Vec<_>>();
                with_indefinite_article(&format!("{} card", join_with_or(&words)))
            } else {
                let desc = filter.description();
                let stripped = strip_leading_article(&desc).to_ascii_lowercase();
                if (tag.as_str().starts_with("discarded_")
                    || tag.as_str().starts_with("exiled_")
                    || tag.as_str().starts_with("revealed_"))
                    && stripped == "land"
                {
                    "a land card".to_string()
                } else if (tag.as_str().starts_with("discarded_")
                    || tag.as_str().starts_with("exiled_")
                    || tag.as_str().starts_with("revealed_"))
                    && stripped == "creature"
                {
                    "a creature card".to_string()
                } else {
                    with_indefinite_article(&desc)
                }
            };
            return format!(
                "If {} didn't {} {} this way, {}",
                describe_player_filter(player),
                verb,
                object_text,
                false_branch
            );
        }
    }

    if matches!(condition, crate::effect::Condition::ThisSpellEscaped) {
        let branch = false_branch.trim().trim_end_matches('.');
        if branch.is_empty() {
            return "Unless it escaped".to_string();
        }
        return format!("{branch} unless it escaped");
    }

    format!(
        "Unless {}, {}",
        lowercase_first(&describe_condition(condition)),
        false_branch
    )
}

pub(crate) fn describe_exile_then_return(
    tagged: &crate::effects::TaggedEffect,
    move_back: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if move_back.zone != Zone::Battlefield {
        return None;
    }
    let crate::target::ChooseSpec::Tagged(return_tag) = &move_back.target else {
        return None;
    };
    if !return_tag.as_str().starts_with("exiled_") || return_tag != &tagged.tag {
        return None;
    }
    let exile_move = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile_move.zone != Zone::Exile {
        return None;
    }
    let target = describe_choose_spec(&exile_move.target);
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "those cards"
    } else {
        "that card"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Exile {target}, then return {return_object} to the battlefield{tapped_suffix}{controller_suffix}"
    ))
}

pub(super) fn describe_source_exile_then_return(first: &Effect, second: &Effect) -> Option<String> {
    let exile_move =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_back =
        unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile_move.zone != Zone::Exile || move_back.zone != Zone::Battlefield {
        return None;
    }
    if !matches!(exile_move.target.unhinted(), ChooseSpec::Source) {
        return None;
    }
    if !matches!(move_back.target.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    {
        return None;
    }

    let target = describe_source_motion_reference(&exile_move.target, "this");
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "them"
    } else {
        "it"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Exile {target}, then return {return_object} to the battlefield{tapped_suffix}{controller_suffix}"
    ))
}

pub(super) fn describe_source_motion_reference(spec: &ChooseSpec, named_fallback: &str) -> String {
    let Some(surface) = spec.source_reference_surface() else {
        return named_fallback.to_string();
    };
    match surface {
        crate::target::SourceReferenceSurface::ThisPermanentType(text)
            if matches!(
                text.to_ascii_lowercase().as_str(),
                "this"
                    | "it"
                    | "itself"
                    | "him"
                    | "himself"
                    | "her"
                    | "herself"
                    | "them"
                    | "themselves"
            ) =>
        {
            named_fallback.to_string()
        }
        crate::target::SourceReferenceSurface::ThisPermanentType(text) => text.clone(),
        crate::target::SourceReferenceSurface::FullName(_)
        | crate::target::SourceReferenceSurface::ShortName(_) => named_fallback.to_string(),
    }
}

pub(super) fn describe_exile_then_return_transformed_with_counter(
    exile_effect: &Effect,
    return_effect: &Effect,
    transform_effect: &Effect,
    put_counter_effect: &Effect,
) -> Option<String> {
    let exile_tag = wrapped_effect_tag(exile_effect);
    let exile_move = unwrap_basic_tag_wrappers(exile_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_back = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let transform = unwrap_basic_tag_wrappers(transform_effect)
        .downcast_ref::<crate::effects::TransformEffect>()?;
    let put_counter = unwrap_basic_tag_wrappers(put_counter_effect)
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if move_back.zone != Zone::Battlefield
        || exile_move.zone != Zone::Exile
        || put_counter.distributed
        || put_counter.target_count.is_some()
    {
        return None;
    }
    let crate::target::ChooseSpec::Tagged(return_tag) = &move_back.target else {
        return None;
    };
    let exile_target_is_source = matches!(exile_move.target.unhinted(), ChooseSpec::Source);
    let returns_exiled_source =
        return_tag.as_str() == "__source_exiled__" && exile_target_is_source;
    let returns_wrapped_exile = return_tag.as_str().starts_with("exiled_")
        && exile_tag.is_some_and(|exile_tag| exile_tag == return_tag);
    if !returns_exiled_source && !returns_wrapped_exile {
        return None;
    }
    if !matches!(&transform.target, ChooseSpec::Tagged(tag) if tag == return_tag)
        || !matches!(&put_counter.target, ChooseSpec::Tagged(tag) if tag == return_tag)
    {
        return None;
    }

    let target = if exile_target_is_source {
        describe_source_motion_reference(&exile_move.target, "it")
    } else {
        describe_choose_spec(&exile_move.target)
    };
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "them"
    } else {
        "it"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    let counter_text = describe_put_counter_phrase(&put_counter.amount, put_counter.counter_type);
    Some(format!(
        "Exile {target}, then put {return_object} onto the battlefield{tapped_suffix} transformed{controller_suffix} with {counter_text} on it"
    ))
}

pub(crate) fn describe_exile_return_then_transform(
    exile_effect: &Effect,
    return_effect: &Effect,
    transform_effect: &Effect,
) -> Option<String> {
    let exile_move = exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let move_back = return_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let transform = transform_effect.downcast_ref::<crate::effects::TransformEffect>()?;
    if exile_move.zone != Zone::Exile || move_back.zone != Zone::Battlefield {
        return None;
    }
    let return_and_transform_source = matches!(move_back.target.unhinted(), ChooseSpec::Source)
        && matches!(transform.target.unhinted(), ChooseSpec::Source);
    let direct_same_target = exile_move.target.unhinted() == move_back.target.unhinted()
        && move_back.target.unhinted() == transform.target.unhinted();
    let source_name_exile = return_and_transform_source
        && is_plain_source_name_exile_filter(exile_move.target.unhinted());
    if !direct_same_target && !source_name_exile {
        return None;
    }

    let target = if return_and_transform_source {
        describe_source_motion_reference(&move_back.target, "this creature")
    } else if matches!(exile_move.target.unhinted(), ChooseSpec::Source) {
        describe_source_motion_reference(&exile_move.target, "this creature")
    } else {
        describe_choose_spec(&exile_move.target)
    };
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "them"
    } else {
        "it"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Exile {target}, then return {return_object} to the battlefield{tapped_suffix} transformed{controller_suffix}"
    ))
}

pub(super) fn is_plain_source_name_exile_filter(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Object(filter) = spec else {
        return false;
    };
    filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.len() == 1
        && filter.name.is_none()
        && !filter.source
        && filter.any_of.is_empty()
}

pub(super) fn move_to_zone_for_transform_compaction(
    effect: &Effect,
) -> Option<(
    &crate::effects::MoveToZoneEffect,
    Option<&crate::tag::TagKey>,
)> {
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return Some((move_to_zone, None));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let move_to_zone = tagged
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        return Some((move_to_zone, Some(&tagged.tag)));
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return move_to_zone_for_transform_compaction(&with_id.effect);
    }
    None
}

pub(super) fn transform_targets_returned_object(
    move_back: &crate::effects::MoveToZoneEffect,
    returned_tag: Option<&crate::tag::TagKey>,
    transform: &crate::effects::TransformEffect,
) -> bool {
    if let Some(returned_tag) = returned_tag
        && matches!(&transform.target, ChooseSpec::Tagged(transform_tag) if transform_tag == returned_tag)
    {
        return true;
    }
    move_back.target.unhinted() == transform.target.unhinted()
}

pub(crate) fn describe_return_then_transform(
    return_effect: &Effect,
    transform_effect: &Effect,
) -> Option<String> {
    let (move_back, returned_tag) = move_to_zone_for_transform_compaction(return_effect)?;
    let transform = transform_effect.downcast_ref::<crate::effects::TransformEffect>()?;
    if move_back.zone != Zone::Battlefield {
        return None;
    }
    if !transform_targets_returned_object(move_back, returned_tag, transform) {
        return None;
    }

    let return_object = if choose_spec_allows_multiple(&move_back.target) {
        "them"
    } else {
        "it"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&move_back.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Return {return_object} to the battlefield{tapped_suffix} transformed{controller_suffix}"
    ))
}

pub(crate) fn describe_reveal_top_then_if_put_into_hand(
    reveal_top: &crate::effects::RevealTopEffect,
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let reveal_tag = reveal_top.tag.as_ref()?;
    let Condition::TaggedObjectMatches(cond_tag, filter) = &conditional.condition else {
        return None;
    };
    if cond_tag != reveal_tag {
        return None;
    }
    let move_to_zone = conditional.if_true[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand {
        return None;
    }
    if !matches!(
        move_to_zone.target.base(),
        ChooseSpec::Tagged(tag) if tag == reveal_tag
    ) {
        return None;
    }

    let subject = describe_player_filter(&reveal_top.player);
    let is_you = subject == "you";
    let reveal_sentence = if is_you {
        "Reveal the top card of your library".to_string()
    } else {
        let mut reveal_subject = subject;
        if matches!(
            reveal_top.player,
            PlayerFilter::Defending | PlayerFilter::Attacking | PlayerFilter::DamagedPlayer
        ) {
            if let Some(rest) = reveal_subject.strip_prefix("the ") {
                reveal_subject = rest.to_string();
            }
        }
        let verb = player_verb(&reveal_subject, "reveal", "reveals");
        format!("{reveal_subject} {verb} the top card of their library")
    };

    // Match the common oracle pattern for "if it's a <type> card".
    let desc = filter.description();
    let stripped = strip_leading_article(&desc).trim().to_ascii_lowercase();
    let noun_phrase = if stripped.ends_with(" card") {
        stripped.clone()
    } else if matches!(
        stripped.as_str(),
        "land"
            | "creature"
            | "artifact"
            | "enchantment"
            | "planeswalker"
            | "battle"
            | "permanent"
            | "instant"
            | "sorcery"
    ) {
        format!("{stripped} card")
    } else {
        return None;
    };
    let condition_text = format!("it's {}", with_indefinite_article(&noun_phrase));

    let move_sentence = if is_you {
        "put it into your hand".to_string()
    } else {
        "that player puts it into their hand".to_string()
    };

    Some(format!(
        "{reveal_sentence}. If {condition_text}, {move_sentence}"
    ))
}

pub(super) fn move_to_zone_for_tag<'a>(
    effect: &'a Effect,
    tag: &crate::TagKey,
) -> Option<&'a crate::effects::MoveToZoneEffect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return move_to_zone_for_tag(&tagged.effect, tag);
    }
    let move_to_zone = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    matches!(move_to_zone.target.base(), ChooseSpec::Tagged(target_tag) if target_tag == tag)
        .then_some(move_to_zone)
}

pub(super) fn describe_reveal_top_may_put_otherwise_hand(
    reveal_top: &crate::effects::RevealTopEffect,
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if if_effect.condition != with_id.id
        || !matches!(if_effect.predicate, EffectPredicate::DidNotHappen)
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 1
    {
        return None;
    }

    let reveal_tag = reveal_top.tag.as_ref()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [may_effect] = may.effects.as_slice() else {
        return None;
    };
    let conditional = may_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let Condition::TaggedObjectMatches(cond_tag, _) = &conditional.condition else {
        return None;
    };
    if cond_tag != reveal_tag {
        return None;
    }

    let battlefield_move = move_to_zone_for_tag(&conditional.if_true[0], reveal_tag)?;
    if battlefield_move.zone != Zone::Battlefield || battlefield_move.to_top {
        return None;
    }
    let hand_move = move_to_zone_for_tag(&if_effect.then[0], reveal_tag)?;
    if hand_move.zone != Zone::Hand || hand_move.to_top {
        return None;
    }

    let revealer = describe_player_filter(&reveal_top.player);
    let revealer_is_you = revealer == "you";
    let reveal_sentence = if revealer_is_you {
        "Reveal the top card of your library".to_string()
    } else {
        let verb = player_verb(&revealer, "reveal", "reveals");
        format!("{revealer} {verb} the top card of their library")
    };

    let decider = may
        .decider
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    let may_prefix = if decider == "you" {
        "You may".to_string()
    } else {
        format!("{decider} may")
    };

    let tapped_suffix = if battlefield_move.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&battlefield_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let controller_suffix = match battlefield_move.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    let condition = lowercase_first(&describe_condition(&conditional.condition));

    let hand_clause = if revealer_is_you {
        "put that card into your hand".to_string()
    } else {
        format!("{revealer} puts that card into their hand")
    };

    Some(format!(
        "{reveal_sentence}. {may_prefix} put that card onto the battlefield{tapped_suffix}{controller_suffix} if {condition}. Otherwise, {hand_clause}"
    ))
}

pub(crate) fn describe_tagged_target_then_power_damage(
    tagged: &crate::effects::TaggedEffect,
    deal: &crate::effects::DealDamageEffect,
) -> Option<String> {
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let Value::PowerOf(source_spec) = &deal.amount else {
        return None;
    };
    let source_tag = match source_spec.as_ref() {
        ChooseSpec::Tagged(tag) => tag,
        _ => return None,
    };
    if source_tag.as_str() != tagged.tag.as_str() {
        return None;
    }

    if let ChooseSpec::Player(
        PlayerFilter::ControllerOf(controller_ref) | PlayerFilter::OwnerOf(controller_ref),
    ) = deal.target.base()
        && matches!(controller_ref, crate::filter::ObjectRef::Tagged(tag) if tag.as_str() == source_tag.as_str())
    {
        return None;
    }

    let source_text = describe_choose_spec(&target_only.target);
    if matches!(
        deal.target,
        ChooseSpec::Tagged(ref target_tag) if target_tag == source_tag
    ) {
        return Some(format!(
            "{source_text} deals damage to itself equal to its power"
        ));
    }

    let raw_target = describe_choose_spec(&deal.target);
    // Multi-target full damage reads "each of two other target creatures".
    let target_text = if let Some((count_word, rest)) = raw_target.split_once(" target other ") {
        format!("each of {count_word} other target {rest}")
    } else {
        raw_target
    };
    Some(format!(
        "{source_text} deals damage equal to its power to {target_text}"
    ))
}

pub(super) fn describe_execute_power_damage_from_tag<'a>(
    effect: &'a Effect,
    source_tag: &crate::TagKey,
) -> Option<&'a crate::effects::DealDamageEffect> {
    let mut effect = effect;
    loop {
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            effect = &tag_all.effect;
            continue;
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            effect = &tagged.effect;
            continue;
        }
        break;
    }
    let with_source = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    let ChooseSpec::Tagged(tag) = &with_source.source else {
        return None;
    };
    if tag.as_str() != source_tag.as_str() {
        return None;
    }
    let deal = with_source
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let Value::PowerOf(power_source) = &deal.amount else {
        return None;
    };
    let ChooseSpec::Tagged(power_tag) = power_source.as_ref() else {
        return None;
    };
    if power_tag.as_str() != source_tag.as_str() {
        return None;
    }
    Some(deal)
}

pub(super) fn describe_damage_amount_clause(amount: &Value) -> (String, Option<String>) {
    if is_effect_count_reference(amount, None) {
        return ("that much".to_string(), None);
    }
    if matches!(amount.unhinted(), Value::XTimes(2)) {
        return ("twice X damage".to_string(), None);
    }
    if let Some((amount_text, where_x)) = describe_damage_amount_with_revealed_count_where_x(amount)
    {
        return (amount_text, Some(where_x));
    }
    if value_prefers_equal_to(amount)
        || power_damage_prefers_equal_to(amount)
        || (!value_prefers_where_x(amount) && count_damage_prefers_equal_to(amount))
    {
        return (format!("damage equal to {}", describe_value(amount)), None);
    }
    if let Some(where_x) = describe_where_x_basis(amount) {
        return ("X damage".to_string(), Some(where_x));
    }
    (format!("{} damage", describe_value(amount)), None)
}

pub(super) fn describe_damage_target(target: &ChooseSpec) -> String {
    if let Some(text) = describe_counted_any_damage_target(target) {
        return text;
    }
    describe_choose_spec(target)
}

pub(super) fn describe_counted_any_damage_target(target: &ChooseSpec) -> Option<String> {
    let ChooseSpec::WithCount(inner, count) = target.unhinted() else {
        return None;
    };
    if !matches!(inner.unhinted(), ChooseSpec::AnyTarget) || count.random {
        return None;
    }
    if count.is_up_to_dynamic_x() {
        return Some("each of up to X targets".to_string());
    }
    if count.is_dynamic_x() {
        return Some("each of X targets".to_string());
    }

    match (count.min, count.max) {
        (0, Some(1)) => Some("up to one target".to_string()),
        (0, Some(max)) if max > 1 => {
            let count_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
            Some(format!("each of up to {count_text} targets"))
        }
        (min, Some(max)) if min == max && max > 1 => {
            let count_text = number_word(max as i32).unwrap_or_else(|| max.to_string());
            Some(format!("each of {count_text} targets"))
        }
        _ => None,
    }
}

pub(super) fn count_damage_prefers_equal_to(amount: &Value) -> bool {
    match amount.unhinted() {
        Value::Count(_) | Value::CountScaled(_, _) => true,
        Value::CountersOnSource(_) | Value::CountersOn(_, _) => true,
        Value::Scaled(inner, _) => count_damage_prefers_equal_to(inner),
        _ => false,
    }
}

pub(super) fn power_damage_prefers_equal_to(amount: &Value) -> bool {
    match amount.unhinted() {
        Value::SourcePower | Value::SourceToughness | Value::PowerOf(_) | Value::ToughnessOf(_) => {
            true
        }
        Value::Scaled(inner, _) => power_damage_prefers_equal_to(inner),
        Value::Add(left, right) => {
            power_damage_prefers_equal_to(left) || power_damage_prefers_equal_to(right)
        }
        _ => false,
    }
}

pub(super) fn describe_counter_count_with_where_x(
    value: &Value,
    counter_type: CounterType,
) -> Option<(String, String)> {
    if !value_prefers_where_x(value) {
        return None;
    }
    let where_x = describe_where_x_basis(value)?;
    Some((
        format!("X {} counters", describe_counter_type(counter_type)),
        where_x,
    ))
}

pub(crate) fn describe_target_power_damage_to_other_and_self(
    target_effect: &Effect,
    other_damage_effect: &Effect,
    self_damage_effect: &Effect,
) -> Option<String> {
    let tagged_target = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged_target
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let other_damage =
        describe_execute_power_damage_from_tag(other_damage_effect, &tagged_target.tag)?;
    let self_damage =
        describe_execute_power_damage_from_tag(self_damage_effect, &tagged_target.tag)?;
    if !matches!(other_damage.target, ChooseSpec::AnyOtherTarget) {
        return None;
    }
    if !matches!(
        self_damage.target,
        ChooseSpec::Tagged(ref target_tag) if target_tag.as_str() == tagged_target.tag.as_str()
    ) {
        return None;
    }

    let subject = capitalize_first(&describe_choose_spec(&target_only.target));
    let verb = if choose_spec_is_plural(&target_only.target) {
        "deal"
    } else {
        "deals"
    };
    Some(format!(
        "{subject} {verb} X damage to {} and X damage to itself, where X is its power",
        describe_choose_spec(&other_damage.target)
    ))
}

pub(crate) fn cleanup_decompiled_text(text: &str) -> String {
    let mut out = text.to_string();
    fn combine_until_end_choice_grant(text: &str, marker: &str) -> Option<String> {
        let (head, tail) = text.split_once(marker)?;
        if !head.contains(" gets ") {
            return None;
        }
        let choice = tail.strip_suffix(" until end of turn")?;
        Some(format!(
            "{head} and gains your choice of {choice} until end of turn"
        ))
    }

    for marker in [
        " until end of turn. it gains your choice of ",
        " until end of turn. this creature gains your choice of ",
        " until end of turn. This creature gains your choice of ",
    ] {
        if let Some(combined) = combine_until_end_choice_grant(&out, marker) {
            out = combined;
        }
    }

    for (from, to) in [
        (
            "votes are revealed, for each vote",
            "votes are revealed. For each vote",
        ),
        (", then for each vote", ". For each vote"),
        ("you gets", "you get"),
        ("you puts", "you put"),
        ("a artifact", "an artifact"),
        ("a Assassin", "an Assassin"),
        ("a another", "another"),
        ("a enchantment", "an enchantment"),
        ("a untapped", "an untapped"),
        ("a opponent", "an opponent"),
        (" ors ", " or "),
        ("creature are", "creatures are"),
        ("creatures that shares ", "creatures that share "),
        ("Creatures that shares ", "Creatures that share "),
        ("Creatures token", "Creature tokens"),
        ("creatures token", "creature tokens"),
        ("that many color plus one", "that many colors plus one"),
        ("target any target", "any target"),
        (" to any while ", " to any target while "),
        (" to any until ", " to any target until "),
        (" to any.", " to any target."),
        (" to any,", " to any target,"),
    ] {
        while out.contains(from) {
            out = out.replace(from, to);
        }
    }
    while out.contains("target target") {
        out = out.replace("target target", "target");
    }
    while out.contains("Target target") {
        out = out.replace("Target target", "Target");
    }
    out
}

pub(crate) fn describe_inline_ability(ability: &Ability) -> String {
    describe_inline_ability_with_self_subject(ability, "this creature")
}

pub(super) fn rewrite_cost_bound_x_phrases(
    mut effects: String,
    costs: &[crate::costs::Cost],
) -> String {
    if let Some(x_phrase) = removed_counters_this_way_x_phrase(costs) {
        effects = effects.replace("where X is X", &format!("where X is {x_phrase}"));
        effects = effects.replace(
            "deals X damage to",
            &format!("deals damage equal to {x_phrase} to"),
        );
        effects = effects.replace(
            "Deal X damage to",
            &format!("Deal damage equal to {x_phrase} to"),
        );
    }
    effects
}

pub(super) fn removed_counters_this_way_x_phrase(costs: &[crate::costs::Cost]) -> Option<String> {
    fn counter_phrase(counter_type: Option<CounterType>) -> String {
        match counter_type {
            Some(counter_type) => format!("{} counters", counter_type.description()),
            None => "counters".to_string(),
        }
    }

    for cost in costs {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        if let Some(remove_among) =
            effect.downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
            && remove_among.dynamic_count
            && !remove_among.display_x
        {
            return Some(format!(
                "the number of {} removed this way",
                counter_phrase(remove_among.counter_type)
            ));
        }
        if let Some(remove_from_source) =
            effect.downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
        {
            if remove_from_source.display_x {
                continue;
            }
            return Some(format!(
                "the number of {} removed this way",
                counter_phrase(remove_from_source.counter_type)
            ));
        }
    }
    None
}

pub(crate) fn describe_inline_ability_with_self_subject(
    ability: &Ability,
    self_subject: &str,
) -> String {
    if let Some(keyword) = describe_keyword_ability(ability) {
        return keyword;
    }
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            describe_static_ability_with_subject(static_ability, self_subject)
        }
        AbilityKind::Triggered(triggered) => {
            describe_triggered_inline_ability(triggered, self_subject)
        }
        AbilityKind::Activated(activated) if activated.is_mana_ability() => {
            let mut line = String::new();
            if !activated.mana_cost.costs().is_empty() {
                line.push_str(&describe_cost_list(activated.mana_cost.costs()));
            }
            let mut payload = String::new();
            let mana_symbols = activated.mana_symbols();
            if !mana_symbols.is_empty() {
                payload.push_str("Add ");
                payload.push_str(
                    &mana_symbols
                        .iter()
                        .copied()
                        .map(describe_mana_symbol)
                        .collect::<Vec<_>>()
                        .join(""),
                );
            } else if !activated.effects.is_empty() {
                let rendered =
                    super::ast_render::describe_mana_ability_resolution_program(&activated.effects)
                        .unwrap_or_else(|| {
                            super::ast_render::describe_resolution_program(&activated.effects)
                        });
                payload.push_str(&rendered);
            }
            if !payload.is_empty() {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                line.push_str(&payload);
            }
            if let Some(prefix) = station_threshold_prefix(activated) {
                line = prefix_rendered_ability_body(line, &format!("{prefix} | "));
            } else if let Some(condition) = &activated.activation_condition {
                let clause = describe_mana_activation_condition(condition);
                if !clause.is_empty() {
                    if !line.is_empty() {
                        line.push_str(". ");
                    }
                    line.push_str(&clause);
                }
            }
            for clause in describe_mana_usage_restriction_clauses_for_activated(activated) {
                if !line.is_empty() {
                    line.push_str(". ");
                }
                line.push_str(&clause);
            }
            if line.is_empty() {
                "a mana ability".to_string()
            } else {
                normalize_ability_self_reference_surface(&line, self_subject)
            }
        }
        AbilityKind::Activated(activated) => {
            if let Some(level_up) = describe_level_up_activation(activated) {
                return level_up;
            }
            if let Some(level) = activated
                .additional_restrictions
                .iter()
                .find_map(|restriction| restriction.strip_prefix("__ironsmith_class_level:"))
            {
                let effects = activated.effects.flattened_default_effects();
                if let [effect] = effects
                    && let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>()
                    && put.counter_type == crate::CounterType::Level
                    && matches!(put.target, ChooseSpec::Source)
                {
                    return format!(
                        "{}: Level {level}",
                        describe_cost_list(activated.mana_cost.costs())
                    );
                }
            }
            let mut line = String::new();
            let mut pre = Vec::new();
            let mut trailing_x_definition = None;
            if !activated.mana_cost.costs().is_empty() {
                let (cost_text, x_definition) =
                    describe_cost_list_with_trailing_x_definition(activated.mana_cost.costs());
                pre.push(cost_text);
                trailing_x_definition = x_definition;
            }
            if !activated.choices.is_empty()
                && !(!activated.effects.is_empty()
                    && choices_are_simple_targets(&activated.choices))
            {
                pre.push(format!(
                    "choose {}",
                    activated
                        .choices
                        .iter()
                        .map(describe_choose_spec)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !pre.is_empty() {
                line.push_str(&pre.join(", "));
            }
            if !activated.effects.is_empty() {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                let effects = super::ast_render::describe_resolution_program(&activated.effects);
                let mut effects = rewrite_damage_phrases_for_permanent_abilities(
                    &effects,
                    &capitalize_first(self_subject),
                    true,
                );
                effects = rewrite_cost_bound_x_phrases(effects, activated.mana_cost.costs());
                let self_subject = if self_subject == "this spell" {
                    "this creature"
                } else {
                    self_subject
                };
                effects = replace_this_spell_self_reference(effects, self_subject);
                line.push_str(&normalize_ability_self_reference_surface(
                    &effects,
                    self_subject,
                ));
            }
            if let Some(x_definition) = trailing_x_definition {
                append_sentence_clause(&mut line, &x_definition);
            }
            let restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
            );
            if !restriction_clauses.is_empty() {
                append_activation_clause(
                    &mut line,
                    &join_activation_restriction_clauses(&restriction_clauses),
                );
            }
            for clause in describe_mana_usage_restriction_clauses_for_activated(activated) {
                append_activation_clause(&mut line, &clause);
            }
            let line = if line.is_empty() {
                "an activated ability".to_string()
            } else if activated.is_exhaust_ability() {
                format!(
                    "Exhaust — {}",
                    normalize_ability_self_reference_surface(&line, self_subject)
                )
            } else {
                normalize_ability_self_reference_surface(&line, self_subject)
            };
            let line = if let Some(prefix) = level_range_activation_prefix(activated) {
                format!("{prefix}. {line}")
            } else {
                line
            };
            if let Some(label) = activated_presentation_label(activated)
                && !line.starts_with(label)
            {
                format!("{label} — {line}")
            } else {
                line
            }
        }
    }
}

pub(super) fn activated_presentation_label(
    activated: &crate::ability::ActivatedAbility,
) -> Option<&str> {
    activated
        .additional_restrictions
        .iter()
        .find_map(|restriction| restriction.strip_prefix("__ironsmith_activation_label:"))
        .or_else(|| inferred_throw_activation_label(activated))
}

pub(super) fn describe_ninjutsu_activation(
    ability: &Ability,
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    if ability.functional_zones != [Zone::Hand]
        || !activated.choices.is_empty()
        || !matches!(activated.timing, ActivationTiming::DuringCombat)
        || !activated.additional_restrictions.is_empty()
        || !activated.activation_restrictions.is_empty()
        || activated.activation_condition.is_some()
        || !activated.mana_usage_restrictions.is_empty()
    {
        return None;
    }

    let mut mana_cost = None;
    let mut saw_ninjutsu_cost = false;
    for cost in activated.mana_cost.costs() {
        if let Some(cost) = cost.mana_cost_ref() {
            if mana_cost.is_some() {
                return None;
            }
            mana_cost = Some(cost);
            continue;
        }
        if cost.effect_ref().is_some_and(|effect| {
            effect
                .downcast_ref::<crate::effects::NinjutsuCostEffect>()
                .is_some()
        }) {
            saw_ninjutsu_cost = true;
            continue;
        }
        return None;
    }

    let effects = activated.effects.flattened_default_effects();
    if !saw_ninjutsu_cost
        || effects.len() != 1
        || effects[0]
            .downcast_ref::<crate::effects::NinjutsuEffect>()
            .is_none()
    {
        return None;
    }

    Some(format!("Ninjutsu {}", mana_cost?.to_oracle()))
}

pub(crate) fn level_range_activation_prefix(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    let range = activated
        .additional_restrictions
        .iter()
        .find_map(|restriction| restriction.strip_prefix("__ironsmith_level_range:"))?;
    let (min, max) = range.split_once(':')?;
    if max == "+" {
        Some(format!("Level {min}+"))
    } else if min == max {
        Some(format!("Level {min}"))
    } else {
        Some(format!("Level {min}-{max}"))
    }
}

pub(super) fn inferred_throw_activation_label(
    activated: &crate::ability::ActivatedAbility,
) -> Option<&'static str> {
    let unattach_tag = activated
        .mana_cost
        .costs()
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::UnattachObjectsEffect>()
                .and_then(|unattach| match unattach.objects.base() {
                    ChooseSpec::Tagged(tag) => Some(tag.as_str()),
                    _ => None,
                })
        })?;

    activated
        .effects
        .flattened_default_effects()
        .iter()
        .any(|effect| effect_is_distributed_damage_from_tag(effect, unattach_tag))
        .then_some("Throw ...")
}

pub(super) fn effect_is_distributed_damage_from_tag(effect: &Effect, tag: &str) -> bool {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_is_distributed_damage_from_tag(tagged.effect.as_ref(), tag);
    }

    effect
        .downcast_ref::<crate::effects::DealDistributedDamageEffect>()
        .is_some_and(|damage| {
            matches!(
                &damage.amount,
                crate::effect::Value::ManaValueOf(spec)
                    if matches!(spec.as_ref(), ChooseSpec::Tagged(amount_tag) if amount_tag.as_str() == tag)
            )
        })
}

pub(super) fn describe_granted_ability_phrase(ability: &Ability, self_subject: &str) -> String {
    let text = normalize_granted_triggered_ability_surface(
        describe_inline_ability_with_self_subject(ability, self_subject),
    );
    strip_redundant_granted_subject(text, self_subject)
}

pub(super) fn replace_this_spell_self_reference(text: String, subject: &str) -> String {
    const CAST_THIS_SPELL: &str = "__ironsmith_cast_this_spell__";
    let protected = text.replace("cast this spell", CAST_THIS_SPELL);
    let replaced = protected
        .replace("This spell", &capitalize_first(subject))
        .replace("this spell", &lowercase_first(subject));
    replaced.replace(CAST_THIS_SPELL, "cast this spell")
}

pub(super) fn strip_redundant_granted_subject(text: String, self_subject: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let subject_lower = self_subject.to_ascii_lowercase();
    let subject_prefix = format!("{subject_lower} ");
    if let Some(rest) = lower.strip_prefix(&subject_prefix)
        && (rest.starts_with("can't ") || rest.starts_with("cant "))
    {
        return text[subject_prefix.len()..].to_string();
    }
    text
}

pub(super) fn describe_add_mana_destination_suffix(player: &PlayerFilter) -> String {
    if matches!(player, PlayerFilter::You) {
        String::new()
    } else {
        format!(" to {}", describe_mana_pool_owner(player))
    }
}

/// When another player adds the mana, the runtime also gives THAT player the
/// color choice (`choose_mana_colors` is invoked for the resolved player), so
/// oracle idiom is subject-form: "That player adds one mana of any color they
/// choose" (Spectral Searchlight, Stadium Vendors).
pub(super) fn describe_other_player_adds_any_color(
    player: &PlayerFilter,
    amount: &Value,
    any_one_color: bool,
) -> Option<String> {
    if matches!(player, PlayerFilter::You) {
        return None;
    }
    let subject = match player {
        PlayerFilter::TaggedPlayer(_) | PlayerFilter::ChosenPlayer => "That player".to_string(),
        PlayerFilter::Target(inner) if matches!(**inner, PlayerFilter::Any) => {
            "Target player".to_string()
        }
        PlayerFilter::Target(inner) if matches!(**inner, PlayerFilter::Opponent) => {
            "Target opponent".to_string()
        }
        _ => return None,
    };
    let color_phrase = if any_one_color {
        "mana of any one color they choose"
    } else {
        "mana of any color they choose"
    };
    Some(format!(
        "{subject} adds {} {color_phrase}",
        describe_mana_amount_for_add_effect(amount)
    ))
}

pub(super) fn describe_mana_amount_for_add_effect(value: &Value) -> String {
    if let Value::Fixed(amount) = value
        && *amount >= 0
        && let Some(word) = small_number_word(*amount as u32)
    {
        return word.to_string();
    }
    describe_value(value)
}

pub(super) fn describe_static_ability_with_subject(
    static_ability: &crate::static_abilities::StaticAbility,
    subject: &str,
) -> String {
    if let Some(spec) = static_ability.conditional_spell_keyword_spec() {
        return describe_conditional_spell_keyword_spec(spec);
    }
    if static_ability.id() == crate::static_abilities::StaticAbilityId::ChooseColorAsEnters {
        let mut line = format!("As {subject} enters, choose a color");
        if let Some(excluded) = static_ability
            .color_choice_as_enters()
            .and_then(|choice| choice.excluded)
        {
            line.push_str(" other than ");
            line.push_str(excluded.name());
        }
        return line;
    }

    let rendered = static_ability.display();
    let trimmed = rendered.trim();
    if trimmed.is_empty() {
        return rendered;
    }
    let static_display = trimmed.trim_end_matches('.').to_ascii_lowercase();
    if static_ability.id() == crate::static_abilities::StaticAbilityId::GrantAbility
        && matches!(
            static_display.as_str(),
            "creatures have blocks each combat if able"
                | "all creatures have blocks each combat if able"
        )
    {
        return format!("All creatures able to block {subject} do so");
    }
    if trimmed.starts_with("This spell ") || trimmed.starts_with("this spell ") {
        return trimmed.to_string();
    }
    if trimmed.starts_with("This card ") || trimmed.starts_with("this card ") {
        return capitalize_first(trimmed);
    }
    if !subject.starts_with("this ") {
        if let Some(rest) = trimmed.strip_prefix("As this enters") {
            return format!("As {subject} enters{rest}");
        }
        if let Some(rest) = trimmed.strip_prefix("as this enters") {
            return format!("As {subject} enters{rest}");
        }
    }
    if trimmed.starts_with("Cards in ") || trimmed.starts_with("Each ") {
        if let Some(body) = trimmed.strip_suffix(" as long as it's your turn") {
            return format!("During your turn, {}", lowercase_first(body));
        }
        return trimmed.to_string();
    }

    let capitalized_subject = capitalize_first(subject);
    let mut line = if let Some(rest) = trimmed.strip_prefix("This ") {
        if let Some(subject_kind) = subject.strip_prefix("this ")
            && let Some(tail) = rest.strip_prefix(subject_kind)
            && tail.starts_with(' ')
        {
            format!("{capitalized_subject}{tail}")
        } else {
            format!("{capitalized_subject} {rest}")
        }
    } else if let Some(rest) = trimmed.strip_prefix("this ") {
        format!("{subject} {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("Attacks ") {
        format!("{capitalized_subject} attacks {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("attacks ") {
        format!("{subject} attacks {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("Enters ") {
        format!("{capitalized_subject} enters {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("enters ") {
        format!("{subject} enters {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("Escapes ") {
        format!("{capitalized_subject} escapes {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("escapes ") {
        format!("{subject} escapes {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("Can't ") {
        format!("{capitalized_subject} can't {rest}")
    } else if let Some(rest) = trimmed.strip_prefix("can't ") {
        format!("{subject} can't {rest}")
    } else {
        normalize_ability_self_reference_surface(trimmed, subject)
    };

    line = line.replace(
        " as long as PlayerHasCitysBlessing { player: You }",
        " as long as you have the city's blessing",
    );
    line = line.replace("This creature creature ", "This creature ");
    line = line.replace("this creature creature ", "this creature ");
    line = line.replace("This land land ", "This land ");
    line = line.replace("this land land ", "this land ");
    line = line.replace(
        "number of other creature artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    if let Some((ability_subject, predicate)) = line.rsplit_once(" has ") {
        let normalized = normalize_keyword_predicate_case(predicate.trim_end_matches('.'));
        let verb = if subject_text_uses_have(ability_subject) {
            "have"
        } else {
            "has"
        };
        if let Some(keyword) = normalized.strip_suffix(" as long as it's your turn") {
            return format!(
                "During your turn, {} {verb} {keyword}",
                lowercase_first(ability_subject)
            );
        }
        if normalized != predicate || verb != "has" {
            line = format!("{ability_subject} {verb} {normalized}");
        }
    }

    if let Some(rest) = line.strip_prefix("This creature has ")
        && let Some(keyword) = rest.strip_suffix(" as long as it's your turn")
    {
        return format!(
            "During your turn, this creature has {}",
            lowercase_first(keyword)
        );
    }
    if let Some(rest) = line.strip_prefix("During your turn, this creature has ")
        && rest.to_ascii_lowercase().starts_with("prevent ")
    {
        return format!("During your turn, {}", lowercase_first(rest));
    }
    if let Some(rest) = line.strip_prefix("This creature gets ")
        && let Some(pump) = rest.strip_suffix(" as long as it's your turn")
    {
        return format!("During your turn, this creature gets {pump}");
    }

    line
}

pub(super) fn describe_conditional_spell_keyword_spec(
    spec: crate::static_abilities::ConditionalSpellKeywordSpec,
) -> String {
    let keyword = match spec.keyword {
        crate::static_abilities::ConditionalSpellKeywordKind::Flash => "flash",
        crate::static_abilities::ConditionalSpellKeywordKind::Cascade => "cascade",
    };
    let metric = match spec.metric {
        crate::static_abilities::GraveyardCountMetric::CardTypes => "card types",
        crate::static_abilities::GraveyardCountMetric::ManaValues => "mana values",
    };
    let threshold =
        number_word(spec.threshold as i32).unwrap_or_else(|| spec.threshold.to_string());
    format!(
        "This spell has {keyword} as long as there are {threshold} or more {metric} among cards in your graveyard."
    )
}

pub(super) fn subject_text_uses_have(subject: &str) -> bool {
    let lower = subject.to_ascii_lowercase();
    lower.contains("creatures")
        || lower.contains("permanents")
        || lower.contains("artifacts")
        || lower.contains("enchantments")
        || lower.contains("lands")
}

pub(super) fn rewrite_capped_trigger_surface(
    triggered: &crate::ability::TriggeredAbility,
    trigger_frequency: Option<TriggerFrequencySurface>,
) -> Option<String> {
    let capped_once = matches!(
        trigger_frequency,
        Some(TriggerFrequencySurface::AbilityMaxTimesEachTurn(1))
            | Some(TriggerFrequencySurface::DoThisMaxTimesEachTurn(1))
    );
    if !capped_once {
        return None;
    }

    if let Some(zone_change) = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        && zone_change.player == crate::triggers::zone_changes::PlayerRelation::Any
        && zone_change.count_mode == crate::triggers::zone_changes::CountMode::Each
        && zone_change.from
            == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        && zone_change.to == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
        && zone_change
            .object_filter
            .card_types
            .contains(&CardType::Creature)
    {
        let subject = pluralize_noun_phrase(strip_leading_article(
            &zone_change.object_filter.description(),
        ));
        return Some(format!("Whenever one or more {subject} die"));
    }

    if let Some(sacrifice) = triggered
        .trigger
        .downcast_ref::<crate::triggers::other::PlayerSacrificesTrigger>()
    {
        let subject = pluralize_noun_phrase(strip_leading_article(&sacrifice.filter.description()));
        return match sacrifice.player {
            PlayerFilter::You => Some(format!("Whenever you sacrifice one or more {subject}")),
            PlayerFilter::Opponent => Some(format!(
                "Whenever one or more opponents sacrifice one or more {subject}"
            )),
            PlayerFilter::Any => Some(format!(
                "Whenever one or more players sacrifice one or more {subject}"
            )),
            _ => None,
        };
    }

    None
}

pub(super) fn describe_trigger_surface_with_frequency(
    triggered: &crate::ability::TriggeredAbility,
    trigger_frequency: Option<TriggerFrequencySurface>,
) -> String {
    if let Some(rewritten) = rewrite_capped_trigger_surface(triggered, trigger_frequency) {
        return rewritten;
    }

    if let Some(chapters) = triggered.trigger.saga_chapters() {
        let chapter_text = chapters
            .iter()
            .filter_map(|chapter| chapter_number_to_roman(*chapter))
            .collect::<Vec<_>>();
        if !chapter_text.is_empty() && chapter_text.len() == chapters.len() {
            return chapter_text.join(", ");
        }
    }

    if let Some(zone_change) = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
        && zone_change.player == crate::triggers::zone_changes::PlayerRelation::Any
        && zone_change.count_mode == crate::triggers::zone_changes::CountMode::Each
        && zone_change.from
            == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        && zone_change.to == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
        && zone_change.object_filter.owner == Some(PlayerFilter::You)
    {
        let mut filter = zone_change.object_filter.clone();
        filter.owner = None;
        let subject = with_indefinite_article(&filter.description());
        return format!("Whenever {subject} is put into your graveyard from the battlefield");
    }

    if triggered
        .trigger
        .downcast_ref::<crate::triggers::phase_step::BeginningOfEndStepTrigger>()
        .is_some_and(|trigger| trigger.player == PlayerFilter::Any)
        && triggered.intervening_if.as_ref().is_some_and(|condition| {
            matches!(
                condition,
                Condition::ValueComparison {
                    left: Value::Count(filter),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    ..
                } if is_source_exiled_cards_filter(filter)
            )
        })
    {
        return "At the beginning of the end step".to_string();
    }

    if triggered_unblocked_attacker_untap_remove_from_combat(triggered) {
        if let Some(trigger) = triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::AttacksAndIsntBlockedTrigger>(
        ) {
            return format!(
                "Whenever {} attacks and isn't blocked this combat",
                with_indefinite_article(&trigger.filter.description())
            );
        }
        if triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::ThisAttacksAndIsntBlockedTrigger>()
            .is_some()
        {
            return "Whenever this creature attacks and isn't blocked this combat".to_string();
        }
    }

    let mut trigger_surface = describe_this_attacks_or_dies_trigger(&triggered.trigger)
        .or_else(|| describe_this_blocks_or_becomes_blocked_by_trigger(&triggered.trigger))
        .or_else(|| describe_becomes_blocked_trigger(&triggered.trigger))
        .unwrap_or_else(|| triggered.trigger.display());
    if matches!(
        trigger_frequency,
        Some(TriggerFrequencySurface::FirstTimeThisTurn)
    ) {
        trigger_surface.push_str(" for the first time each turn");
    }
    trigger_surface
}

pub(super) fn triggered_unblocked_attacker_untap_remove_from_combat(
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    let [segment] = triggered.effects.segments.as_slice() else {
        return false;
    };
    segment.self_replacements.is_empty()
        && describe_untap_triggering_then_remove_from_combat(&segment.default_effects).is_some()
}

/// "Whenever this creature blocks or becomes blocked by a creature" — an
/// OrTrigger pairing this-blocks-object with this-becomes-blocked-by-object
/// over the same filter compacts to the oracle's joint surface.
pub(super) fn describe_this_blocks_or_becomes_blocked_by_trigger(
    trigger: &crate::triggers::Trigger,
) -> Option<String> {
    let or_trigger = trigger.downcast_ref::<crate::triggers::OrTrigger>()?;
    let [first, second] = or_trigger.triggers.as_slice() else {
        return None;
    };
    let blocks = first
        .downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::ThisBlocksObjectTrigger>())?;
    let blocked_by = first
        .downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>()
        .or_else(|| second.downcast_ref::<crate::triggers::ThisBecomesBlockedByObjectTrigger>())?;
    if blocks.blocked_filter != blocked_by.blocker_filter {
        return None;
    }
    let filter_text = with_indefinite_article(&blocks.blocked_filter.description());
    Some(format!(
        "Whenever this creature blocks or becomes blocked by {filter_text}"
    ))
}

pub(super) fn describe_becomes_blocked_trigger(
    trigger: &crate::triggers::Trigger,
) -> Option<String> {
    let trigger = trigger.downcast_ref::<crate::triggers::BecomesBlockedTrigger>()?;
    let description = trigger.filter.description();
    let subject = if description.starts_with("enchanted ")
        || description.starts_with("equipped ")
        || description.starts_with("fortified ")
    {
        description
    } else {
        with_indefinite_article(&description)
    };
    Some(format!("Whenever {subject} becomes blocked"))
}

pub(super) fn describe_this_attacks_or_dies_trigger(
    trigger: &crate::triggers::Trigger,
) -> Option<String> {
    let or_trigger = trigger.downcast_ref::<crate::triggers::OrTrigger>()?;
    if or_trigger.triggers.len() != 2 {
        return None;
    }
    let has_attacks = or_trigger.triggers.iter().any(trigger_is_this_attacks);
    let has_dies = or_trigger.triggers.iter().any(|trigger| {
        trigger
            .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
            .is_some_and(|zone_change| {
                zone_change.this_object
                    && zone_change.from
                        == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
                    && zone_change.to
                        == crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
                    && zone_change.player == crate::triggers::zone_changes::PlayerRelation::Any
            })
    });

    (has_attacks && has_dies).then(|| "Whenever this creature attacks or dies".to_string())
}

pub(super) fn describe_triggered_resolution_text(
    triggered: &crate::ability::TriggeredAbility,
    subject: &str,
    rewrite_it_deals: bool,
) -> Option<String> {
    if let Some(text) = describe_return_triggering_object_then_remove_all_abilities(triggered) {
        return Some(text);
    }

    if triggered_deals_same_damage_to_each_other_opponent(triggered) {
        return Some("it deals that much damage to each other opponent".to_string());
    }

    if let Some(text) = describe_this_attacks_target_creature_blocks_it(triggered) {
        return Some(text);
    }

    if let Some(keyword) = triggered
        .trigger
        .downcast_ref::<crate::triggers::KeywordActionTrigger>()
        && keyword.action == crate::events::KeywordActionKind::Discover
        && keyword.player == PlayerFilter::You
        && let [segment] = triggered.effects.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let [effect] = segment.default_effects.as_slice()
        && let Some(discover) = effect.downcast_ref::<crate::effects::DiscoverEffect>()
        && discover.player == PlayerFilter::You
        && matches!(
            discover.count,
            crate::effect::Value::EventValue(EventValueSpec::Amount)
        )
    {
        return Some("discover again for the same value".to_string());
    }

    if triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
        .is_some()
        && let [segment] = triggered.effects.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let [effect] = segment.default_effects.as_slice()
        && let Some(surveil) = effect.downcast_ref::<crate::effects::SurveilEffect>()
        && surveil.player == PlayerFilter::You
        && value_prefers_where_x(&surveil.count)
        && matches!(
            surveil.count.unhinted(),
            Value::EventValue(EventValueSpec::Amount)
        )
    {
        return Some(
            "surveil X, where X is the amount of damage it dealt to that player".to_string(),
        );
    }

    if triggered.effects.is_empty() {
        return None;
    }

    let mut effects = super::ast_render::describe_resolution_program(&triggered.effects);
    if effects.contains("Whenever that creature ") {
        effects = effects.replace(", draw ", ", you draw ");
    }
    effects = rewrite_damaged_player_reference_for_damage_trigger(triggered, effects);
    effects = rewrite_triggering_artifact_reference_for_tap_or_ability_trigger(triggered, effects);
    effects = rewrite_source_no_counter_resolution_surface(effects, subject);
    effects = rewrite_damage_phrases_for_permanent_abilities(&effects, subject, rewrite_it_deals);
    effects = normalize_ability_self_reference_surface(&effects, subject);
    effects = split_sacrifice_then_lose_life_resolution(effects);
    Some(effects)
}

pub(super) fn describe_this_attacks_target_creature_blocks_it(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !trigger_is_this_attacks(&triggered.trigger) {
        return None;
    }

    let mut triggering_tag = None;
    let mut target_tag = None;
    let mut must_block = None;
    for effect in triggered.effects.flattened_default_effects() {
        if let Some(tag_triggering) =
            effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        {
            triggering_tag = Some(tag_triggering.tag.clone());
            continue;
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(target_only) = tagged
                .effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
            && matches!(
                &target_only.target,
                ChooseSpec::Target(inner)
                    if matches!(
                        inner.as_ref(),
                        ChooseSpec::Object(filter)
                            if filter.card_types == [CardType::Creature]
                    )
            )
        {
            target_tag = Some(tagged.tag.clone());
            continue;
        }
        if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>()
            && cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
                &cant.restriction
        {
            must_block = Some((blockers, attacker));
        }
    }

    let triggering_tag = triggering_tag.as_ref()?;
    let target_tag = target_tag.as_ref()?;
    let (blockers, attacker) = must_block?;
    if object_filter_has_tagged_constraint(blockers, target_tag)
        && object_filter_has_tagged_constraint(attacker, triggering_tag)
    {
        return Some("target creature blocks it this turn if able".to_string());
    }
    None
}

pub(super) fn split_sacrifice_then_lose_life_resolution(text: String) -> String {
    let Some((sacrifice, life_loss)) = text.split_once(", then lose ") else {
        return text;
    };
    if !sacrifice.starts_with("Sacrifice ") || life_loss.trim().is_empty() {
        return text;
    }
    format!("{sacrifice}. You lose {}", life_loss.trim())
}

pub(super) fn rewrite_triggering_artifact_reference_for_tap_or_ability_trigger(
    triggered: &crate::ability::TriggeredAbility,
    text: String,
) -> String {
    if triggered.trigger.display()
        != "Whenever an artifact becomes tapped or a player activates an artifact's ability without {T} in its activation cost"
    {
        return text;
    }
    text.replace("that object's controller", "that artifact's controller")
}

pub(super) fn rewrite_source_no_counter_resolution_surface(
    mut text: String,
    subject: &str,
) -> String {
    if subject != "this creature" && subject != "this enchantment" {
        return text;
    }

    for needle in ["if there are no ", "If there are no "] {
        let mut search_from = 0;
        while let Some(relative_start) = text[search_from..].find(needle) {
            let start = search_from + relative_start;
            let counter_start = start + needle.len();
            let Some(relative_end) = text[counter_start..].find(" counters on it") else {
                break;
            };
            let counter_end = counter_start + relative_end;
            let counter_text = &text[counter_start..counter_end];
            if counter_text.starts_with("more ") {
                search_from = counter_end;
                continue;
            }
            let condition_prefix = if needle.starts_with('I') { "If" } else { "if" };
            let replacement = if subject == "this creature" {
                format!(
                    "{condition_prefix} this creature doesn't have {} on it",
                    with_indefinite_article(&format!("{counter_text} counter"))
                )
            } else {
                format!("{condition_prefix} this enchantment has no {counter_text} counters on it")
            };
            let replace_end = counter_end + " counters on it".len();
            text.replace_range(start..replace_end, &replacement);
            search_from = start + replacement.len();
        }
    }

    text
}

pub(super) fn describe_return_triggering_object_then_remove_all_abilities(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !triggered.choices.is_empty() || !trigger_is_this_dies(&triggered.trigger) {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }

    let [tag_triggering, return_effect, remove_abilities] = segment.default_effects.as_slice()
    else {
        return None;
    };
    let tag_triggering =
        tag_triggering.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    if tag_triggering.tag.as_str() != "triggering" {
        return None;
    }

    let return_effect = unwrap_basic_tag_wrappers(return_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if return_effect.zone != Zone::Battlefield
        || return_effect.to_top
        || return_effect.enters_tapped
        || return_effect.enters_attacking
        || return_effect.enters_face_down
        || return_effect.transfer_exiled_with_source_links
        || return_effect.battlefield_controller != ironsmith_core::BattlefieldController::Owner
        || !matches!(
            return_effect.target.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"
        )
    {
        return None;
    }

    let remove_abilities =
        remove_abilities.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if remove_abilities.until != Until::Forever
        || !matches!(
            remove_abilities.target,
            crate::continuous::EffectTarget::Source
        )
        || !matches!(
            remove_abilities.target_spec.as_ref(),
            Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "triggering"
        )
        || remove_abilities.modification.is_some()
        || !remove_abilities.additional_modifications.is_empty()
        || !matches!(
            remove_abilities.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::RemoveAllAbilities]
        )
        || remove_abilities.condition.is_some()
    {
        return None;
    }

    Some(
        "return it to the battlefield under its owner's control and it loses all abilities"
            .to_string(),
    )
}

pub(super) fn triggered_deals_same_damage_to_each_other_opponent(
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    if !triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
        .is_some_and(|trigger| trigger.player == PlayerFilter::Opponent)
    {
        return false;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return false;
    };
    let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() else {
        return false;
    };
    if !matches!(
        &for_players.filter,
        PlayerFilter::Excluding { base, excluded }
            if matches!(base.as_ref(), PlayerFilter::Opponent)
                && matches!(excluded.as_ref(), PlayerFilter::DamagedPlayer)
    ) {
        return false;
    }
    let [inner] = for_players.effects.as_slice() else {
        return false;
    };
    let Some(deal_damage) = inner.downcast_ref::<crate::effects::DealDamageEffect>() else {
        return false;
    };
    matches!(
        deal_damage.amount,
        Value::EventValue(EventValueSpec::Amount)
    ) && matches!(
        deal_damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) && !deal_damage.source_is_combat
}

pub(super) fn rewrite_damaged_player_reference_for_damage_trigger(
    triggered: &crate::ability::TriggeredAbility,
    effects: String,
) -> String {
    let references_damaged_player = triggered
        .trigger
        .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
        .is_some()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::combat::DealsDamageTrigger>()
            .is_some_and(|trigger| trigger.damaged_player.is_some());

    if !references_damaged_player {
        return effects;
    }
    effects
        .replace("the damaged player's", "their")
        .replace("The damaged player's", "Their")
        .replace("the damaged player", "that player")
        .replace("The damaged player", "That player")
}

pub(super) fn describe_unique_creature_control_leader_upkeep_control_change(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let upkeep = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()?;
    if upkeep.player != PlayerFilter::You
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let Condition::PlayerControlsMost { player, filter } = triggered.intervening_if.as_ref()?
    else {
        return None;
    };
    if *player != PlayerFilter::Any || filter.card_types != [CardType::Creature] {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let control = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if control.target != crate::continuous::EffectTarget::Source
        || control.until != Until::Forever
        || control.modification.is_some()
        || !control.additional_modifications.is_empty()
        || control.condition.is_some()
        || !matches!(
            control.runtime_modifications.as_slice(),
            [
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    PlayerFilter::IteratedPlayer
                )
            ]
        )
    {
        return None;
    }

    Some(
        "At the beginning of your upkeep, if a player controls more creatures than each other player, the player who controls the most creatures gains control of this creature"
            .to_string(),
    )
}

pub(super) fn is_oath_of_ghouls_player(player: &PlayerFilter) -> bool {
    matches!(player, PlayerFilter::Active | PlayerFilter::IteratedPlayer)
}

pub(super) fn describe_oath_of_ghouls_triggered_ability(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let upkeep = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()?;
    if upkeep.player != PlayerFilter::Any
        || !triggered.choices.is_empty()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let Condition::AnOpponentHasFewerThanPlayer { player, filter } =
        triggered.intervening_if.as_ref()?
    else {
        return None;
    };
    if !is_oath_of_ghouls_player(player) || !oath_of_ghouls_creature_graveyard_filter(filter, None)
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [may_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let decider = may.decider.as_ref()?;
    if !is_oath_of_ghouls_player(decider) {
        return None;
    }
    let [return_effect] = may.effects.as_slice() else {
        return None;
    };
    let return_from_gy =
        return_effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    if return_from_gy.random || !exact_count(&return_from_gy.target.count(), 1) {
        return None;
    }
    let ChooseSpec::Object(return_filter) = return_from_gy.target.base() else {
        return None;
    };
    if !oath_of_ghouls_creature_graveyard_filter(return_filter, Some(decider)) {
        return None;
    }

    Some(
        "At the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand"
            .to_string(),
    )
}

pub(super) fn describe_flurry_copy_exile_suspend_triggered_ability(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !matches!(
        triggered.presentation_label.as_ref(),
        Some(PresentationLabel::AbilityWord(label)) if label.eq_ignore_ascii_case("Flurry")
    ) || triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
    {
        return None;
    }
    let spell_cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()?;
    if spell_cast.caster != PlayerFilter::You
        || spell_cast.during_turn.is_some()
        || spell_cast.min_spells_this_turn.is_some()
        || spell_cast.exact_spells_this_turn != Some(2)
        || spell_cast.from_not_hand
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [
        tag_effect,
        copy_effect,
        move_effect,
        put_effect,
        conditional_effect,
    ] = segment.default_effects.as_slice()
    else {
        return None;
    };
    let tag_triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let tag = &tag_triggering.tag;

    let tagged_copy = copy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if tagged_copy.tag.as_str() != "__copied_stack_object__" {
        return None;
    }
    let with_id = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()?;
    let copy = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if copy.copier != PlayerFilter::You
        || copy.count != Value::Fixed(1)
        || !copy.removed_supertypes.is_empty()
        || !matches!(&copy.target, ChooseSpec::Tagged(found) if found == tag)
    {
        return None;
    }

    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Exile
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || !matches!(&move_to_zone.target, ChooseSpec::Tagged(found) if found == tag)
    {
        return None;
    }

    let put = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != CounterType::Time
        || put.amount != Value::Fixed(4)
        || put.target_count.is_some()
        || put.distributed
        || !matches!(&put.target, ChooseSpec::Tagged(found) if found == tag)
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
        || !condition_is_tagged_object_without_suspend(&conditional.condition, tag)
    {
        return None;
    }
    let apply = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply_grants_suspend_to_tag(apply, tag) {
        return None;
    }

    Some(
        "Flurry — Whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend"
            .to_string(),
    )
}

pub(super) fn describe_tap_lands_sharing_mana_types_with_triggering_land(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::TapForManaTrigger>()?;
    if trigger.player != PlayerFilter::Any
        || trigger.filter.zone != Some(Zone::Battlefield)
        || trigger.filter.controller != Some(PlayerFilter::Opponent)
        || trigger.filter.card_types.as_slice() != [CardType::Land]
    {
        return None;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let tap = effect.downcast_ref::<crate::effects::TapEffect>()?;
    let ChooseSpec::All(filter) = tap.target.base() else {
        return None;
    };
    if filter.zone != Some(Zone::Battlefield)
        || filter.controller != Some(PlayerFilter::IteratedPlayer)
        || filter.card_types.as_slice() != [CardType::Land]
    {
        return None;
    }

    Some(
        "Whenever a land an opponent controls is tapped for mana, tap all lands that player controls that could produce any type of mana that land could produce"
            .to_string(),
    )
}

pub(super) fn describe_triggered_inline_ability(
    triggered: &crate::ability::TriggeredAbility,
    self_subject: &str,
) -> String {
    if let Some(rendered) = describe_backup_keyword(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_unique_creature_control_leader_upkeep_control_change(triggered)
    {
        return rendered;
    }
    if let Some(rendered) = describe_oath_of_ghouls_triggered_ability(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_flurry_copy_exile_suspend_triggered_ability(triggered) {
        return rendered;
    }
    if let Some(rendered) = describe_tap_lands_sharing_mana_types_with_triggering_land(triggered) {
        return rendered;
    }

    let (intervening_condition, trigger_frequency) = triggered
        .intervening_if
        .as_ref()
        .map(split_trigger_intervening_if)
        .unwrap_or((None, None));
    let mut intervening_condition = if trigger_is_state_based(&triggered.trigger) {
        None
    } else {
        intervening_condition
    };
    intervening_condition = intervening_condition
        .and_then(|condition| remove_presentation_label_chosen_option(&condition, triggered));
    let mut line = describe_trigger_surface_with_frequency(triggered, trigger_frequency);
    if triggered_deals_same_damage_to_each_other_opponent(triggered) {
        line = line.replace("combat damage to a player", "combat damage to an opponent");
    }
    if matches!(intervening_condition, Some(Condition::YourTurn))
        && line.to_ascii_lowercase().ends_with("becomes tapped")
    {
        line.push_str(" during your turn");
        intervening_condition = None;
    }
    if let Some(condition) = intervening_condition {
        line.push_str(", if ");
        line.push_str(&describe_trigger_intervening_condition(
            &condition,
            triggered,
            Some(self_subject),
        ));
    }

    let mut clauses = Vec::new();
    if !triggered.choices.is_empty()
        && !(!triggered.effects.is_empty() && choices_are_simple_targets(&triggered.choices))
    {
        clauses.push(format!(
            "choose {}",
            triggered
                .choices
                .iter()
                .map(describe_choose_spec)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if let Some(effects) = describe_triggered_resolution_text(triggered, self_subject, true) {
        clauses.push(effects);
    }

    if !clauses.is_empty() {
        if clauses.len() == 1 {
            let only = clauses[0].trim_start();
            if let Some(rest) = only.strip_prefix("If ") {
                line.push_str(", if ");
                line.push_str(rest.trim_start());
            } else if let Some(rest) = only.strip_prefix("if ") {
                line.push_str(", if ");
                line.push_str(rest.trim_start());
            } else if only.contains(" if ") && !only.contains(". ") && !only.contains(": ") {
                line.push_str(", ");
                line.push_str(&lowercase_first(only));
            } else if triggered.presentation_label.is_some() {
                line.push_str(", ");
                line.push_str(&lowercase_first(only));
            } else if triggered.trigger.saga_chapters().is_some() {
                line.push_str(" — ");
                line.push_str(only);
            } else {
                line.push_str(": ");
                line.push_str(only);
            }
        } else {
            line.push_str(": ");
            line.push_str(&clauses.join(": "));
        }
    }

    match trigger_frequency {
        Some(TriggerFrequencySurface::AbilityMaxTimesEachTurn(max)) => {
            if max == 1 {
                line.push_str(". This ability triggers only once each turn");
            } else if max == 2 {
                line.push_str(". This ability triggers only twice each turn");
            } else {
                line.push_str(". This ability triggers only ");
                line.push_str(&max.to_string());
                line.push_str(" times each turn");
            }
        }
        Some(TriggerFrequencySurface::DoThisMaxTimesEachTurn(max)) => {
            if max == 1 {
                line.push_str(". Do this only once each turn");
            } else if max == 2 {
                line.push_str(". Do this only twice each turn");
            } else {
                line.push_str(". Do this only ");
                line.push_str(&max.to_string());
                line.push_str(" times each turn");
            }
        }
        _ => {}
    }

    line = normalize_redundant_short_name_etb_surface(line, triggered, self_subject);
    line = normalize_modal_named_source_etb_surface(line, triggered, self_subject);
    line = normalize_spellcast_trigger_mana_value_surface(triggered, line);
    if triggered_deals_same_damage_to_each_other_opponent(triggered) {
        line = line.replace("combat damage to a player", "combat damage to an opponent");
    }
    if triggered_has_you_difference_draw(triggered) {
        line = line.replace(
            "you draw cards equal to the difference",
            "draw cards equal to the difference",
        );
    }
    if triggered.presentation_label.is_none()
        && line.starts_with("Whenever you cast a spell that targets this creature")
    {
        line = format!("Heroic — {line}");
    }
    if triggered.presentation_label.is_none()
        && line.starts_with("Whenever another creature you control enters")
    {
        line = format!("Alliance — {line}");
    }
    apply_triggered_presentation_label(triggered, line)
}

pub(super) fn describe_trigger_intervening_condition(
    condition: &Condition,
    triggered: &crate::ability::TriggeredAbility,
    self_subject: Option<&str>,
) -> String {
    if let Condition::SourceHasNoCounter(counter_type) = condition {
        if let Some(subject) = self_subject {
            if subject == "this creature" {
                return format!(
                    "this creature doesn't have {} on it",
                    with_indefinite_article(&format!("{} counter", counter_type.description()))
                );
            }
            if subject == "this enchantment" {
                return format!(
                    "this enchantment has no {} counters on it",
                    counter_type.description()
                );
            }
        }
    }
    if let Condition::TriggeringObjectHadCounters {
        counter_type,
        min_count,
    } = condition
        && trigger_is_this_dies(&triggered.trigger)
    {
        return format!(
            "it had {} on it",
            triggering_object_counter_phrase(*counter_type, *min_count)
        );
    }
    if let Condition::PlayerCardsInHandOrFewer { player, count } = condition
        && *player == PlayerFilter::You
        && triggered_has_you_difference_draw(triggered)
    {
        let threshold = count + 1;
        if threshold > 0 {
            let count_text = number_word(threshold).unwrap_or_else(|| threshold.to_string());
            return format!("you have fewer than {count_text} cards in hand");
        }
    }
    if matches!(condition, Condition::SourceIsInZone(Zone::Graveyard))
        && let Some(subject) = source_return_from_graveyard_subject(triggered)
    {
        return format!("{subject} is in your graveyard");
    }
    describe_condition(condition)
}

pub(super) fn triggering_object_counter_phrase(
    counter_type: CounterType,
    min_count: u32,
) -> String {
    let counter = counter_type.description();
    if min_count == 1 {
        return with_indefinite_article(&format!("{counter} counter"));
    }
    format!("{min_count} or more {counter} counters")
}

pub(super) fn triggered_has_you_difference_draw(
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .any(|effect| {
            effect
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .is_some_and(|draw| {
                    draw.player == PlayerFilter::You
                        && draw.count.has_surface_hint(ValueSurfaceHint::Difference)
                })
        })
}

pub(super) fn source_return_from_graveyard_subject(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .find_map(source_return_from_graveyard_subject_in_effect)
}

pub(super) fn source_return_from_graveyard_subject_in_effect(effect: &Effect) -> Option<String> {
    if let Some(return_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        && matches!(return_to_battlefield.target.unhinted(), ChooseSpec::Source)
    {
        return Some(describe_choose_spec(&return_to_battlefield.target));
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return if_effect
            .then
            .iter()
            .chain(if_effect.else_.iter())
            .find_map(source_return_from_graveyard_subject_in_effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return source_return_from_graveyard_subject_in_effect(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return source_return_from_graveyard_subject_in_effect(&with_id.effect);
    }
    None
}

pub(super) fn describe_delayed_coin_flip_result(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot || schedule.start_next_turn || schedule.until_end_of_turn {
        return None;
    }
    let trigger_text = schedule.trigger.display().to_ascii_lowercase();
    if !trigger_text.contains("beginning of") || !trigger_text.contains("end step") {
        return None;
    }

    let effects: &[Effect] = &schedule.effects;
    let [flip_effect, branch_effect] = effects else {
        return None;
    };
    let flip_with_id = flip_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let flip_coin = flip_with_id
        .effect
        .downcast_ref::<crate::effects::FlipCoinEffect>()?;
    if flip_coin.player != PlayerFilter::You {
        return None;
    }

    let if_effect = branch_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != flip_with_id.id
        || !matches!(if_effect.predicate, EffectPredicate::DidNotHappen)
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [choose_effect, sacrifice_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    let sacrifice_text = describe_choose_then_sacrifice(choose, sacrifice)?;
    if !sacrifice_text.starts_with("you sacrifice ") {
        return None;
    }
    if !choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str().starts_with("targeted_")
    }) {
        return None;
    }

    let object = if choose.filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else if choose.filter.card_types.contains(&CardType::Artifact) {
        "artifact"
    } else if choose.filter.card_types.contains(&CardType::Enchantment) {
        "enchantment"
    } else if choose.filter.card_types.contains(&CardType::Planeswalker) {
        "planeswalker"
    } else if choose.filter.card_types.contains(&CardType::Land) {
        "land"
    } else {
        "permanent"
    };

    Some(format!(
        "Flip a coin at the beginning of the next end step. If you lose the flip, sacrifice that {object}"
    ))
}

pub(super) fn describe_delayed_each_player_discard_hand_return_exiled(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot || schedule.start_next_turn || schedule.until_end_of_turn {
        return None;
    }
    let trigger_text = schedule.trigger.display().to_ascii_lowercase();
    if !trigger_text.contains("beginning of") || !trigger_text.contains("end step") {
        return None;
    }

    let effects = schedule.effects.flattened_default_effects();
    let Some(for_players_effect) = effects.first() else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let (discard_effect, return_effect) = match (for_players.effects.as_slice(), effects) {
        ([discard_effect, return_effect], [_]) => (discard_effect, return_effect),
        ([discard_effect], [_, return_effect]) => (discard_effect, return_effect),
        _ => return None,
    };
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardHandEffect>()?;
    if discard.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let return_spec = if let Some(return_to_hand) =
        return_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()
    {
        &return_to_hand.spec
    } else {
        let move_to_hand = return_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        if move_to_hand.zone != Zone::Hand {
            return None;
        }
        &move_to_hand.target
    };
    if !choose_spec_is_all_exiled_cards_tagged_this_way(return_spec) {
        return None;
    }

    Some(
        "At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way"
            .to_string(),
    )
}

pub(super) fn choose_spec_is_all_exiled_cards_tagged_this_way(spec: &ChooseSpec) -> bool {
    match spec.base() {
        ChooseSpec::All(filter) if filter.zone == Some(Zone::Exile) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && tag_is_exiled_this_way(&constraint.tag)
            })
        }
        ChooseSpec::Tagged(tag) => tag_is_exiled_this_way(tag),
        _ => false,
    }
}

pub(super) fn tag_is_exiled_this_way(tag: &TagKey) -> bool {
    tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        || tag.as_str().starts_with("exiled_")
        || crate::cards::is_sentence_helper_tag(tag.as_str(), "exiled")
}

pub(super) fn describe_next_spell_delayed_trigger(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
    additional: bool,
) -> Option<String> {
    if !schedule.one_shot || !schedule.until_end_of_turn || schedule.start_next_turn {
        return None;
    }
    let trigger_display =
        cleanup_decompiled_text(schedule.trigger.display().trim().trim_end_matches('.'));
    let trigger_action = trigger_display
        .strip_prefix("Whenever you ")
        .or_else(|| trigger_display.strip_prefix("When you "))?
        .trim();
    if !trigger_action.starts_with("cast ") && !trigger_action.contains(" activate ") {
        return None;
    }
    let trigger_action = trigger_action
        .strip_suffix(" this turn")
        .unwrap_or(trigger_action)
        .to_string();
    let trigger_can_activate_ability = trigger_action.contains("activate ");
    let mut delayed_text = lowercase_first(&describe_effect_list(&schedule.effects));
    if let Some(rest) = delayed_text.strip_prefix("copy it") {
        let copied_object = if trigger_can_activate_ability {
            "spell or ability"
        } else {
            "spell"
        };
        delayed_text = format!("copy that {copied_object}{rest}");
    }
    let copying_twice = delayed_text.contains(" 2 time");
    delayed_text = delayed_text
        .replace(
            "copy that spell or ability 2 time(s)",
            "copy that spell or ability twice",
        )
        .replace(
            "copy that spell or ability 2 times",
            "copy that spell or ability twice",
        )
        .replace(
            "copy that spell or ability 2 time",
            "copy that spell or ability twice",
        )
        .replace("copy that spell 2 time(s)", "copy that spell twice")
        .replace("copy that spell 2 times", "copy that spell twice")
        .replace("copy that spell 2 time", "copy that spell twice");
    if copying_twice {
        delayed_text = delayed_text
            .replace(
                ", then you may choose new targets for the copy",
                ". You may choose new targets for the copies",
            )
            .replace(
                ". You may choose new targets for the copy",
                ". You may choose new targets for the copies",
            );
    }
    if additional {
        delayed_text =
            delayed_text.replacen("copy that spell", "copy that spell an additional time", 1);
    }
    Some(format!(
        "When you next {trigger_action} this turn, {delayed_text}"
    ))
}

pub(super) fn describe_play_card_this_way_delayed_trigger(
    schedule: &crate::effects::ScheduleDelayedTriggerEffect,
) -> Option<String> {
    if !schedule.one_shot || !schedule.until_end_of_turn || schedule.start_next_turn {
        return None;
    }
    let either = schedule
        .trigger
        .downcast_ref::<crate::triggers::OrTrigger>()?;
    let [first, second] = either.triggers.as_slice() else {
        return None;
    };
    let (spell, land) = if let (Some(spell), Some(land)) = (
        first.downcast_ref::<crate::triggers::SpellCastTrigger>(),
        second.downcast_ref::<crate::triggers::PlayerPlaysLandTrigger>(),
    ) {
        (spell, land)
    } else if let (Some(land), Some(spell)) = (
        first.downcast_ref::<crate::triggers::PlayerPlaysLandTrigger>(),
        second.downcast_ref::<crate::triggers::SpellCastTrigger>(),
    ) {
        (spell, land)
    } else {
        return None;
    };
    let spell_filter = spell.filter.as_ref()?;
    if spell.caster != PlayerFilter::You
        || spell.during_turn.is_some()
        || spell.min_spells_this_turn.is_some()
        || spell.exact_spells_this_turn.is_some()
        || spell.from_not_hand
        || land.player != PlayerFilter::You
        || spell_filter != &land.filter
        || !spell_filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return None;
    }
    let delayed = lowercase_first(&describe_effect_list(&schedule.effects));
    Some(format!("When you play a card this way, {delayed}"))
}

pub(super) fn describe_unless_pays_lose_game_payment(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let unless_pays = effect.downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    if unless_pays.player != PlayerFilter::You {
        return None;
    }
    let [lose_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    let lose_game = lose_effect.downcast_ref::<crate::effects::LoseTheGameEffect>()?;
    if lose_game.player != PlayerFilter::You {
        return None;
    }
    let display = describe_total_cost_payment(&unless_pays.cost);
    Some(display.strip_prefix("Pay ").unwrap_or(&display).to_string())
}

pub(super) fn normalize_modal_named_source_etb_surface(
    line: String,
    triggered: &crate::ability::TriggeredAbility,
    _subject: &str,
) -> String {
    let Some(zone_trigger) = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
    else {
        return line;
    };
    if zone_trigger.this_object
        || zone_trigger.to
            != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || zone_trigger.object_filter.subtypes.len() != 1
        || !line.contains("choose one or both")
    {
        return line;
    }
    let subtype = format!("{:?}", zone_trigger.object_filter.subtypes[0]);
    let prefix = format!("Whenever a {subtype} enters,");
    if let Some(start) = line.find(&prefix) {
        if start == 0 || line[..start].ends_with(": ") {
            let rest = &line[start + prefix.len()..];
            return format!("{}When this creature enters,{rest}", &line[..start]);
        }
    }
    line
}
