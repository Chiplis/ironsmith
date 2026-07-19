use super::*;

pub(crate) fn describe_same_actor_draw_then_gain(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let draw =
        unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let gain =
        unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::GainLifeEffect>()?;

    let actor = choose_spec_player_filter(&gain.player)?;
    if !player_filters_refer_to_same_player(&draw.player, &actor)
        || draw.count.unhinted() != gain.amount.unhinted()
    {
        return None;
    }

    let draw_basis = describe_where_x_basis(&draw.count)?;
    let gain_basis = describe_where_x_basis(&gain.amount)?;
    if draw_basis != gain_basis {
        return None;
    }

    let subject = describe_player_filter(&actor);
    Some(format!(
        "{subject} {} X cards and {} X life, where X is {draw_basis}",
        player_verb(&subject, "draw", "draws"),
        player_verb(&subject, "gain", "gains"),
    ))
}

pub(crate) fn describe_same_actor_gain_then_draw(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let gain = unwrap_basic_tag_wrappers(first).downcast_ref::<crate::effects::GainLifeEffect>()?;
    let draw =
        unwrap_basic_tag_wrappers(second).downcast_ref::<crate::effects::DrawCardsEffect>()?;

    let actor = choose_spec_player_filter(&gain.player)?;
    if !player_filters_refer_to_same_player(&actor, &draw.player) {
        return None;
    }

    let gain_basis = describe_where_x_basis(&gain.amount);
    let draw_basis = describe_where_x_basis(&draw.count);
    let (life, cards, suffix) = match (gain_basis, draw_basis) {
        (None, None) => (
            describe_life_amount_phrase(&gain.amount),
            describe_card_count(&draw.count),
            String::new(),
        ),
        (Some(gain_basis), Some(draw_basis)) if gain_basis == draw_basis => (
            "X life".to_string(),
            "X cards".to_string(),
            format!(", where X is {gain_basis}"),
        ),
        _ => return None,
    };

    let subject = describe_player_filter(&actor);
    Some(format!(
        "{subject} {} {life} and {} {cards}{suffix}",
        player_verb(&subject, "gain", "gains"),
        player_verb(&subject, "draw", "draws"),
    ))
}

pub(crate) fn describe_gain_life_then_scry(
    gain: &crate::effects::GainLifeEffect,
    scry: &crate::effects::ScryEffect,
) -> Option<String> {
    let crate::target::ChooseSpec::Player(gain_player) = gain.player.base() else {
        return None;
    };
    if !player_filters_refer_to_same_player(gain_player, &scry.player) {
        return None;
    }
    if !matches!(gain.amount, Value::Fixed(_) | Value::X) {
        return None;
    }

    let player = describe_player_filter(gain_player);
    let gain_clause = format!(
        "{player} {} {} life",
        player_verb(&player, "gain", "gains"),
        describe_value(&gain.amount)
    );
    if *gain_player == PlayerFilter::You {
        return Some(format!(
            "{gain_clause} and scry {}",
            describe_value(&scry.count)
        ));
    }
    Some(format!(
        "{gain_clause} and {player} {} {}",
        player_verb(&player, "scry", "scries"),
        describe_value(&scry.count)
    ))
}

pub(crate) fn describe_scry_then_draw(
    scry: &crate::effects::ScryEffect,
    draw: &crate::effects::DrawCardsEffect,
) -> Option<String> {
    if !player_filters_refer_to_same_player(&scry.player, &draw.player) {
        return None;
    }
    let scry_count = describe_where_x_basis(&scry.count)
        .map(|basis| format!("X, where X is {basis}"))
        .unwrap_or_else(|| describe_value(&scry.count));
    if scry.player == PlayerFilter::You {
        return Some(format!(
            "Scry {}, then draw {}",
            scry_count,
            describe_card_count(&draw.count)
        ));
    }

    let player = describe_player_filter(&scry.player);
    Some(format!(
        "{player} {} {}, then {} {}",
        player_verb(&player, "scry", "scries"),
        scry_count,
        player_verb(&player, "draw", "draws"),
        describe_card_count(&draw.count)
    ))
}

pub(crate) fn describe_draw_for_each_turn_history(
    draw: &crate::effects::DrawCardsEffect,
) -> Option<String> {
    let basis = describe_turn_history_for_each_basis(&draw.count)?;
    let player = describe_player_filter(&draw.player);
    Some(format!(
        "{player} {} a card for each {basis}",
        player_verb(&player, "draw", "draws")
    ))
}

/// Keep a shared dynamic amount intact when a quantified player and each
/// permanent they control receive the same damage. The older generic bundle
/// surface rendered only the amount head and silently discarded its `where X`
/// clause.
pub(crate) fn describe_for_players_history_damage_and_controlled_damage(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let [player_effect, controlled_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let player_damage = player_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        player_damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }

    let for_each = controlled_effect.downcast_ref::<crate::effects::ForEachObject>()?;
    let [object_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let object_effect = unwrap_basic_tag_wrappers(object_effect);
    let object_damage = object_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if object_damage.amount != player_damage.amount
        || !matches!(object_damage.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let (amount, where_x) = describe_damage_amount_clause(&player_damage.amount);
    let where_x = where_x?;
    let objects = describe_each_controlled_by_iterated(&for_each.filter)?;
    let player_filter = describe_for_each_player_filter(&for_players.filter);
    let each_player = strip_leading_article(&player_filter);
    Some(format!(
        "Deal {amount} to each {each_player} and {objects}, where X is {where_x}"
    ))
}

pub(crate) fn describe_where_x_basis(value: &Value) -> Option<String> {
    if value_prefers_equal_to(value) && !value_prefers_where_x(value) {
        return None;
    }
    if let Some(surface) = describe_explicit_where_x_surface(value) {
        return Some(surface.to_string());
    }
    match value.unhinted() {
        Value::Count(filter) => {
            let mut subject = pluralize_noun_phrase(&describe_for_each_count_filter(filter));
            if filter.zone == Some(Zone::Battlefield)
                && filter.controller.is_none()
                && !subject.contains("battlefield")
            {
                subject.push_str(" on the battlefield");
            }
            Some(format!("the number of {subject}"))
        }
        Value::BasicLandTypesAmong(filter) => Some(format!(
            "the number of {}",
            describe_basic_land_types_among(filter)
        )),
        Value::CardTypesAmong(filter) => Some(format!(
            "the number of card types among {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::ColorsAmong(filter) => {
            Some(format!("the number of {}", describe_colors_among(filter)))
        }
        Value::DistinctPowers(filter) => Some(format!(
            "the number of different powers among {}",
            describe_for_each_count_filter(filter)
        )),
        Value::GreatestCount(filter) => Some(format!(
            "the greatest number of {}",
            pluralize_noun_phrase(&describe_for_each_count_filter(filter))
        )),
        Value::TotalPower(filter) => Some(format!(
            "the total power of {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::TotalToughness(filter) => Some(format!(
            "the total toughness of {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::TotalManaValue(filter) => Some(format!(
            "the total mana value of {}",
            describe_count_filter_value_subject(filter)
        )),
        Value::LifeTotalDifference(_) => Some(describe_value(value)),
        Value::CountScaled(filter, multiplier) => {
            let counted = pluralize_noun_phrase(&describe_for_each_count_filter(filter));
            Some(if *multiplier == 1 {
                format!("the number of {counted}")
            } else if *multiplier == 2 {
                format!("twice the number of {counted}")
            } else {
                format!("{multiplier} times the number of {counted}")
            })
        }
        Value::SourcePower => Some("this creature's power".to_string()),
        Value::SourceToughness => Some("this creature's toughness".to_string()),
        Value::SourceMutationCount => {
            Some("the number of times this creature has mutated".to_string())
        }
        Value::PowerOf(spec) => Some(describe_dynamic_counter_basis(spec, "power")),
        Value::ToughnessOf(spec) => Some(describe_dynamic_counter_basis(spec, "toughness")),
        Value::ManaValueOf(spec) => Some(describe_dynamic_counter_basis(spec, "mana value")),
        Value::Devotion { .. } | Value::DevotionToChosenColor(_) => Some(describe_value(value)),
        _ => {
            let rendered = describe_value(value);
            if value_prefers_where_x(value)
                || rendered.starts_with("the number of ")
                || rendered.starts_with("the greatest power ")
                || rendered.starts_with("the greatest toughness ")
                || rendered.starts_with("the greatest mana value ")
                || rendered.contains(" plus the number of ")
            {
                Some(rendered)
            } else {
                None
            }
        }
    }
}

pub(super) fn describe_shared_spells_cast_modal_x_basis(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
    if choose_mode.modes.len() < 2 {
        return None;
    }

    let mut shared_value: Option<&Value> = None;
    for mode in &choose_mode.modes {
        let [effect] = mode.effects.as_slice() else {
            return None;
        };
        let effect = unwrap_basic_tag_wrappers(effect);
        let value = if let Some(scry) = effect.downcast_ref::<crate::effects::ScryEffect>() {
            &scry.count
        } else if let Some(deal) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
            &deal.amount
        } else if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
            &gain.amount
        } else {
            return None;
        };
        let value = value.unhinted();
        if !matches!(
            value,
            Value::SpellsCastThisTurn(_)
                | Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SpellsCast { .. })
        ) {
            return None;
        }
        if let Some(existing) = shared_value {
            if existing != value {
                return None;
            }
        } else {
            shared_value = Some(value);
        }
    }

    match shared_value? {
        Value::SpellsCastThisTurn(PlayerFilter::You) => {
            Some("the number of spells you've cast this turn".to_string())
        }
        value => describe_where_x_basis(value),
    }
}

pub(crate) fn describe_deal_damage_then_gain_life(
    deal: &crate::effects::DealDamageEffect,
    gain: &crate::effects::GainLifeEffect,
) -> Option<String> {
    let where_x = if deal.amount == gain.amount {
        describe_where_x_basis(&deal.amount)
    } else if matches!(deal.amount, Value::X) {
        describe_where_x_basis(&gain.amount)
    } else if matches!(gain.amount, Value::X) {
        describe_where_x_basis(&deal.amount)
    } else {
        None
    }?;

    let target = describe_choose_spec(&deal.target);
    let player = describe_choose_spec(&gain.player);
    Some(format!(
        "Deal X damage to {target} and {player} {} X life, where X is {where_x}",
        player_verb(&player, "gain", "gains")
    ))
}

pub(super) fn deal_damage_effect_view(
    effect: &Effect,
) -> Option<&crate::effects::DealDamageEffect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return deal_damage_effect_view(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return deal_damage_effect_view(&with_id.effect);
    }
    effect.downcast_ref::<crate::effects::DealDamageEffect>()
}

pub(super) fn describe_for_players_lose_life_then_gain_life(
    for_players: &crate::effects::ForPlayersEffect,
    gain: &crate::effects::GainLifeEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Opponent
        || for_players.effects.len() != 1
        || gain.player != ChooseSpec::Player(PlayerFilter::You)
    {
        return None;
    }
    let lose = for_players.effects[0].downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose.player != ChooseSpec::Player(PlayerFilter::IteratedPlayer) || lose.amount != gain.amount
    {
        return None;
    }
    if lose.amount == Value::X {
        return Some("Each opponent loses X life and you gain X life".to_string());
    }
    let where_x = describe_where_x_basis(&lose.amount)?;
    Some(format!(
        "Each opponent loses X life and you gain X life, where X is {where_x}"
    ))
}

pub(crate) fn describe_lose_life_then_gain_life(
    lose: &crate::effects::LoseLifeEffect,
    gain: &crate::effects::GainLifeEffect,
) -> Option<String> {
    let where_x = if lose.amount == gain.amount {
        describe_where_x_basis(&lose.amount)
    } else if matches!(lose.amount, Value::X) {
        describe_where_x_basis(&gain.amount)
    } else if matches!(gain.amount, Value::X) {
        describe_where_x_basis(&lose.amount)
    } else {
        None
    }?;

    let lose_player = describe_choose_spec(&lose.player);
    let gain_player = describe_choose_spec(&gain.player);
    Some(format!(
        "{lose_player} {} X life and {gain_player} {} X life, where X is {where_x}",
        player_verb(&lose_player, "lose", "loses"),
        player_verb(&gain_player, "gain", "gains")
    ))
}

pub(super) fn is_clash_win_predicate(predicate: &EffectPredicate) -> bool {
    matches!(
        predicate,
        EffectPredicate::Value(Comparison::GreaterThan(0))
    )
}

/// Render a result-controlled clash loop from its structured repeat effect.
/// The loop predicate is the count returned by `ClashEffect`: one when the
/// resolving effect's controller wins and zero otherwise.
pub(super) fn describe_clash_repeat_process(
    repeat: &crate::effects::RepeatProcessEffect,
) -> Option<String> {
    if !is_clash_win_predicate(&repeat.predicate) {
        return None;
    }

    let (clash_index, clash) = repeat
        .effects
        .iter()
        .enumerate()
        .find_map(|(index, effect)| {
            let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
            (with_id.id == repeat.condition)
                .then(|| {
                    with_id
                        .effect
                        .downcast_ref::<crate::effects::ClashEffect>()
                        .map(|clash| (index, clash))
                })
                .flatten()
        })?;
    if clash_index + 1 != repeat.effects.len() {
        return None;
    }

    let clash_text = match clash.opponent_mode {
        crate::effects::ClashOpponentMode::AnyOpponent => "Clash with an opponent",
        crate::effects::ClashOpponentMode::TargetOpponent => "Clash with target opponent",
        crate::effects::ClashOpponentMode::DefendingPlayer => "Clash with defending player",
    };
    let mut setup = describe_effect_list(&repeat.effects[..clash_index]);
    setup = setup.trim().trim_end_matches('.').to_string();
    if setup.is_empty() {
        return Some(format!("{clash_text}. If you win, repeat this process"));
    }

    if setup.starts_with("Lose ")
        && repeat.effects.first().is_some_and(|effect| {
            unwrap_basic_tag_wrappers(effect)
                .downcast_ref::<crate::effects::LoseLifeEffect>()
                .is_some_and(|lose| lose.player == ChooseSpec::Player(PlayerFilter::You))
        })
    {
        setup = format!("You {}", lowercase_first(&setup));
    }

    Some(format!(
        "{setup}, then {}. If you win, repeat this process",
        lowercase_first(clash_text)
    ))
}

fn linked_result_actor(effect: &Effect) -> Option<PlayerFilter> {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
        return Some(discard.player.clone());
    }
    if let Some(sacrifice) = sacrifice_view(effect) {
        return Some(sacrifice.player.clone());
    }
    if let Some(pay) = effect.downcast_ref::<crate::effects::PayManaEffect>() {
        return choose_spec_player_filter(&pay.player);
    }
    if let Some(pay) = effect.downcast_ref::<crate::effects::PayLifeEffect>() {
        return choose_spec_player_filter(&pay.player);
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return Some(
            move_to_zone
                .actor_surface
                .clone()
                .unwrap_or(PlayerFilter::You),
        );
    }
    if effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some()
    {
        return Some(PlayerFilter::You);
    }
    if effect
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
        .is_some()
    {
        return Some(PlayerFilter::You);
    }
    effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .map(|_| PlayerFilter::You)
}

fn linked_result_player_pronoun(player: &PlayerFilter) -> &'static str {
    if *player == PlayerFilter::You {
        "you"
    } else {
        "they"
    }
}

fn describe_linked_action_result_condition(
    effect: &Effect,
    predicate: &EffectPredicate,
) -> Option<String> {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        let who = linked_result_player_pronoun(&unless_pays.player);
        return match predicate {
            EffectPredicate::DidNotHappen | EffectPredicate::WasDeclined => {
                Some(format!("If {who} do"))
            }
            EffectPredicate::Happened => Some(format!("If {who} don't")),
            _ => None,
        };
    }

    let actor = linked_result_actor(effect)?;
    let who = linked_result_player_pronoun(&actor);
    if effect
        .downcast_ref::<crate::effects::PayManaEffect>()
        .is_some()
        || effect
            .downcast_ref::<crate::effects::PayLifeEffect>()
            .is_some()
    {
        return match predicate {
            EffectPredicate::DidNotHappen | EffectPredicate::WasDeclined => {
                Some(format!("If {who} don't"))
            }
            EffectPredicate::Happened | EffectPredicate::Chosen => Some(format!("If {who} do")),
            _ => None,
        };
    }
    match predicate {
        EffectPredicate::DidNotHappen => Some(format!("If {who} can't")),
        EffectPredicate::Happened | EffectPredicate::Chosen => Some(format!("If {who} do")),
        _ => None,
    }
}

fn describe_may_result_condition(may: &crate::effects::MayEffect, accepted: bool) -> String {
    let who = may
        .decider
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    match may.decider.as_ref() {
        None | Some(PlayerFilter::You) => {
            if accepted {
                "If you do".to_string()
            } else {
                "If you don't".to_string()
            }
        }
        Some(PlayerFilter::Target(inner)) if matches!(inner.as_ref(), PlayerFilter::Opponent) => {
            if accepted {
                "If they do".to_string()
            } else {
                "If they don't".to_string()
            }
        }
        _ => {
            if accepted {
                format!("If {who} does")
            } else {
                format!("If {who} doesn't")
            }
        }
    }
}

/// The non-taken branch of these exact result-producing actions has an
/// explicit, structurally provable condition. Other setup/predicate pairs do
/// not imply an authored inverse clause and retain the generic `Otherwise`.
pub(crate) fn describe_explicit_alternative_result_condition(
    effect: &Effect,
    predicate: &EffectPredicate,
) -> Option<String> {
    if effect
        .downcast_ref::<crate::effects::FlipCoinEffect>()
        .is_some_and(|flip| flip.player == PlayerFilter::You)
    {
        return match predicate {
            EffectPredicate::Happened => Some("If you lose the flip".to_string()),
            EffectPredicate::DidNotHappen => Some("If you win the flip".to_string()),
            _ => None,
        };
    }
    let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
    match predicate {
        EffectPredicate::Happened | EffectPredicate::Chosen => {
            Some(describe_may_result_condition(may, false))
        }
        EffectPredicate::DidNotHappen | EffectPredicate::WasDeclined => {
            Some(describe_may_result_condition(may, true))
        }
        _ => None,
    }
}

pub(crate) fn describe_with_id_if_clause(
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if let Some(compact) = describe_declined_may_mill_then_damage(with_id, if_effect) {
        return Some(compact);
    }
    if let Some(compact) = describe_removed_counters_then_exile_by_mana_value(with_id, if_effect) {
        return Some(compact);
    }

    let setup_is_coin_flip = with_id
        .effect
        .downcast_ref::<crate::effects::FlipCoinEffect>()
        .is_some();
    let then_text = setup_is_coin_flip
        .then(|| describe_coin_flip_outcome_branch(&if_effect.then))
        .flatten()
        .or_else(|| describe_conditional_branch_effect_list(&if_effect.then))
        .unwrap_or_else(|| describe_result_branch_effect_list(&if_effect.then));
    let else_text = describe_effect_list(&if_effect.else_);

    let condition = if with_id
        .effect
        .downcast_ref::<crate::effects::ClashEffect>()
        .is_some()
    {
        if is_clash_win_predicate(&if_effect.predicate) {
            "If you win".to_string()
        } else if matches!(if_effect.predicate, EffectPredicate::DidNotHappen) {
            "Otherwise".to_string()
        } else {
            format!("If {}", describe_effect_predicate(&if_effect.predicate))
        }
    } else if with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .and_then(|for_players| for_players.effects.first())
        .and_then(|effect| effect.downcast_ref::<crate::effects::TaggedEffect>())
        .and_then(|tagged| tagged.effect.downcast_ref::<crate::effects::MillEffect>())
        .is_some()
        && matches!(if_effect.predicate, EffectPredicate::Happened)
    {
        "When one or more cards are milled this way".to_string()
    } else if let Some(condition) =
        describe_linked_action_result_condition(&with_id.effect, &if_effect.predicate)
    {
        condition
    } else if matches!(if_effect.predicate, EffectPredicate::DealtDamageToPlayer) {
        "If a player is dealt damage this way".to_string()
    } else if with_id
        .effect
        .downcast_ref::<crate::effects::DiscardEffect>()
        .is_some_and(|discard| discard.player == PlayerFilter::DamagedPlayer)
        && matches!(if_effect.predicate, EffectPredicate::Happened)
    {
        "If the player does".to_string()
    } else if let Some(may) = with_id.effect.downcast_ref::<crate::effects::MayEffect>() {
        if let Some(condition) = describe_may_have_source_deal_damage_condition(may, if_effect) {
            condition
        } else {
            match if_effect.predicate {
                EffectPredicate::DidNotHappen | EffectPredicate::WasDeclined => {
                    describe_may_result_condition(may, false)
                }
                EffectPredicate::Happened | EffectPredicate::Chosen => {
                    describe_may_result_condition(may, true)
                }
                _ => format!("If {}", describe_effect_predicate(&if_effect.predicate)),
            }
        }
    } else if let Some(for_players) = with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
    {
        if for_players.effects.len() == 1
            && for_players.effects[0]
                .downcast_ref::<crate::effects::MayEffect>()
                .is_some()
        {
            let who = describe_for_each_player_filter(&for_players.filter);
            match if_effect.predicate {
                EffectPredicate::DidNotHappen => format!("If {who} doesn't"),
                _ => format!("If {who} does"),
            }
        } else {
            match if_effect.predicate {
                EffectPredicate::Happened => "If it happened".to_string(),
                EffectPredicate::HappenedNotReplaced => {
                    "If it happened and wasn't replaced".to_string()
                }
                _ => format!("If {}", describe_effect_predicate(&if_effect.predicate)),
            }
        }
    } else if let Some(roll_die) = with_id
        .effect
        .downcast_ref::<crate::effects::RollDieEffect>()
    {
        if let EffectPredicate::Value(cmp) = &if_effect.predicate {
            let player = describe_player_filter(&roll_die.player);
            let result_text = describe_roll_result_comparison(cmp)?;
            let verb = player_verb(&player, "roll", "rolls");
            if matches!(cmp, Comparison::Equal(_)) {
                format!("If the result is {result_text}")
            } else if player == "you" {
                format!("If you roll {result_text}")
            } else {
                format!("If {player} {verb} {result_text}")
            }
        } else {
            format!("If {}", describe_effect_predicate(&if_effect.predicate))
        }
    } else if let Some(repeat) = with_id
        .effect
        .downcast_ref::<crate::effects::RepeatEffectsEffect>()
    {
        if let EffectPredicate::Value(cmp) = &if_effect.predicate {
            if repeat.effects.len() == 1
                && let Some(roll_die) =
                    repeat.effects[0].downcast_ref::<crate::effects::RollDieEffect>()
            {
                let player = describe_player_filter(&roll_die.player);
                let result_text = describe_roll_result_comparison(cmp)?;
                let verb = player_verb(&player, "roll", "rolls");
                if matches!(cmp, Comparison::Equal(_)) {
                    format!("If the result is {result_text}")
                } else if player == "you" {
                    format!("If you roll {result_text}")
                } else {
                    format!("If {player} {verb} {result_text}")
                }
            } else {
                format!("If {}", describe_effect_predicate(&if_effect.predicate))
            }
        } else {
            format!("If {}", describe_effect_predicate(&if_effect.predicate))
        }
    } else if setup_is_coin_flip {
        match if_effect.predicate {
            EffectPredicate::Happened => "If you win the flip".to_string(),
            EffectPredicate::DidNotHappen => "If you lose the flip".to_string(),
            _ => format!("If {}", describe_effect_predicate(&if_effect.predicate)),
        }
    } else if effect_moves_object_to_exile(&with_id.effect)
        && matches!(if_effect.predicate, EffectPredicate::Happened)
    {
        if is_reflexive_choose_one_followup(if_effect, &then_text) {
            "When you do".to_string()
        } else if then_text.contains("except it's a ") {
            "If you exiled a card this way".to_string()
        } else {
            "If you do".to_string()
        }
    } else if let Some(target) = excess_damage_condition_target_from_effect(&with_id.effect)
        && matches!(if_effect.predicate, EffectPredicate::ExcessDamageDealt)
    {
        format!("If excess damage was dealt to {target} this way")
    } else {
        match if_effect.predicate {
            EffectPredicate::Happened => "If it happened".to_string(),
            EffectPredicate::HappenedNotReplaced => {
                let setup_is_destroy = with_id
                    .effect
                    .downcast_ref::<crate::effects::DestroyEffect>()
                    .is_some()
                    || with_id
                        .effect
                        .downcast_ref::<crate::effects::TaggedEffect>()
                        .and_then(|tagged| {
                            tagged
                                .effect
                                .downcast_ref::<crate::effects::DestroyEffect>()
                        })
                        .is_some();
                if setup_is_destroy {
                    "If that permanent dies this way".to_string()
                } else {
                    "If it happened and wasn't replaced".to_string()
                }
            }
            _ => format!("If {}", describe_effect_predicate(&if_effect.predicate)),
        }
    };

    let then_text =
        if let Some(compact) = describe_unless_damage_paid_followup_branch(with_id, if_effect) {
            compact
        } else if matches!(condition.as_str(), "If you do" | "If you don't") {
            strip_redundant_where_x_suffix_after_setup(then_text, &with_id.effect, &if_effect.then)
        } else {
            then_text
        };

    if else_text.is_empty() {
        Some(format!("{condition}, {}", lowercase_first(&then_text)))
    } else if let Some(alternative_condition) =
        describe_explicit_alternative_result_condition(&with_id.effect, &if_effect.predicate)
    {
        Some(format!(
            "{condition}, {}. {alternative_condition}, {}",
            lowercase_first(&then_text),
            lowercase_first(&else_text)
        ))
    } else {
        Some(format!(
            "{condition}, {}. Otherwise, {}",
            lowercase_first(&then_text),
            lowercase_first(&else_text)
        ))
    }
}

pub(super) fn describe_unless_damage_paid_followup_branch(
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if !matches!(if_effect.predicate, EffectPredicate::DidNotHappen) {
        return None;
    }
    let unless_pays = unwrap_basic_tag_wrappers(&with_id.effect)
        .downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    single_damage_effect_view(&unless_pays.effects)?;
    let branch_damage = single_damage_effect_view(&if_effect.then)?;
    if branch_damage.unpreventable {
        return None;
    }

    let (amount, where_x) = describe_damage_amount_clause(&branch_damage.amount);
    let mut text = format!(
        "Deal {amount} to {}",
        describe_damage_target(&branch_damage.target)
    );
    if let Some(where_x) = where_x {
        text.push_str(&format!(", where X is {where_x}"));
    }
    Some(text)
}

pub(super) fn single_damage_effect_view(
    effects: &[Effect],
) -> Option<&crate::effects::DealDamageEffect> {
    let [effect] = effects else {
        return None;
    };
    damage_effect_view(effect)
}

pub(super) fn damage_effect_view(effect: &Effect) -> Option<&crate::effects::DealDamageEffect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return damage_effect_view(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return damage_effect_view(&tag_all.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return damage_effect_view(&with_id.effect);
    }
    if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        return with_source
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>();
    }
    effect.downcast_ref::<crate::effects::DealDamageEffect>()
}

pub(in crate::compiled_text) fn damage_with_source_view(
    effect: &Effect,
) -> Option<(Option<&ChooseSpec>, &crate::effects::DealDamageEffect)> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return damage_with_source_view(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return damage_with_source_view(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return damage_with_source_view(&tag_all.effect);
    }
    if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        let damage = with_source
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()?;
        return Some((Some(&with_source.source), damage));
    }
    effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .map(|damage| (None, damage))
}

pub(super) fn compatible_damage_sources<'a>(
    outer: Option<&'a ChooseSpec>,
    inner: Option<&'a ChooseSpec>,
) -> Option<Option<&'a ChooseSpec>> {
    match (outer, inner) {
        (None, None) => Some(None),
        (Some(source), None) | (None, Some(source)) => Some(Some(source)),
        (Some(outer), Some(inner)) if outer.unhinted() == inner.unhinted() => Some(Some(outer)),
        _ => None,
    }
}

pub(super) fn describe_damage_fanout_filter(filter: &ObjectFilter) -> Option<String> {
    if !matches!(filter.zone, None | Some(Zone::Battlefield))
        || describe_tagged_this_way_action(filter).is_some()
    {
        return None;
    }

    // Creature/permanent damage instructions conventionally quantify the
    // battlefield implicitly ("each creature"), while the generic filter
    // renderer must retain explicit provenance for zone-sensitive actions.
    // Remove the zone only in this tightly scoped damage surface.
    let mut display_filter = filter.clone();
    display_filter.zone = None;
    let rendered = describe_for_each_count_filter(&display_filter);
    let rendered = conjoin_quantified_card_types(rendered, &display_filter.card_types);
    (!rendered.trim().is_empty()).then_some(rendered)
}

fn conjoin_quantified_card_types(mut rendered: String, card_types: &[CardType]) -> String {
    if card_types.len() < 2 {
        return rendered;
    }

    let names = card_types
        .iter()
        .map(|card_type| describe_card_type_word_local(*card_type).to_string())
        .collect::<Vec<_>>();
    let conjunction = join_with_and(&names);
    let disjunction = join_with_or(&names);
    if rendered.contains(&disjunction) {
        return rendered.replacen(&disjunction, &conjunction, 1);
    }

    // Count surfaces pluralize the first type in a compound card noun
    // ("instants or sorcery cards"). A quantified union instead uses the
    // singular type adjectives: "instant and sorcery cards".
    let mut plural_first_names = names.clone();
    plural_first_names[0] = pluralize_noun_phrase(&plural_first_names[0]);
    let plural_first_disjunction = join_with_or(&plural_first_names);
    if rendered.contains(&plural_first_disjunction) {
        rendered = rendered.replacen(&plural_first_disjunction, &conjunction, 1);
    }
    rendered
}

fn damage_count_filter(value: &Value) -> Option<&ObjectFilter> {
    match value.unhinted() {
        Value::Count(filter) | Value::CountScaled(filter, _) | Value::GreatestCount(filter) => {
            Some(filter)
        }
        Value::Scaled(inner, _) => damage_count_filter(inner),
        _ => None,
    }
}

fn describe_damage_source_subject(source: &ChooseSpec) -> String {
    let has_explicit_surface = source.source_reference_surface().is_some();
    let mut subject = describe_choose_spec(source);
    if subject == "this source" {
        subject = "this creature".to_string();
    } else if subject == "it" && !has_explicit_surface {
        subject = "that creature".to_string();
    } else if subject.eq_ignore_ascii_case("target creature") {
        subject = "that creature".to_string();
    }
    subject
}

/// Render the structural bulk-damage shape produced by `DealDamageEach` and
/// by power-damage whose target is an unselected object set. This intentionally
/// runs before the generic per-object loop surface: the loop is an execution
/// detail, while oracle text uses "each ..." and states the source/amount once.
pub(super) fn describe_for_each_iterated_damage(
    for_each: &crate::effects::ForEachObject,
    outer_source: Option<&ChooseSpec>,
) -> Option<String> {
    let [inner_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let (inner_source, damage) = damage_with_source_view(inner_effect)?;
    let source = compatible_damage_sources(outer_source, inner_source)?;
    if damage.unpreventable || !matches!(damage.target.base(), ChooseSpec::Iterated) {
        return None;
    }

    let filter = describe_damage_fanout_filter(&for_each.filter)?;
    if source.is_some_and(|source| matches!(source.base(), ChooseSpec::Iterated)) {
        let stat = match damage.amount.unhinted() {
            Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Iterated) => "power",
            Value::ToughnessOf(spec) if matches!(spec.base(), ChooseSpec::Iterated) => "toughness",
            Value::SourcePower => "power",
            Value::SourceToughness => "toughness",
            _ => return None,
        };
        return Some(format!(
            "Each {filter} deals damage to itself equal to its {stat}"
        ));
    }

    let (mut amount, mut where_x) = describe_damage_amount_clause(&damage.amount);
    if let Some(count_filter) = damage_count_filter(&damage.amount) {
        amount = conjoin_quantified_card_types(amount, &count_filter.card_types);
        where_x =
            where_x.map(|basis| conjoin_quantified_card_types(basis, &count_filter.card_types));
    }
    let mut rendered = if let Some(source) = source {
        let subject = describe_damage_source_subject(source);
        let verb = if choose_spec_is_plural(source) {
            "deal"
        } else {
            "deals"
        };
        format!("{subject} {verb} {amount} to each {filter}")
    } else {
        format!("Deal {amount} to each {filter}")
    };
    if let Some(where_x) = where_x {
        rendered.push_str(&format!(", where X is {where_x}"));
    }
    Some(rendered)
}

pub(super) fn strip_redundant_where_x_suffix_after_setup(
    then_text: String,
    setup_effect: &Effect,
    then_effects: &[Effect],
) -> String {
    let setup_text = describe_effect(setup_effect);
    let Some((then_head, then_basis)) = then_text.rsplit_once(", where X is ") else {
        return then_text;
    };
    let text_basis_matches = setup_text
        .rsplit_once(", where X is ")
        .filter(|(setup_head, _)| !setup_head.is_empty())
        .is_some_and(|(_, setup_basis)| {
            then_basis.trim_end_matches('.') == setup_basis.trim_end_matches('.')
        });
    let value_basis_matches = setup_where_x_value(setup_effect)
        .zip(then_where_x_value(then_effects))
        .is_some_and(|(setup_value, then_value)| {
            values_equivalent_ignoring_source_surface(setup_value, then_value)
        });
    if text_basis_matches || value_basis_matches {
        then_head.to_string()
    } else {
        then_text
    }
}

pub(super) fn setup_where_x_value(effect: &Effect) -> Option<&Value> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return setup_where_x_value(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return setup_where_x_value(&tagged.effect);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return may.effects.iter().find_map(effect_where_x_value);
    }
    if let Some(pay_mana) = effect.downcast_ref::<crate::effects::PayManaEffect>() {
        return pay_mana.x_value.as_ref();
    }
    effect_where_x_value(effect)
}

pub(super) fn then_where_x_value(effects: &[Effect]) -> Option<&Value> {
    effects.iter().find_map(effect_where_x_value)
}

pub(super) fn effect_where_x_value(effect: &Effect) -> Option<&Value> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_where_x_value(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_where_x_value(&tagged.effect);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return may.effects.iter().find_map(effect_where_x_value);
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        return Some(&draw.count);
    }
    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        return Some(&lose.amount);
    }
    if let Some(pay) = effect.downcast_ref::<crate::effects::PayLifeEffect>() {
        return Some(&pay.amount);
    }
    if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        return Some(&gain.amount);
    }
    if let Some(mill) = effect.downcast_ref::<crate::effects::MillEffect>() {
        return Some(&mill.count);
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some(&damage.amount);
    }
    None
}

pub(super) fn values_equivalent_ignoring_source_surface(left: &Value, right: &Value) -> bool {
    let left = left.unhinted();
    let right = right.unhinted();
    if left == right {
        return true;
    }
    match (left, right) {
        (
            Value::CountersOn(left_spec, left_counter),
            Value::CountersOn(right_spec, right_counter),
        ) => {
            left_counter == right_counter
                && choose_specs_equivalent_ignoring_source_surface(left_spec, right_spec)
        }
        (
            Value::CountersOnSource(left_counter),
            Value::CountersOn(right_spec, Some(right_counter)),
        )
        | (
            Value::CountersOn(right_spec, Some(right_counter)),
            Value::CountersOnSource(left_counter),
        ) => left_counter == right_counter && matches!(right_spec.unhinted(), ChooseSpec::Source),
        _ => false,
    }
}

pub(super) fn choose_specs_equivalent_ignoring_source_surface(
    left: &ChooseSpec,
    right: &ChooseSpec,
) -> bool {
    left.unhinted() == right.unhinted()
}

pub(super) fn object_filters_equivalent_ignoring_source_surface(
    left: &ObjectFilter,
    right: &ObjectFilter,
) -> bool {
    if left == right {
        return true;
    }
    fn clear_surface(filter: &mut ObjectFilter) {
        filter.source_surface = None;
        for child in &mut filter.any_of {
            clear_surface(child);
        }
    }
    let mut left = left.clone();
    let mut right = right.clone();
    clear_surface(&mut left);
    clear_surface(&mut right);
    left == right
}

pub(super) fn describe_conditional_branch_effect_list(effects: &[Effect]) -> Option<String> {
    describe_lose_life_then_create_shared_dynamic_branch(effects)
        .or_else(|| describe_destroy_no_regeneration_this_way_branch(effects))
        .or_else(|| describe_conditional_dynamic_token_branch(effects))
        .or_else(|| describe_copy_then_choose_new_targets_branch(effects))
        .or_else(|| describe_remove_from_combat_then_tap_branch(effects))
        .or_else(|| describe_gain_control_then_may_retarget_branch(effects))
        .or_else(|| describe_compact_choose_mode_branch(effects))
        .or_else(|| describe_source_labeled_choose_mode_branch(effects))
}

fn describe_coin_flip_outcome_branch(effects: &[Effect]) -> Option<String> {
    describe_conditional_branch_effect_list(effects)
        .or_else(|| describe_triggering_spell_return_branch(effects))
        .or_else(|| describe_tagged_counter_spell_branch(effects))
}

fn wrapped_with_id(effect: &Effect) -> Option<&crate::effects::WithIdEffect> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Some(with_id);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return wrapped_with_id(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return wrapped_with_id(&tag_all.effect);
    }
    None
}

fn describe_copy_then_choose_new_targets_branch(effects: &[Effect]) -> Option<String> {
    let [copy_effect, retarget_effect] = effects else {
        return None;
    };
    let copy_with_id = wrapped_with_id(copy_effect)?;
    let copy = copy_with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    let retarget = unwrap_basic_tag_wrappers(retarget_effect)
        .downcast_ref::<crate::effects::ChooseNewTargetsEffect>()?;
    if copy.count != Value::Fixed(1)
        || !copy.removed_supertypes.is_empty()
        || retarget.from_effect != copy_with_id.id
        || !retarget.may
        || !matches!(retarget.chooser, None | Some(PlayerFilter::You))
    {
        return None;
    }

    let copied_spell = describe_stack_object_copy_target(&copy.target);
    Some(format!(
        "Copy {copied_spell}, and you may choose new targets for the copy"
    ))
}

fn describe_remove_from_combat_then_tap_branch(effects: &[Effect]) -> Option<String> {
    let [remove_effect, tap_effect] = effects else {
        return None;
    };
    let remove = unwrap_basic_tag_wrappers(remove_effect)
        .downcast_ref::<crate::effects::RemoveFromCombatEffect>()?;
    let tap = unwrap_basic_tag_wrappers(tap_effect).downcast_ref::<crate::effects::TapEffect>()?;
    if remove.spec.unhinted() != tap.target.unhinted() {
        return None;
    }

    Some(format!(
        "Remove {} from combat and tap it",
        describe_choose_spec(&remove.spec)
    ))
}

fn describe_gain_control_then_may_retarget_branch(effects: &[Effect]) -> Option<String> {
    let [control_effect, may_effect] = effects else {
        return None;
    };
    let control = unwrap_basic_tag_wrappers(control_effect)
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !is_gain_control_effect(control) || control.until != Until::Forever {
        return None;
    }
    let controlled = control.target_spec.as_ref()?;

    let may = unwrap_basic_tag_wrappers(may_effect).downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You)) {
        return None;
    }
    let [retarget_effect] = may.effects.as_slice() else {
        return None;
    };
    let retarget = unwrap_basic_tag_wrappers(retarget_effect)
        .downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
    if retarget.chooser != PlayerFilter::You
        || !matches!(retarget.mode, crate::effects::RetargetMode::All)
        || retarget.require_change
        || retarget.new_target_restriction.is_some()
        || controlled.unhinted() != retarget.target.unhinted()
    {
        return None;
    }

    Some("You gain control of that spell and may choose new targets for it".to_string())
}

fn describe_triggering_spell_return_branch(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let returned =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    if returned.destination_player_surface.is_some()
        || returned.exiled_with_source_surface.is_some()
        || !matches!(returned.spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
    {
        return None;
    }

    Some("Return that spell to its owner's hand".to_string())
}

fn describe_tagged_counter_spell_branch(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let counter =
        unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::CounterEffect>()?;
    if !matches!(counter.target.base(), ChooseSpec::Tagged(_)) {
        return None;
    }
    Some("Counter that spell".to_string())
}

pub(super) fn describe_lose_life_then_create_shared_dynamic_branch(
    effects: &[Effect],
) -> Option<String> {
    let [lose_effect, create_effect] = effects else {
        return None;
    };
    let lose =
        unwrap_basic_tag_wrappers(lose_effect).downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let create = unwrap_basic_tag_wrappers(create_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if !values_equivalent_ignoring_source_surface(&lose.amount, &create.count) {
        return None;
    }

    let where_x = describe_where_x_basis(&lose.amount)?;
    let suffix = format!(", where X is {where_x}");
    let lose_text = describe_effect(lose_effect);
    let create_text = describe_effect(create_effect);
    let lose_clause = lose_text.strip_suffix(&suffix)?;
    let create_clause = create_text.strip_suffix(&suffix)?;

    Some(format!(
        "{lose_clause} and {}{suffix}",
        lowercase_first(create_clause)
    ))
}

pub(super) fn describe_compact_choose_mode_branch(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let choose = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    describe_endure_mode(choose)
        .or_else(|| describe_tap_or_untap_mode(choose))
        .or_else(|| describe_put_counter_choice_mode(choose))
        .or_else(|| describe_put_or_remove_counter_mode(choose))
}

pub(super) fn describe_conditional_dynamic_token_branch(effects: &[Effect]) -> Option<String> {
    let [create_effect, set_pt_effect] = effects else {
        return None;
    };
    let create = unwrap_basic_tag_wrappers(create_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let set_pt = unwrap_basic_tag_wrappers(set_pt_effect)
        .downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()?;
    let created_tag = wrapped_effect_tag(create_effect)?;
    if create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::You
        || create.controller_target.is_some()
        || create.enters_attacking
        || create.exile_at_end_of_combat
        || create.sacrifice_at_end_of_combat
        || create.sacrifice_at_next_end_step
        || create.exile_at_next_end_step
        || set_pt.duration != Until::Forever
        || set_pt.power.unhinted() != set_pt.toughness.unhinted()
        || matches!(set_pt.power.unhinted(), Value::Fixed(_))
        || !matches!(&set_pt.target, ChooseSpec::Tagged(tag) if tag == created_tag)
    {
        return None;
    }

    let token_blueprint = describe_token_blueprint(&create.token);
    let token_phrase = if let Some(rest) = token_blueprint.strip_prefix("0/0 ") {
        if create.enters_tapped {
            format!("tapped X/X {rest}")
        } else {
            format!("X/X {rest}")
        }
    } else if token_blueprint.starts_with("X/X ") {
        if create.enters_tapped {
            format!("tapped {token_blueprint}")
        } else {
            token_blueprint
        }
    } else {
        return None;
    };

    let where_x =
        describe_where_x_basis(&set_pt.power).unwrap_or_else(|| describe_value(&set_pt.power));
    Some(format!(
        "create {}, where X is {where_x}",
        with_indefinite_article(&token_phrase)
    ))
}

pub(super) fn describe_source_labeled_choose_mode_branch(effects: &[Effect]) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let choose = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose.random
        || choose.allow_repeated_modes
        || choose.disallow_previously_chosen_modes
        || choose.mode_point_costs.iter().any(|cost| *cost != 1)
    {
        return None;
    }
    let modes = choose
        .modes
        .iter()
        .map(|mode| {
            let source = mode.source_text.trim();
            (!source.is_empty()).then(|| ensure_trailing_period(source.trim_end_matches('.')))
        })
        .collect::<Option<Vec<_>>>()?;
    if modes.is_empty() {
        return None;
    }
    let header = describe_mode_choice_header(
        &choose.choose_count,
        Some(&choose.min_choose_count),
        Some(choose.modes.len()),
    );
    Some(format!("{header}\n• {}", modes.join("\n• ")))
}

pub(super) fn conditional_branch_destroy_no_regeneration_effect<'a>(
    effect: &'a Effect,
) -> Option<&'a crate::effects::DestroyNoRegenerationEffect> {
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        return Some(destroy);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return conditional_branch_destroy_no_regeneration_effect(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return conditional_branch_destroy_no_regeneration_effect(&tag_all.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return conditional_branch_destroy_no_regeneration_effect(&with_id.effect);
    }
    None
}

pub(super) fn describe_destroy_no_regeneration_this_way_branch(
    effects: &[Effect],
) -> Option<String> {
    let [effect] = effects else {
        return None;
    };
    let destroy = conditional_branch_destroy_no_regeneration_effect(effect)?;
    let ChooseSpec::All(filter) = destroy.spec.base() else {
        return None;
    };
    let noun = simple_filter_plural_noun(filter)?;
    Some(format!(
        "Destroy all {noun}. {} destroyed this way can't be regenerated",
        capitalize_first(&noun)
    ))
}

pub(super) fn describe_removed_counters_then_exile_by_mana_value(
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 1
    {
        return None;
    }

    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let removed_counters = may.effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .is_some_and(|nested| {
                nested.id == with_id.id
                    && nested
                        .effect
                        .downcast_ref::<crate::effects::RemoveCountersEffect>()
                        .is_some()
            })
            || effect
                .downcast_ref::<crate::effects::RemoveCountersEffect>()
                .is_some()
    });
    if !removed_counters {
        return None;
    }

    let exile = if_effect.then[0].downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(filter) = &exile.spec else {
        return None;
    };
    if !is_nonland_permanent_filter(filter) || !filter.other {
        return None;
    }
    let Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) = filter.mana_value.as_ref()
    else {
        return None;
    };
    if !matches!(value.as_ref(), Value::EffectValue(id) if *id == with_id.id) {
        return None;
    }

    let text = "If you do, exile each other nonland permanent with mana value less than or equal to the number of counters removed this way";
    Some(text.to_string())
}

pub(super) fn describe_declined_may_mill_then_damage(
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if !matches!(if_effect.predicate, EffectPredicate::DidNotHappen)
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 2
    {
        return None;
    }
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(
        may.decider.as_ref(),
        Some(crate::filter::PlayerFilter::Target(inner))
            if matches!(inner.as_ref(), crate::filter::PlayerFilter::Opponent)
    ) {
        return None;
    }

    let tagged_mill = if_effect.then[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let mill = tagged_mill
        .effect
        .downcast_ref::<crate::effects::MillEffect>()?;
    let damage = if_effect.then[1].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(mill.player, PlayerFilter::You)
        || !matches!(
            damage.target,
            ChooseSpec::Player(crate::filter::PlayerFilter::Target(ref inner))
                if matches!(inner.as_ref(), crate::filter::PlayerFilter::Opponent)
        )
    {
        return None;
    }
    let Value::TotalManaValue(filter) = &damage.amount else {
        return None;
    };
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == tagged_mill.tag
    }) {
        return None;
    }

    let mill_text = lowercase_first(&describe_effect(&if_effect.then[0]));
    Some(format!(
        "If the player doesn't, {mill_text}, then this creature deals damage to that player equal to the total mana value of those cards"
    ))
}

pub(super) fn describe_pay_mana_cost(pay_mana: &crate::effects::PayManaEffect) -> String {
    let cost = pay_mana.cost.to_oracle();
    match pay_mana.x_value.as_ref() {
        Some(value)
            if cost == "{X}"
                && value.has_surface_hint(ValueSurfaceHint::ForEach)
                && crate::compiled_text::describe_counter_for_each_basis(value).is_some() =>
        {
            let (multiplier, basis) = crate::compiled_text::describe_counter_for_each_basis(value)
                .expect("counter for-each basis checked in match guard");
            format!("{{{multiplier}}} for each {basis}")
        }
        Some(value) => format!("{cost}, where X is {}", describe_value(value)),
        None => cost,
    }
}

pub(super) fn describe_optional_setup_effect_for_if_happened(
    with_id: &crate::effects::WithIdEffect,
) -> Option<String> {
    if let Some(pay_mana) = with_id
        .effect
        .downcast_ref::<crate::effects::PayManaEffect>()
    {
        let player = describe_choose_spec(&pay_mana.player);
        if player == "you" {
            return Some(format!("You may pay {}", describe_pay_mana_cost(pay_mana)));
        }
        return Some(format!(
            "{player} may pay {}",
            describe_pay_mana_cost(pay_mana)
        ));
    }
    if let Some(pay_life) = with_id
        .effect
        .downcast_ref::<crate::effects::PayLifeEffect>()
    {
        let player = describe_choose_spec(&pay_life.player);
        let payment = describe_life_amount_phrase(&pay_life.amount);
        if player == "you" {
            return Some(format!("You may pay {payment}"));
        }
        return Some(format!("{player} may pay {payment}"));
    }

    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You)) || may.effects.len() != 2 {
        return None;
    }
    let pay_mana = unwrap_basic_tag_wrappers(&may.effects[0])
        .downcast_ref::<crate::effects::PayManaEffect>()?;
    let life_payment = unwrap_basic_tag_wrappers(&may.effects[1]);
    let (life_player, life_amount) = if let Some(pay_life) =
        life_payment.downcast_ref::<crate::effects::PayLifeEffect>()
    {
        (&pay_life.player, &pay_life.amount)
    } else if let Some(lose_life) = life_payment.downcast_ref::<crate::effects::LoseLifeEffect>() {
        (&lose_life.player, &lose_life.amount)
    } else {
        return None;
    };
    if describe_choose_spec(&pay_mana.player) != "you"
        || life_player != &ChooseSpec::Player(PlayerFilter::You)
    {
        return None;
    }

    Some(format!(
        "You may pay {} and {}",
        pay_mana.cost.to_oracle(),
        describe_life_amount_phrase(life_amount)
    ))
}

pub(super) fn describe_roll_result_comparison(cmp: &Comparison) -> Option<String> {
    match cmp {
        Comparison::Equal(n) => Some(n.to_string()),
        Comparison::BetweenInclusive(min, max) => Some(format!("{min}-{max}")),
        Comparison::OneOf(values) if !values.is_empty() => {
            let nums = values.iter().map(i32::to_string).collect::<Vec<_>>();
            let list = match nums.len() {
                0 => return None,
                1 => nums[0].clone(),
                2 => format!("{} or {}", nums[0], nums[1]),
                _ => format!(
                    "{}, or {}",
                    nums[..nums.len() - 1].join(", "),
                    nums[nums.len() - 1]
                ),
            };
            Some(list)
        }
        _ => None,
    }
}

pub(super) fn describe_for_players_may_clause(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<(String, String, String)> {
    if for_players.effects.len() != 1 {
        return None;
    }
    let may = for_players.effects[0].downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.is_some() {
        return None;
    }

    let subject = describe_for_players_subject(&for_players.filter)?.to_string();
    let each_player =
        strip_leading_article(&describe_for_each_player_filter(&for_players.filter)).to_string();

    let action = describe_for_players_may_action(&for_players.filter, &may.effects)?;

    Some((subject, each_player, action))
}

pub(super) fn describe_for_players_didnt_followup(effects: &[Effect]) -> Option<String> {
    if effects.len() == 2
        && let Some(lose) = effects[0].downcast_ref::<crate::effects::LoseLifeEffect>()
        && let Some(draw) = effects[1].downcast_ref::<crate::effects::DrawCardsEffect>()
        && matches!(
            lose.player,
            ChooseSpec::Player(PlayerFilter::IteratedPlayer)
        )
        && draw.player == PlayerFilter::You
        && draw.count == Value::Fixed(1)
    {
        let amount = describe_value(&lose.amount);
        return Some(format!(
            "that player loses {amount} life and you draw a card"
        ));
    }

    let mut followup = describe_effect_list(effects);
    if let Some(rest) = followup.strip_prefix("you lose ") {
        followup = format!("that player loses {rest}");
    } else if let Some(rest) = followup.strip_prefix("you ") {
        let normalized = normalize_third_person_verb_phrase(rest);
        followup = format!("that player {normalized}");
    }
    Some(followup)
}

pub(crate) fn describe_with_id_then_for_players_if_didnt(
    with_id: &crate::effects::WithIdEffect,
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let antecedent = with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let same_partition = antecedent.filter == for_players.filter;
    let opponents_within_all =
        antecedent.filter == PlayerFilter::Any && for_players.filter == PlayerFilter::Opponent;
    if (!same_partition && !opponents_within_all) || for_players.effects.len() != 1 {
        return None;
    }
    let if_effect = for_players.effects[0].downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::DidNotHappen
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let followup = describe_for_players_didnt_followup(&if_effect.then)?;

    if antecedent.effects.len() == 1 {
        let setup = describe_effect(&with_id.effect)
            .trim()
            .trim_end_matches('.')
            .to_string();
        let each_player =
            strip_leading_article(&describe_for_each_player_filter(&for_players.filter))
                .to_string();
        if !setup.is_empty()
            && let Some(action) = followup
                .strip_prefix("that player ")
                .or_else(|| followup.strip_prefix("That player "))
        {
            return Some(format!("{setup}. Each {each_player} who can't {action}"));
        }
    }

    let (subject, each_player, action) = describe_for_players_may_clause(antecedent)?;

    if antecedent.filter == PlayerFilter::You {
        Some(format!("{subject} may {action}. If you don't, {followup}"))
    } else {
        Some(format!(
            "{subject} may {action}. For each {each_player} who doesn't, {followup}"
        ))
    }
}

pub(super) fn describe_may_sacrifice_reflexive_condition(
    may: &crate::effects::MayEffect,
    predicate: &EffectPredicate,
) -> Option<String> {
    let [choose_effect, sacrifice_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    describe_choose_then_sacrifice(choose, sacrifice)?;
    describe_counted_reflexive_sacrifice_condition(predicate, choose, sacrifice)
}

pub(super) fn describe_may_choose_then_sacrifice(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    let [choose_effect, sacrifice_effect] = may.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(sacrifice_effect)?;
    let compact = describe_choose_then_sacrifice(choose, sacrifice)?;
    let decider = may.decider.as_ref().unwrap_or(&choose.chooser);
    if decider != &choose.chooser || sacrifice.player != &choose.chooser {
        return None;
    }

    let (from_prefix, to_prefix) = match decider {
        PlayerFilter::Any => ("a player sacrifices ", "any player may sacrifice "),
        PlayerFilter::Opponent => ("an opponent sacrifices ", "any opponent may sacrifice "),
        PlayerFilter::NotYou => (
            "a player other than you sacrifices ",
            "any player other than you may sacrifice ",
        ),
        PlayerFilter::You => ("you sacrifice ", "you may sacrifice "),
        _ => return None,
    };
    let action = compact.strip_prefix(from_prefix)?;
    Some(format!("{to_prefix}{action}"))
}

pub(crate) fn describe_with_id_then_reflexive_trigger(
    with_id: &crate::effects::WithIdEffect,
    reflexive: &crate::effects::ReflexiveTriggerEffect,
) -> Option<String> {
    if reflexive.condition != with_id.id {
        return None;
    }

    let setup = describe_optional_setup_effect_for_if_happened(with_id)
        .unwrap_or_else(|| describe_effect(&with_id.effect));
    let setup = capitalize_first(&setup);
    let triggered = describe_result_branch_effect_list(&reflexive.effects);
    let triggered = lowercase_first(&triggered);
    let condition = if let Some(may) = with_id.effect.downcast_ref::<crate::effects::MayEffect>() {
        let who = may
            .decider
            .as_ref()
            .map(describe_player_filter)
            .unwrap_or_else(|| "you".to_string());
        if let Some(condition) =
            describe_may_sacrifice_reflexive_condition(may, &reflexive.predicate)
        {
            condition
        } else {
            match reflexive.predicate {
                EffectPredicate::DidNotHappen => {
                    if who == "you" {
                        "When you don't".to_string()
                    } else {
                        format!("When {who} doesn't")
                    }
                }
                _ => {
                    if who == "you" {
                        "When you do".to_string()
                    } else {
                        format!("When {who} does")
                    }
                }
            }
        }
    } else {
        match reflexive.predicate {
            EffectPredicate::Happened => "When you do".to_string(),
            EffectPredicate::HappenedNotReplaced => "When you do and it isn't replaced".to_string(),
            EffectPredicate::AffectedObjectMatchesCardType {
                card_type: CardType::Land,
                negated: true,
            } if with_id
                .effect
                .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
                .is_some() =>
            {
                "When you exile a nonland card this way".to_string()
            }
            _ => format!("When {}", describe_effect_predicate(&reflexive.predicate)),
        }
    };

    Some(format!("{setup}. {condition}, {triggered}"))
}

pub(super) fn describe_exile_play_then_reflexive_trigger(
    with_id: &crate::effects::WithIdEffect,
    reflexive: &crate::effects::ReflexiveTriggerEffect,
    grant: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    if reflexive.condition != with_id.id
        || reflexive.predicate
            != (EffectPredicate::AffectedObjectMatchesCardType {
                card_type: CardType::Land,
                negated: true,
            })
    {
        return None;
    }
    let exile = with_id
        .effect
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let setup = describe_exile_top_then_play(exile, grant, false)?;
    let triggered = lowercase_first(&describe_result_branch_effect_list(&reflexive.effects));
    let triggered = triggered
        .replace("its mana value", "the exiled card's mana value")
        .replace("that object's mana value", "the exiled card's mana value");
    Some(format!(
        "{setup}. When you exile a nonland card this way, {triggered}"
    ))
}

pub(crate) fn describe_with_id_then_choose_new_targets(
    with_id: &crate::effects::WithIdEffect,
    choose_new: &crate::effects::ChooseNewTargetsEffect,
) -> Option<String> {
    if choose_new.from_effect != with_id.id {
        return None;
    }

    let base = describe_effect(&with_id.effect);
    let chooser = choose_new
        .chooser
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    let choose_phrase = if choose_new.may {
        if chooser == "you" {
            "You may choose new targets for the copy".to_string()
        } else {
            format!("{chooser} may choose new targets for the copy")
        }
    } else if chooser == "you" {
        "You choose new targets for the copy".to_string()
    } else {
        format!("{chooser} chooses new targets for the copy")
    };

    Some(format!("{base}. {choose_phrase}"))
}

pub(crate) fn describe_with_id_then_may_choose_new_targets(
    with_id: &crate::effects::WithIdEffect,
    may: &crate::effects::MayEffect,
) -> Option<String> {
    let copy_effect = if let Some(tagged) = with_id
        .effect
        .downcast_ref::<crate::effects::TaggedEffect>()
    {
        tagged.effect.as_ref()
    } else {
        with_id.effect.as_ref()
    };
    let _copy_spell = copy_effect.downcast_ref::<crate::effects::CopySpellEffect>()?;
    if may.effects.len() != 1 {
        return None;
    }
    let retarget = may.effects[0].downcast_ref::<crate::effects::RetargetStackObjectEffect>()?;
    if !matches!(retarget.target, ChooseSpec::Tagged(_))
        || !matches!(retarget.mode, crate::effects::RetargetMode::All)
        || retarget.require_change
    {
        return None;
    }

    let base = describe_effect(&with_id.effect);
    let chooser = may
        .decider
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| describe_player_filter(&retarget.chooser));
    let choose_phrase = if chooser == "you" {
        "You may choose new targets for the copy".to_string()
    } else {
        format!("{chooser} may choose new targets for the copy")
    };

    Some(format!("{base}. {choose_phrase}"))
}

pub(super) enum SearchDestination {
    Battlefield {
        tapped: bool,
        controller: PlayerFilter,
    },
    Hand,
    Graveyard,
    Exile,
    LibraryTop,
}

pub(crate) fn describe_search_origin_zones(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let zones = choose_search_zones(choose)?;

    let owner = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    let owner_text = describe_possessive_player_filter(owner);
    let has_zone = |z: Zone| zones.contains(&z);
    let zone_text = match zones.as_slice() {
        [Zone::Library] => format!("{owner_text} library"),
        _ if zones.len() == 3
            && has_zone(Zone::Graveyard)
            && has_zone(Zone::Hand)
            && has_zone(Zone::Library) =>
        {
            let conjunction = if choose.chooser == PlayerFilter::You && *owner == PlayerFilter::You
            {
                "and/or"
            } else {
                "and"
            };
            format!("{owner_text} graveyard, hand, {conjunction} library")
        }
        _ if zones.len() == 2 && has_zone(Zone::Hand) && has_zone(Zone::Library) => {
            format!("{owner_text} hand and library")
        }
        _ if zones.len() == 2 && has_zone(Zone::Graveyard) && has_zone(Zone::Library) => {
            format!("{owner_text} library and/or graveyard")
        }
        _ if zones.len() == 3
            && has_zone(Zone::Library)
            && has_zone(Zone::Graveyard)
            && has_zone(Zone::OutsideGame) =>
        {
            format!("{owner_text} library, graveyard, and/or outside the game")
        }
        [Zone::Graveyard] => format!("{owner_text} graveyard"),
        [Zone::Hand] => format!("{owner_text} hand"),
        [Zone::OutsideGame] => "outside the game".to_string(),
        other => {
            let parts = other
                .iter()
                .map(|zone| match zone {
                    Zone::Battlefield => "battlefield".to_string(),
                    Zone::Hand => "hand".to_string(),
                    Zone::Graveyard => "graveyard".to_string(),
                    Zone::Library => "library".to_string(),
                    Zone::Stack => "stack".to_string(),
                    Zone::Exile => "exile".to_string(),
                    Zone::Command => "command zone".to_string(),
                    Zone::Ante => "ante".to_string(),
                    Zone::OutsideGame => "outside the game".to_string(),
                })
                .collect::<Vec<_>>();
            match parts.as_slice() {
                [only] => format!("{owner_text} {only}"),
                [first, second] => format!("{owner_text} {first} and {second}"),
                [rest @ .., last] => {
                    format!("{owner_text} {} and {last}", rest.join(", "))
                }
                [] => return None,
            }
        }
    };

    Some(zone_text)
}

pub(super) fn describe_delirium_countered_spell_same_name_search(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    let crate::effect::Condition::PlayerHasCardTypesInGraveyardOrMore { player, count } =
        &conditional.condition
    else {
        return None;
    };
    if *player != PlayerFilter::You || *count != 4 || !conditional.if_false.is_empty() {
        return None;
    }

    let [choose_effect, for_each_effect, shuffle_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose.search_mode != SearchSelectionMode::Optional
        || choose.reveal
    {
        return None;
    }
    let zones = choose_search_zones(choose)?;
    if zones.len() != 3
        || !zones.contains(&Zone::Graveyard)
        || !zones.contains(&Zone::Hand)
        || !zones.contains(&Zone::Library)
    {
        return None;
    }
    let same_name_tag = choose
        .filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        })?
        .tag
        .as_str();
    let countered_spell_search = same_name_tag.starts_with("countered_")
        && matches!(
            choose.filter.owner.as_ref(),
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
        );
    let source_exiled_card_search = same_name_tag == crate::tag::SOURCE_EXILED_TAG
        && choose.filter.owner.as_ref() == Some(&PlayerFilter::target_opponent());
    if !countered_spell_search && !source_exiled_card_search {
        return None;
    }
    let mut unqualified_filter = choose.filter.clone();
    unqualified_filter.owner = None;
    unqualified_filter.tagged_constraints.clear();
    if unqualified_filter != ObjectFilter::default() {
        return None;
    }

    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }
    let move_effect =
        if let Some(tagged) = for_each.effects[0].downcast_ref::<crate::effects::TaggedEffect>() {
            tagged.effect.as_ref()
        } else {
            &for_each.effects[0]
        };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Exile
        || move_to_zone.to_top
        || !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let expected_shuffle_player = if countered_spell_search {
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
    } else {
        PlayerFilter::target_opponent()
    };
    if shuffle.player != expected_shuffle_player {
        return None;
    }

    Some(if countered_spell_search {
        concat!(
            "Delirium — If there are four or more card types among cards in your graveyard, ",
            "search the graveyard, hand, and library of that spell's controller for any number ",
            "of cards with the same name as that spell, exile those cards, then that player shuffles"
        )
        .to_string()
    } else {
        concat!(
            "Delirium — If there are four or more card types among cards in your graveyard, ",
            "search that player's graveyard, hand, and library for any number of cards with the ",
            "same name as the exiled card, exile those cards, then that player shuffles"
        )
        .to_string()
    })
}

pub(crate) fn describe_search_choose_for_each(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachTaggedEffect,
    shuffle: Option<&crate::effects::ShuffleLibraryEffect>,
    shuffle_before_move: bool,
) -> Option<String> {
    fn is_same_name_search(filter: &ObjectFilter) -> bool {
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        })
    }

    fn matches_search_move_target(spec: &ChooseSpec, tag: &str) -> bool {
        matches!(spec, ChooseSpec::Iterated)
            || matches!(spec.base(), ChooseSpec::Tagged(found) if found.as_str() == tag)
            || choose_spec_references_exact_tag(spec, &TagKey::from(tag))
    }

    let search_like = choose.is_search
        || (choose.tag.as_str().starts_with("searched_")
            && choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library)));
    if !search_like {
        return None;
    }
    if for_each.tag != choose.tag || for_each.effects.is_empty() || for_each.effects.len() > 2 {
        return None;
    }
    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let searched_multiple_zones = choose_search_zones(choose).is_some_and(|zones| zones.len() > 1);
    let shuffle_clause = if describe_player_filter(search_owner_filter) == "you" {
        "shuffle".to_string()
    } else {
        "that player shuffles".to_string()
    };

    let move_effect = unwrap_basic_tag_wrappers(&for_each.effects[0]);
    let attachment_suffix = if let Some(attach_effect) = for_each.effects.get(1) {
        let attach = unwrap_basic_tag_wrappers(attach_effect)
            .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
        if !matches_search_move_target(&attach.objects, choose.tag.as_str()) {
            return None;
        }
        Some(format!(
            " attached to {}",
            describe_choose_spec(&attach.target)
        ))
    } else {
        None
    };

    let destination =
        if let Some(put) = move_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>() {
            if !matches_search_move_target(&put.target, choose.tag.as_str()) {
                return None;
            }
            SearchDestination::Battlefield {
                tapped: put.tapped,
                controller: put.controller.clone(),
            }
        } else if let Some(return_to_hand) =
            move_effect.downcast_ref::<crate::effects::ReturnToHandEffect>()
        {
            if !matches_search_move_target(&return_to_hand.spec, choose.tag.as_str()) {
                return None;
            }
            SearchDestination::Hand
        } else if let Some(move_to_zone) =
            move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        {
            if !matches_search_move_target(&move_to_zone.target, choose.tag.as_str()) {
                return None;
            }
            if move_to_zone.zone == Zone::Battlefield {
                SearchDestination::Battlefield {
                    tapped: false,
                    controller: choose.chooser.clone(),
                }
            } else if move_to_zone.zone == Zone::Hand {
                SearchDestination::Hand
            } else if move_to_zone.zone == Zone::Graveyard {
                SearchDestination::Graveyard
            } else if move_to_zone.zone == Zone::Exile {
                SearchDestination::Exile
            } else if move_to_zone.zone == Zone::Library && move_to_zone.to_top {
                SearchDestination::LibraryTop
            } else {
                return None;
            }
        } else {
            return None;
        };

    if let Some(shuffle) = shuffle
        && !same_search_player_filter(&shuffle.player, search_owner_filter)
    {
        return None;
    }

    let mut implied_filter = choose.filter.clone();
    // The searched library owner is already called out by "Search ... library".
    implied_filter.owner = None;
    // The search origin already names the library, so repeating "in library" in the
    // object description makes oracle-like text noisier without adding information.
    if searched_library && implied_filter.zone == Some(Zone::Library) {
        implied_filter.zone = None;
    }
    // Keep the library context available while choosing the noun, then remove
    // only its redundant location clause. Explicit permanent-card filters stay
    // on the older typed path so their subtype ordering remains unchanged.
    let implied_filter_text = if implied_filter == ObjectFilter::default() {
        "card".to_string()
    } else if searched_library
        && choose.filter.zone == Some(Zone::Library)
        && !filter_explicitly_selects_permanent_cards(&choose.filter)
    {
        describe_nonbattlefield_card_filter_without_zone(&implied_filter, Zone::Library)
    } else {
        let desc = implied_filter.description();
        // For multi-zone searches the zone is None, so the base noun defaults
        // to "permanent". Replace it with "card" for search contexts.
        if searched_multiple_zones && implied_filter.zone.is_none() {
            desc.replacen("permanent", "card", 1)
        } else {
            desc
        }
    };
    let filter_text = if choose.description.trim().is_empty()
        || choose.description.trim().eq_ignore_ascii_case("choose")
        || choose.description.trim().eq_ignore_ascii_case("objects")
    {
        implied_filter_text
    } else {
        normalize_search_descriptor_for_origin(choose.description.trim(), searched_library)
    };
    let selection_text = if choose.count.max == Some(1) {
        with_indefinite_article(&filter_text)
    } else if describe_runtime_choice_count(choose).is_some() {
        describe_search_selection_from_filter_text(choose, &filter_text)
    } else {
        let mut count_text = describe_choice_count(&choose.count);
        if count_text == "any number" {
            count_text = if is_same_name_search(&choose.filter)
                && choose.search_mode == crate::effect::SearchSelectionMode::AllMatching
            {
                "all".to_string()
            } else {
                "any number of".to_string()
            };
        }
        if filter_text.eq_ignore_ascii_case("card") {
            if count_text == "all" {
                "all cards".to_string()
            } else if count_text == "any number of" {
                "any number of cards".to_string()
            } else {
                format!("{count_text} cards")
            }
        } else {
            format!("{count_text} {}", pluralize_noun_phrase(&filter_text))
        }
    };
    let selection_text = if is_same_name_search(&choose.filter)
        && (!choose.filter.card_types.is_empty()
            || !choose.filter.all_card_types.is_empty()
            || !choose.filter.subtypes.is_empty()
            || choose.filter.name.is_some())
    {
        let mut typed_filter = choose.filter.clone();
        typed_filter.tagged_constraints.retain(|constraint| {
            constraint.relation != crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        });
        let typed_text = if typed_filter == ObjectFilter::default() {
            "card".to_string()
        } else {
            normalize_search_descriptor_for_origin(&typed_filter.description(), searched_library)
        };
        let type_cards = describe_search_selection_with_cards(&typed_text);
        let count_prefix = if choose.count.max == Some(1) {
            "a".to_string()
        } else if choose.count.max.is_some() {
            describe_choice_count(&choose.count)
        } else if choose.search_mode == crate::effect::SearchSelectionMode::Optional {
            "any number of".to_string()
        } else {
            "all".to_string()
        };
        format!("{count_prefix} {type_cards} with that name")
    } else {
        describe_search_selection_with_cards_preserving_where(&selection_text)
    };
    let pronoun = if choose.count.max == Some(1) {
        "it"
    } else {
        "them"
    };
    let reveal_clause = if choose.reveal {
        format!(", reveal {pronoun}")
    } else {
        String::new()
    };
    let same_name_exile_search =
        is_same_name_search(&choose.filter) && matches!(destination, SearchDestination::Exile);
    if attachment_suffix.is_some() && !matches!(&destination, SearchDestination::Battlefield { .. })
    {
        return None;
    }

    let mut text;
    match destination {
        SearchDestination::Battlefield { tapped, controller } => {
            let control_suffix = if same_search_player_filter(&controller, search_owner_filter) {
                String::new()
            } else {
                format!(
                    " under {} control",
                    describe_possessive_player_filter(&controller)
                )
            };
            text = if selection_text.contains(", where X is ") && !shuffle_before_move {
                format!(
                    "Search {search_origin} for {selection_text}{reveal_clause}. Put {pronoun} onto the battlefield{control_suffix}"
                )
            } else if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {search_origin} for {}{}, {}, then put {} onto the battlefield{}",
                    selection_text, reveal_clause, shuffle_clause, pronoun, control_suffix
                )
            } else {
                let put_joiner = if searched_multiple_zones || attachment_suffix.is_some() {
                    " and put"
                } else {
                    ", put"
                };
                format!(
                    "Search {search_origin} for {}{}{put_joiner} {} onto the battlefield{}",
                    selection_text, reveal_clause, pronoun, control_suffix
                )
            };
            if tapped {
                text.push_str(" tapped");
            }
            if let Some(attachment_suffix) = attachment_suffix.as_deref() {
                text.push_str(attachment_suffix);
            }
        }
        SearchDestination::Hand => {
            text = if selection_text.contains(", where X is ") && !shuffle_before_move {
                if let Some(revealed) = reveal_clause.trim_start().strip_prefix(", reveal ") {
                    let move_pronoun = if pronoun == "those cards" {
                        "them"
                    } else {
                        pronoun
                    };
                    format!(
                        "Search {search_origin} for {selection_text}. Reveal {revealed}, put {move_pronoun} into {} hand",
                        describe_possessive_player_filter(search_owner_filter)
                    )
                } else {
                    format!(
                        "Search {search_origin} for {selection_text}. Put {pronoun} into {} hand",
                        describe_possessive_player_filter(search_owner_filter)
                    )
                }
            } else if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {search_origin} for {}{}, {}, then put {} into {} hand",
                    selection_text,
                    reveal_clause,
                    shuffle_clause,
                    pronoun,
                    describe_possessive_player_filter(search_owner_filter)
                )
            } else {
                format!(
                    "Search {search_origin} for {}{}, put {} into {} hand",
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(search_owner_filter)
                )
            };
        }
        SearchDestination::Graveyard => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {search_origin} for {}{}, {}, then put {} into {} graveyard",
                    selection_text,
                    reveal_clause,
                    shuffle_clause,
                    pronoun,
                    describe_possessive_player_filter(search_owner_filter)
                )
            } else {
                format!(
                    "Search {search_origin} for {}{}, put {} into {} graveyard",
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(search_owner_filter)
                )
            };
        }
        SearchDestination::Exile => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {search_origin} for {}{}, {}, then exile {}",
                    selection_text, reveal_clause, shuffle_clause, pronoun,
                )
            } else if selection_text.starts_with("all ") {
                format!(
                    "Search {search_origin} for {}{} and exile {}",
                    selection_text, reveal_clause, pronoun,
                )
            } else if selection_text.contains(", where X is ") {
                format!(
                    "Search {search_origin} for {}{} and exile {}",
                    selection_text, reveal_clause, pronoun,
                )
            } else {
                format!(
                    "Search {search_origin} for {}{} and exile {}",
                    selection_text, reveal_clause, pronoun,
                )
            };
        }
        SearchDestination::LibraryTop => {
            let move_reference =
                if choose.count.max == Some(1) && choose.filter.has_explicit_card_noun() {
                    "the card"
                } else {
                    pronoun
                };
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {search_origin} for {}{}, then {} and put {} on top of {} library",
                    selection_text,
                    reveal_clause,
                    shuffle_clause,
                    move_reference,
                    describe_possessive_player_filter(search_owner_filter)
                )
            } else {
                format!(
                    "Search {search_origin} for {}{}, put {} on top of {} library",
                    selection_text,
                    reveal_clause,
                    move_reference,
                    describe_possessive_player_filter(search_owner_filter)
                )
            };
            if !choose.count.is_single() {
                text.push_str(" in any order");
            }
        }
    }
    if shuffle.is_some() && !shuffle_before_move && searched_library {
        if searched_multiple_zones {
            if same_name_exile_search {
                text.push_str(". ");
                text.push_str(if describe_player_filter(search_owner_filter) == "you" {
                    "Then shuffle"
                } else {
                    "Then that player shuffles"
                });
            } else {
                let conditional_shuffle_clause =
                    if describe_player_filter(search_owner_filter) == "you" {
                        "If you search your library this way, shuffle".to_string()
                    } else {
                        format!(
                            "If {} searches their library this way, {}",
                            describe_player_filter(search_owner_filter),
                            shuffle_clause
                        )
                    };
                text.push_str(". ");
                text.push_str(&conditional_shuffle_clause);
            }
        } else if describe_player_filter(search_owner_filter) == "you" {
            text.push_str(", then shuffle");
        } else {
            text.push_str(". Then that player shuffles");
        }
    }
    Some(text)
}

pub(crate) fn describe_search_choose_then_move(
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: Option<&crate::effects::RevealTaggedEffect>,
    move_to_zone: &crate::effects::MoveToZoneEffect,
    shuffle: Option<&crate::effects::ShuffleLibraryEffect>,
) -> Option<String> {
    let search_like = choose.is_search
        || (choose.tag.as_str().starts_with("searched_")
            && choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library)));
    if !search_like {
        return None;
    }
    if let Some(reveal) = reveal
        && reveal.tag != choose.tag
    {
        return None;
    }
    if !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(found) if found.as_str() == choose.tag.as_str())
    {
        return None;
    }

    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let shuffle_clause = if describe_player_filter(search_owner_filter) == "you" {
        "shuffle".to_string()
    } else {
        "that player shuffles".to_string()
    };

    let mut display_filter = choose.filter.clone();
    display_filter.owner = None;
    if searched_library && display_filter.zone == Some(Zone::Library) {
        display_filter.zone = None;
    }
    // For multi-zone searches the filter zone is None, causing the description
    // to default to "permanent".  Set it to Library so the noun is "card".
    // NOTE: do not set display_filter.zone to Library here for multi-zone
    // searches — that would leak "in library" into the description. Instead,
    // we post-process the noun from "permanent" to "card" below.
    let searched_multiple_zones = choose_search_zones(choose).is_some_and(|zones| zones.len() > 1);
    let raw_filter_text = if display_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        let desc =
            normalize_search_descriptor_for_origin(&display_filter.description(), searched_library);
        if searched_multiple_zones && display_filter.zone.is_none() {
            desc.replacen("permanent", "card", 1)
        } else {
            desc
        }
    };
    let filter_text = describe_search_selection_with_cards_preserving_where(
        &describe_search_selection_from_filter_text(choose, &raw_filter_text),
    );
    let pronoun = if choose.count.max == Some(1) {
        "it"
    } else if filter_text.contains(" cards") {
        "those cards"
    } else {
        "them"
    };
    let reveal_clause = if reveal.is_some() {
        format!(", reveal {pronoun}")
    } else {
        String::new()
    };

    let mut text = match move_to_zone.zone {
        Zone::Hand => {
            if filter_text.contains(", where X is ") {
                if let Some(revealed) = reveal_clause.trim_start().strip_prefix(", reveal ") {
                    let move_pronoun = if pronoun == "those cards" {
                        "them"
                    } else {
                        pronoun
                    };
                    format!(
                        "Search {search_origin} for {filter_text}. Reveal {revealed}, put {move_pronoun} into {} hand",
                        describe_possessive_player_filter(search_owner_filter)
                    )
                } else {
                    format!(
                        "Search {search_origin} for {filter_text}. Put {pronoun} into {} hand",
                        describe_possessive_player_filter(search_owner_filter)
                    )
                }
            } else {
                format!(
                    "Search {search_origin} for {filter_text}{reveal_clause}, put {pronoun} into {} hand",
                    describe_possessive_player_filter(search_owner_filter)
                )
            }
        }
        Zone::Battlefield => {
            let tapped = if move_to_zone.enters_tapped {
                " tapped"
            } else {
                ""
            };
            let controller_suffix = match move_to_zone.battlefield_controller {
                crate::effects::BattlefieldController::Preserve => "",
                crate::effects::BattlefieldController::Owner => {
                    if choose.count.max == Some(1) {
                        " under its owner's control"
                    } else {
                        " under their owners' control"
                    }
                }
                crate::effects::BattlefieldController::You
                    if move_to_zone.controller_surface_explicit =>
                {
                    " under your control"
                }
                crate::effects::BattlefieldController::You => "",
            };
            if filter_text.contains(", where X is ") {
                format!(
                    "Search {search_origin} for {filter_text}{reveal_clause}. Put {pronoun} onto the battlefield{tapped}{controller_suffix}"
                )
            } else {
                format!(
                    "Search {search_origin} for {filter_text}{reveal_clause}, put {pronoun} onto the battlefield{tapped}{controller_suffix}"
                )
            }
        }
        Zone::Graveyard => format!(
            "Search {search_origin} for {filter_text}{reveal_clause}, put {pronoun} into {} graveyard",
            describe_possessive_player_filter(search_owner_filter)
        ),
        Zone::Exile => {
            format!("Search {search_origin} for {filter_text}{reveal_clause}, exile {pronoun}")
        }
        Zone::Library if move_to_zone.to_top => format!(
            "Search {search_origin} for {filter_text}{reveal_clause}, put {pronoun} on top of {} library",
            describe_possessive_player_filter(search_owner_filter)
        ),
        _ => return None,
    };
    if let Some(shuffle) = shuffle {
        if !same_search_player_filter(&shuffle.player, search_owner_filter) {
            return None;
        }
        text.push_str(", then ");
        text.push_str(&shuffle_clause);
    }
    Some(text)
}

pub(super) fn describe_search_choose_then_exile(
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: Option<&crate::effects::RevealTaggedEffect>,
    exile: &crate::effects::ExileEffect,
    shuffle: Option<&crate::effects::ShuffleLibraryEffect>,
) -> Option<String> {
    let search_like = choose.is_search
        || (choose.tag.as_str().starts_with("searched_")
            && choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library)));
    if !search_like {
        return None;
    }
    if let Some(reveal) = reveal
        && reveal.tag != choose.tag
    {
        return None;
    }
    if !matches!(exile.spec.base(), ChooseSpec::Tagged(found) if found.as_str() == choose.tag.as_str())
    {
        return None;
    }

    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let shuffle_clause = if describe_player_filter(search_owner_filter) == "you" {
        "shuffle".to_string()
    } else {
        "that player shuffles".to_string()
    };

    let mut display_filter = choose.filter.clone();
    display_filter.owner = None;
    if searched_library && display_filter.zone == Some(Zone::Library) {
        display_filter.zone = None;
    }
    // For multi-zone searches the filter zone is None, causing the description
    // to default to "permanent".  Set it to Library so the noun is "card".
    // NOTE: do not set display_filter.zone to Library here for multi-zone
    // searches — that would leak "in library" into the description. Instead,
    // we post-process the noun from "permanent" to "card" below.
    let searched_multiple_zones = choose_search_zones(choose).is_some_and(|zones| zones.len() > 1);
    let raw_filter_text = if display_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        let desc =
            normalize_search_descriptor_for_origin(&display_filter.description(), searched_library);
        if searched_multiple_zones && display_filter.zone.is_none() {
            desc.replacen("permanent", "card", 1)
        } else {
            desc
        }
    };
    let filter_text = describe_search_selection_with_cards_preserving_where(
        &describe_search_selection_from_filter_text(choose, &raw_filter_text),
    );
    let pronoun = if choose.count.max == Some(1) {
        "it"
    } else {
        "them"
    };
    let reveal_clause = if reveal.is_some() {
        format!(", reveal {pronoun}")
    } else {
        String::new()
    };
    let face_down_suffix = if exile.face_down { " face down" } else { "" };

    let mut text = format!(
        "Search {search_origin} for {filter_text}{reveal_clause}, exile {pronoun}{face_down_suffix}"
    );
    if let Some(shuffle) = shuffle {
        if !same_search_player_filter(&shuffle.player, search_owner_filter) {
            return None;
        }
        text.push_str(", then ");
        text.push_str(&shuffle_clause);
    }
    Some(text)
}

pub(super) fn describe_search_choose_then_return_to_hand(
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: Option<&crate::effects::RevealTaggedEffect>,
    return_to_hand: &crate::effects::ReturnToHandEffect,
    shuffle: Option<&crate::effects::ShuffleLibraryEffect>,
) -> Option<String> {
    let search_like = choose.is_search
        || (choose.tag.as_str().starts_with("searched_")
            && choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library)));
    if !search_like {
        return None;
    }
    if let Some(reveal) = reveal
        && reveal.tag != choose.tag
    {
        return None;
    }
    if !return_to_hand_uses_chosen_tag(return_to_hand, choose.tag.as_str()) {
        return None;
    }

    let search_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    let search_origin = describe_search_origin_zones(choose)?;
    let searched_library =
        choose_search_zones(choose).is_some_and(|zones| zones.contains(&Zone::Library));
    let shuffle_clause = if describe_player_filter(search_owner_filter) == "you" {
        "shuffle".to_string()
    } else {
        "that player shuffles".to_string()
    };

    let mut display_filter = choose.filter.clone();
    display_filter.owner = None;
    if searched_library && display_filter.zone == Some(Zone::Library) {
        display_filter.zone = None;
    }
    // For multi-zone searches the filter zone is None, causing the description
    // noun to default to "permanent". Replace with "card" in search contexts.
    let searched_multiple_zones = choose_search_zones(choose).is_some_and(|zones| zones.len() > 1);
    let filter_desc = if display_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        let desc =
            normalize_search_descriptor_for_origin(&display_filter.description(), searched_library);
        if searched_multiple_zones && display_filter.zone.is_none() {
            desc.replacen("permanent", "card", 1)
        } else {
            desc
        }
    };
    let selection = describe_search_selection_with_cards_preserving_where(
        &describe_search_selection_from_filter_text(choose, &filter_desc),
    );
    let pronoun = if choose.count.max == Some(1) {
        "it"
    } else {
        "them"
    };
    let reveal_clause = if reveal.is_some() {
        format!(", reveal {pronoun}")
    } else {
        String::new()
    };

    let mut text = format!(
        "Search {search_origin} for {selection}{reveal_clause}, put {pronoun} into {} hand",
        describe_possessive_player_filter(search_owner_filter)
    );
    if let Some(shuffle) = shuffle {
        if !same_search_player_filter(&shuffle.player, search_owner_filter) {
            return None;
        }
        text.push_str(", then ");
        text.push_str(&shuffle_clause);
    }
    Some(text)
}

pub(super) fn describe_search_color_count_selection(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let crate::filter::Comparison::EqualExpr(color_count) = choose.filter.color_count.as_ref()?
    else {
        return None;
    };
    let crate::effect::Value::Add(left, right) = color_count.as_ref() else {
        return None;
    };
    if !matches!(
        (left.as_ref(), right.as_ref()),
        (
            crate::effect::Value::ColorsAmong(_),
            crate::effect::Value::Fixed(1)
        ) | (
            crate::effect::Value::Fixed(1),
            crate::effect::Value::ColorsAmong(_)
        )
    ) {
        return None;
    }

    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.controller = None;
    filter.color_count = None;
    Some(format!(
        "{} that's exactly that many colors plus one",
        describe_search_selection_with_cards(&filter.description())
    ))
}

pub(super) fn search_color_count_references_sacrifice_cost(
    choose: &crate::effects::ChooseObjectsEffect,
) -> bool {
    fn colors_among_sacrifice_cost(value: &Value) -> bool {
        let Value::ColorsAmong(filter) = value else {
            return false;
        };
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str().starts_with("sacrifice_cost_")
        })
    }

    let Some(crate::filter::Comparison::EqualExpr(color_count)) =
        choose.filter.color_count.as_ref()
    else {
        return false;
    };
    let Value::Add(left, right) = color_count.as_ref() else {
        return false;
    };
    colors_among_sacrifice_cost(left.as_ref()) || colors_among_sacrifice_cost(right.as_ref())
}

pub(super) fn describe_search_face_down_exile_shuffle_conditional_cast_else_hand(
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_effect(tagged.effect.as_ref());
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap_effect(with_id.effect.as_ref());
        }
        effect
    }

    fn bargained_mana_value_limit(condition: &Condition, tag: &str) -> Option<i32> {
        fn mana_value_limit(condition: &Condition, tag: &str) -> Option<i32> {
            let Condition::ValueComparison {
                left,
                operator,
                right,
            } = condition
            else {
                return None;
            };
            if !matches!(
                operator,
                crate::effect::ValueComparisonOperator::LessThanOrEqual
            ) {
                return None;
            }
            let Value::ManaValueOf(spec) = left else {
                return None;
            };
            if !matches!(spec.as_ref(), ChooseSpec::Tagged(found) if found.as_str() == tag) {
                return None;
            }
            let Value::Fixed(limit) = right else {
                return None;
            };
            Some(*limit)
        }

        let Condition::And(left, right) = condition else {
            return None;
        };
        match (left.as_ref(), right.as_ref()) {
            (Condition::ThisSpellPaidLabel(label), value_condition)
                if label.display_label().eq_ignore_ascii_case("bargain") =>
            {
                mana_value_limit(value_condition, tag)
            }
            (value_condition, Condition::ThisSpellPaidLabel(label))
                if label.display_label().eq_ignore_ascii_case("bargain") =>
            {
                mana_value_limit(value_condition, tag)
            }
            _ => None,
        }
    }

    fn cast_tagged_from_may(effect: &Effect) -> Option<&crate::effects::CastTaggedEffect> {
        let may = unwrap_effect(effect).downcast_ref::<crate::effects::MayEffect>()?;
        let [cast_effect] = may.effects.as_slice() else {
            return None;
        };
        unwrap_effect(cast_effect).downcast_ref::<crate::effects::CastTaggedEffect>()
    }

    fn effects_move_tag_to_hand(effects: &[Effect], tag: &str) -> bool {
        let [effect] = effects else {
            return false;
        };
        unwrap_effect(effect)
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|move_to_zone| move_to_hand_uses_chosen_tag(move_to_zone, tag))
    }

    if !choose.is_search
        || choose.count.max != Some(1)
        || choose_search_zones(choose) != Some(vec![Zone::Library])
        || !exile.face_down
        || !exile_uses_chosen_tag(&exile.spec, choose.tag.as_str())
        || shuffle.player != choose.chooser
    {
        return None;
    }

    let limit = bargained_mana_value_limit(&conditional.condition, choose.tag.as_str())?;
    let [may_cast, declined_fallback] = conditional.if_true.as_slice() else {
        return None;
    };
    let cast_tagged = cast_tagged_from_may(may_cast)?;
    if cast_tagged.tag != choose.tag
        || cast_tagged.allow_land
        || !cast_tagged.without_paying_mana_cost
    {
        return None;
    }

    let if_declined =
        unwrap_effect(declined_fallback).downcast_ref::<crate::effects::IfEffect>()?;
    if !matches!(
        &if_declined.predicate,
        crate::effect::EffectPredicate::WasDeclined
    ) || !if_declined.else_.is_empty()
        || !effects_move_tag_to_hand(&if_declined.then, choose.tag.as_str())
        || !effects_move_tag_to_hand(&conditional.if_false, choose.tag.as_str())
    {
        return None;
    }

    let search_clause = describe_search_choose_then_exile(choose, None, exile, Some(shuffle))?;
    Some(format!(
        "{search_clause}. If this spell was bargained, you may cast the exiled card without paying its mana cost if that spell's mana value is {limit} or less. Put it into your hand if it wasn't cast this way"
    ))
}

pub(crate) fn describe_search_choose_then_exile_and_cast(
    choose: &crate::effects::ChooseObjectsEffect,
    move_effect: &Effect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
    cast_effect: &Effect,
) -> Option<String> {
    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return tagged.effect.as_ref();
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return with_id.effect.as_ref();
        }
        effect
    }

    fn extract_cast_tagged(effect: &Effect) -> Option<&crate::effects::CastTaggedEffect> {
        let effect = unwrap_effect(effect);
        if let Some(cast_tagged) = effect.downcast_ref::<crate::effects::CastTaggedEffect>() {
            return Some(cast_tagged);
        }
        let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
        if may.effects.len() != 1 {
            return None;
        }
        may.effects[0].downcast_ref::<crate::effects::CastTaggedEffect>()
    }

    if !choose.is_search
        || choose.count.max != Some(1)
        || choose_search_zones(choose) != Some(vec![Zone::Library])
    {
        return None;
    }

    let move_to_zone =
        unwrap_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Exile
        || !matches!(
            &move_to_zone.target,
            ChooseSpec::Tagged(tag) if tag == &choose.tag
        )
        || shuffle.player != choose.chooser
    {
        return None;
    }

    let cast_tagged = extract_cast_tagged(cast_effect)?;
    let move_wrapper_tag = move_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| &tagged.tag);
    if cast_tagged.tag != choose.tag
        && cast_tagged.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
        && !crate::cards::is_sentence_helper_tag(cast_tagged.tag.as_str(), "exiled")
        && move_wrapper_tag != Some(&cast_tagged.tag)
    {
        return None;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let color_count_selection = describe_search_color_count_selection(choose);
    let counts_sacrificed_creature = search_color_count_references_sacrifice_cost(choose);
    let selection = color_count_selection.unwrap_or_else(|| {
        let mut filter = choose.filter.clone();
        filter.zone = None;
        filter.owner = None;
        filter.controller = None;
        filter.color_count = None;
        describe_search_selection_with_cards(&filter.description())
    });
    let cast_clause = if cast_tagged.allow_land {
        "You may play the exiled card".to_string()
    } else if cast_tagged.without_paying_mana_cost {
        "You may cast the exiled card without paying its mana cost".to_string()
    } else {
        "You may cast the exiled card".to_string()
    };

    let search_clause = if counts_sacrificed_creature {
        format!(
            "Count the colors of the sacrificed creature, then search {search_origin} for {selection}"
        )
    } else {
        format!("Search {search_origin} for {selection}")
    };

    Some(format!(
        "{search_clause}. Exile that card, then shuffle. {cast_clause}"
    ))
}

/// Render a library search whose chosen card is cast directly before the
/// searched library is shuffled. Keeping this as one structural bundle lets
/// the shuffle refer back to the searched player instead of repeating the
/// original target expression.
pub(crate) fn describe_search_choose_then_cast_then_shuffle(
    choose: &crate::effects::ChooseObjectsEffect,
    cast_effect: &Effect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_effect(tagged.effect.as_ref());
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap_effect(with_id.effect.as_ref());
        }
        effect
    }

    fn extract_cast_tagged(effect: &Effect) -> Option<&crate::effects::CastTaggedEffect> {
        let effect = unwrap_effect(effect);
        if let Some(cast_tagged) = effect.downcast_ref::<crate::effects::CastTaggedEffect>() {
            return Some(cast_tagged);
        }
        let may = effect.downcast_ref::<crate::effects::MayEffect>()?;
        let [cast_effect] = may.effects.as_slice() else {
            return None;
        };
        unwrap_effect(cast_effect).downcast_ref::<crate::effects::CastTaggedEffect>()
    }

    if !choose.is_search
        || choose.count.max != Some(1)
        || choose_search_zones(choose) != Some(vec![Zone::Library])
    {
        return None;
    }

    let search_owner = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
    if !same_search_player_filter(&shuffle.player, search_owner) {
        return None;
    }

    let cast_tagged = extract_cast_tagged(cast_effect)?;
    if cast_tagged.tag != choose.tag
        || cast_tagged.player != PlayerFilter::You
        || cast_tagged.as_copy
        || cast_tagged.cost_reduction.is_some()
    {
        return None;
    }

    let search_origin = describe_search_origin_zones(choose)?;
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.controller = None;
    let selection = describe_search_selection_with_cards(&filter.description());
    let action = if cast_tagged.allow_land {
        "play"
    } else {
        "cast"
    };
    let payment = if cast_tagged.without_paying_mana_cost {
        " without paying its mana cost"
    } else {
        ""
    };
    let shuffle_clause = if describe_player_filter(search_owner) == "you" {
        "Then shuffle"
    } else {
        "Then that player shuffles"
    };

    Some(format!(
        "Search {search_origin} for {selection}. You may {action} that card{payment}. {shuffle_clause}"
    ))
}

pub(crate) fn describe_choose_then_for_each_same_name_search_to_battlefield(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachTaggedEffect,
    move_effect: &Effect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    fn is_same_name_search(filter: &ObjectFilter) -> bool {
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        })
    }

    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return tagged.effect.as_ref();
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return with_id.effect.as_ref();
        }
        effect
    }

    if choose.is_search || for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }

    let may = for_each.effects[0].downcast_ref::<crate::effects::MayEffect>()?;
    let [search_effect] = may.effects.as_slice() else {
        return None;
    };
    let search_choose = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !search_choose.is_search
        || choose_search_zones(search_choose) != Some(vec![Zone::Library])
        || search_choose.count.max != Some(1)
        || !is_same_name_search(&search_choose.filter)
    {
        return None;
    }

    let move_to_zone =
        unwrap_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &search_choose.tag)
        || move_to_zone.zone != Zone::Battlefield
        || shuffle.player != search_choose.chooser
    {
        return None;
    }

    let chosen_sentence = format!("Choose {}", describe_choose_selection(choose));
    let mut chosen_kind = strip_leading_article(&choose.filter.description())
        .replace(" in the battlefield", "")
        .replace(" you control", "");
    if let Some(rest) = chosen_kind.strip_prefix("other ") {
        chosen_kind = rest.to_string();
    }
    let chosen_kind = chosen_kind.trim().to_string();
    if chosen_kind.is_empty() {
        return None;
    }
    let chosen_plural = pluralize_noun_phrase(&chosen_kind);
    let search_origin = describe_search_origin_zones(search_choose)?;
    let actor = describe_player_filter(may.decider.as_ref().unwrap_or(&search_choose.chooser));
    let actor_clause = if actor == "you" {
        "you may".to_string()
    } else {
        format!("{} may", actor)
    };

    Some(format!(
        "{chosen_sentence}. For each of those {chosen_plural}, {actor_clause} search {search_origin} for a card with the same name as that {chosen_kind}. Put those cards onto the battlefield{}, then shuffle",
        if move_to_zone.enters_tapped {
            " tapped"
        } else {
            ""
        }
    ))
}

pub(super) fn describe_for_each_same_name_search_to_battlefield(
    for_each: &crate::effects::ForEachObject,
    move_effect: &Effect,
    shuffle: &crate::effects::ShuffleLibraryEffect,
) -> Option<String> {
    fn is_same_name_search(filter: &ObjectFilter) -> bool {
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        })
    }

    fn unwrap_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return tagged.effect.as_ref();
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return with_id.effect.as_ref();
        }
        effect
    }

    let [may_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [search_effect] = may.effects.as_slice() else {
        return None;
    };
    let search_choose = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !search_choose.is_search
        || choose_search_zones(search_choose) != Some(vec![Zone::Library])
        || search_choose.count.max != Some(1)
        || !is_same_name_search(&search_choose.filter)
    {
        return None;
    }

    let move_to_zone =
        unwrap_effect(move_effect).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag == &search_choose.tag)
        || move_to_zone.zone != Zone::Battlefield
        || shuffle.player != search_choose.chooser
    {
        return None;
    }

    let mut iterated_subject = strip_leading_article(&for_each.filter.description()).to_string();
    iterated_subject = iterated_subject.replace(" in the battlefield", "");
    let reference_phrase = for_each_subject_reference_phrase(&iterated_subject);

    let mut search_filter = search_choose.filter.clone();
    search_filter.zone = None;
    search_filter.tagged_constraints.retain(|constraint| {
        constraint.relation != crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    });
    let search_subject = if search_filter == ObjectFilter::default() {
        "a card".to_string()
    } else {
        describe_search_selection_with_cards(&search_filter.description())
    };
    let search_origin = describe_search_origin_zones(search_choose)?;
    let actor = describe_player_filter(may.decider.as_ref().unwrap_or(&search_choose.chooser));
    let actor_clause = if actor == "you" {
        "you may".to_string()
    } else {
        format!("{actor} may")
    };
    let tapped_suffix = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };

    Some(format!(
        "For each {iterated_subject}, {actor_clause} search {search_origin} for {search_subject} with the same name as {reference_phrase}. Put those cards onto the battlefield{tapped_suffix}, then shuffle"
    ))
}

pub(crate) fn describe_search_sequence(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.effects.len() < 2 || sequence.effects.len() > 3 {
        return None;
    }
    let choose = sequence.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if let Some(for_each) =
        sequence.effects[1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        let shuffle = if sequence.effects.len() == 3 {
            Some(sequence.effects[2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?)
        } else {
            None
        };
        return describe_search_choose_for_each(choose, for_each, shuffle, false);
    }
    if sequence.effects.len() == 3
        && let Some(shuffle) =
            sequence.effects[1].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(for_each) =
            sequence.effects[2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        return describe_search_choose_for_each(choose, for_each, Some(shuffle), true);
    }
    None
}

pub(crate) fn describe_reveal_until_sequence(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.effects.len() != 3 {
        return None;
    }
    let choose = sequence.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = sequence.effects[1].downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = sequence.effects[2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if choose_primary_zone(choose) != Some(Zone::Library)
        || !choose.top_only
        || !choose.reveal
        || choose.is_search
    {
        return None;
    }
    if for_each.tag != choose.tag {
        return None;
    }
    if shuffle.player != choose.chooser {
        return None;
    }
    if for_each.effects.len() != 1 {
        return None;
    }
    let put = for_each.effects[0].downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    if !matches!(put.target, ChooseSpec::Iterated) || put.tapped {
        return None;
    }
    if put.controller != choose.chooser {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let library_owner = describe_possessive_player_filter(&choose.chooser);

    let shares_card_type_with_it = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::target::TaggedOpbjectRelation::SharesCardType
            && constraint.tag.as_str() == "__it__"
    });
    let selection = if shares_card_type_with_it {
        "a card that shares a card type with it".to_string()
    } else {
        strip_leading_article(&choose.filter.description()).to_string()
    };

    Some(format!(
        "{chooser} reveals cards from the top of {library_owner} library until they reveal {selection}, puts that card onto the battlefield, then shuffles"
    ))
}

pub(super) fn describe_choose_type_then_phase_out(
    choose: &crate::effects::ChooseObjectsEffect,
    phase_out: &crate::effects::PhaseOutEffect,
) -> Option<String> {
    if choose.is_search
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
    {
        return None;
    }

    let ChooseSpec::All(phase_filter) = &phase_out.spec else {
        return None;
    };
    let shares_chosen_type = phase_filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == choose.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SharesCardType
    });
    if !shares_chosen_type || !phase_filter.nontoken {
        return None;
    }

    let has_teferi_realm_options = choose.filter.card_types.contains(&CardType::Artifact)
        && choose.filter.card_types.contains(&CardType::Creature)
        && choose.filter.card_types.contains(&CardType::Land)
        && choose.filter.card_types.contains(&CardType::Enchantment)
        && choose.filter.excluded_subtypes.contains(&Subtype::Aura);
    if !has_teferi_realm_options {
        return None;
    }

    Some(format!(
        "{} chooses artifact, creature, land, or non-Aura enchantment. All nontoken permanents of that type phase out",
        capitalize_first(&describe_player_filter(&choose.chooser))
    ))
}

pub(super) fn describe_damaged_player_gain_control_then_rewards(
    control_effect: &Effect,
    reward_effect: &Effect,
) -> Option<String> {
    let with_id = control_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let apply = with_id
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(apply.target, crate::continuous::EffectTarget::Source)
        || !matches!(apply.until, Until::Forever)
        || !apply.additional_modifications.is_empty()
        || apply.modification.is_some()
        || !matches!(
            apply.runtime_modifications.as_slice(),
            [
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    PlayerFilter::DamagedPlayer
                )
            ]
        )
    {
        return None;
    }

    let if_effect = reward_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != with_id.id
        || if_effect.predicate != EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }

    let [draw_effect, create_effect, lose_life_effect] = if_effect.then.as_slice() else {
        return None;
    };
    let draw = draw_cards_view(draw_effect)?;
    if draw.player != PlayerFilter::You || !is_effect_count_reference(&draw.count, None) {
        return None;
    }
    let create = created_token_effect(create_effect)?;
    if create.controller != PlayerFilter::You
        || !create.enters_tapped
        || create.token.card.name != "Treasure"
        || !is_effect_count_reference(&create.count, None)
    {
        return None;
    }
    let lose_life = lose_life_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose_life.player != ChooseSpec::Player(PlayerFilter::You)
        || !is_effect_count_reference(&lose_life.amount, None)
    {
        return None;
    }

    Some(
        "that player gains control of this creature. If they do, you draw that many cards, create that many tapped Treasure tokens, then lose that much life"
            .to_string(),
    )
}

pub(super) fn describe_simple_exiled_card_target(spec: &ChooseSpec) -> Option<String> {
    let ChooseSpec::Target(inner) = spec else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    if filter.zone != Some(Zone::Exile)
        || filter.owner.is_some()
        || filter.controller.is_some()
        || !filter.card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.colors.is_some()
        || filter.required_colors.is_some()
        || filter.sticker.is_some()
        || filter.source
        || !filter.tagged_constraints.is_empty()
    {
        return None;
    }

    let base = match filter.face_down {
        Some(false) => "face-up exiled card",
        Some(true) => "face-down exiled card",
        None => "exiled card",
    };
    Some(format!("target {base}"))
}

pub(super) fn describe_source_card_from_exile_target(spec: &ChooseSpec) -> Option<&'static str> {
    let ChooseSpec::Object(filter) = spec.base() else {
        return None;
    };
    if filter.source && filter.zone == Some(Zone::Exile) {
        Some("this card from exile")
    } else {
        None
    }
}

pub(super) fn is_reflexive_choose_one_followup(
    if_effect: &crate::effects::IfEffect,
    then_text: &str,
) -> bool {
    if !matches!(if_effect.predicate, EffectPredicate::Happened) || !if_effect.else_.is_empty() {
        return false;
    }
    let [effect] = if_effect.then.as_slice() else {
        return false;
    };
    let Some(choose) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() else {
        return false;
    };
    matches!(&choose.choose_count, Value::Fixed(1))
        && matches!(&choose.min_choose_count, Value::Fixed(1))
        && then_text
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("choose one")
}

pub(super) fn effect_moves_object_to_exile(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .is_some_and(|move_to| move_to.zone == Zone::Exile)
    {
        return true;
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_moves_object_to_exile(&tagged.effect);
    }
    false
}

pub(crate) fn describe_effect(effect: &Effect) -> String {
    with_effect_render_depth(|| describe_effect_impl(effect))
}

pub(super) fn describe_may_have_source_deal_damage_to_decider(
    may: &crate::effects::MayEffect,
) -> Option<String> {
    let decider = may.decider.as_ref()?;
    let who = match decider {
        PlayerFilter::Opponent => "any opponent",
        PlayerFilter::Any => "any player",
        _ => return None,
    };
    let [effect] = may.effects.as_slice() else {
        return None;
    };
    let damage = effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.unpreventable || damage.source_is_combat {
        return None;
    }
    let ChooseSpec::Player(target_player) = &damage.target else {
        return None;
    };
    if target_player != decider {
        return None;
    }
    Some(format!(
        "{who} may have it deal {} damage to them",
        describe_value(&damage.amount)
    ))
}

pub(super) fn describe_may_have_source_deal_damage_condition(
    may: &crate::effects::MayEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    describe_may_have_source_deal_damage_to_decider(may)?;
    Some(match if_effect.predicate {
        EffectPredicate::DidNotHappen => "If no one does".to_string(),
        _ => "If a player does".to_string(),
    })
}

pub(super) fn describe_cards_in_hand_difference_conditional(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let Condition::PlayerCardsInHandOrFewer { player, count } = &conditional.condition else {
        return None;
    };
    if *player != PlayerFilter::You {
        return None;
    }
    let draw = conditional.if_true[0].downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You
        || !draw.count.has_surface_hint(ValueSurfaceHint::Difference)
    {
        return None;
    }
    let threshold = count + 1;
    if threshold <= 0 {
        return None;
    }
    let count_text = number_word(threshold).unwrap_or_else(|| threshold.to_string());
    Some(format!(
        "If you have fewer than {count_text} cards in hand, draw cards equal to the difference"
    ))
}

pub(super) fn describe_unless_any_player_pays_search_prefix(
    unless_pays: &crate::effects::UnlessPaysEffect,
    payment_text: &str,
) -> Option<String> {
    if unless_pays.player != PlayerFilter::Any {
        return None;
    }
    let search_text = if let [effect] = unless_pays.effects.as_slice()
        && let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
    {
        describe_search_sequence(sequence)?
    } else {
        let [effect] = unless_pays.effects.as_slice() else {
            return None;
        };
        let mut text = describe_effect_list(&unless_pays.effects);
        if !text.starts_with("Search ") {
            return None;
        }
        if let Some(search_library) =
            unwrap_basic_tag_wrappers(effect).downcast_ref::<crate::effects::SearchLibraryEffect>()
            && search_library.player == PlayerFilter::You
            && search_library.destination == Zone::Hand
        {
            text = text.replace("put it into hand", "put it into your hand");
        }
        text
    };
    Some(format!(
        "Unless any player pays {payment_text}, {}",
        lowercase_first(&search_text)
    ))
}

pub(super) fn describe_endure_mode(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
    if choose_mode.modes.len() != 2
        || choose_mode.choose_count != Value::Fixed(1)
        || choose_mode.min_choose_count != Value::Fixed(1)
        || choose_mode.allow_repeated_modes
    {
        return None;
    }

    let mut counter_amount = None;
    let mut token_size = None;
    for mode in &choose_mode.modes {
        let [effect] = mode.effects.as_slice() else {
            return None;
        };
        if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
            if put.counter_type != CounterType::PlusOnePlusOne
                || !matches!(put.target.base(), ChooseSpec::Source)
                || put.target_count.is_some()
                || put.distributed
            {
                return None;
            }
            counter_amount = Some(put.amount.clone());
            continue;
        }
        if let Some(create) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
            token_size = Some(endure_spirit_token_size(create)?);
            continue;
        }
        return None;
    }

    let amount = counter_amount?;
    if token_size.as_ref() != Some(&amount) {
        return None;
    }
    Some(format!("it endures {}", describe_value(&amount)))
}

pub(super) fn describe_inline_pt_modifier_choice(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
    if choose_mode.modes.len() != 2
        || choose_mode.choose_count != Value::Fixed(1)
        || choose_mode.min_choose_count != Value::Fixed(1)
        || choose_mode.allow_repeat
        || choose_mode.allow_repeated_modes
        || choose_mode.random
        || choose_mode.chooser.is_some()
        || choose_mode.modes.iter().any(|mode| !mode.source_text.trim().is_empty())
    {
        return None;
    }

    fn extract(
        mode: &crate::effect::EffectMode,
    ) -> Option<(
        &crate::effects::ApplyContinuousEffect,
        &Value,
        &Value,
    )> {
        let [effect] = mode.effects.as_slice() else {
            return None;
        };
        let apply = unwrap_basic_tag_wrappers(effect)
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        let [crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power,
            toughness,
        }] = apply.runtime_modifications.as_slice()
        else {
            return None;
        };
        Some((apply, power, toughness))
    }

    let (first, first_power, first_toughness) = extract(&choose_mode.modes[0])?;
    let (second, second_power, second_toughness) = extract(&choose_mode.modes[1])?;
    let mut first_shape = first.clone();
    let mut second_shape = second.clone();
    first_shape.runtime_modifications.clear();
    second_shape.runtime_modifications.clear();
    if first_shape != second_shape {
        return None;
    }

    let (target, plural) = describe_apply_continuous_target(first);
    let verb = if plural { "get" } else { "gets" };
    let first_pt = format!(
        "{}/{}",
        describe_signed_value(first_power),
        describe_signed_value(first_toughness)
    );
    let second_pt = format!(
        "{}/{}",
        describe_signed_value(second_power),
        describe_signed_value(second_toughness)
    );
    let tail = describe_apply_continuous_tail(first)
        .map(|tail| format!(" {tail}"))
        .unwrap_or_default();
    Some(format!(
        "{target} {verb} {first_pt} or {second_pt}{tail}"
    ))
}

pub(crate) fn describe_tap_or_untap_mode(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
    if choose_mode.modes.len() != 2 {
        return None;
    }
    let is_choose_one = matches!(choose_mode.choose_count, Value::Fixed(1))
        && matches!(choose_mode.min_choose_count, Value::Fixed(1));
    if !is_choose_one {
        return None;
    }
    let mut shared_target: Option<String> = None;
    let mut tap_target: Option<String> = None;
    let mut untap_target: Option<String> = None;
    let mut tap_is_all = false;
    let mut untap_is_all = false;
    let mut saw_tap = false;
    let mut saw_untap = false;
    let modes_are_bare_tap_verbs = choose_mode.modes.iter().all(|mode| {
        matches!(
            mode.source_text.trim().to_ascii_lowercase().as_str(),
            "tap" | "untap"
        )
    });
    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        let effect = unwrap_basic_tag_wrappers(&mode.effects[0]);
        if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
            saw_tap = true;
            let candidate = describe_choose_spec(&tap.target);
            tap_target = Some(candidate.clone());
            tap_is_all = matches!(tap.target.base(), ChooseSpec::All(_));
            if let Some(existing) = &shared_target {
                if existing != &candidate {
                    shared_target = None;
                }
            } else {
                shared_target = Some(candidate);
            }
            continue;
        }
        if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
            saw_untap = true;
            let candidate = describe_choose_spec(&untap.target);
            untap_target = Some(candidate.clone());
            untap_is_all = matches!(untap.target.base(), ChooseSpec::All(_));
            if let Some(existing) = &shared_target {
                if existing != &candidate {
                    shared_target = None;
                }
            } else {
                shared_target = Some(candidate);
            }
            continue;
        }
        return None;
    }
    if saw_tap && saw_untap {
        if modes_are_bare_tap_verbs && let Some(target) = shared_target {
            return Some(format!("Tap or untap {target}"));
        }
        if tap_is_all && untap_is_all {
            let tap_target = tap_target.unwrap_or_else(|| "all those permanents".to_string());
            let untap_target = untap_target.unwrap_or_else(|| "all those permanents".to_string());
            return Some(format!("Tap {tap_target}, or untap {untap_target}."));
        }
    }
    None
}

pub(crate) fn describe_put_counter_choice_mode(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
    if choose_mode.modes.len() < 2
        || choose_mode.choose_count != Value::Fixed(1)
        || choose_mode.min_choose_count != Value::Fixed(1)
        || choose_mode.allow_repeated_modes
    {
        return None;
    }

    let mut counter_names = Vec::new();
    let mut shared_structural_target: Option<String> = None;
    let mut shared_display_target: Option<String> = None;
    for mode in &choose_mode.modes {
        let [effect] = mode.effects.as_slice() else {
            return None;
        };
        let put = unwrap_basic_tag_wrappers(effect)
            .downcast_ref::<crate::effects::PutCountersEffect>()?;
        if !matches!(put.amount, Value::Fixed(1))
            || put
                .target_count
                .is_some_and(|count| count != ChoiceCount::exactly(1))
            || put.distributed
        {
            return None;
        }
        let structural_target = describe_choose_spec(&put.target);
        if let Some(existing) = &shared_structural_target {
            if existing != &structural_target {
                return None;
            }
        } else {
            shared_structural_target = Some(structural_target.clone());
        }
        let display_target =
            put_counter_choice_mode_source_target(&mode.source_text).unwrap_or(structural_target);
        if let Some(existing) = &shared_display_target {
            if existing != &display_target {
                return None;
            }
        } else {
            shared_display_target = Some(display_target);
        }
        counter_names.push(put.counter_type.description().into_owned());
    }

    let target = shared_display_target?;
    let first = counter_names.first()?.clone();
    let first = with_indefinite_article(&first);
    let mut options = Vec::with_capacity(counter_names.len());
    options.push(first);
    options.extend(counter_names.into_iter().skip(1));
    Some(format!(
        "Put your choice of {} counter on {target}",
        join_with_or(&options)
    ))
}

pub(super) fn put_counter_choice_mode_source_target(source_text: &str) -> Option<String> {
    let source = source_text.trim().trim_end_matches('.');
    let lower = source.to_ascii_lowercase();
    let marker = " counter on ";
    let idx = lower.rfind(marker)?;
    let target = source.get(idx + marker.len()..)?.trim();
    (!target.is_empty()).then(|| lowercase_first(target))
}

pub(crate) fn describe_put_or_remove_counter_mode(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
    if choose_mode.modes.len() != 2 {
        return None;
    }
    let is_choose_one = matches!(choose_mode.choose_count, Value::Fixed(1))
        && matches!(choose_mode.min_choose_count, Value::Fixed(1));
    if !is_choose_one {
        return None;
    }

    let mut put_mode: Option<(&crate::effects::PutCountersEffect, String)> = None;
    let mut remove_mode: Option<(&crate::effects::RemoveCountersEffect, String)> = None;
    let mut remove_followup_mode: Option<crate::effects::ChooseModeEffect> = None;

    for mode in &choose_mode.modes {
        let description = describe_effect_list(&mode.effects);
        if mode.effects.len() == 1 {
            let effect = unwrap_basic_tag_wrappers(&mode.effects[0]);
            if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
                put_mode = Some((put, description));
                continue;
            }
            if let Some(remove) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() {
                remove_mode = Some((remove, description));
                continue;
            }
            return None;
        }

        if mode.effects.len() == 2
            && let Some(with_id) = mode.effects[0].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(remove) = with_id
                .effect
                .downcast_ref::<crate::effects::RemoveCountersEffect>()
            && let Some(if_effect) = mode.effects[1].downcast_ref::<crate::effects::IfEffect>()
            && if_effect.condition == with_id.id
            && matches!(if_effect.predicate, EffectPredicate::Happened)
            && if_effect.else_.is_empty()
            && if_effect.then.len() == 1
            && let Some(followup_choose) =
                if_effect.then[0].downcast_ref::<crate::effects::ChooseModeEffect>()
        {
            remove_mode = Some((remove, description));
            remove_followup_mode = Some(followup_choose.clone());
            continue;
        }

        return None;
    }

    let (put_effect, put_description) = put_mode?;
    let (remove_effect, remove_description) = remove_mode?;
    if put_effect.target != remove_effect.target {
        return None;
    }

    let put_clause = put_description.trim().trim_end_matches('.');
    let remove_clause = lowercase_first(remove_description.trim().trim_end_matches('.'));
    if !put_clause.to_ascii_lowercase().starts_with("put ") || !remove_clause.starts_with("remove ")
    {
        return None;
    }

    if let Some(followup_choose) = remove_followup_mode {
        let followup_text = describe_effect(&Effect::new(followup_choose));
        let followup_clause = lowercase_first(followup_text.trim());
        let removed_counter =
            describe_put_counter_phrase(&remove_effect.count, remove_effect.counter_type);
        return Some(format!(
            "{put_clause} or {remove_clause}. When you remove {removed_counter} this way, {followup_clause}"
        ));
    }

    Some(format!("{put_clause} or {remove_clause}"))
}

pub(crate) fn describe_conditional_damage_instead(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if conditional.if_true.len() != 1 || conditional.if_false.len() != 1 {
        return None;
    }
    let true_damage = conditional.if_true[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    let false_damage =
        conditional.if_false[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if true_damage.source_is_combat || false_damage.source_is_combat {
        return None;
    }
    if true_damage.target != false_damage.target {
        return None;
    }

    let base_amount = describe_value(&false_damage.amount);
    let instead_amount = describe_value(&true_damage.amount);
    let target = describe_choose_spec(&true_damage.target);
    let condition = describe_condition(&conditional.condition);
    Some(format!(
        "Deal {base_amount} damage to {target}. It deals {instead_amount} damage instead if {condition}"
    ))
}

pub(crate) fn describe_conditional_choose_both_instead(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if conditional.if_true.len() != 1 || conditional.if_false.len() != 1 {
        return None;
    }
    let choose_true = conditional.if_true[0].downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let choose_false =
        conditional.if_false[0].downcast_ref::<crate::effects::ChooseModeEffect>()?;

    if choose_true.modes.len() != choose_false.modes.len()
        || choose_true
            .modes
            .iter()
            .zip(choose_false.modes.iter())
            .any(|(left, right)| {
                describe_effect_list(&left.effects).trim()
                    != describe_effect_list(&right.effects).trim()
            })
    {
        return None;
    }

    // Pattern: "Choose one. If <condition>, you may choose both instead."
    if choose_true.choose_count != Value::Fixed(2)
        || choose_true.min_choose_count != Value::Fixed(1)
        || choose_false.choose_count != Value::Fixed(1)
        || choose_false.min_choose_count != choose_false.choose_count
    {
        return None;
    }

    let condition = describe_condition(&conditional.condition);
    let timing = if choose_both_condition_is_cast_time(&conditional.condition) {
        " as you cast this spell"
    } else {
        ""
    };
    let mut out = format!("Choose one. If {condition}{timing}, you may choose both instead.");
    for mode in &choose_true.modes {
        let rendered = describe_effect_list(&mode.effects);
        let description = capitalize_first(&ensure_trailing_period(rendered.trim()));
        if description.trim().is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str("• ");
        out.push_str(description.trim());
    }
    Some(out)
}

pub(super) fn choose_both_condition_is_cast_time(condition: &crate::effect::Condition) -> bool {
    match condition {
        crate::effect::Condition::YouControlCommander => true,
        crate::effect::Condition::PlayerControls { .. } => true,
        crate::effect::Condition::And(left, right) => {
            choose_both_condition_is_cast_time(left) && choose_both_condition_is_cast_time(right)
        }
        _ => false,
    }
}

pub(crate) fn describe_conditional_replacement_instead(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if !conditional.if_false.is_empty() || conditional.if_true.is_empty() {
        return None;
    }

    let condition = describe_condition(&conditional.condition);

    let true_branch = describe_effect_clause_list(&conditional.if_true)
        .unwrap_or_else(|| describe_effect_list(&conditional.if_true));
    let true_branch = true_branch.trim().trim_end_matches('.');
    let condition_lower = condition.to_ascii_lowercase();
    if (condition_lower == "you would proliferate"
        || condition_lower == "an opponent would proliferate"
        || condition_lower == "a player would proliferate")
        && true_branch.to_ascii_lowercase().starts_with("proliferate")
        && !true_branch.to_ascii_lowercase().contains(" instead")
    {
        let branch = if let Some(rest) = true_branch.strip_prefix("Proliferate") {
            format!("proliferate{rest}")
        } else {
            true_branch.to_string()
        };
        return Some(format!("If {condition}, {branch} instead"));
    }

    if true_branch.is_empty()
        || !true_branch.to_ascii_lowercase().starts_with("exile ")
        || true_branch.to_ascii_lowercase().contains(" instead")
    {
        return None;
    }

    if condition.contains(" would leave the battlefield")
        || condition.contains(" would be put into ")
        || condition.contains(" would go ")
    {
        return Some(format!("If {condition}, {true_branch} instead"));
    }

    if condition.eq_ignore_ascii_case("it matches permanent")
        && true_branch.eq_ignore_ascii_case("exile it")
    {
        return Some(
            "If it would leave the battlefield, exile it instead of putting it anywhere else"
                .to_string(),
        );
    }

    None
}

pub(super) fn describe_target_color_set_conditional_destroy(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    let crate::effect::Condition::Not(inner) = &conditional.condition else {
        return None;
    };
    if !matches!(
        inner.as_ref(),
        crate::effect::Condition::TargetObjectsHaveDifferentColorSets
    ) || !conditional.if_false.is_empty()
    {
        return None;
    }
    let [effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let effect = unwrap_basic_tag_wrappers(effect);

    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        let target = describe_choose_spec(&destroy.spec);
        let pronoun = if choose_spec_allows_multiple(&destroy.spec) {
            "They"
        } else {
            "It"
        };
        return Some(format!(
            "Destroy {target} unless either one is a color the other isn't. {pronoun} can't be regenerated"
        ));
    }
    let destroy = effect.downcast_ref::<crate::effects::DestroyEffect>()?;
    Some(format!(
        "Destroy {} unless either one is a color the other isn't",
        describe_choose_spec(&destroy.spec)
    ))
}

pub(super) fn unwrap_for_each_attachment_wrappers(effect: &Effect) -> &Effect {
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_for_each_attachment_wrappers(&tag_all.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_for_each_attachment_wrappers(&tagged.effect);
    }
    effect
}

pub(super) fn tagged_create_token_effect(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::CreateTokenEffect)> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_create_token_effect(&with_id.effect);
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let create_token = unwrap_for_each_attachment_wrappers(&tagged.effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    Some((&tagged.tag, create_token))
}

pub(super) fn created_token_effect(effect: &Effect) -> Option<&crate::effects::CreateTokenEffect> {
    if let Some((_, create_token)) = tagged_create_token_effect(effect) {
        return Some(create_token);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return created_token_effect(&with_id.effect);
    }
    unwrap_for_each_attachment_wrappers(effect).downcast_ref::<crate::effects::CreateTokenEffect>()
}

pub(super) fn choose_spec_references_exact_tag(spec: &ChooseSpec, tag: &TagKey) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter == &ObjectFilter::tagged(tag.clone())
        }
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_exact_tag(inner, tag)
        }
        _ => false,
    }
}

pub(super) fn choose_spec_references_created_object(
    spec: &ChooseSpec,
    tag: Option<&TagKey>,
) -> bool {
    if let Some(tag) = tag {
        return choose_spec_references_exact_tag(spec, tag);
    }
    match spec {
        ChooseSpec::All(filter) | ChooseSpec::Object(filter) => {
            filter.card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
        }
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_created_object(inner, tag)
        }
        _ => false,
    }
}

/// Battlefield is the implicit zone for an iterated permanent noun in oracle
/// text ("For each land, ..."); count surfaces keep the explicit suffix.
pub(super) fn strip_battlefield_zone_suffix(filter_text: String) -> String {
    filter_text
        .strip_suffix(" on the battlefield")
        .map(str::to_string)
        .unwrap_or(filter_text)
}

pub(super) fn describe_for_each_object_filter_subject(filter: &ObjectFilter) -> String {
    let description = filter.description();
    let subject = strip_indefinite_article(&description).trim();
    // Battlefield is the implicit zone for an iterated permanent noun in
    // oracle text ("For each land, ..."), so the explicit zone suffix only
    // survives for other zones.
    let subject = subject.strip_suffix(" on the battlefield").unwrap_or(subject);
    if let Some(rest) = subject.strip_prefix("opponent's ") {
        return format!("{rest} your opponents control");
    }
    subject.to_string()
}

pub(super) fn describe_iterated_object_reference_noun(filter: &ObjectFilter) -> &'static str {
    if filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else if filter.card_types.contains(&CardType::Land) {
        "land"
    } else if matches!(
        filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
    ) {
        "card"
    } else if matches!(filter.zone, Some(Zone::Battlefield) | None) {
        "permanent"
    } else {
        "object"
    }
}

pub(super) fn describe_for_each_created_token_attachment(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    let [create_effect, attach_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let created_tag = tagged_create_token_effect(create_effect).map(|(tag, _)| tag);
    let create_token = created_token_effect(create_effect)?;
    let attach = unwrap_for_each_attachment_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(attach.target, ChooseSpec::Iterated)
        || !choose_spec_references_created_object(&attach.objects, created_tag)
        || create_token.count != Value::Fixed(1)
        || !matches!(&create_token.controller, PlayerFilter::You)
        || create_token.controller_target.is_some()
        || create_token.enters_tapped
        || create_token.enters_attacking
        || create_token.exile_at_end_of_combat
        || create_token.sacrifice_at_end_of_combat
        || create_token.sacrifice_at_next_end_step
        || create_token.exile_at_next_end_step
    {
        return None;
    }

    let subject = describe_for_each_object_filter_subject(&for_each.filter);
    let token = with_indefinite_article(&describe_token_blueprint(&create_token.token));
    let target_noun = describe_iterated_object_reference_noun(&for_each.filter);
    Some(format!(
        "For each {subject}, create {token} attached to that {target_noun}"
    ))
}

pub(super) fn describe_for_each_tagged_created_token_attachment(
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    let [create_effect, attach_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let (created_tag, create_token) = tagged_create_token_effect(create_effect)?;
    let attach = unwrap_for_each_attachment_wrappers(attach_effect)
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    if !matches!(attach.target, ChooseSpec::Iterated)
        || !choose_spec_references_exact_tag(&attach.objects, created_tag)
        || create_token.count != Value::Fixed(1)
        || !matches!(
            &create_token.controller,
            PlayerFilter::You | PlayerFilter::IteratedPlayer
        )
        || create_token.controller_target.is_some()
        || create_token.enters_tapped
        || create_token.enters_attacking
        || create_token.exile_at_end_of_combat
        || create_token.sacrifice_at_end_of_combat
        || create_token.sacrifice_at_next_end_step
        || create_token.exile_at_next_end_step
        || !create_token.token.card.subtypes.contains(&Subtype::Role)
    {
        return None;
    }

    let token = with_indefinite_article(&describe_token_blueprint(&create_token.token));
    Some(format!(
        "For each of those creatures, create {token} attached to it"
    ))
}

pub(super) fn tagged_attach_with_id(
    effect: &Effect,
) -> Option<(
    &crate::effects::WithIdEffect,
    Option<&TagKey>,
    &crate::effects::AttachObjectsEffect,
)> {
    let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    if let Some(tagged) = with_id
        .effect
        .downcast_ref::<crate::effects::TaggedEffect>()
    {
        let attach = tagged
            .effect
            .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
        return Some((with_id, Some(&tagged.tag), attach));
    }
    let attach = with_id
        .effect
        .downcast_ref::<crate::effects::AttachObjectsEffect>()?;
    Some((with_id, None, attach))
}

pub(super) fn attachment_target_reference(target: &ChooseSpec) -> &'static str {
    let description = describe_choose_spec(target).to_ascii_lowercase();
    if description.contains("creature") {
        "that creature"
    } else if description.contains("artifact") {
        "that artifact"
    } else if description.contains("enchantment") {
        "that enchantment"
    } else if description.contains("land") {
        "that land"
    } else if description.contains("permanent") {
        "that permanent"
    } else {
        "it"
    }
}

pub(super) fn describe_attached_target_fight_effects(
    effects: &[Effect],
    attachment_target_tag: &TagKey,
    target_reference: &str,
) -> Option<String> {
    let visible_effects = effects
        .iter()
        .filter(|effect| {
            effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_none()
        })
        .collect::<Vec<_>>();
    let [fight_effect] = visible_effects.as_slice() else {
        return None;
    };
    let fight = fight_effect.downcast_ref::<crate::effects::FightEffect>()?;
    if !matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == attachment_target_tag) {
        return None;
    }

    Some(format!(
        "{target_reference} fights {}",
        describe_choose_spec(&fight.creature2)
    ))
}

pub(super) fn describe_create_attached_token_then_reflexive_fight(
    create_effect: &Effect,
    attach_effect: &Effect,
    followup_effect: &Effect,
) -> Option<String> {
    let (created_tag, create_token) = tagged_create_token_effect(create_effect)?;
    let (with_id, attachment_target_tag, attach) = tagged_attach_with_id(attach_effect)?;
    let attachment_target_tag = attachment_target_tag?;
    let (condition, predicate, followup_effects) =
        if let Some(if_effect) = followup_effect.downcast_ref::<crate::effects::IfEffect>() {
            if !if_effect.else_.is_empty() {
                return None;
            }
            (
                if_effect.condition,
                &if_effect.predicate,
                if_effect.then.as_slice(),
            )
        } else if let Some(reflexive) =
            followup_effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
        {
            (
                reflexive.condition,
                &reflexive.predicate,
                reflexive.effects.as_slice(),
            )
        } else {
            return None;
        };

    if condition != with_id.id
        || predicate != &EffectPredicate::Happened
        || !choose_spec_references_exact_tag(&attach.objects, created_tag)
        || create_token.count != Value::Fixed(1)
        || !matches!(&create_token.controller, PlayerFilter::You)
        || create_token.controller_target.is_some()
        || create_token.enters_tapped
        || create_token.enters_attacking
        || create_token.exile_at_end_of_combat
        || create_token.sacrifice_at_end_of_combat
        || create_token.sacrifice_at_next_end_step
        || create_token.exile_at_next_end_step
    {
        return None;
    }

    let target_reference = attachment_target_reference(&attach.target);
    let followup = describe_attached_target_fight_effects(
        followup_effects,
        attachment_target_tag,
        target_reference,
    )?;
    let token = with_indefinite_article(&describe_token_blueprint(&create_token.token));
    Some(format!(
        "Create {token} attached to {}. When you do, {followup}",
        describe_choose_spec(&attach.target)
    ))
}

#[cfg(test)]
mod alternative_result_condition_tests {
    use super::*;
    use ironsmith_core::EffectId;

    fn render_linked_branches(setup: Effect, predicate: EffectPredicate) -> String {
        let setup = Effect::with_id(7, setup);
        let branch = Effect::if_then_else(
            crate::effect::EffectId(7),
            predicate,
            vec![Effect::draw(1)],
            vec![Effect::draw(2)],
        );
        let setup = setup
            .downcast_ref::<crate::effects::WithIdEffect>()
            .expect("typed setup wrapper");
        let branch = branch
            .downcast_ref::<crate::effects::IfEffect>()
            .expect("typed result branch");
        describe_with_id_if_clause(setup, branch).expect("linked branch should render")
    }

    fn render_sequential_linked_branches(setup: Effect) -> String {
        describe_effect_list(&[
            Effect::with_id(7, setup),
            Effect::if_then_else(
                crate::effect::EffectId(7),
                EffectPredicate::Happened,
                vec![Effect::draw(1)],
                vec![],
            ),
            Effect::if_then_else(
                crate::effect::EffectId(7),
                EffectPredicate::DidNotHappen,
                vec![Effect::draw(2)],
                vec![],
            ),
        ])
    }

    #[test]
    fn exact_may_and_coin_results_render_explicit_alternative_conditions() {
        assert_eq!(
            render_linked_branches(
                Effect::may_single(Effect::gain_life(1)),
                EffectPredicate::Chosen,
            ),
            "If you do, you draw a card. If you don't, you draw two cards"
        );
        assert_eq!(
            render_linked_branches(
                Effect::flip_coin(PlayerFilter::You),
                EffectPredicate::Happened,
            ),
            "If you win the flip, you draw a card. If you lose the flip, you draw two cards"
        );
    }

    #[test]
    fn unrelated_or_mismatched_result_pairs_keep_generic_otherwise() {
        let roll = render_linked_branches(
            Effect::roll_die(6, PlayerFilter::You),
            EffectPredicate::Value(Comparison::Equal(1)),
        );
        assert!(roll.contains(". Otherwise, "), "{roll}");

        let mismatched_coin = render_linked_branches(
            Effect::flip_coin(PlayerFilter::You),
            EffectPredicate::Chosen,
        );
        assert!(
            mismatched_coin.contains(". Otherwise, "),
            "{mismatched_coin}"
        );

        let mismatched_may = render_linked_branches(
            Effect::may_single(Effect::gain_life(1)),
            EffectPredicate::Value(Comparison::Equal(1)),
        );
        assert!(mismatched_may.contains(". Otherwise, "), "{mismatched_may}");

        let opponent_flip = render_linked_branches(
            Effect::flip_coin(PlayerFilter::Opponent),
            EffectPredicate::Happened,
        );
        assert!(opponent_flip.contains(". Otherwise, "), "{opponent_flip}");
    }

    #[test]
    fn sequential_explicit_alternatives_do_not_collapse_to_otherwise() {
        let coin = render_sequential_linked_branches(Effect::flip_coin(PlayerFilter::You));
        assert!(
            coin.contains("If you win the flip, you draw a card"),
            "{coin}"
        );
        assert!(
            coin.contains("If you lose the flip, you draw two cards"),
            "{coin}"
        );
        assert!(!coin.contains("Otherwise"), "{coin}");

        let may = render_sequential_linked_branches(Effect::may_single(Effect::gain_life(1)));
        assert!(may.contains("If you do, you draw a card"), "{may}");
        assert!(may.contains("If you don't, you draw two cards"), "{may}");
        assert!(!may.contains("Otherwise"), "{may}");
    }

    #[test]
    fn sequential_generic_result_pair_keeps_otherwise() {
        let roll = render_sequential_linked_branches(Effect::roll_die(6, PlayerFilter::You));
        assert!(roll.contains("Otherwise, you draw two cards"), "{roll}");
    }
}
