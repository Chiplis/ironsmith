use super::*;

pub(super) fn value_has_surface_hint(value: &Value, hint: ValueSurfaceHint) -> bool {
    value.has_surface_hint(hint)
}

pub(super) fn value_prefers_where_x(value: &Value) -> bool {
    value_has_surface_hint(value, ValueSurfaceHint::WhereXIs)
}

pub(super) fn value_prefers_equal_to(value: &Value) -> bool {
    value_has_surface_hint(value, ValueSurfaceHint::EqualTo)
}

pub(super) fn describe_consult_stop_text(
    selection: &str,
    stop_rule: &crate::effects::ConsultTopOfLibraryStopRule,
    max_exposed: Option<&Value>,
) -> String {
    let match_text = match stop_rule {
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1)) => {
            selection.to_string()
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count)
            if value_prefers_where_x(count) =>
        {
            if let Some(basis) = describe_where_x_basis(count) {
                format!("X {}, where X is {basis}", pluralize_noun_phrase(selection))
            } else {
                format!(
                    "{} {}",
                    describe_value(count),
                    pluralize_noun_phrase(selection)
                )
            }
        }
        crate::effects::ConsultTopOfLibraryStopRule::MatchCount(count) => format!(
            "{} {}",
            describe_value(count),
            pluralize_noun_phrase(selection)
        ),
    };
    if let Some(max_exposed) = max_exposed {
        format!(
            "{match_text} or {} cards, whichever comes first",
            describe_value(max_exposed)
        )
    } else {
        match_text
    }
}

pub(super) const STATION_THRESHOLD_RESTRICTION_PREFIX: &str = "__ironsmith_station_threshold:";

pub(super) fn station_threshold_prefix(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    let threshold = activated
        .additional_restrictions
        .iter()
        .find_map(|restriction| {
            restriction
                .strip_prefix(STATION_THRESHOLD_RESTRICTION_PREFIX)
                .and_then(|value| value.parse::<i32>().ok())
        })?;
    let crate::effect::Condition::ValueComparison {
        left,
        operator,
        right,
    } = activated.activation_condition.as_ref()?
    else {
        return None;
    };
    if !matches!(left, Value::CountersOnSource(crate::CounterType::Charge))
        || !matches!(
            operator,
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual
        )
    {
        return None;
    }
    let Value::Fixed(condition_threshold) = right else {
        return None;
    };
    if threshold != *condition_threshold {
        return None;
    }
    Some(format!("{threshold}+"))
}

pub(super) fn prefix_rendered_ability_body(line: String, prefix: &str) -> String {
    if let Some((heading, body)) = line.split_once(": ")
        && is_render_heading_prefix(heading)
    {
        return format!("{heading}: {prefix}{body}");
    }
    format!("{prefix}{line}")
}

pub(super) fn describe_pay_any_energy_amount(
    pay_any_energy: &crate::effects::PayAnyEnergyEffect,
) -> Option<&'static str> {
    match pay_any_energy.min_amount {
        0 => Some("any amount of {E}"),
        1 => Some("one or more {E}"),
        _ => None,
    }
}

pub(super) fn is_target_permanent_or_suspended_card(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Target(inner) = spec else {
        return false;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return false;
    };
    let permanent = crate::target::ObjectFilter::permanent();
    let suspended = crate::target::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Exile)
        .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
        .with_counter_type(crate::object::CounterType::Time);
    filter.any_of.len() == 2
        && filter.zone.is_none()
        && filter.any_of.iter().any(|arm| arm == &permanent)
        && filter.any_of.iter().any(|arm| arm == &suspended)
}

pub(super) fn is_source_damaged_death_graveyard_card_spec(spec: &ChooseSpec) -> bool {
    if !spec.count().is_single() {
        return false;
    }
    let ChooseSpec::Object(filter) = spec.base() else {
        return false;
    };
    filter.zone == Some(Zone::Graveyard)
        && filter.card_types.as_slice() == [CardType::Creature]
        && filter.entered_graveyard_from_battlefield_this_turn
        && filter.dealt_damage_by_source_this_turn.is_some()
}

pub(super) fn is_time_travel_object_set(spec: &ChooseSpec) -> bool {
    let ChooseSpec::All(filter) = spec else {
        return false;
    };
    let permanent = crate::target::ObjectFilter::permanent()
        .you_control()
        .with_counter_type(crate::object::CounterType::Time);
    let suspended = crate::target::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Exile)
        .owned_by(crate::target::PlayerFilter::You)
        .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
        .with_counter_type(crate::object::CounterType::Time);
    filter.any_of.len() == 2
        && filter.zone.is_none()
        && filter.any_of.iter().any(|arm| arm == &permanent)
        && filter.any_of.iter().any(|arm| arm == &suspended)
}

pub(super) fn describe_discard_hand_add_mana_draw_sequence(effects: &[&Effect]) -> Option<String> {
    let [discard_effect, mana_effect, draw_effect] = effects else {
        return None;
    };
    let discard_with_id = discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let discard = discard_with_id
        .effect
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    let mana_with_id = mana_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let mana = mana_with_id
        .effect
        .downcast_ref::<crate::effects::AddScaledManaEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;

    let Value::Count(count_filter) = &discard.count else {
        return None;
    };
    let hand_filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        ..Default::default()
    };
    if discard.player != PlayerFilter::You
        || discard.random
        || discard.any_number
        || count_filter != &hand_filter
        || discard.card_filter.as_ref() != Some(&hand_filter)
        || mana.player != PlayerFilter::You
        || draw.player != PlayerFilter::You
    {
        return None;
    }

    if !matches!(
        &mana.amount,
        Value::EffectMetric {
            effect_id,
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        } if *effect_id == discard_with_id.id
    ) {
        return None;
    }
    let Value::EffectValueOffset(draw_effect_id, draw_offset) = &draw.count else {
        return None;
    };
    if *draw_effect_id != mana_with_id.id || *draw_offset < 0 {
        return None;
    }

    let mana_text = mana
        .mana
        .iter()
        .copied()
        .map(describe_mana_symbol)
        .collect::<Vec<_>>()
        .join("");
    if mana_text.is_empty() {
        return None;
    }

    let draw_text = match *draw_offset {
        0 => "that many cards".to_string(),
        1 => "that many cards plus one".to_string(),
        offset => format!("that many cards plus {offset}"),
    };

    Some(format!(
        "Discard all the cards in your hand. Add {mana_text} for each card discarded this way, then draw {draw_text}"
    ))
}

pub(super) fn value_is_discarded_count_for_effect(
    value: &Value,
    id: crate::effect::EffectId,
) -> bool {
    let value = value.unhinted();
    matches!(
        value,
        Value::EffectMetric {
            effect_id,
            source: crate::effect::EffectMetricSource::Outcome,
            metric: crate::effect::EffectMetric::Count,
        } if *effect_id == id
    ) || value_is_affected_count_for_effect(value, id)
}

pub(super) fn plural_discard_subject(filter: &PlayerFilter) -> Option<&'static str> {
    match filter {
        PlayerFilter::Any => Some("Each player"),
        PlayerFilter::Opponent => Some("Each opponent"),
        PlayerFilter::NotYou => Some("Each other player"),
        PlayerFilter::You => Some("You"),
        _ => None,
    }
}

pub(super) fn describe_discard_then_draw_for_discarded(effects: &[Effect]) -> Option<String> {
    let [discard_effect, draw_effect] = effects else {
        return None;
    };
    let with_id = discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You
        || !value_is_discarded_count_for_effect(&draw.count, with_id.id)
    {
        return None;
    }

    let (subject, discard) = if let Some(discard) = with_id
        .effect
        .downcast_ref::<crate::effects::DiscardEffect>()
    {
        (plural_discard_subject(&discard.player)?, discard)
    } else {
        let for_players = with_id
            .effect
            .downcast_ref::<crate::effects::ForPlayersEffect>()?;
        if for_players.starting_with_controller {
            return None;
        }
        let [iterated_effect] = for_players.effects.as_slice() else {
            return None;
        };
        let discard = iterated_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
        if discard.player != PlayerFilter::IteratedPlayer {
            return None;
        }
        (describe_for_players_subject(&for_players.filter)?, discard)
    };

    if discard.any_number {
        return None;
    }

    let random_suffix = if discard.random { " at random" } else { "" };
    Some(format!(
        "{subject} {} {}{random_suffix}. You draw a card for each card discarded this way",
        if subject == "You" {
            "discard"
        } else {
            "discards"
        },
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    ))
}

pub(super) fn describe_for_players_may_discard_then_draw_if_discarded(
    effects: &[Effect],
) -> Option<String> {
    let [may_discard_effect, draw_if_discarded_effect] = effects else {
        return None;
    };
    let may_discard_with_id = may_discard_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let for_players = may_discard_with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.starting_with_controller {
        return None;
    }
    let subject = describe_for_players_subject(&for_players.filter)?;
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
    if draw_if_discarded.condition != may_discard_with_id.id
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

pub(super) fn describe_look_reorder_then_may_shuffle(effects: &[Effect]) -> Option<String> {
    let [look_effect, reorder_effect, may_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reorder = reorder_effect.downcast_ref::<crate::effects::ReorderLibraryTopEffect>()?;
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let (target_only, shuffle_effect) = match may.effects.as_slice() {
        [shuffle_effect] => (None, shuffle_effect),
        [target_effect, shuffle_effect] => {
            let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            (Some(target_only), shuffle_effect)
        }
        _ => return None,
    };
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if look.reveal
        || reorder.tag != look.tag
        || !matches!(
            &look.player,
            PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Any)
        )
        || shuffle.player != look.player
        || !matches!(may.decider, None | Some(PlayerFilter::You))
    {
        return None;
    }
    if let Some(target_only) = target_only
        && target_only.target != ChooseSpec::target_player()
    {
        return None;
    }

    let look_text = describe_effect(look_effect)
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{look_text}, then put them back in any order. You may have that player shuffle"
    ))
}

pub(super) fn may_action_this_way_phrase(action: &str) -> Option<String> {
    let action = action.trim().trim_end_matches('.');
    let lower = action.to_ascii_lowercase();
    let phrase = if lower == "draw a card" {
        "drew a card this way".to_string()
    } else if let Some(rest) = lower.strip_prefix("draw ") {
        format!("drew {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("discard ") {
        format!("discarded {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("gain ") {
        format!("gained {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("lose ") {
        format!("lost {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("sacrifice ") {
        format!("sacrificed {rest} this way")
    } else if let Some(rest) = lower.strip_prefix("pay ") {
        format!("paid {rest} this way")
    } else {
        return None;
    };
    Some(phrase)
}

pub(super) fn describe_for_players_happened_followup(effects: &[Effect]) -> Option<String> {
    let mut followup = describe_effect_list(effects)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if followup.is_empty() {
        return None;
    }
    if let Some(rest) = followup.strip_prefix("that player ") {
        followup = rest.to_string();
    } else if let Some(rest) = followup.strip_prefix("you ") {
        followup = rest.to_string();
    }
    followup = normalize_third_person_verb_phrase(&followup);
    Some(lowercase_first(&followup))
}

pub(super) fn describe_for_players_may_action(
    _filter: &PlayerFilter,
    effects: &[Effect],
) -> Option<String> {
    let mut action = describe_effect_list(effects);
    if let Some(rest) = action.strip_prefix("that player ") {
        action = rest.to_string();
    }
    if let Some(rest) = action.strip_prefix("you ") {
        action = rest.to_string();
    }
    let action = normalize_you_verb_phrase(&action);
    Some(lowercase_may_clause(&action))
}

pub(super) fn describe_sequential_any_player_may_action(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if !for_players.starting_with_controller
        || !for_players.stop_after_first_happened
        || for_players.filter != PlayerFilter::Any
    {
        return None;
    }
    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let action = describe_for_players_may_action(&for_players.filter, &may.effects)?;
    Some(format!("Any player may {action}"))
}

pub(super) fn describe_for_players_may_happened_sequence(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.effects.len() != 2 {
        return None;
    }
    let with_id = for_players.effects[0].downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let if_effect = for_players.effects[1].downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let subject = describe_for_players_subject(&for_players.filter)?.to_string();
    let each_player =
        strip_leading_article(&describe_for_each_player_filter(&for_players.filter)).to_string();
    let action = describe_for_players_may_action(&for_players.filter, &may.effects)?;
    let did_action = may_action_this_way_phrase(&action)?;
    let followup = describe_for_players_happened_followup(&if_effect.then)?;
    Some(format!(
        "{subject} may {action}, then each {each_player} who {did_action} {followup}"
    ))
}

pub(super) fn describe_with_id_then_for_players_if_happened(
    with_id: &crate::effects::WithIdEffect,
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let antecedent = with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if antecedent.filter != for_players.filter
        || antecedent.starting_with_controller
        || for_players.starting_with_controller
        || for_players.effects.len() != 1
    {
        return None;
    }
    let if_effect = for_players.effects[0].downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let (subject, each_player, action) = describe_for_players_may_clause(antecedent)?;
    let did_action = may_action_this_way_phrase(&action)?;
    let followup = describe_for_players_happened_followup(&if_effect.then)?;
    Some(format!(
        "{subject} may {action}, then each {each_player} who {did_action} {followup}"
    ))
}

pub(super) fn apply_grants_chosen_color_protection(
    apply: &crate::effects::ApplyContinuousEffect,
) -> bool {
    if apply.until != Until::EndOfTurn
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || apply.condition.is_some()
    {
        return false;
    }
    let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
        return false;
    };
    matches!(
        ability.protection_from(),
        Some(crate::ability::ProtectionFrom::ChosenColor)
    )
}

pub(super) fn is_radiance_target_creature_spec(spec: &ChooseSpec) -> bool {
    let ChooseSpec::Target(inner) = spec else {
        return false;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return false;
    };
    filter.card_types.as_slice() == [CardType::Creature]
        && filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && !filter.other
        && filter.tagged_constraints.is_empty()
        && filter.any_of.is_empty()
}

pub(super) fn shared_color_other_creature_filter(filter: &ObjectFilter) -> Option<&crate::TagKey> {
    if filter.card_types.as_slice() != [CardType::Creature]
        || filter.zone != Some(Zone::Battlefield)
        || filter.other
    {
        return None;
    }
    let shares = filter.tagged_constraints.iter().find(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
    })?;
    let excludes_same = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == shares.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    });
    excludes_same.then_some(&shares.tag)
}

pub(super) fn describe_choose_color_target_and_shared_color_protection(
    effects: &[Effect],
) -> Option<String> {
    let [choose_effect, target_grant_effect, shared_grant_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseColorEffect>()?;
    if choose.chooser != PlayerFilter::You {
        return None;
    }

    let target_grant = unwrap_basic_tag_wrappers(target_grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let shared_grant = unwrap_basic_tag_wrappers(shared_grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !apply_grants_chosen_color_protection(target_grant)
        || !apply_grants_chosen_color_protection(shared_grant)
    {
        return None;
    }
    if !matches!(target_grant.target, crate::continuous::EffectTarget::Source)
        || !target_grant
            .target_spec
            .as_ref()
            .is_some_and(is_radiance_target_creature_spec)
    {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(shared_filter) = &shared_grant.target else {
        return None;
    };
    shared_color_other_creature_filter(shared_filter)?;

    Some(
        "Radiance — Choose a color. Target creature and each other creature that shares a color with it gain protection from the chosen color until end of turn"
            .to_string(),
    )
}

pub(super) fn apply_grants_inline_ability_until_eot(
    apply: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if apply.until != Until::EndOfTurn
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || apply.condition.is_some()
    {
        return None;
    }
    let Some(crate::continuous::Modification::AddAbilityGeneric(ability)) = &apply.modification
    else {
        return None;
    };
    Some(
        describe_inline_ability(ability)
            .trim()
            .trim_end_matches('.')
            .to_string(),
    )
}

pub(super) fn describe_target_and_shared_color_inline_ability_grant(
    effects: &[Effect],
) -> Option<String> {
    let [target_grant_effect, shared_grant_effect] = effects else {
        return None;
    };
    let target_grant = unwrap_basic_tag_wrappers(target_grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let shared_grant = unwrap_basic_tag_wrappers(shared_grant_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let ability_text = apply_grants_inline_ability_until_eot(target_grant)?;
    if apply_grants_inline_ability_until_eot(shared_grant)? != ability_text {
        return None;
    }
    if !matches!(target_grant.target, crate::continuous::EffectTarget::Source)
        || !target_grant
            .target_spec
            .as_ref()
            .is_some_and(is_radiance_target_creature_spec)
    {
        return None;
    };
    let crate::continuous::EffectTarget::Filter(shared_filter) = &shared_grant.target else {
        return None;
    };
    shared_color_other_creature_filter(shared_filter)?;

    Some(format!(
        "Radiance — Until end of turn, target creature and each other creature that shares a color with it gain \"{ability_text}.\""
    ))
}

pub(super) fn describe_choose_phase_then_skip_chosen_this_turn(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, conditional_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseNamedOptionEffect>()?;
    let options = choose
        .options
        .iter()
        .map(|option| option.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if options != ["draw step", "main phase", "combat phase"] {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::SourceChosenOption(draw_option) = &conditional.condition else {
        return None;
    };
    if !draw_option.eq_ignore_ascii_case("draw step") || conditional.if_true.len() != 1 {
        return None;
    }
    let draw_skip = conditional.if_true[0].downcast_ref::<crate::effects::SkipDrawStepEffect>()?;
    if draw_skip.player != choose.chooser || conditional.if_false.len() != 1 {
        return None;
    }

    let main_conditional =
        conditional.if_false[0].downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::SourceChosenOption(main_option) = &main_conditional.condition
    else {
        return None;
    };
    if !main_option.eq_ignore_ascii_case("main phase")
        || main_conditional.if_true.len() != 1
        || main_conditional.if_false.len() != 1
    {
        return None;
    }
    let main_skip = main_conditional.if_true[0]
        .downcast_ref::<crate::effects::SkipMainPhasesThisTurnEffect>()?;
    let combat_skip = main_conditional.if_false[0]
        .downcast_ref::<crate::effects::SkipCombatPhasesThisTurnEffect>()?;
    if main_skip.player != choose.chooser || combat_skip.player != choose.chooser {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let skip_subject = if chooser == "that player" {
        "The player".to_string()
    } else {
        capitalize_first(&chooser)
    };
    Some(format!(
        "{chooser} {choose_verb} draw step, main phase, or combat phase. {skip_subject} skips each instance of the chosen step or phase this turn"
    ))
}

pub(super) fn title_case_vote_option(option: &str) -> String {
    option
        .split_whitespace()
        .enumerate()
        .map(|(idx, word)| {
            if idx > 0 && matches!(word, "a" | "an" | "and" | "of" | "or" | "the") {
                return word.to_string();
            }
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn describe_search_basic_land_battlefield_tapped_shuffle(
    effects: &[Effect],
) -> Option<String> {
    let [search_effect, shuffle_effect] = effects else {
        return None;
    };
    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sequence = search_with_id
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, put_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_each = put_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put_effect] = put_each.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = shuffle.then.as_slice() else {
        return None;
    };
    let shuffle_library = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if !choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.owner != Some(PlayerFilter::You)
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || !choose.filter.supertypes.contains(&Supertype::Basic)
        || !choose.count.is_single()
        || put_each.tag != choose.tag
        || !matches!(put.target, ChooseSpec::Iterated)
        || !put.tapped
        || put.controller != PlayerFilter::You
        || shuffle.condition != search_with_id.id
        || shuffle.predicate != EffectPredicate::Happened
        || !shuffle.else_.is_empty()
        || shuffle_library.player != PlayerFilter::You
    {
        return None;
    }

    Some(
        "search your library for a basic land card and put it onto the battlefield tapped. If you search your library this way, shuffle"
            .to_string(),
    )
}

pub(super) fn describe_may_search_basic_land_then_shuffle(
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = search_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    let actor = may.decider.as_ref()?;
    let search_effects = if let [sequence_effect] = may.effects.as_slice()
        && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        may.effects.as_slice()
    };
    let [choose_effect, put_effect] = search_effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_each = put_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let if_effect = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = if_effect.then.as_slice() else {
        return None;
    };
    let shuffle_library = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if if_effect.condition != search_with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
        || choose.chooser != *actor
        || choose.filter.owner.as_ref() != Some(actor)
        || shuffle_library.player != *actor
        || !choose.is_search
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || !choose.filter.supertypes.contains(&Supertype::Basic)
    {
        return None;
    }

    let mut compact =
        describe_search_choose_for_each(choose, put_each, Some(shuffle_library), false)?;
    compact = compact
        .replace("its controller's library", "their library")
        .replace("that object's controller's library", "their library")
        .replace("its owner's library", "their library")
        .replace("that object's owner's library", "their library");
    let rest = compact.strip_prefix("Search ")?;
    Some(format!(
        "{} may search {}",
        capitalize_first(&describe_player_filter(actor)),
        lowercase_first(rest)
    ))
}

pub(super) fn same_search_player_filter(left: &PlayerFilter, right: &PlayerFilter) -> bool {
    left == right
        || matches!(
            (left, right),
            (PlayerFilter::ControllerOf(_), PlayerFilter::ControllerOf(_))
                | (PlayerFilter::OwnerOf(_), PlayerFilter::OwnerOf(_))
        )
}

pub(super) fn normalize_actor_owned_search_origin(actor: &PlayerFilter, text: String) -> String {
    match actor {
        PlayerFilter::ControllerOf(_) => text
            .replace("its controller's library", "their library")
            .replace("that object's controller's library", "their library"),
        PlayerFilter::OwnerOf(_) => text
            .replace("its owner's library", "their library")
            .replace("that object's owner's library", "their library"),
        _ => text,
    }
}

pub(super) fn describe_may_search_sequence_then_shuffle(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    let actor = may.decider.as_ref()?;
    let [sequence_effect] = may.effects.as_slice() else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, for_each_effect, shuffle_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let search_owner = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);

    if !same_search_player_filter(&choose.chooser, actor)
        || !same_search_player_filter(search_owner, actor)
        || !same_search_player_filter(&shuffle.player, actor)
    {
        return None;
    }

    let compact = describe_search_choose_for_each(choose, for_each, Some(shuffle), false)?;
    let compact = normalize_actor_owned_search_origin(actor, compact);
    let rest = compact.strip_prefix("Search ")?;
    Some(format!(
        "{} may search {}",
        capitalize_first(&describe_player_filter(actor)),
        lowercase_first(rest)
    ))
}

pub(super) fn describe_wrapped_search_for_each_then_conditional_shuffle(
    effects: &[&Effect],
) -> Option<String> {
    let [search_effect, for_each_effect, shuffle_effect, ..] = effects else {
        return None;
    };
    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let choose = search_with_id
        .effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (_, for_each) = for_each_tagged_for_compaction(for_each_effect)?;
    let if_effect = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = if_effect.then.as_slice() else {
        return None;
    };
    let shuffle = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if if_effect.condition != search_with_id.id
        || if_effect.predicate != EffectPredicate::SearchedLibrary
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    describe_search_choose_for_each(choose, for_each, Some(shuffle), false)
}

pub(super) fn unwrap_tags_for_from_the_ashes_shape(effect: &Effect) -> &Effect {
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_tags_for_from_the_ashes_shape(&tag_all.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_tags_for_from_the_ashes_shape(&tagged.effect);
    }
    effect
}

pub(super) fn is_destroy_all_nonbasic_lands_effect(effect: &Effect) -> bool {
    let Some(destroy) = unwrap_tags_for_from_the_ashes_shape(effect)
        .downcast_ref::<crate::effects::DestroyEffect>()
    else {
        return false;
    };
    let ChooseSpec::All(filter) = &destroy.spec else {
        return false;
    };
    filter.card_types.as_slice() == [CardType::Land]
        && filter.excluded_supertypes.contains(&Supertype::Basic)
}

pub(super) fn describe_destroyed_land_controller_basic_search_then_player_shuffle(
    destroy_effect: &Effect,
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    if !is_destroy_all_nonbasic_lands_effect(destroy_effect) {
        return None;
    }

    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let destroyed_loop = search_with_id
        .effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [may_effect] = destroyed_loop.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [search_sequence_effect] = may.effects.as_slice() else {
        return None;
    };
    let search_sequence =
        search_sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, put_effect] = search_sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_each = put_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put_effect] = put_each.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = shuffle.then.as_slice() else {
        return None;
    };
    let shuffle_library = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if may.decider != Some(PlayerFilter::IteratedPlayer)
        || !choose.is_search
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || !choose.filter.supertypes.contains(&Supertype::Basic)
        || !choose.count.is_single()
        || put_each.tag != choose.tag
        || !matches!(put.target, ChooseSpec::Iterated)
        || put.tapped
        || put.controller != PlayerFilter::IteratedPlayer
        || shuffle.condition != search_with_id.id
        || shuffle.predicate != EffectPredicate::Happened
        || !shuffle.else_.is_empty()
        || shuffle_library.player != PlayerFilter::IteratedPlayer
    {
        return None;
    }

    Some("Destroy all nonbasic lands. For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield. Then each player who searched their library this way shuffles".to_string())
}

pub(super) fn describe_destroyed_land_basic_search_then_player_shuffle(
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let search_effects = if let Some(destroyed_loop) = search_with_id
        .effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect>(
    ) {
        destroyed_loop.effects.as_slice()
    } else if let Some(destroyed_loop) = search_with_id
        .effect
        .downcast_ref::<crate::effects::ForEachObject>()
    {
        destroyed_loop.effects.as_slice()
    } else {
        return None;
    };

    if describe_optional_basic_land_search_effects(search_effects).is_none() {
        return None;
    }
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [shuffle_then] = shuffle.then.as_slice() else {
        return None;
    };
    let shuffle_library = shuffle_then.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if shuffle.condition != search_with_id.id
        || shuffle.predicate != EffectPredicate::Happened
        || !shuffle.else_.is_empty()
        || shuffle_library.player != PlayerFilter::IteratedPlayer
    {
        return None;
    }

    Some("For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield. Then each player who searched their library this way shuffles".to_string())
}

pub(super) fn player_filter_is_target_opponentish(player: &PlayerFilter) -> bool {
    matches!(player, PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Opponent) || player_filter_is_target_opponentish(inner))
}

pub(super) fn search_target_opponent_library_to_graveyard_sequence(
    sequence: &crate::effects::SequenceEffect,
) -> Option<()> {
    let [choose_effect, move_each_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let move_each = move_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [move_effect] = move_each.effects.as_slice() else {
        return None;
    };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;

    if !choose.is_search
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose
            .filter
            .owner
            .as_ref()
            .is_none_or(|owner| !player_filter_is_target_opponentish(owner))
        || choose.filter.card_types.as_slice() != [CardType::Creature]
        || choose.count.min != 0
        || choose.count.max != Some(3)
        || move_each.tag != choose.tag
        || !matches!(move_to_zone.target, ChooseSpec::Iterated)
        || move_to_zone.zone != Zone::Graveyard
    {
        return None;
    }

    Some(())
}

pub(super) fn describe_destroy_then_search_target_opponent_to_graveyard_then_shuffle(
    destroy_effect: &Effect,
    search_effect: &Effect,
    shuffle_effect: &Effect,
) -> Option<String> {
    let sequence = search_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    search_target_opponent_library_to_graveyard_sequence(sequence)?;
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !player_filter_is_target_opponentish(&shuffle.player) {
        return None;
    }

    let destroy_text = describe_effect(destroy_effect)
        .trim_end_matches('.')
        .to_string();
    if destroy_text.is_empty() {
        return None;
    }
    let mut search_text = lowercase_first(describe_effect(search_effect).trim_end_matches('.'));
    search_text = search_text.replace("up to 3", "up to three");
    search_text = search_text.replace("target opponent's graveyard", "their graveyard");
    if let Some(replaced) = search_text.strip_suffix(", put them into their graveyard") {
        search_text = format!("{replaced} and put them into their graveyard");
    }

    Some(format!(
        "{destroy_text}, then {search_text}. Then that player shuffles"
    ))
}

pub(super) fn describe_for_each_tagged_optional_basic_land_search(
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    describe_optional_basic_land_search_effects(for_each.effects.as_slice())
}

pub(super) fn describe_optional_basic_land_search_effects(effects: &[Effect]) -> Option<String> {
    let search_effects = if let [may_effect] = effects
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
    {
        may.effects.as_slice()
    } else {
        effects
    };
    let search_effects = if let [sequence_effect] = search_effects
        && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        search_effects
    };
    let (choose_effect, put_effect) = if let [may_effect, put_effect] = search_effects
        && let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>()
    {
        let may_effects = if let [sequence_effect] = may.effects.as_slice()
            && let Some(sequence) = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()
        {
            sequence.effects.as_slice()
        } else {
            may.effects.as_slice()
        };
        let [choose_effect] = may_effects else {
            return None;
        };
        (choose_effect, put_effect)
    } else {
        let [choose_effect, put_effect] = search_effects else {
            return None;
        };
        (choose_effect, put_effect)
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let put_each = put_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [put_effect] = put_each.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    let puts_searched_object = matches!(put.target, ChooseSpec::Iterated)
        || matches!(&put.target, ChooseSpec::Tagged(tag) if tag == &choose.tag);

    if !choose.is_search
        || choose.zone != Some(Zone::Library)
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.card_types.as_slice() != [CardType::Land]
        || !choose.filter.supertypes.contains(&Supertype::Basic)
        || !choose.count.is_single()
        || put_each.tag != choose.tag
        || !puts_searched_object
        || put.tapped
    {
        return None;
    }

    Some("For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield".to_string())
}

pub(super) fn describe_council_dilemma_named_vote_sequence(effects: &[Effect]) -> Option<String> {
    let [vote_effect, repeat_effects @ ..] = effects else {
        return None;
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || repeat_effects.len() != options.len()
        || options
            .iter()
            .any(|option| !option.effects_per_vote.is_empty())
    {
        return None;
    }

    let mut clauses = Vec::new();
    for (option, repeat_effect) in options.iter().zip(repeat_effects.iter()) {
        let repeat = repeat_effect.downcast_ref::<crate::effects::RepeatEffectsEffect>()?;
        let Value::VoteCount(repeat_option) = &repeat.count else {
            return None;
        };
        if !repeat_option.eq_ignore_ascii_case(&option.name) {
            return None;
        }

        let body = describe_search_basic_land_battlefield_tapped_shuffle(&repeat.effects)
            .unwrap_or_else(|| {
                let mut body = describe_effect_list(&repeat.effects)
                    .trim()
                    .trim_end_matches('.')
                    .to_string();
                body = body.replace(
                    ", put it onto the battlefield tapped",
                    " and put it onto the battlefield tapped",
                );
                body
            });
        clauses.push(format!(
            "For each {} vote, {}",
            title_case_vote_option(&option.name),
            lowercase_first(&body)
        ));
    }

    let option_names = options
        .iter()
        .map(|option| title_case_vote_option(&option.name))
        .collect::<Vec<_>>();
    let mut text = format!(
        "Council's dilemma — Starting with you, each player votes for {}",
        join_with_or(&option_names)
    );
    if !clauses.is_empty() {
        text.push_str(". ");
        text.push_str(&clauses.join(". "));
    }
    Some(text)
}

pub(super) fn describe_named_vote_per_vote_effects(
    vote: &crate::effects::VoteEffect,
) -> Option<String> {
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || options
            .iter()
            .any(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let option_names = options
        .iter()
        .map(|option| option.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut clauses = Vec::new();
    for option in options {
        let body = describe_effect_list(&option.effects_per_vote)
            .trim()
            .trim_end_matches('.')
            .to_string();
        if body.is_empty() {
            return None;
        }
        clauses.push(format!(
            "{} for each {} vote",
            lowercase_first(&body),
            option.name.to_ascii_lowercase()
        ));
    }

    let combined = compact_repeated_vote_clause_subjects(&clauses);
    Some(format!(
        "Council's dilemma — Starting with you, each player votes for {}. {}",
        join_with_or(&option_names),
        capitalize_first(&combined)
    ))
}

pub(super) fn describe_secret_named_vote_repeat_followup(
    option: &str,
    repeat: &crate::effects::RepeatEffectsEffect,
) -> Option<String> {
    let option = option.to_ascii_lowercase();
    if let [effect] = repeat.effects.as_slice()
        && let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>()
        && draw.player == PlayerFilter::You
        && draw.count == Value::Fixed(1)
    {
        return Some(format!(
            "You draw cards equal to the number of {option} votes"
        ));
    }

    let mut body = describe_effect_list(&repeat.effects)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if body.is_empty() {
        return None;
    }
    if let Some(rest) = body.strip_prefix("Deal ") {
        body = format!("This deals {rest}");
    }
    Some(format!(
        "{} for each {option} vote",
        capitalize_first(&body)
    ))
}

pub(super) fn describe_secret_named_vote_intervening_followup(effect: &Effect) -> Option<String> {
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()
        && choose.chooser == PlayerFilter::You
        && choose.filter == PlayerFilter::Opponent
        && choose.random
    {
        return Some("Then choose an opponent at random".to_string());
    }
    None
}

pub(super) fn describe_secret_named_vote_followup_sequence(effects: &[Effect]) -> Option<String> {
    let [vote_effect, rest @ ..] = effects else {
        return None;
    };
    if rest.is_empty() {
        return None;
    }
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let crate::effects::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if !vote.secret
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || options
            .iter()
            .any(|option| !option.effects_per_vote.is_empty())
    {
        return None;
    }

    let option_names = options
        .iter()
        .map(|option| option.name.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut parts = vec![format!(
        "Secret council — Each player secretly votes for {}, then those votes are revealed",
        join_with_or(&option_names)
    )];

    for effect in rest {
        if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
            let Value::VoteCount(option) = &repeat.count else {
                return None;
            };
            if !option_names
                .iter()
                .any(|known| known.eq_ignore_ascii_case(option))
            {
                return None;
            }
            parts.push(describe_secret_named_vote_repeat_followup(option, repeat)?);
            continue;
        }
        parts.push(describe_secret_named_vote_intervening_followup(effect)?);
    }

    Some(parts.join(". "))
}

pub(super) fn compact_repeated_vote_clause_subjects(clauses: &[String]) -> String {
    for prefix in [
        "each opponent ",
        "each player ",
        "each other player ",
        "you ",
    ] {
        if clauses.iter().all(|clause| clause.starts_with(prefix)) {
            let rests = clauses
                .iter()
                .map(|clause| clause[prefix.len()..].to_string())
                .collect::<Vec<_>>();
            return format!("{prefix}{}", join_with_and(&rests));
        }
    }
    join_with_and(clauses)
}

pub(super) fn describe_planeswalk_chaos_vote_sequence(effects: &[&Effect]) -> Option<String> {
    let [vote_effect, planeswalk_effect, chaos_effect] = effects else {
        return None;
    };
    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let ironsmith_core::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() != 2
        || options[0].name != "planeswalk"
        || options[1].name != "chaos"
        || !options
            .iter()
            .all(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let planeswalk = planeswalk_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let chaos = chaos_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !planeswalk.if_false.is_empty()
        || !chaos.if_false.is_empty()
        || !matches!(
            &planeswalk.condition,
            Condition::VoteOptionGetsMoreVotes(option) if option == "planeswalk"
        )
        || !matches!(
            &chaos.condition,
            Condition::VoteOptionGetsMoreVotesOrTied(option) if option == "chaos"
        )
    {
        return None;
    }

    let [planeswalk_action] = planeswalk.if_true.as_slice() else {
        return None;
    };
    let [chaos_action] = chaos.if_true.as_slice() else {
        return None;
    };
    let planeswalk_emit =
        planeswalk_action.downcast_ref::<crate::effects::EmitKeywordActionEffect>()?;
    let chaos_emit = chaos_action.downcast_ref::<crate::effects::EmitKeywordActionEffect>()?;
    if planeswalk_emit.action != crate::events::KeywordActionKind::Planeswalk
        || planeswalk_emit.amount != 1
        || chaos_emit.action != crate::events::KeywordActionKind::ChaosEnsues
        || chaos_emit.amount != 1
    {
        return None;
    }

    Some(
        "Will of the Planeswalkers — Starting with you, each player votes for planeswalk or chaos. If planeswalk gets more votes, planeswalk. If chaos gets more votes or the vote is tied, chaos ensues"
            .to_string(),
    )
}

pub(super) fn describe_named_vote_conditional_sequence(effects: &[&Effect]) -> Option<String> {
    let [vote_effect, followups @ ..] = effects else {
        return None;
    };
    if followups.is_empty() {
        return None;
    }

    let vote = vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;
    let ironsmith_core::VoteChoice::NamedOptions(options) = &vote.choice else {
        return None;
    };
    if vote.secret
        || vote.controller_extra_votes != 0
        || vote.controller_optional_extra_votes != 0
        || options.len() < 2
        || !options
            .iter()
            .all(|option| option.effects_per_vote.is_empty())
    {
        return None;
    }

    let option_names = options
        .iter()
        .map(|option| option.name.to_string())
        .collect::<Vec<_>>();
    let mut clauses = Vec::new();
    for effect in followups {
        let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
        if !conditional.if_false.is_empty() {
            return None;
        }
        let condition_option = match &conditional.condition {
            Condition::VoteOptionGetsMoreVotes(option)
            | Condition::VoteOptionGetsMoreVotesOrTied(option) => option,
            _ => return None,
        };
        if !option_names
            .iter()
            .any(|option| option.eq_ignore_ascii_case(condition_option))
        {
            return None;
        }
        let body = describe_effect_clause_list(&conditional.if_true)
            .unwrap_or_else(|| describe_effect_list(&conditional.if_true))
            .trim()
            .trim_end_matches('.')
            .to_string();
        if body.is_empty() {
            return None;
        }
        clauses.push(format!(
            "If {}, {}",
            describe_condition(&conditional.condition),
            lowercase_first(&body)
        ));
    }

    let mut text = format!(
        "Will of the council — Starting with you, each player votes for {}",
        join_with_or(&option_names)
    );
    text.push_str(". ");
    text.push_str(&clauses.join(". "));
    Some(text)
}

pub(super) fn is_you_and_target_opponent_participants(
    choice: &crate::effects::SecretChoiceEffect,
) -> bool {
    matches!(
        choice.participants.as_slice(),
        [PlayerFilter::You, PlayerFilter::Target(inner)] if **inner == PlayerFilter::Opponent
    )
}

pub(super) fn describe_secret_choice_match_sequence(effects: &[Effect]) -> Option<String> {
    let [choice_effect, conditional_effect] = effects else {
        return None;
    };
    let choice = choice_effect.downcast_ref::<crate::effects::SecretChoiceEffect>()?;
    if !is_you_and_target_opponent_participants(choice) {
        return None;
    }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !matches!(conditional.condition, Condition::SecretChoicesMatch)
        || conditional.if_true.is_empty()
        || conditional.if_false.is_empty()
    {
        return None;
    }

    let option_names = choice.options.clone();
    let if_true = describe_sacrifice_then_put_source_exiled_into_hands(&conditional.if_true)
        .or_else(|| describe_effect_clause_list(&conditional.if_true))
        .unwrap_or_else(|| describe_effect_list(&conditional.if_true))
        .trim()
        .trim_end_matches('.')
        .to_string();
    let if_false = describe_effect_clause_list(&conditional.if_false)
        .unwrap_or_else(|| describe_effect_list(&conditional.if_false))
        .trim()
        .trim_end_matches('.')
        .to_string();
    if if_true.is_empty() || if_false.is_empty() {
        return None;
    }

    Some(format!(
        "You and target opponent each secretly choose {}. Then those choices are revealed. If they match, {}. Otherwise, {}",
        join_with_or(&option_names),
        lowercase_first(&if_true),
        lowercase_first(&if_false)
    ))
}

pub(super) fn describe_sacrifice_then_put_source_exiled_into_hands(
    effects: &[Effect],
) -> Option<String> {
    let [sacrifice_effect, move_effect] = effects else {
        return None;
    };
    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target, ChooseSpec::Source) {
        return None;
    }
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::All(filter) = move_to_zone.target.base() else {
        return None;
    };
    if move_to_zone.zone != Zone::Hand
        || filter.zone != Some(Zone::Exile)
        || !filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == ironsmith_core::SOURCE_EXILED_TAG)
    {
        return None;
    }
    Some(
        "Sacrifice this artifact and put all cards exiled with it into their owners' hands"
            .to_string(),
    )
}

pub(super) fn library_position_from_top_text(position: &Value, one_as_on_top: bool) -> String {
    if let Value::Fixed(value) = position
        && let Ok(value) = u32::try_from(*value)
        && let Some(ordinal) = ordinal_word(value)
    {
        if value == 1 && one_as_on_top {
            "on top".to_string()
        } else {
            format!("{ordinal} from the top")
        }
    } else {
        format!("{} from the top", describe_value(position))
    }
}

/// Compile a list of effects to human-readable text (for stack ability display).
pub fn compile_effect_list(effects: &[Effect]) -> String {
    normalize_compile_effect_list_surface(&describe_effect_list(effects))
}

pub(crate) fn describe_spell_mastery_reanimation_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program
        .segments
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let effects = program
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .collect::<Vec<_>>();
    describe_spell_mastery_reanimation_effects(&effects)
}

pub(super) fn unwrap_basic_tag_wrappers(effect: &Effect) -> &Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_basic_tag_wrappers(&with_id.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_basic_tag_wrappers(&tag_all.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_basic_tag_wrappers(&tagged.effect);
    }
    effect
}

pub(super) fn direct_wrapped_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| &tagged.tag)
}

pub(super) fn wrapped_effect_tag(effect: &Effect) -> Option<&crate::TagKey> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return wrapped_effect_tag(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(&tagged.tag);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(&tag_all.tag);
    }
    None
}

pub(super) fn describe_power_damage_exchange_clause(effects: &[Effect]) -> Option<String> {
    fn power_value_references(value: &Value, spec: &ChooseSpec) -> bool {
        matches!(
            value.unhinted(),
            Value::PowerOf(power_spec) if power_spec.unhinted() == spec.unhinted()
        )
    }

    fn demonstrative_reference_for_target(spec: &ChooseSpec) -> Option<&'static str> {
        let ChooseSpec::Target(inner) = spec.unhinted() else {
            return None;
        };
        let ChooseSpec::Object(filter) = inner.unhinted() else {
            return None;
        };
        if filter.card_types.contains(&CardType::Creature) {
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

    let [first_effect, tagged_target_effect, reciprocal_effect] = effects else {
        return None;
    };
    let first_exec = first_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    let first_damage = first_exec
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !power_value_references(&first_damage.amount, &first_exec.source) {
        return None;
    }

    let tagged = tagged_target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if first_damage.target.unhinted() != target_only.target.unhinted() {
        return None;
    }

    let reciprocal_exec =
        reciprocal_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(&reciprocal_exec.source, ChooseSpec::Tagged(tag) if tag == &tagged.tag) {
        return None;
    }
    let reciprocal_damage = reciprocal_exec
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        reciprocal_damage.amount.unhinted(),
        Value::PowerOf(power_spec)
            if matches!(power_spec.unhinted(), ChooseSpec::Tagged(tag) if tag == &tagged.tag)
    ) {
        return None;
    }
    if reciprocal_damage.target.unhinted() != first_exec.source.unhinted() {
        return None;
    }

    let source_text = describe_choose_spec(&first_exec.source);
    let target_text = describe_choose_spec(&first_damage.target);
    let reciprocal_source = demonstrative_reference_for_target(&target_only.target)?;
    Some(format!(
        "{source_text} deals damage equal to its power to {target_text}, then {reciprocal_source} deals damage equal to its power to {source_text}"
    ))
}

pub(super) fn describe_copy_tagged_then_may_cast_copy(effects: &[Effect]) -> Option<String> {
    let [copy_effect, may_effect] = effects else {
        return None;
    };

    let copy_spell =
        unwrap_basic_tag_wrappers(copy_effect).downcast_ref::<crate::effects::CopySpellEffect>()?;
    if copy_spell.count != Value::Fixed(1) || !copy_spell.removed_supertypes.is_empty() {
        return None;
    }
    let ChooseSpec::Tagged(copy_tag) = &copy_spell.target else {
        return None;
    };

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast_tagged = unwrap_basic_tag_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if !cast_tagged.as_copy || &cast_tagged.tag != copy_tag {
        return None;
    }

    let verb = if cast_tagged.allow_land {
        "play"
    } else {
        "cast"
    };
    let mut text = format!("Copy it. You may {verb} the copy");
    if cast_tagged.without_paying_mana_cost {
        text.push_str(" without paying its mana cost");
    }
    if let Some(reduction) = cast_tagged.cost_reduction.as_ref() {
        text.push_str(&format!(
            ". That copy costs {} less to cast",
            reduction.to_oracle()
        ));
    }
    Some(text)
}

pub(super) fn may_draws_one_for_you(effect: &Effect) -> Option<()> {
    let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, Some(PlayerFilter::You)) || may.effects.len() != 1 {
        return None;
    }
    let draw = may.effects[0].downcast_ref::<crate::effects::DrawCardsEffect>()?;
    (draw.player == PlayerFilter::You && draw.count == Value::Fixed(1)).then_some(())
}

pub(super) fn describe_may_draw_then_source_enchanted_additional_draw(
    effects: &[Effect],
) -> Option<String> {
    let [first, second] = effects else {
        return None;
    };
    may_draws_one_for_you(first)?;

    let conditional = second.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.condition != crate::effect::Condition::SourceIsEnchanted
        || !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
    {
        return None;
    }
    may_draws_one_for_you(&conditional.if_true[0])?;

    Some(format!(
        "You may draw a card. You may draw an additional card if {}",
        lowercase_first(&describe_condition(&conditional.condition))
    ))
}

pub(super) fn describe_spell_mastery_reanimation_effects(effects: &[&Effect]) -> Option<String> {
    let [move_effect, conditional_effect] = effects else {
        return None;
    };

    let move_tag = direct_wrapped_effect_tag(move_effect)?;
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
        || move_to_zone.enters_tapped
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
    if target_filter.zone != Some(Zone::Graveyard)
        || !target_filter.card_types.contains(&CardType::Creature)
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let Condition::ValueComparison {
        left: Value::Count(condition_filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(2),
    } = &conditional.condition
    else {
        return None;
    };
    if condition_filter.zone != Some(Zone::Graveyard)
        || condition_filter.owner != Some(PlayerFilter::You)
        || condition_filter.card_types != vec![CardType::Instant, CardType::Sorcery]
    {
        return None;
    }

    let put_counters = unwrap_basic_tag_wrappers(&conditional.if_true[0])
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put_counters.distributed
        || put_counters.target_count.is_some()
        || !matches!(&put_counters.target, ChooseSpec::Tagged(tag) if tag == move_tag)
    {
        return None;
    }

    let move_text = describe_effect(move_effect).replace(" in a graveyard", " from a graveyard");
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
        "{move_text}. Spell mastery — If there are two or more instant and/or sorcery cards in \
         your graveyard, that creature enters with {counter_suffix} on it"
    ))
}

pub(super) fn normalize_compile_effect_list_surface(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if lower
        == "each opponent chooses a creature card, then put it onto the battlefield under your control"
    {
        return "Each opponent chooses a creature card in their graveyard. Put those cards onto the battlefield under your control".to_string();
    }
    if lower
        == "destroy all nonbasic lands. for each land destroyed this way, its controller may search its controller's library for a basic land card. for each tagged 'searched' object, put them onto the battlefield. if you do, shuffle that player's library"
    {
        return "Destroy all nonbasic lands. For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield. Then each player who searched their library this way shuffles".to_string();
    }
    line.to_string()
}

pub(super) fn describe_gain_life_then_distribute_creatures_died_counters(
    effects: &[Effect],
) -> Option<String> {
    let [gain_effect, put_effect] = effects else {
        return None;
    };
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if !matches!(gain.amount, Value::CreaturesDiedThisTurn)
        || !matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
    {
        return None;
    }

    let put = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !put.distributed
        || !matches!(put.amount, Value::CreaturesDiedThisTurn)
        || put.counter_type != crate::object::CounterType::PlusOnePlusOne
    {
        return None;
    }
    let ChooseSpec::WithCount(inner, count) = &put.target else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    if count.min != 0
        || count.max.is_some()
        || put.target_count != Some(*count)
        || filter.zone != Some(Zone::Battlefield)
        || filter.controller != Some(PlayerFilter::You)
        || !filter.card_types.contains(&CardType::Creature)
    {
        return None;
    }

    Some(format!(
        "You gain that much life and distribute that many {} counters among {}",
        describe_counter_type(put.counter_type),
        describe_choose_spec(&put.target)
    ))
}

pub(super) fn is_effect_count_reference(
    value: &Value,
    effect_id: Option<crate::effect::EffectId>,
) -> bool {
    match value {
        Value::SurfaceHinted { value, .. } => is_effect_count_reference(value, effect_id),
        Value::EffectValue(id) => effect_id.map_or(true, |expected| *id == expected),
        Value::EventValue(EventValueSpec::Amount) => true,
        Value::EffectMetric {
            effect_id: id,
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            ..
        } => effect_id.map_or(true, |expected| *id == expected),
        Value::PendingEffectMetric {
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            ..
        } => effect_id.is_none(),
        _ => false,
    }
}

pub(super) fn effect_count_reference_offset(
    value: &Value,
    effect_id: Option<crate::effect::EffectId>,
) -> Option<i32> {
    match value {
        Value::SurfaceHinted { value, .. } => effect_count_reference_offset(value, effect_id),
        Value::EffectValueOffset(id, offset)
            if effect_id.map_or(true, |expected| *id == expected) =>
        {
            Some(*offset)
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset) => Some(*offset),
        Value::EffectMetricOffset {
            effect_id: id,
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            offset,
            ..
        } if effect_id.map_or(true, |expected| *id == expected) => Some(*offset),
        Value::PendingEffectMetricOffset {
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            offset,
            ..
        } if effect_id.is_none() => Some(*offset),
        _ => None,
    }
}

pub(super) fn describe_each_object_subject(target: &ChooseSpec) -> Option<String> {
    match target {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            Some(format!("Each {}", filter.description()))
        }
        _ => None,
    }
}

pub(super) fn possessive_subject(subject: &str) -> String {
    if subject.ends_with('s') {
        format!("{subject}'")
    } else {
        format!("{subject}'s")
    }
}

pub(super) fn possessive_object_subject(subject: &str) -> String {
    match subject {
        "it" => "its".to_string(),
        "them" | "they" => "their".to_string(),
        _ => possessive_subject(subject),
    }
}

pub(super) fn dynamic_pt_scale_multiplier_for_target(
    value: &Value,
    target: &ChooseSpec,
    power_axis: bool,
) -> Option<i32> {
    match value.unhinted() {
        Value::SourcePower if power_axis && choose_spec_references_source_object(target) => Some(1),
        Value::SourceToughness if !power_axis && choose_spec_references_source_object(target) => {
            Some(1)
        }
        Value::PowerOf(spec) if power_axis => {
            dynamic_pt_choose_specs_equivalent(spec, target).then_some(1)
        }
        Value::ToughnessOf(spec) if !power_axis => {
            dynamic_pt_choose_specs_equivalent(spec, target).then_some(1)
        }
        Value::Scaled(inner, multiplier) if *multiplier > 0 => {
            dynamic_pt_scale_multiplier_for_target(inner, target, power_axis)
                .map(|base| base * *multiplier)
        }
        _ => None,
    }
}

pub(super) fn choose_spec_references_source_object(spec: &ChooseSpec) -> bool {
    match spec.unhinted() {
        ChooseSpec::Source => true,
        ChooseSpec::Object(filter) => filter.source,
        _ => false,
    }
}

pub(super) fn dynamic_pt_choose_specs_equivalent(left: &ChooseSpec, right: &ChooseSpec) -> bool {
    choose_specs_equivalent_ignoring_source_surface(left, right)
        || (choose_spec_references_source_object(left)
            && choose_spec_references_source_object(right))
}

pub(super) fn describe_dynamic_pt_scale_action(
    target: &ChooseSpec,
    power: &Value,
    toughness: &Value,
    duration: &Until,
) -> Option<String> {
    let power_multiplier = dynamic_pt_scale_multiplier_for_target(power, target, true)?;
    let toughness_multiplier = dynamic_pt_scale_multiplier_for_target(toughness, target, false)?;
    if power_multiplier != toughness_multiplier {
        return None;
    }
    let verb = match power_multiplier + 1 {
        2 => "Double",
        3 => "Triple",
        _ => return None,
    };
    let target_text = describe_choose_spec(target);
    Some(format!(
        "{verb} {} power and toughness {}",
        possessive_object_subject(&target_text),
        describe_until(duration)
    ))
}

pub(super) fn may_causative_clause(inner: &str) -> Option<String> {
    let trimmed = inner.trim();
    let lower = trimmed.to_ascii_lowercase();
    if ![
        "a ", "an ", "all ", "another ", "each ", "it ", "other ", "that ", "the ", "those ",
        "target ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
    {
        return None;
    }

    let base_pt_marker = " has base power and toughness ";
    if let Some(idx) = lower.find(base_pt_marker) {
        let subject = lowercase_first(trimmed[..idx].trim());
        let rest = trimmed[idx + base_pt_marker.len()..].trim();
        if !subject.is_empty() && !rest.is_empty() {
            return Some(format!(
                "have {} base power and toughness become {rest}",
                possessive_subject(&subject)
            ));
        }
    }

    let replacements = [
        (" becomes ", "become"),
        (" gets ", "get"),
        (" gains ", "gain"),
        (" has ", "have"),
        (" loses ", "lose"),
        (" reveals ", "reveal"),
        (" fights ", "fight"),
        (" deals ", "deal"),
    ];
    replacements.iter().find_map(|(from, to)| {
        lower.find(from).and_then(|idx| {
            let subject = lowercase_first(trimmed[..idx].trim());
            let rest = trimmed[idx + from.len()..].trim();
            if subject.is_empty() || rest.is_empty() {
                None
            } else {
                Some(format!("have {subject} {to} {rest}"))
            }
        })
    })
}

pub(super) fn prevention_put_counters_follow_up(
    follow_up_effects: &[Effect],
) -> Option<&crate::effects::PutCountersEffect> {
    let [effect] = follow_up_effects else {
        return None;
    };
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() {
        return None;
    }
    if !is_effect_count_reference(&put.amount, None) {
        return None;
    }
    if !matches!(put.target, ChooseSpec::AnyTarget) {
        return None;
    }
    Some(put)
}

pub(super) fn prevention_gain_life_follow_up(
    follow_up_effects: &[Effect],
) -> Option<&crate::effects::GainLifeEffect> {
    let [effect] = follow_up_effects else {
        return None;
    };
    let gain = effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if !matches!(
        gain.player,
        ChooseSpec::Player(crate::target::PlayerFilter::You)
    ) {
        return None;
    }
    if !matches!(
        gain.amount.unhinted(),
        Value::EventValue(EventValueSpec::Amount)
    ) {
        return None;
    }
    Some(gain)
}

pub(super) fn prevention_exile_prevented_top_follow_up(
    follow_up_effects: &[Effect],
) -> Option<&crate::effects::ExileTopOfLibraryEffect> {
    let [effect] = follow_up_effects else {
        return None;
    };
    let exile = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if !matches!(exile.player, PlayerFilter::You) {
        return None;
    }
    if !matches!(
        exile.count.unhinted(),
        Value::EventValue(EventValueSpec::Amount)
    ) {
        return None;
    }
    Some(exile)
}

pub(super) fn prevention_damage_any_target_follow_up(
    follow_up_effects: &[Effect],
) -> Option<&crate::effects::DealDamageEffect> {
    let [effect] = follow_up_effects else {
        return None;
    };
    let damage =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        damage.amount.unhinted(),
        Value::EventValue(EventValueSpec::Amount)
    ) {
        return None;
    }
    if !matches!(damage.target, ChooseSpec::AnyTarget) {
        return None;
    }
    Some(damage)
}

pub(super) fn describe_implicit_source_combat_damage_prevention(
    source: &ChooseSpec,
    until: &Until,
) -> Option<String> {
    if !matches!(until, Until::EndOfTurn) {
        return None;
    }
    match source {
        ChooseSpec::Tagged(tag) if is_implicit_reference_tag(tag.as_str()) => {
            Some("Prevent all combat damage that would be dealt by it this turn".to_string())
        }
        _ => None,
    }
}

pub(super) fn put_counters_effect_for_source(
    effect: &Effect,
) -> Option<&crate::effects::PutCountersEffect> {
    if let Some(put_counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        return Some(put_counters);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return put_counters_effect_for_source(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return put_counters_effect_for_source(&with_id.effect);
    }
    None
}

pub(super) fn describe_source_exile_with_counters_pair(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let exile = first.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile.zone != Zone::Exile || !matches!(exile.target, ChooseSpec::Source) {
        return None;
    }
    let put_counters = put_counters_effect_for_source(second)?;
    if put_counters.distributed
        || put_counters.target_count.is_some()
        || !matches!(put_counters.target, ChooseSpec::Source)
    {
        return None;
    }

    let exile_text = describe_effect_impl(first);
    let subject = exile_text
        .strip_prefix("Exile ")
        .map(|text| text.trim_end_matches('.').to_string())
        .unwrap_or_else(|| describe_choose_spec(&exile.target));
    Some(format!(
        "Exile {subject} with {} on it",
        describe_put_counter_phrase(&put_counters.amount, put_counters.counter_type),
    ))
}

pub(super) fn value_is_source_exiled_mana_value(value: &Value) -> bool {
    matches!(
        value,
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    )
}

pub(super) fn is_source_exiled_cards_filter(filter: &ObjectFilter) -> bool {
    if filter.zone != Some(Zone::Exile)
        || !filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
        })
    {
        return false;
    }

    let mut base = filter.clone();
    base.zone = None;
    base.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
    });
    base == ObjectFilter::default()
}

pub(super) fn describe_exiled_card_copy_target_filter(
    filter: &ObjectFilter,
) -> Option<&'static str> {
    if filter.zone != Some(Zone::Exile) {
        return None;
    }

    let mut base = filter.clone();
    let face_down = base.face_down;
    base.zone = None;
    base.face_down = None;
    base.attacking = false;
    base.attacking_player_or_planeswalker_controlled_by = None;

    if base != ObjectFilter::default() {
        return None;
    }

    Some(match face_down {
        Some(false) => "the face-up exiled card",
        Some(true) => "the face-down exiled card",
        None => "the exiled card",
    })
}

pub(super) fn describe_consult_exile_may_cast_rest_bottom_sequence(
    effects: &[&Effect],
) -> Option<String> {
    if effects.len() != 3 {
        return None;
    }

    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Exile {
        return None;
    }

    let may = effects[1].downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != consult.match_tag || cast.allow_land || cast.as_copy {
        return None;
    }
    if let Some(decider) = may.decider.as_ref()
        && decider != &cast.player
    {
        return None;
    }

    let move_to_zone =
        unwrap_basic_tag_wrappers(effects[2]).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Library
        || move_to_zone.to_top
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Object(filter)
                if describe_exiled_card_copy_target_filter(&filter).is_some()
        )
    {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let subject_verb = player_verb(&player, "exile", "exiles");
    let put_verb = player_verb(&player, "put", "puts");
    let pronoun = if player == "you" { "you" } else { "they" };
    let selection = describe_search_selection_with_cards(&consult.filter.description());
    let stop_text =
        describe_consult_stop_text(&selection, &consult.stop_rule, consult.max_exposed.as_ref());
    let caster = describe_player_filter(&cast.player);
    let free_cast_suffix = if cast.without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };

    Some(format!(
        "{player} {subject_verb} cards from the top of {library_owner} library until {pronoun} exile {stop_text}. {caster} may cast that card{free_cast_suffix}. Then {player} {put_verb} the exiled cards that weren't cast this way on the bottom of {library_owner} library in a random order"
    ))
}

pub(super) fn describe_library_consult_selection_with_cards(filter: &ObjectFilter) -> String {
    let mut display_filter = filter.clone();
    display_filter.zone = None;
    if filter.zone == Some(Zone::Battlefield) && display_filter == ObjectFilter::default() {
        return "a permanent card".to_string();
    }
    let mut selection = display_filter.description();
    if display_filter.card_types.is_empty()
        && display_filter.all_card_types.is_empty()
        && display_filter.excluded_card_types == vec![CardType::Land]
    {
        if selection == "nonland permanent" {
            selection = "a nonland card".to_string();
        } else if let Some(rest) = selection.strip_prefix("nonland permanent with ") {
            selection = format!("a nonland card with {rest}");
        } else if let Some(rest) = selection.strip_prefix("nonland card with ") {
            selection = format!("a nonland card with {rest}");
        }
    }
    describe_search_selection_with_cards(&selection)
}

pub(super) fn describe_consult_may_cast_remainder_bottom_sequence(
    effects: &[&Effect],
) -> Option<String> {
    if effects.len() != 3 {
        return None;
    }

    let consult = effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let may = effects[1].downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if cast.tag != consult.match_tag || cast.allow_land || cast.as_copy {
        return None;
    }
    if let Some(decider) = may.decider.as_ref()
        && decider != &cast.player
    {
        return None;
    }

    let remainder =
        effects[2].downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != consult.all_tag
        || remainder.player != consult.player
        || remainder
            .keep_tagged
            .as_ref()
            .is_some_and(|tag| tag != &consult.match_tag)
    {
        return None;
    }

    let player = describe_player_filter(&consult.player);
    let library_owner = describe_possessive_player_filter(&consult.player);
    let (subject_verb, followup_verb, remainder_subject) = match consult.mode {
        crate::effects::consult_helpers::LibraryConsultMode::Reveal => (
            player_verb(&player, "reveal", "reveals"),
            "reveal",
            "all revealed cards not cast this way",
        ),
        crate::effects::consult_helpers::LibraryConsultMode::Exile => (
            player_verb(&player, "exile", "exiles"),
            "exile",
            "the exiled cards that weren't cast this way",
        ),
    };
    let pronoun = if player == "you" { "you" } else { "they" };
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let stop_text =
        describe_consult_stop_text(&selection, &consult.stop_rule, consult.max_exposed.as_ref());
    let caster = describe_player_filter(&cast.player);
    let free_cast_suffix = if cast.without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };
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
        "{player} {subject_verb} cards from the top of {library_owner} library until {pronoun} {followup_verb} {stop_text}. {caster} may cast that card{free_cast_suffix}. Put {remainder_subject} on the bottom of {library_owner} library{order_text}"
    ))
}

pub(super) fn source_and_source_exiled_return_text(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() != 2 {
        return None;
    }

    let mut source_text = None;
    let mut has_source_exiled = false;
    for branch in &filter.any_of {
        if is_source_exiled_cards_filter(branch) {
            has_source_exiled = true;
            continue;
        }

        if branch.source {
            let mut base = branch.clone();
            base.source = false;
            let is_saga = base.subtypes == [Subtype::Saga];
            if is_saga {
                base.subtypes.clear();
            }
            if base == ObjectFilter::default() {
                source_text = Some(if is_saga { "this Saga" } else { "this card" });
            }
        }
    }

    if has_source_exiled {
        source_text.map(|source_text| {
            format!("Return {source_text} and the exiled card to their owner's hand")
        })
    } else {
        None
    }
}

pub(super) fn has_vote_winners_tag(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == crate::effects::VOTE_WINNERS_TAG
    })
}

pub(super) fn describe_return_all_to_battlefield_effect(
    return_all: &crate::effects::ReturnAllToBattlefieldEffect,
) -> String {
    let source_linked_exile = return_all.filter.zone == Some(Zone::Exile)
        && return_all
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            });
    let helper_linked_exile = if return_all.filter.zone == Some(Zone::Exile) {
        let mut base = return_all.filter.clone();
        base.zone = None;
        base.tagged_constraints.retain(|constraint| {
            !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && crate::cards::is_sentence_helper_tag(constraint.tag.as_str(), "exiled"))
        });
        base == ObjectFilter::default()
            && return_all
                .filter
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        && crate::cards::is_sentence_helper_tag(constraint.tag.as_str(), "exiled")
                })
    } else {
        false
    };
    let mut filter_text = if helper_linked_exile {
        "the exiled cards".to_string()
    } else if source_linked_exile
        && return_all.filter.card_types.len() == 1
        && return_all.filter.card_types[0] == CardType::Creature
        && return_all.filter.subtypes.is_empty()
    {
        "creature cards exiled with this enchantment".to_string()
    } else {
        describe_for_each_filter(&return_all.filter)
    };
    filter_text = filter_text
        .replace("permanent card exiled", "permanent cards exiled")
        .replace("card exiled", "cards exiled")
        .replace(" card in your graveyard", " cards from your graveyard");
    let controller_suffix = match return_all.battlefield_controller {
        crate::effects::BattlefieldController::Preserve
        | crate::effects::BattlefieldController::Owner => {
            if filter_text.contains(" from your graveyard") {
                ""
            } else {
                " under their owners' control"
            }
        }
        crate::effects::BattlefieldController::You => " under your control",
    };
    let face_down_suffix = if return_all.face_down {
        " face down"
    } else {
        ""
    };
    format!(
        "Return{}{filter_text} to the battlefield{}{}{}",
        if helper_linked_exile { " " } else { " all " },
        if return_all.tapped { " tapped" } else { "" },
        face_down_suffix,
        controller_suffix,
    )
}

pub(super) fn effect_exiles_triggering_object(effect: &Effect) -> bool {
    let triggering = TagKey::from("triggering");
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return move_to_zone.zone == Zone::Exile
            && choose_spec_references_exact_tag(&move_to_zone.target, &triggering);
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return choose_spec_references_exact_tag(&exile.spec, &triggering);
    }
    false
}

pub(super) fn describe_exile_it_then_return_all_to_battlefield(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    if !effect_exiles_triggering_object(first) {
        return None;
    }
    let return_all = second.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    Some(format!(
        "Exile it, then {}",
        lowercase_first(&describe_return_all_to_battlefield_effect(return_all))
    ))
}

pub(super) fn describe_source_sacrifice_then_return_source_exiled(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let with_id = first.downcast_ref::<crate::effects::WithIdEffect>()?;
    let sacrifice = with_id
        .effect
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target, ChooseSpec::Source) {
        return None;
    }

    let if_effect = second.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [return_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let return_all =
        return_effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()?;
    if !is_source_exiled_cards_filter(&return_all.filter)
        || return_all.tapped
        || return_all.face_down
        || return_all.battlefield_controller != crate::effects::BattlefieldController::Owner
    {
        return None;
    }

    Some(
        "sacrifice it. If you do, return those cards to the battlefield under their owners' control"
            .to_string(),
    )
}

pub(super) fn describe_prevention_follow_up_target(target: &ChooseSpec) -> &'static str {
    let described = describe_choose_spec(target);
    if described.contains("creature") {
        "that creature"
    } else if described.contains("player") {
        "that player"
    } else if described.contains("permanent") {
        "that permanent"
    } else {
        "it"
    }
}

pub(super) fn render_iterative_library_repeat_process(
    repeat: &crate::effects::RepeatProcessEffect,
) -> Option<String> {
    if repeat.predicate != ironsmith_core::EffectPredicate::WasDeclined {
        return None;
    }
    let [exile_effect, conditional_effect] = repeat.effects.as_slice() else {
        return None;
    };
    let exile_effect = exile_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map(|with_id| with_id.effect.as_ref())
        .unwrap_or(exile_effect);
    let conditional_effect = conditional_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .map(|with_id| with_id.effect.as_ref())
        .unwrap_or(conditional_effect);
    let exile = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    if exile.count != Value::Fixed(1)
        || exile.player != PlayerFilter::You
        || exile.moved_tags.len() != 1
        || exile.accumulated_tags.len() != 1
    {
        return None;
    }
    let current_tag = &exile.moved_tags[0];
    let exiled_tag = &exile.accumulated_tags[0];
    if current_tag.as_str() != "iterative_library_current"
        || exiled_tag.as_str() != "iterative_library_exiled"
    {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    let [may_move_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let may_move = may_move_effect.downcast_ref::<crate::effects::MayMoveToZoneEffect>()?;
    if may_move.zone != Zone::Hand
        || may_move.decider != PlayerFilter::You
        || !matches!(&may_move.target, ChooseSpec::Tagged(tag) if tag == current_tag)
    {
        return None;
    }

    Some(
        "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first"
            .to_string(),
    )
}

pub(super) fn choose_primary_zone(choose: &crate::effects::ChooseObjectsEffect) -> Option<Zone> {
    choose.filter.zone.or(choose.zone)
}

pub(super) fn object_filter_has_tag(filter: &ObjectFilter, tag: &crate::tag::TagKey) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == *tag
    })
}

pub(super) fn choose_spec_player_filter(spec: &ChooseSpec) -> Option<PlayerFilter> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => choose_spec_player_filter(spec),
        ChooseSpec::Target(inner) => Some(PlayerFilter::Target(Box::new(
            choose_spec_player_filter(inner)?,
        ))),
        ChooseSpec::Player(filter) => Some(filter.clone()),
        _ => None,
    }
}

pub(super) fn hand_choice_selection_from_it(
    choose: &crate::effects::ChooseObjectsEffect,
) -> String {
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.controller = None;
    filter.tagged_constraints.clear();
    let mut selection = if choose_primary_zone(choose) == Some(Zone::Hand)
        && choose.filter.card_types.is_empty()
        && choose.filter.excluded_card_types == vec![CardType::Land]
    {
        "nonland card".to_string()
    } else if choose_primary_zone(choose) == Some(Zone::Hand)
        && choose.filter.card_types.is_empty()
        && choose.filter.excluded_card_types.is_empty()
        && choose.filter.subtypes.is_empty()
        && choose.filter.colors.is_none()
        && choose.filter.mana_value.is_none()
    {
        "card".to_string()
    } else {
        filter.description()
    };
    if !selection.contains("card") {
        selection.push_str(" card");
    }
    with_indefinite_article(&selection)
}

pub(super) fn describe_discard_reveal_hand_choose_discard_chosen(
    effects: &[&Effect],
) -> Option<String> {
    let [
        discard_cost_effect,
        look_effect,
        choose_effect,
        discard_chosen_effect,
    ] = effects
    else {
        return None;
    };
    let discard_cost = discard_cost_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    let discarded_tag = discard_cost.tag.as_ref()?;
    if discard_cost.player != PlayerFilter::You
        || discard_cost.count != Value::Fixed(0)
        || !discard_cost.any_number
        || discard_cost.random
        || discard_cost.card_filter.is_some()
    {
        return None;
    }

    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_dynamic_x()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.filter.owner.as_ref() != Some(&look_player)
        || !matches!(
            choose.count_value.as_ref(),
            Some(Value::Count(filter)) if object_filter_has_tag(filter, discarded_tag)
        )
    {
        return None;
    }

    let discard_chosen = discard_chosen_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard_chosen.random
        || discard_chosen.any_number
        || discard_chosen.player != look_player
        || !matches!(&discard_chosen.count, Value::Count(filter) if object_filter_has_tag(filter, &choose.tag))
        || !discard_chosen
            .card_filter
            .as_ref()
            .is_some_and(|filter| object_filter_has_tag(filter, &choose.tag))
    {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let selection = hand_choice_selection_from_it(choose);
    Some(format!(
        "Discard any number of cards. {} {} their hand, then you choose {selection} from it for each card discarded this way. That player discards those cards",
        capitalize_first(&revealer),
        reveal_verb
    ))
}

pub(super) fn describe_reveal_hand_subset_choose_then_discard(
    effects: &[&Effect],
) -> Option<String> {
    let [reveal_effect, choose_effect, discard_effect] = effects else {
        return None;
    };
    let reveal = reveal_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !reveal.reveal
        || reveal.is_search
        || choose.is_search
        || !choose.count.is_single()
        || choose_primary_zone(reveal) != Some(Zone::Hand)
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || reveal.filter.owner.as_ref() != Some(&reveal.chooser)
        || choose.filter.owner.as_ref() != Some(&reveal.chooser)
        || discard.player != reveal.chooser
        || discard.count != Value::Fixed(1)
        || discard.random
    {
        return None;
    }
    let (count_text, count_suffix) = if reveal.count.dynamic_x {
        let count_value = reveal.count_value.as_ref()?;
        (
            "a number of".to_string(),
            format!(" equal to {}", describe_value(count_value)),
        )
    } else {
        let reveal_count = reveal.count.max.filter(|max| *max == reveal.count.min)?;
        (
            small_number_word(reveal_count as u32).unwrap_or_else(|| reveal_count.to_string()),
            String::new(),
        )
    };
    let card_filter = discard.card_filter.as_ref()?;
    let chooses_revealed = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == reveal.tag
    });
    let discards_chosen = card_filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == choose.tag
    });
    if !chooses_revealed || !discards_chosen {
        return None;
    }

    let player = describe_player_filter(&reveal.chooser);
    let verb = player_verb(&player, "reveal", "reveals");
    let followup = if reveal.count.dynamic_x {
        ". You choose one of those cards"
    } else {
        " and you choose one of them"
    };
    Some(format!(
        "{} {} {count_text} cards from their hand{count_suffix}{followup}. That player discards that card",
        capitalize_first(&player),
        verb
    ))
}

pub(super) fn describe_look_hand_choose_then_discard_or_exile(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, choose_effect, action_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let action_effect = unwrap_basic_tag_wrappers(action_effect);

    if let Some((reveal_text, choice_text, look_player)) =
        describe_reveal_hand_choose_from_it_or_graveyard(look, choose)
    {
        if let Some(discard) = action_effect.downcast_ref::<crate::effects::DiscardEffect>() {
            if !discard_discards_chosen_card(discard, choose, &look_player) {
                return None;
            }
            return Some(format!(
                "{reveal_text}. You choose {choice_text}. That player discards that card"
            ));
        }
        if let Some(exile) = action_effect.downcast_ref::<crate::effects::ExileEffect>()
            && exile_uses_chosen_tag(&exile.spec, choose.tag.as_str())
        {
            return Some(format!(
                "{reveal_text}. You choose {choice_text}. Exile that card"
            ));
        }
        if let Some(move_to_zone) = action_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            && move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str())
        {
            return Some(format!(
                "{reveal_text}. You choose {choice_text}. Exile that card"
            ));
        }
        return None;
    }

    let (reveal_text, choice_text, look_player) =
        describe_reveal_hand_choose_from_it(look, choose)?;

    if let Some(discard) = action_effect.downcast_ref::<crate::effects::DiscardEffect>() {
        if !discard_discards_chosen_card(discard, choose, &look_player) {
            return None;
        }
        return Some(format!(
            "{reveal_text}. You choose {choice_text} from it. That player discards that card"
        ));
    }

    if let Some(exile) = action_effect.downcast_ref::<crate::effects::ExileEffect>()
        && exile_uses_chosen_tag(&exile.spec, choose.tag.as_str())
    {
        return Some(format!(
            "{reveal_text}. You choose {choice_text} from it and exile that card"
        ));
    }
    if let Some(move_to_zone) = action_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str())
    {
        return Some(format!(
            "{reveal_text}. You choose {choice_text} from it and exile that card"
        ));
    }
    None
}

pub(super) fn describe_reveal_hand_then_same_player_discards(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, discard_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let discard = unwrap_basic_tag_wrappers(discard_effect)
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if !look.reveal {
        return None;
    }
    let looked_player = choose_spec_player_filter(&look.target)?;
    if !player_filters_refer_to_same_player(&looked_player, &discard.player) {
        return None;
    }

    let player = describe_player_filter(&discard.player);
    let discard_count = if discard.any_number {
        match discard.card_filter.as_ref() {
            Some(filter) => format!(
                "any number of {}",
                pluralize_discard_card_phrase(&describe_discard_card_phrase(filter))
            ),
            None => "any number of cards".to_string(),
        }
    } else {
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    };
    let random_suffix = if discard.random { " at random" } else { "" };
    Some(format!(
        "{} {} their hand and {} {discard_count}{random_suffix}",
        capitalize_first(&player),
        player_verb(&player, "reveal", "reveals"),
        player_verb(&player, "discard", "discards"),
    ))
}

pub(super) fn describe_reveal_hand_choose_from_it_or_graveyard(
    look: &crate::effects::LookAtHandEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<(String, String, PlayerFilter)> {
    if !look.reveal
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose.zone != Some(Zone::Hand)
        || !choose.additional_zones.contains(&Zone::Graveyard)
    {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    let hand_arm = choose.filter.any_of.iter().find(|option| {
        option.zone == Some(Zone::Hand)
            && option.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == "__it__"
                    && matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    )
            })
    })?;
    let graveyard_arm = choose.filter.any_of.iter().find(|option| {
        option.zone == Some(Zone::Graveyard)
            && option
                .owner
                .as_ref()
                .is_some_and(|owner| owner == &look_player)
    })?;

    let hand_choice = describe_card_choice_without_zone(hand_arm)?;
    let graveyard_choice = describe_card_choice_without_zone(graveyard_arm)?;
    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    Some((
        format!("{} {} their hand", capitalize_first(&revealer), reveal_verb),
        format!("{hand_choice} from it or {graveyard_choice} from their graveyard"),
        look_player,
    ))
}

pub(super) fn describe_card_choice_without_zone(filter: &ObjectFilter) -> Option<String> {
    let mut display = filter.clone();
    display.zone = None;
    display.owner = None;
    display.controller = None;
    display.tagged_constraints.clear();
    display.any_of.clear();
    let mut choice =
        if display.card_types.is_empty() && display.excluded_card_types == vec![CardType::Land] {
            "nonland card".to_string()
        } else if display.card_types.is_empty()
            && display.excluded_card_types.is_empty()
            && display.subtypes.is_empty()
            && display.colors.is_none()
            && display.mana_value.is_none()
            && display.supertypes.is_empty()
        {
            "card".to_string()
        } else {
            display.description()
        };
    if !choice.contains("card") {
        choice.push_str(" card");
    }
    Some(with_indefinite_article(&choice))
}

pub(super) fn describe_reveal_hand_choose_from_it(
    look: &crate::effects::LookAtHandEffect,
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<(String, String, PlayerFilter)> {
    if !look.reveal
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Hand)
    {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    if !choose
        .filter
        .owner
        .as_ref()
        .is_some_and(|owner| player_filters_refer_to_same_player(owner, &look_player))
    {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let choice_text = hand_choice_from_it_text(choose)?;
    Some((
        format!("{} {} their hand", capitalize_first(&revealer), reveal_verb),
        choice_text,
        look_player,
    ))
}

pub(super) fn player_filters_refer_to_same_player(
    left: &PlayerFilter,
    right: &PlayerFilter,
) -> bool {
    if left == right {
        return true;
    }
    match (left, right) {
        (PlayerFilter::Target(inner), other) | (other, PlayerFilter::Target(inner)) => {
            player_filters_refer_to_same_player(inner, other)
        }
        _ => false,
    }
}

pub(super) fn discard_discards_chosen_card(
    discard: &crate::effects::DiscardEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    expected_player: &PlayerFilter,
) -> bool {
    discard.count == Value::Fixed(1)
        && !discard.random
        && !discard.any_number
        && &discard.player == expected_player
        && discard
            .card_filter
            .as_ref()
            .is_some_and(|filter| object_filter_has_tag(filter, &choose.tag))
}

pub(super) fn hand_choice_from_it_text(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.controller = None;
    let mut choice =
        if filter.card_types.is_empty() && filter.excluded_card_types == vec![CardType::Land] {
            "nonland card".to_string()
        } else {
            filter.description()
        };
    if choice == "nonland permanent" {
        choice = "nonland card".to_string();
    } else if let Some(rest) = choice.strip_prefix("permanent with ") {
        choice = format!("card with {rest}");
    } else if let Some(rest) = choice.strip_prefix("with ") {
        choice = format!("card with {rest}");
    }
    if !choice.contains("card") {
        choice.push_str(" card");
    }
    Some(with_indefinite_article(&choice))
}

pub(super) fn tagged_move_to_library_nth_from_effect(
    effect: &Effect,
) -> Option<&crate::effects::MoveToLibraryNthFromTopEffect> {
    unwrap_basic_tag_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToLibraryNthFromTopEffect>()
}

pub(super) fn choose_owner_matches_looked_player(
    choose: &crate::effects::ChooseObjectsEffect,
    looked_player: &str,
) -> bool {
    choose.filter.owner.as_ref().is_none_or(|owner| {
        let owner_text = describe_player_filter(owner);
        owner_text == looked_player
            || owner_text == format!("target {looked_player}")
            || (looked_player.starts_with("target ") && owner_text == looked_player)
    })
}

pub(super) fn describe_hand_choose_then_library_placement(effects: &[&Effect]) -> Option<String> {
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
    if !choose_owner_matches_looked_player(choose, &looked_player) {
        return None;
    }

    let selection = hand_choice_selection_from_it(choose);
    if let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
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

    if let Some(move_to_library) = tagged_move_to_library_nth_from_effect(move_effect)
        && matches!(&move_to_library.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        let position = library_position_from_top_text(&move_to_library.position, true);
        let reveal_verb = player_verb(&looked_player, "reveal", "reveals");
        return Some(format!(
            "{} {reveal_verb} their hand. You choose {selection} from it. That player puts that card into their library {position}",
            capitalize_first(&looked_player)
        ));
    }

    None
}

pub(super) fn describe_target_player_choose_hand_top_library_any_order(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, choose_effect, move_effect] = effects else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.target != ChooseSpec::target_player() {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.is_search
        || choose.chooser != PlayerFilter::target_player()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || choose.filter.owner.as_ref() != Some(&PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_library_uses_chosen_tag(move_to_zone, choose.tag.as_str()) || !move_to_zone.to_top {
        return None;
    }

    let chosen = describe_choose_selection(choose);
    let moved_ref = if choose.count.is_single() {
        "it"
    } else {
        "them"
    };
    let order_suffix = if choose.count.is_single() {
        ""
    } else {
        " in any order"
    };
    Some(format!(
        "Target player chooses {chosen} from their hand and puts {moved_ref} on top of their library{order_suffix}"
    ))
}

pub(super) fn choose_spec_is_target_permanent(spec: &ChooseSpec) -> bool {
    describe_choose_spec(spec) == "target permanent"
}

pub(super) fn card_types_are_permanent_card_types(card_types: &[CardType]) -> bool {
    let required = [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    card_types.len() == required.len()
        && required
            .iter()
            .all(|card_type| card_types.contains(card_type))
}

pub(super) fn object_filter_is_plain_permanent_card(filter: &ObjectFilter) -> bool {
    if filter.zone.is_some() || !card_types_are_permanent_card_types(&filter.card_types) {
        return false;
    }
    let mut rest = filter.clone();
    rest.card_types.clear();
    rest == ObjectFilter::default()
}

pub(super) fn player_filter_is_owner_of_tag(player: &PlayerFilter, tag: &TagKey) -> bool {
    matches!(
        player,
        PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(owner_tag)) if owner_tag == tag
    )
}

pub(super) fn move_revealed_tag_to_battlefield(effect: &Effect, tag: &TagKey) -> bool {
    let Some(move_to_zone) =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return false;
    };
    move_to_zone.zone == Zone::Battlefield
        && !move_to_zone.to_top
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(move_tag) if move_tag == tag)
}

pub(super) fn describe_target_permanent_shuffle_reveal_permanent_card(
    effects: &[Effect],
) -> Option<String> {
    let [
        move_effect,
        shuffle_effect,
        reveal_effect,
        conditional_effect,
    ] = effects
    else {
        return None;
    };
    let moved_tag = wrapped_effect_tag(move_effect)?;
    let move_to_library = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_library.zone != Zone::Library
        || move_to_library.to_top
        || !choose_spec_is_target_permanent(&move_to_library.target)
    {
        return None;
    }

    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !player_filter_is_owner_of_tag(&shuffle.player, moved_tag) {
        return None;
    }
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTopEffect>()?;
    let reveal_tag = reveal.tag.as_ref()?;
    if !player_filter_is_owner_of_tag(&reveal.player, moved_tag) {
        return None;
    }

    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(condition_tag, filter) = &conditional.condition else {
        return None;
    };
    if condition_tag != reveal_tag
        || !object_filter_is_plain_permanent_card(filter)
        || !conditional.if_false.is_empty()
    {
        return None;
    }
    let [move_revealed] = conditional.if_true.as_slice() else {
        return None;
    };
    if !move_revealed_tag_to_battlefield(move_revealed, reveal_tag) {
        return None;
    }

    Some(
        "The owner of target permanent shuffles it into their library, then reveals the top card of their library. If it's a permanent card, they put it onto the battlefield"
            .to_string(),
    )
}

pub(super) fn describe_reveal_hand_choose_discard_then_scry(effects: &[&Effect]) -> Option<String> {
    let [look_effect, choose_effect, discard_effect, scry_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (reveal_text, choice_text, look_player) =
        describe_reveal_hand_choose_from_it(look, choose)?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !discard_discards_chosen_card(discard, choose, &look_player) {
        return None;
    }
    let scry = scry_effect.downcast_ref::<crate::effects::ScryEffect>()?;
    if scry.player != PlayerFilter::You {
        return None;
    }
    Some(format!(
        "{reveal_text}. You choose {choice_text} from it. That player discards that card. Scry {}",
        describe_value(&scry.count)
    ))
}

pub(super) fn describe_reveal_hand_choose_discard_then_adventure_move(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, choose_effect, discard_effect, may_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (reveal_text, choice_text, look_player) =
        describe_reveal_hand_choose_from_it(look, choose)?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !discard_discards_chosen_card(discard, choose, &look_player) {
        return None;
    }

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::You) {
        return None;
    }
    let [move_effect] = may.effects.as_slice() else {
        return None;
    };
    let move_effect = move_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(move_effect);
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Graveyard || move_to_zone.enters_tapped {
        return None;
    }
    let ChooseSpec::WithCount(spec, count) = &move_to_zone.target else {
        return None;
    };
    if !count.is_single() {
        return None;
    }
    let ChooseSpec::Object(filter) = spec.as_ref() else {
        return None;
    };
    if filter.zone != Some(Zone::Exile)
        || filter.owner.as_ref() != Some(&look_player)
        || filter.subtypes != vec![Subtype::Adventure]
    {
        return None;
    }

    Some(format!(
        "{reveal_text}. You choose {choice_text} from it. That player discards that card. You may put a card that has an Adventure that player owns from exile into that player's graveyard"
    ))
}

pub(super) fn describe_reveal_hand_choose_gain_toughness_then_discard(
    effects: &[&Effect],
) -> Option<String> {
    let [look_effect, choose_effect, gain_effect, discard_effect] = effects else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let (reveal_text, choice_text, look_player) =
        describe_reveal_hand_choose_from_it(look, choose)?;
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }
    let Value::ToughnessOf(spec) = &gain.amount else {
        return None;
    };
    let ChooseSpec::Object(filter) = spec.as_ref() else {
        return None;
    };
    if !object_filter_has_tag(filter, &choose.tag) {
        return None;
    }
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if !discard_discards_chosen_card(discard, choose, &look_player) {
        return None;
    }

    Some(format!(
        "{reveal_text}. You choose {choice_text} from it. You gain life equal to that creature card's toughness, then that player discards that card"
    ))
}

pub(super) fn describe_reveal_hand_choose_graveyard_or_hand_exile(
    effects: &[&Effect],
) -> Option<String> {
    let (look_effect, choose_effect, move_effect, lose_effect) = match effects {
        [look_effect, choose_effect, move_effect] => {
            (*look_effect, *choose_effect, *move_effect, None)
        }
        [look_effect, choose_effect, move_effect, lose_effect] => (
            *look_effect,
            *choose_effect,
            *move_effect,
            Some(*lose_effect),
        ),
        _ => return None,
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal {
        return None;
    }
    let look_player = choose_spec_player_filter(&look.target)?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }

    let revealer = describe_choose_spec(&look.target);
    let reveal_verb = player_verb(&revealer, "reveal", "reveals");
    let mut text = if let Some((reveal_text, choice_text, _)) =
        describe_reveal_hand_choose_from_it_or_graveyard(look, choose)
    {
        format!("{reveal_text}. You choose {choice_text}. Exile that card")
    } else {
        if choose.is_search
            || choose.chooser != PlayerFilter::You
            || choose_exact_count(choose) != Some(1)
            || choose_primary_zone(choose) != Some(Zone::Graveyard)
            || choose.additional_zones != vec![Zone::Hand]
            || choose.filter.owner.as_ref() != Some(&look_player)
            || choose.filter.excluded_card_types != vec![CardType::Land]
        {
            return None;
        }
        format!(
            "{} {} their hand. You choose a nonland card from that player's graveyard or hand and exile it",
            capitalize_first(&revealer),
            reveal_verb
        )
    };
    if let Some(lose_effect) = lose_effect {
        let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
        if lose.player != ChooseSpec::Player(PlayerFilter::You) {
            return None;
        }
        text.push_str(&format!(
            ". You lose {}",
            describe_life_amount_phrase(&lose.amount)
        ));
    }
    Some(text)
}
