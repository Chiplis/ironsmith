use super::*;

pub(super) fn normalize_redundant_short_name_etb_surface(
    line: String,
    triggered: &crate::ability::TriggeredAbility,
    subject: &str,
) -> String {
    let Some(zone_trigger) = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
    else {
        return line;
    };
    if !zone_trigger.this_object
        || zone_trigger.to
            != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
    {
        return line;
    }
    let Some(crate::target::SourceReferenceSurface::ShortName(surface)) =
        &zone_trigger.this_object_surface
    else {
        return line;
    };
    if triggered_has_you_difference_draw(triggered) {
        return line;
    }

    let Some((start, prefix_len)) = [
        format!("When {surface} enters,"),
        format!("When {surface} enters the battlefield,"),
    ]
    .into_iter()
    .find_map(|prefix| {
        line.find(prefix.as_str())
            .filter(|start| *start == 0 || line[..*start].ends_with(": "))
            .map(|start| (start, prefix.len()))
    }) else {
        for generic_subject in [subject, "this creature", "this permanent", "this artifact"] {
            let generic_prefix = format!("When {generic_subject} enters,");
            if let Some(start) = line
                .find(generic_prefix.as_str())
                .filter(|start| *start == 0 || line[..*start].ends_with(": "))
            {
                let rest = &line[start + generic_prefix.len()..];
                return format!("{}When {surface} enters,{rest}", &line[..start]);
            }
        }
        return line;
    };
    let rest = &line[start + prefix_len..];
    if surface.contains('/') {
        return format!("{}When {surface} enters,{rest}", &line[..start]);
    }
    let rest_lower = rest.to_ascii_lowercase();
    if rest_lower.contains("behold ") {
        return line;
    }
    if rest_lower.contains("another target") {
        return line;
    }
    if rest
        .to_ascii_lowercase()
        .contains(surface.to_ascii_lowercase().as_str())
    {
        return line;
    }
    // Honor the oracle's surface: when the card names itself in its ETB trigger
    // ("When Katara enters"), keep the name rather than collapsing it to the
    // generic subject. Type-surface triggers ("this artifact") still render the
    // type; this branch only fires for a captured ShortName surface.
    format!("{}When {surface} enters,{rest}", &line[..start])
}

pub(super) fn normalize_spellcast_trigger_mana_value_surface(
    triggered: &crate::ability::TriggeredAbility,
    line: String,
) -> String {
    if triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .is_none()
    {
        return line;
    }

    line.replace(
        "where X is a card in that player's hand's mana value",
        "where X is that spell's mana value",
    )
    .replace(
        "where X is a card in that object's controller's hand's mana value",
        "where X is that spell's mana value",
    )
    .replace(
        "where X is a card in your hand's mana value",
        "where X is that spell's mana value",
    )
    .replace(
        "unless that object's controller pays",
        "unless that player pays",
    )
}

pub(super) fn apply_triggered_presentation_label(
    triggered: &crate::ability::TriggeredAbility,
    line: String,
) -> String {
    let Some(presentation_label) = triggered.presentation_label.as_ref() else {
        return line;
    };
    match presentation_label {
        PresentationLabel::CaseSolved => format!("Solved — {line}"),
        PresentationLabel::CaseToSolve => {
            if let Some(condition) = triggered.intervening_if.as_ref() {
                let condition = capitalize_first(&describe_condition(condition));
                format!(
                    "To solve — {condition}. (If unsolved, solve at the beginning of your end step.)"
                )
            } else {
                line
            }
        }
        PresentationLabel::Keyword(PresentationKeyword::Recover(cost)) => {
            format!("Recover {cost}")
        }
        PresentationLabel::AbilityWord(label) if label.trim().is_empty() => line,
        PresentationLabel::AbilityWord(label) if label.trim().starts_with("__ironsmith_") => line,
        _ => {
            let Some(label) = presentation_label.display_prefix() else {
                return line;
            };
            let label = label.trim();
            if label.is_empty() || line.starts_with(label) {
                return line;
            }
            let label = if label.eq_ignore_ascii_case("catch") {
                "... Catch"
            } else {
                label
            };
            format!("{label} — {line}")
        }
    }
}

pub(super) fn describe_case_to_solve_triggered_ability(
    triggered: &crate::ability::TriggeredAbility,
) -> Option<String> {
    if !matches!(
        triggered.presentation_label.as_ref()?,
        PresentationLabel::CaseToSolve
    ) {
        return None;
    }
    let condition = triggered.intervening_if.as_ref()?;
    Some(format!(
        "To solve — {}. (If unsolved, solve at the beginning of your end step.)",
        capitalize_first(&describe_condition(condition))
    ))
}

pub(crate) fn granted_ability_self_subject_for_filter(filter: &ObjectFilter) -> &'static str {
    let card_types = if !filter.all_card_types.is_empty() {
        &filter.all_card_types
    } else {
        &filter.card_types
    };

    match card_types.as_slice() {
        [CardType::Creature] => "this creature",
        [CardType::Artifact] => "this artifact",
        [CardType::Enchantment] => "this enchantment",
        [CardType::Land] => "this land",
        [CardType::Planeswalker] => "this planeswalker",
        [CardType::Battle] => "this battle",
        _ => "this permanent",
    }
}

pub(crate) fn granted_ability_self_subject_for_choose_spec(spec: &ChooseSpec) -> &'static str {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            granted_ability_self_subject_for_filter(filter)
        }
        ChooseSpec::Source => "this permanent",
        _ => "this creature",
    }
}

pub(super) fn normalize_duplicate_sacrifice_article(text: &str) -> String {
    let text = text
        .replace("Sacrifice a a ", "Sacrifice a ")
        .replace("Sacrifice a an ", "Sacrifice an ")
        .replace("Sacrifice a artifact ", "Sacrifice an artifact ")
        .replace("sacrifice a a ", "sacrifice a ")
        .replace("sacrifice a an ", "sacrifice an ")
        .replace("sacrifice a artifact ", "sacrifice an artifact ");
    if let Some(rest) = text.strip_prefix("Sacrifice a a ") {
        return format!("Sacrifice a {rest}");
    }
    if let Some(rest) = text.strip_prefix("Sacrifice a an ") {
        return format!("Sacrifice an {rest}");
    }
    if let Some(rest) = text.strip_prefix("sacrifice a a ") {
        return format!("sacrifice a {rest}");
    }
    if let Some(rest) = text.strip_prefix("sacrifice a an ") {
        return format!("sacrifice an {rest}");
    }
    text
}

pub(super) fn normalize_sacrifice_cost_control_phrase(text: &str) -> String {
    let text = normalize_duplicate_sacrifice_article(text);
    for prefix in ["Sacrifice ", "sacrifice "] {
        let Some(rest) = text.strip_prefix(prefix) else {
            continue;
        };
        if let Some(object) = rest.strip_prefix("other ") {
            return format!("{prefix}another {object}");
        }
        let Some(object) = rest.strip_suffix(" you control") else {
            continue;
        };
        if object.starts_with("all ") {
            continue;
        }
        let object = object
            .strip_prefix("other ")
            .map(|rest| format!("another {rest}"))
            .unwrap_or_else(|| object.to_string());
        return format!("{prefix}{object}");
    }
    text
}

pub(crate) fn normalize_cost_phrase(text: &str) -> String {
    if let Some(rest) = text.strip_prefix("you ") {
        let normalized = normalize_you_verb_phrase(rest);
        let normalized = capitalize_first(&normalized);
        if let Some(life_tail) = normalized.strip_prefix("Lose ") {
            if let Some(amount) = life_tail.strip_suffix(" life") {
                return format!("Pay {} life", amount.trim());
            }
            if let Some(amount) = life_tail.strip_suffix(" lives") {
                return format!("Pay {} life", amount.trim());
            }
        }
        return normalize_sacrifice_cost_control_phrase(&normalized);
    }
    if let Some(rest) = text.strip_prefix("You ") {
        let normalized = normalize_you_verb_phrase(rest);
        let normalized = capitalize_first(&normalized);
        if let Some(life_tail) = normalized.strip_prefix("Lose ") {
            if let Some(amount) = life_tail.strip_suffix(" life") {
                return format!("Pay {} life", amount.trim());
            }
            if let Some(amount) = life_tail.strip_suffix(" lives") {
                return format!("Pay {} life", amount.trim());
            }
        }
        return normalize_sacrifice_cost_control_phrase(&normalized);
    }
    if let Some(life_tail) = text.strip_prefix("Lose ") {
        if let Some(amount) = life_tail.strip_suffix(" life") {
            return format!("Pay {} life", amount.trim());
        }
        if let Some(amount) = life_tail.strip_suffix(" lives") {
            return format!("Pay {} life", amount.trim());
        }
    }
    normalize_sacrifice_cost_control_phrase(text)
}

pub(crate) fn describe_cost_component(cost: &crate::costs::Cost) -> String {
    if let Some(mana_cost) = cost.mana_cost_ref() {
        return mana_cost.to_oracle();
    }
    if let Some(dynamic) = cost.dynamic_mana_cost_ref() {
        return describe_dynamic_mana_cost(dynamic);
    }
    if let Some(effect) = cost.effect_ref() {
        if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>()
            && matches!(tap.target, ChooseSpec::Source)
        {
            return "{T}".to_string();
        }
        if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>()
            && matches!(untap.target, ChooseSpec::Source)
        {
            return "{Q}".to_string();
        }
        if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(text) = describe_simple_discard_cost(discard)
        {
            return text;
        }
        if let Some(cost_text) = effect.0.cost_description() {
            return normalize_cost_phrase(&cost_text);
        }
        return normalize_cost_phrase(&describe_effect(effect));
    }
    if cost.requires_tap() {
        return "{T}".to_string();
    }
    if cost.requires_untap() {
        return "{Q}".to_string();
    }
    if let Some(amount) = cost.life_amount() {
        return if amount == 1 {
            "Pay 1 life".to_string()
        } else {
            format!("Pay {amount} life")
        };
    }
    if cost.is_sacrifice_self() {
        return "Sacrifice this".to_string();
    }
    if let Some(filter) = cost.sacrifice_filter() {
        let subject = strip_leading_article(&filter.description())
            .replace(" in the battlefield", "")
            .replace(" on the battlefield", "");
        return format!("Sacrifice {}", with_indefinite_article(&subject));
    }
    let display = cost.display().trim().to_string();
    if display.is_empty() {
        format!("{cost:?}")
    } else {
        display
    }
}

pub(super) fn describe_loyalty_activation_prefix(costs: &[crate::costs::Cost]) -> Option<String> {
    match costs {
        [] => None,
        [cost] => {
            if let Some(effect) = cost.effect_ref() {
                if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>()
                    && put.counter_type == CounterType::Loyalty
                    && matches!(put.target.base(), ChooseSpec::Source)
                    && let Some(amount) = loyalty_prefix_amount(&put.amount)
                {
                    return Some(format!("+{amount}"));
                }
                if let Some(remove) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>()
                    && remove.counter_type == CounterType::Loyalty
                    && matches!(remove.target.base(), ChooseSpec::Source)
                    && let Some(amount) = loyalty_prefix_amount(&remove.count)
                {
                    return Some(format!("−{amount}"));
                }
            }
            loyalty_prefix_from_cost_text(&describe_cost_component(cost))
        }
        _ => None,
    }
}

pub(super) fn describe_loyalty_activation_prefix_for_activated(
    activated: &crate::ability::ActivatedAbility,
) -> Option<String> {
    describe_loyalty_activation_prefix(activated.mana_cost.costs()).or_else(|| {
        (activated.is_loyalty_ability() && activated.mana_cost.costs().is_empty())
            .then(|| "0".to_string())
    })
}

pub(super) fn loyalty_prefix_amount(value: &Value) -> Option<String> {
    match value.unhinted() {
        Value::Fixed(amount) => Some((*amount).max(0).to_string()),
        Value::X => Some("X".to_string()),
        _ => None,
    }
}

pub(super) fn loyalty_prefix_from_cost_text(text: &str) -> Option<String> {
    let lower = text.trim().trim_end_matches('.').to_ascii_lowercase();
    if lower == "put a loyalty counter on this planeswalker"
        || lower == "put a loyalty counter on this source"
    {
        return Some("+1".to_string());
    }
    for suffix in [
        " loyalty counter on this planeswalker",
        " loyalty counters on this planeswalker",
        " loyalty counter on this source",
        " loyalty counters on this source",
    ] {
        if let Some(rest) = lower
            .strip_prefix("put ")
            .and_then(|rest| rest.strip_suffix(suffix))
            && let Some(amount) = loyalty_cost_amount_text(rest)
        {
            return Some(format!("+{amount}"));
        }
    }
    for suffix in [
        " loyalty counter from it",
        " loyalty counters from it",
        " loyalty counter from this planeswalker",
        " loyalty counters from this planeswalker",
        " loyalty counter from this source",
        " loyalty counters from this source",
    ] {
        if let Some(rest) = lower
            .strip_prefix("remove ")
            .and_then(|rest| rest.strip_suffix(suffix))
            && let Some(amount) = loyalty_cost_amount_text(rest)
        {
            return Some(format!("−{amount}"));
        }
    }
    None
}

pub(super) fn loyalty_cost_amount_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.eq_ignore_ascii_case("x") {
        return Some("X".to_string());
    }
    loyalty_cost_amount_word(trimmed).map(|amount| amount.to_string())
}

pub(super) fn loyalty_cost_amount_word(text: &str) -> Option<i32> {
    let text = text.trim();
    ironsmith_core::parse_cardinal_word(text).and_then(|value| value.try_into().ok())
}

pub(super) fn life_lost_this_way_group_size(value: &Value) -> Option<i32> {
    match value.unhinted() {
        Value::EffectMetric {
            metric: crate::effect::EffectMetric::LifeLost,
            ..
        }
        | Value::PendingEffectMetric {
            metric: crate::effect::EffectMetric::LifeLost,
            ..
        }
        | Value::EventValue(EventValueSpec::LifeAmount) => Some(1),
        Value::DividedRoundedDown(inner, divisor) if *divisor > 1 => {
            life_lost_this_way_group_size(inner).map(|_| *divisor)
        }
        _ => None,
    }
}

pub(super) fn describe_simple_discard_cost(
    discard: &crate::effects::DiscardEffect,
) -> Option<String> {
    if discard.random || discard.player != PlayerFilter::You || discard.tag.is_some() {
        return None;
    }
    let Value::Fixed(count) = discard.count else {
        return None;
    };
    let count = count.max(0) as u32;
    let (card_type, supertypes, name_filter, other_filter) = match &discard.card_filter {
        None => (None, Vec::new(), None, false),
        Some(filter) if !filter.any_of.is_empty() => {
            if let Some(filter_text) = describe_discard_any_of_filter(filter) {
                return Some(if count == 1 {
                    format!("Discard {filter_text}")
                } else {
                    format!("Discard {count} {filter_text}s")
                });
            }
            return None;
        }
        Some(filter) if filter.card_types.len() <= 1 => {
            let expected = ObjectFilter {
                zone: Some(Zone::Hand),
                card_types: filter.card_types.clone(),
                supertypes: filter.supertypes.clone(),
                name: filter.name.clone(),
                other: filter.other,
                ..Default::default()
            };
            if filter != &expected {
                return None;
            }
            (
                filter.card_types.first().copied(),
                filter.supertypes.clone(),
                filter.name.as_deref(),
                filter.other,
            )
        }
        Some(_) => return None,
    };

    if let Some(name) = name_filter {
        if count != 1 || !supertypes.is_empty() {
            return None;
        }
        let name = normalize_card_name_for_surface(name);
        return Some(if other_filter {
            format!("Discard another card named {name}")
        } else {
            format!("Discard a card named {name}")
        });
    }

    if supertypes.is_empty() && card_type.is_none() {
        return Some(if count == 1 {
            "Discard a card".to_string()
        } else {
            format!("Discard {count} cards")
        });
    }

    let mut descriptors: Vec<&str> = supertypes
        .iter()
        .map(|supertype| supertype.name())
        .collect();
    if let Some(card_type) = card_type {
        descriptors.push(describe_card_type_word_local(card_type));
    }
    let type_text = with_indefinite_article(&format!("{} card", descriptors.join(" ")));
    Some(if count == 1 {
        format!("Discard {type_text}")
    } else {
        format!("Discard {count} {type_text}")
    })
}

pub(super) fn describe_discard_any_of_filter(filter: &ObjectFilter) -> Option<String> {
    let expected = ObjectFilter {
        zone: Some(Zone::Hand),
        any_of: filter.any_of.clone(),
        ..Default::default()
    };
    if filter != &expected {
        return None;
    }

    let parts = filter
        .any_of
        .iter()
        .map(describe_simple_hand_card_filter)
        .collect::<Option<Vec<_>>>()?;
    Some(parts.join(" or "))
}

pub(super) fn describe_simple_hand_card_filter(filter: &ObjectFilter) -> Option<String> {
    let mut expected = ObjectFilter {
        zone: filter.zone,
        card_types: filter.card_types.clone(),
        subtypes: filter.subtypes.clone(),
        colors: filter.colors,
        name: filter.name.clone(),
        ..Default::default()
    };
    if !matches!(expected.zone, None | Some(Zone::Hand)) {
        return None;
    }
    if filter != &expected {
        return None;
    }
    expected.zone = None;

    if let Some(name) = filter.name.as_deref() {
        return Some(format!(
            "a card named {}",
            normalize_card_name_for_surface(name)
        ));
    }
    if filter.card_types.len() == 1 && filter.subtypes.is_empty() && filter.colors.is_none() {
        return Some(with_indefinite_article(&format!(
            "{} card",
            describe_card_type_word_local(filter.card_types[0])
        )));
    }
    if filter.subtypes.len() == 1 && filter.card_types.is_empty() && filter.colors.is_none() {
        return Some(with_indefinite_article(&format!(
            "{} card",
            filter.subtypes[0].display_name()
        )));
    }
    if let Some(colors) = filter.colors
        && colors.count() == 1
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
    {
        let color = crate::color::Color::ALL
            .into_iter()
            .find(|color| colors.contains(*color))?;
        return Some(with_indefinite_article(&format!("{} card", color.name())));
    }
    None
}

pub(super) fn is_grandeur_activation_cost(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.mana_cost.costs().iter().any(|cost| {
        cost.effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::DiscardEffect>())
            .is_some_and(|discard| {
                discard.player == PlayerFilter::You
                    && discard.count == Value::Fixed(1)
                    && discard
                        .card_filter
                        .as_ref()
                        .is_some_and(|filter| filter.other && filter.name.is_some())
            })
    })
}

pub(super) fn normalize_card_name_for_surface(name: &str) -> String {
    fn titlecase_token(token: &str) -> String {
        let mut out = String::with_capacity(token.len());
        let mut capitalize_next = true;
        for ch in token.chars() {
            if ch.is_ascii_alphabetic() {
                if capitalize_next {
                    out.push(ch.to_ascii_uppercase());
                    capitalize_next = false;
                } else {
                    out.push(ch.to_ascii_lowercase());
                }
            } else {
                out.push(ch);
                capitalize_next = matches!(ch, '-' | '\'' | '`');
            }
        }
        out
    }

    name.split_whitespace()
        .map(titlecase_token)
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn describe_dynamic_mana_cost(dynamic: &ironsmith_core::DynamicManaCost) -> String {
    if matches!(
        dynamic.display_hint,
        ironsmith_core::DynamicManaDisplayHint::ManaEqualTo
    ) && let Some(value) = dynamic.additional_generic.as_ref()
    {
        return format!("mana equal to {}", describe_value(value));
    }

    let mut text = if dynamic.base.is_empty() {
        String::new()
    } else {
        dynamic.base.to_oracle()
    };
    if let Some(multiplier) = dynamic.multiplier.as_ref() {
        if dynamic.base.to_oracle() == "{X}"
            && dynamic.x_value.is_none()
            && dynamic.additional_generic.is_none()
            && matches!(multiplier, Value::Fixed(2))
        {
            return "twice {X}".to_string();
        }
        let each = describe_payment_each_value(multiplier);
        if text.is_empty() {
            text = format!("{{1}} for each {each}");
        } else {
            text = format!("{text} for each {each}");
        }
    }
    if let Some(additional) = dynamic.additional_generic.as_ref() {
        let each = describe_payment_each_value(additional);
        let additional_text = match additional {
            Value::CountScaled(_, multiplier) if *multiplier > 0 => {
                format!("plus an additional {{{multiplier}}} for each {each}")
            }
            Value::Fixed(amount) => format!("plus an additional {{{amount}}}"),
            _ => format!("plus an additional {{1}} for each {each}"),
        };
        if text.is_empty() {
            text = additional_text;
        } else {
            text = format!("{text} {additional_text}");
        }
    }
    if let Some(x_value) = dynamic.x_value.as_ref() {
        if text.is_empty() {
            text = "{X}".to_string();
        }
        text = format!("{text}, where X is {}", describe_value(x_value));
    }
    if text.is_empty() {
        "{0}".to_string()
    } else {
        text
    }
}

pub(super) fn describe_cost_list_with_trailing_x_definition(
    costs: &[crate::costs::Cost],
) -> (String, Option<String>) {
    let mut parts = describe_cost_component_parts(costs);
    let mut trailing = None;
    for cost in costs {
        let Some(dynamic) = cost.dynamic_mana_cost_ref() else {
            continue;
        };
        let Some(x_value) = dynamic.x_value.as_ref() else {
            continue;
        };
        let full = describe_dynamic_mana_cost(dynamic);
        let base_dynamic = ironsmith_core::DynamicManaCost::new(
            dynamic.base.clone(),
            None,
            dynamic.additional_generic.clone(),
            dynamic.multiplier.clone(),
            dynamic.display_hint.clone(),
        );
        let base = describe_dynamic_mana_cost(&base_dynamic);
        if let Some(part) = parts.iter_mut().find(|part| **part == full) {
            *part = base;
        }
        trailing = Some(if value_is_source_exiled_mana_value(x_value) {
            "X is the mana value of that card".to_string()
        } else {
            format!("X is {}", describe_value(x_value))
        });
    }
    (parts.join(", "), trailing)
}

pub(super) fn describe_total_cost_payment(cost: &crate::cost::TotalCost) -> String {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            let parts = describe_cost_component_parts(costs)
                .into_iter()
                .map(|part| part.strip_prefix("Pay ").unwrap_or(&part).to_string())
                .collect::<Vec<_>>();
            match parts.as_slice() {
                [] => "Free".to_string(),
                [one] => one.clone(),
                [left, right] => format!("{left} and {right}"),
                _ => parts.join(", "),
            }
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => branches
            .iter()
            .map(describe_total_cost_payment)
            .map(|part| part.strip_prefix("Pay ").unwrap_or(&part).to_string())
            .collect::<Vec<_>>()
            .join(" or "),
    }
}

pub(super) fn describe_payment_each_value(value: &Value) -> String {
    match value {
        Value::Count(filter) => describe_for_each_filter(filter),
        Value::CountScaled(filter, _) => describe_for_each_filter(filter),
        Value::BasicLandTypesAmong(filter) => {
            describe_basic_land_types_among(filter).replace("basic land types", "basic land type")
        }
        Value::CreatureTypesAmong(filter) => format!(
            "creature type among {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::CardTypesAmong(filter) => format!(
            "card type among {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::ColorsAmong(filter) => describe_colors_among(filter),
        Value::DistinctPowers(filter) => format!(
            "different power among {}",
            describe_for_each_count_filter(filter)
        ),
        Value::PartySize(PlayerFilter::You) => "creature in your party".to_string(),
        _ => describe_value(value),
    }
}

pub(super) fn describe_cost_component_parts(costs: &[crate::costs::Cost]) -> Vec<String> {
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < costs.len() {
        if idx + 1 < costs.len()
            && let Some(remove) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::RemoveCountersEffect>())
            && matches!(remove.target, ChooseSpec::Source)
            && costs[idx + 1].is_sacrifice_self()
        {
            parts.push(format!(
                "Remove {} from this source and sacrifice it",
                describe_put_counter_phrase(&remove.count, remove.counter_type)
            ));
            idx += 2;
            continue;
        }
        if let Some((compact, consumed)) =
            describe_exile_source_and_named_artifact_costs(&costs[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(tap) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::TapEffect>())
            && let Some(compact) = describe_choose_then_tap_cost(choose, tap)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(exile) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
            && let Some(compact) = describe_choose_then_exile(choose, exile)
        {
            parts.push(normalize_cost_phrase(&compact));
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(sacrifice) = costs[idx + 1].effect_ref().and_then(sacrifice_view)
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(normalize_cost_phrase(&compact));
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(unattach) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::UnattachObjectsEffect>())
            && let Some(compact) = describe_choose_then_unattach_cost(choose, unattach)
        {
            parts.push(normalize_cost_phrase(&compact));
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(return_to_hand) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ReturnToHandEffect>())
            && let Some(compact) = describe_choose_then_return_to_hand_cost(choose, return_to_hand)
        {
            parts.push(normalize_cost_phrase(&compact));
            idx += 2;
            continue;
        }
        parts.push(describe_cost_component(&costs[idx]));
        idx += 1;
    }
    parts
}

pub(super) fn describe_choose_then_unattach_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    unattach: &crate::effects::UnattachObjectsEffect,
) -> Option<String> {
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !matches!(unattach.objects.base(), ChooseSpec::Tagged(tag) if tag.as_str() == choose.tag.as_str())
    {
        return None;
    }

    let exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    if exact == 0 {
        return None;
    }
    let noun = if choose.filter.card_types == [CardType::Artifact]
        && choose.filter.subtypes == [crate::types::Subtype::Equipment]
    {
        // Equipment already carries its artifact type in Oracle cost text.
        // The filter keeps Artifact for runtime legality, but the surface noun
        // should remain the typed subtype rather than "artifact Equipment".
        "Equipment".to_string()
    } else {
        choose.filter.description()
    };
    if exact == 1 {
        return Some(format!(
            "Unattach {} from this source",
            with_indefinite_article(&noun)
        ));
    }
    let count = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
    Some(format!(
        "Unattach {count} {} from this source",
        pluralize_noun_phrase(&noun)
    ))
}

pub(super) fn describe_choose_then_return_to_hand_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    return_to_hand: &crate::effects::ReturnToHandEffect,
) -> Option<String> {
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !return_to_hand_uses_chosen_tag(return_to_hand, choose.tag.as_str())
    {
        return None;
    }

    let exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    if exact == 0 {
        return None;
    }
    let noun = choose.filter.description();
    if exact == 1 {
        return Some(format!(
            "Return {} to its owner's hand",
            with_indefinite_article(&noun)
        ));
    }
    let count = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
    Some(format!(
        "Return {count} {} to their owners' hands",
        pluralize_noun_phrase(&noun)
    ))
}

pub(super) fn describe_exile_source_and_named_artifact_costs(
    costs: &[crate::costs::Cost],
) -> Option<(String, usize)> {
    let first_exile = costs
        .first()?
        .effect_ref()?
        .downcast_ref::<crate::effects::ExileEffect>()?;
    if !matches!(first_exile.spec.base(), ChooseSpec::Source) {
        return None;
    }

    let mut idx = 1usize;
    let mut names = Vec::new();
    while idx + 1 < costs.len() {
        let choose = costs[idx]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>());
        let exile = costs[idx + 1]
            .effect_ref()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>());
        let Some((choose, exile)) = choose.zip(exile) else {
            break;
        };
        let Some(name) = named_artifact_exile_cost_name(choose, exile) else {
            break;
        };
        names.push(title_case_card_name_fragment(&name));
        idx += 2;
    }

    if names.is_empty() {
        return None;
    }

    Some((
        format!(
            "Exile this source and artifacts you control named {}",
            join_with_and(&names)
        ),
        idx,
    ))
}

pub(super) fn named_artifact_exile_cost_name(
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_exact_count(choose) != Some(1)
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !exile_uses_chosen_tag(&exile.spec, choose.tag.as_str())
    {
        return None;
    }
    let filter = &choose.filter;
    if filter.controller != Some(PlayerFilter::You)
        || filter.card_types != [CardType::Artifact]
        || filter.name.is_none()
    {
        return None;
    }
    Some(filter.name.clone().unwrap_or_default())
}

pub(super) fn title_case_card_name_fragment(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            if matches!(word, "a" | "an" | "and" | "of" | "the" | "to") {
                return word.to_string();
            }
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut out = first.to_uppercase().to_string();
                    out.push_str(chars.as_str());
                    out
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn title_case_named_card_selection(selection: &str) -> String {
    let Some((head, name)) = selection.split_once(" named ") else {
        return selection.to_string();
    };
    format!("{head} named {}", title_case_card_name_fragment(name))
}

pub(super) fn describe_basic_land_type_search_slots(
    search_slots: &crate::effects::SearchLibrarySlotsEffect,
) -> Option<&'static str> {
    if search_slots.slots.len() != 5 {
        return None;
    }

    let basic_land_types = [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ];
    for subtype in basic_land_types {
        let expected = ObjectFilter::default()
            .in_zone(Zone::Library)
            .with_type(CardType::Land)
            .with_subtype(subtype);
        let has_slot = search_slots.slots.iter().any(|slot| {
            let mut filter = slot.filter.clone();
            filter.owner = None;
            filter == expected
        });
        if !has_slot {
            return None;
        }
    }

    Some("a land card of each basic land type")
}

pub(crate) fn describe_cost_list(costs: &[crate::costs::Cost]) -> String {
    describe_cost_component_parts(costs).join(", ")
}

pub(crate) fn with_indefinite_article(noun: &str) -> String {
    let trimmed = noun.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
    }
    for prefix in [
        "the active player's ",
        "that player's ",
        "target player's ",
        "an opponent's ",
        "opponent's ",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return with_indefinite_article(rest);
        }
    }
    if trimmed.starts_with("a ") || trimmed.starts_with("an ") {
        let (article, rest) = if let Some(rest) = trimmed.strip_prefix("an ") {
            ("an", rest)
        } else if let Some(rest) = trimmed.strip_prefix("a ") {
            ("a", rest)
        } else {
            ("", trimmed)
        };
        if let Some(first) = rest.chars().next() {
            let should_be_an = matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u');
            if should_be_an && article == "a" {
                return format!("an {rest}");
            }
            if !should_be_an && article == "an" {
                return format!("a {rest}");
            }
        }
        return trimmed.to_string();
    }
    if trimmed.starts_with("another ")
        || trimmed.starts_with("target ")
        || trimmed.starts_with("each ")
        || trimmed.starts_with("all ")
        || trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit())
    {
        return trimmed.to_string();
    }
    let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u' | 'x') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

pub(crate) fn ensure_indefinite_article(noun: &str) -> String {
    let trimmed = noun.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
        || lower.starts_with("each ")
        || lower.starts_with("all ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("those ")
        || lower.starts_with("target ")
        || lower.starts_with("any ")
        || lower.starts_with("up to ")
        || lower.starts_with("at least ")
        || lower.chars().next().is_some_and(|ch| ch.is_ascii_digit())
    {
        return trimmed.to_string();
    }

    let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

pub(crate) fn describe_for_each_double_counters(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    if for_each.effects.len() != 1 {
        return None;
    }
    let put = for_each.effects[0].downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed {
        return None;
    }
    if !matches!(put.target.base(), ChooseSpec::Iterated) {
        return None;
    }
    let Value::CountersOn(source, Some(counter_type)) = &put.amount else {
        return None;
    };
    if !matches!(source.base(), ChooseSpec::Iterated) {
        return None;
    }

    let filter_description = for_each.filter.description();
    let filter_text = strip_indefinite_article(&filter_description);
    let has_tagged_iterated_reference =
        for_each.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        });
    if has_tagged_iterated_reference {
        let plural = pluralize_noun_phrase(filter_text);
        return Some(format!(
            "Double the number of {} counters on each of those {}",
            describe_counter_type(counter_type.clone()),
            plural
        ));
    }

    Some(format!(
        "Double the number of {} counters on each {}",
        describe_counter_type(counter_type.clone()),
        filter_text
    ))
}

pub(crate) fn describe_for_each_put_counters_then_untap(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    let [first, second] = for_each.effects.as_slice() else {
        return None;
    };
    let put = first.downcast_ref::<crate::effects::PutCountersEffect>()?;
    let untap = second.downcast_ref::<crate::effects::UntapEffect>()?;
    if put.distributed || put.target_count.is_some() {
        return None;
    }
    if !matches!(put.target.base(), ChooseSpec::Iterated) {
        return None;
    }
    if !matches!(untap.target.base(), ChooseSpec::Iterated)
        && !untap_target_is_implicit_previous_group(untap)
    {
        return None;
    }

    let description = for_each.filter.description();
    let filter_text = strip_indefinite_article(&description);
    Some(format!(
        "Put {} on each {}, then untap them",
        describe_put_counter_phrase(&put.amount, put.counter_type),
        filter_text
    ))
}

pub(crate) fn describe_for_each_devotion_damage(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    if for_each.effects.len() != 1 {
        return None;
    }
    let deal = if let Some(deal) =
        for_each.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()
    {
        deal
    } else if let Some(tagged) = for_each.effects[0].downcast_ref::<crate::effects::TaggedEffect>()
    {
        tagged
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()?
    } else {
        return None;
    };
    if !matches!(deal.target, ChooseSpec::Iterated) {
        return None;
    }
    if !matches!(
        deal.amount,
        Value::Devotion { .. } | Value::DevotionToChosenColor(_)
    ) {
        return None;
    }

    let description = for_each.filter.description();
    let filter_text = strip_indefinite_article(&description);
    Some(format!(
        "Deal damage to each {filter_text} equal to {}",
        describe_value(&deal.amount)
    ))
}

pub(super) fn describe_for_each_sacrifice_by_controller(
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    let [effect] = for_each.effects.as_slice() else {
        return None;
    };
    let sacrifice = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    let target = sacrifice.target.base();
    let sacrifices_iterated = matches!(target, ChooseSpec::Iterated)
        || matches!(target, ChooseSpec::Tagged(tag) if tag.as_str() == "__it__");
    if !sacrifices_iterated {
        return None;
    }

    let description = for_each.filter.description();
    let filter_text = strip_indefinite_article(&description);
    Some(format!(
        "Each {filter_text} is sacrificed by its controller"
    ))
}

pub(crate) fn describe_tap_then_damage_for_tapped_this_way(
    with_id: &crate::effects::WithIdEffect,
    deal: &crate::effects::DealDamageEffect,
) -> Option<String> {
    if !is_effect_count_reference(&deal.amount, Some(with_id.id)) {
        return None;
    }
    if !matches!(deal.target, ChooseSpec::Player(PlayerFilter::Active)) {
        return None;
    }
    let tap = with_id.effect.downcast_ref::<crate::effects::TapEffect>()?;
    let ChooseSpec::All(filter) = tap.target.base() else {
        return None;
    };
    if !matches!(filter.controller, Some(PlayerFilter::Active)) || !filter.untapped {
        return None;
    }

    let full_description_storage = filter.description();
    let full_description = strip_indefinite_article(&full_description_storage);
    let controlled_description = full_description
        .strip_prefix("the active player's ")
        .or_else(|| full_description.strip_prefix("active player's "))
        .map(|rest| format!("{} that player controls", pluralize_noun_phrase(rest)))
        .unwrap_or_else(|| pluralize_noun_phrase(full_description));

    let mut count_filter = filter.clone();
    count_filter.controller = None;
    count_filter.untapped = false;
    let count_description_storage = count_filter.description();
    let count_description =
        pluralize_noun_phrase(strip_indefinite_article(&count_description_storage));
    Some(format!(
        "Tap all {controlled_description} and this permanent deals X damage to the player, where X is the number of {count_description} tapped this way"
    ))
}

pub(crate) fn describe_choose_creature_type_then_x_boost(
    choose: &crate::effects::ChooseCreatureTypeEffect,
    followup: &Effect,
) -> Option<String> {
    if !choose.excluded_subtypes.is_empty() || !matches!(choose.chooser, PlayerFilter::You) {
        return None;
    }
    let followup = if let Some(tagged) = followup.downcast_ref::<crate::effects::TaggedEffect>() {
        &tagged.effect
    } else {
        followup
    };
    let apply = followup.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(apply.until, Until::EndOfTurn)
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
    if !matches!((power, toughness), (Value::X, Value::X)) {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(filter) = &apply.target else {
        return None;
    };
    if filter.card_types.as_slice() != [crate::types::CardType::Creature]
        || !filter.chosen_creature_type
    {
        return None;
    }
    Some("Creatures of the creature type of your choice get +X/+X until end of turn".to_string())
}

pub(super) fn describe_choose_creature_type_then_must_attack(
    choose: &crate::effects::ChooseCreatureTypeEffect,
    followup: &Effect,
) -> Option<String> {
    if !choose.excluded_subtypes.is_empty() || !matches!(choose.chooser, PlayerFilter::You) {
        return None;
    }
    let followup = if let Some(tagged) = followup.downcast_ref::<crate::effects::TaggedEffect>() {
        &tagged.effect
    } else {
        followup
    };
    let apply = followup.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
        return None;
    };
    let crate::continuous::EffectTarget::Filter(filter) = &apply.target else {
        return None;
    };
    if !matches!(apply.until, Until::EndOfTurn)
        || ability.id() != crate::static_abilities::StaticAbilityId::MustAttack
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || filter.card_types.as_slice() != [crate::types::CardType::Creature]
        || !filter.chosen_creature_type
    {
        return None;
    }
    Some("Creatures of the creature type of your choice attack this turn if able.".to_string())
}

pub(super) fn put_counters_each_filter_view(
    effect: &Effect,
) -> Option<(String, &ObjectFilter, Option<&TagKey>)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some((text, filter, _)) = put_counters_each_filter_view(&tagged.effect)
    {
        return Some((text, filter, Some(&tagged.tag)));
    }

    if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        if put.distributed || put.target_count.is_some() {
            return None;
        }
        let ChooseSpec::All(filter) = put.target.base() else {
            return None;
        };
        let description = filter.description();
        let filter_text = strip_indefinite_article(&description);
        return Some((
            format!(
                "Put {} on each {filter_text}",
                describe_put_counter_phrase(&put.amount, put.counter_type)
            ),
            filter,
            None,
        ));
    }

    let for_each = effect.downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.effects.len() != 1 {
        return None;
    }
    let put = for_each.effects[0].downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed || put.target_count.is_some() || !matches!(put.target, ChooseSpec::Iterated)
    {
        return None;
    }

    let description = for_each.filter.description();
    let filter_text = strip_indefinite_article(&description);
    Some((
        format!(
            "Put {} on each {filter_text}",
            describe_put_counter_phrase(&put.amount, put.counter_type)
        ),
        &for_each.filter,
        None,
    ))
}

pub(super) fn untap_target_is_implicit_previous_group(untap: &crate::effects::UntapEffect) -> bool {
    match untap.target.base() {
        ChooseSpec::Tagged(tag) if tag.as_str() == "__it__" => true,
        ChooseSpec::All(filter) => {
            !filter.source
                && filter.zone == Some(Zone::Hand)
                && filter.controller.is_none()
                && filter.owner == Some(PlayerFilter::IteratedPlayer)
                && filter.card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.supertypes.is_empty()
                && filter.tagged_constraints.is_empty()
        }
        _ => false,
    }
}

pub(super) fn untap_target_references_tag(
    untap: &crate::effects::UntapEffect,
    tag: &TagKey,
) -> bool {
    match untap.target.base() {
        ChooseSpec::Tagged(candidate) => candidate == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag == *tag
            })
        }
        _ => false,
    }
}

pub(super) fn describe_put_counters_then_untap_them(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let (put_text, filter, put_tag) = put_counters_each_filter_view(first)?;
    let untap = second.downcast_ref::<crate::effects::UntapEffect>()?;
    let targets_countered_group = if let Some(put_tag) = put_tag {
        untap_target_references_tag(untap, put_tag)
    } else {
        untap_target_is_implicit_previous_group(untap)
    };
    if !targets_countered_group {
        return None;
    }
    let subject = if filter.card_types.contains(&CardType::Creature) {
        "those creatures"
    } else {
        "them"
    };
    Some(format!("{put_text}. Untap {subject}"))
}

pub(crate) fn describe_for_each_tagged_this_way_subject(filter: &ObjectFilter) -> Option<String> {
    let action = filter.tagged_constraints.iter().find_map(|constraint| {
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        let tag = constraint.tag.as_str();
        if tag.starts_with("exiled_") {
            Some("exiled")
        } else if tag.starts_with("destroyed_") {
            Some("destroyed")
        } else if tag.starts_with("sacrificed_") {
            Some("sacrificed")
        } else if tag.starts_with("revealed_") {
            Some("revealed")
        } else if tag.starts_with("discarded_") {
            Some("discarded")
        } else if tag.starts_with("milled_") {
            Some("milled")
        } else {
            None
        }
    })?;

    let mut subject = strip_indefinite_article(&filter.description()).to_string();
    if action == "exiled" {
        if let Some(head) = subject.strip_suffix(" in exile") {
            subject = head.trim().to_string();
        } else if let Some((head, tail)) = subject.split_once(" in exile ") {
            subject = format!("{} {}", head.trim(), tail.trim());
        }
    } else if action == "revealed" {
        if let Some(head) = subject.strip_suffix(" permanent") {
            subject = format!("{} card", head.trim());
        } else if let Some(head) = subject.strip_suffix(" permanents") {
            subject = format!("{} cards", head.trim());
        }
    }
    let subject = subject.trim();
    if subject.is_empty() {
        return None;
    }

    Some(format!("For each {subject} {action} this way"))
}

pub(crate) fn strip_indefinite_article(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed
        .strip_prefix("a ")
        .or_else(|| trimmed.strip_prefix("A "))
    {
        return rest;
    }
    if let Some(rest) = trimmed
        .strip_prefix("an ")
        .or_else(|| trimmed.strip_prefix("An "))
    {
        return rest;
    }
    trimmed
}

pub(crate) fn pluralize_word(word: &str) -> String {
    if word.chars().last().is_some_and(|ch| ch.is_ascii_digit()) {
        return word.to_string();
    }
    if let Some((prefix, last)) = word.rsplit_once(' ')
        && !prefix.is_empty()
        && !last.is_empty()
    {
        return format!("{prefix} {}", pluralize_word(last));
    }
    let lower = word.to_ascii_lowercase();
    if matches!(lower.as_str(), "less" | "greater") {
        return word.to_string();
    }
    if lower == "plains" || lower == "urzas" {
        return word.to_string();
    }
    if lower == "elf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Elves".to_string()
        } else {
            "elves".to_string()
        };
    }
    if lower == "dwarf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Dwarves".to_string()
        } else {
            "dwarves".to_string()
        };
    }
    if lower == "wolf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Wolves".to_string()
        } else {
            "wolves".to_string()
        };
    }
    if lower == "werewolf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Werewolves".to_string()
        } else {
            "werewolves".to_string()
        };
    }
    if lower == "myr" || lower == "merfolk" || lower == "equipment" {
        return word.to_string();
    }
    if lower == "mouse" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Mice".to_string()
        } else {
            "mice".to_string()
        };
    }
    if lower.ends_with('y')
        && lower.len() > 1
        && !matches!(
            lower.chars().nth(lower.len() - 2),
            Some('a' | 'e' | 'i' | 'o' | 'u')
        )
    {
        return format!("{}ies", &word[..word.len() - 1]);
    }
    if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        return format!("{word}es");
    }
    format!("{word}s")
}

pub(crate) fn pluralize_noun_phrase(phrase: &str) -> String {
    let mut base = strip_indefinite_article(phrase).trim();
    let mut trailing = "";
    if let Some(stripped) = base.strip_suffix('.') {
        base = stripped.trim_end();
        trailing = ".";
    }
    if base.contains(" or ") {
        if base.contains(", ") {
            let normalized = base.replace(", or ", ", ");
            let parts = normalized
                .split(", ")
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            if parts.len() > 1 {
                let plural_parts = parts
                    .iter()
                    .map(|part| pluralize_noun_phrase(part))
                    .collect::<Vec<_>>();
                return format!("{}{}", join_with_or(&plural_parts), trailing);
            }
        }
        let parts = base
            .split(" or ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() > 1 {
            let plural_parts = parts
                .iter()
                .map(|part| pluralize_noun_phrase(part))
                .collect::<Vec<_>>();
            return format!("{}{}", plural_parts.join(" or "), trailing);
        }
    }
    if let Some((head, tail)) = base.split_once(" with ") {
        if let Some(relation_tail) = plural_power_toughness_relation_tail(tail.trim()) {
            return format!(
                "{} {}{}",
                pluralize_noun_phrase(head),
                relation_tail,
                trailing
            );
        }
        return format!(
            "{} with {}{}",
            pluralize_noun_phrase(head),
            tail.trim(),
            trailing
        );
    }
    if let Some((head, tail)) = base.split_once(" without ") {
        return format!(
            "{} without {}{}",
            pluralize_noun_phrase(head),
            tail.trim(),
            trailing
        );
    }
    if let Some((head, tail)) = base.split_once(" that ") {
        return format!(
            "{} that {}{}",
            pluralize_noun_phrase(head),
            tail.trim(),
            trailing
        );
    }
    if let Some((head, tail)) = base.split_once(" other than ") {
        return format!(
            "{} other than {}{}",
            pluralize_noun_phrase(head.trim()),
            tail.trim(),
            trailing
        );
    }
    for suffix in [
        " you control of the chosen type",
        " you own of the chosen type",
        " they control of the chosen type",
        " they own of the chosen type",
        " an opponent controls of the chosen type",
        " an opponent owns of the chosen type",
        " target opponent controls of the chosen type",
        " target player controls of the chosen type",
        " target player owns of the chosen type",
        " that player controls of the chosen type",
        " that player owns of the chosen type",
        " you control",
        " you own",
        " they control",
        " they own",
        " an opponent controls",
        " an opponent owns",
        " target opponent controls",
        " target player controls",
        " target player owns",
        " that player controls",
        " that player owns",
        " active player controls",
        " active player owns",
        " defending player controls",
        " defending player owns",
        " attacking player controls",
        " attacking player owns",
        " damaged player controls",
        " damaged player owns",
        " a teammate controls",
        " a teammate owns",
        " in your graveyard",
        " in target player's graveyard",
        " in that player's graveyard",
        " in single graveyard",
        " in a graveyard",
        " in graveyard",
        " in all graveyards",
        " in your hand",
        " in target player's hand",
        " in that player's hand",
        " in a hand",
        " in your library",
        " in target player's library",
        " in that player's library",
        " in a library",
        " in exile",
        " revealed this way",
        " of the chosen type",
        " that aren't of the chosen type",
    ] {
        if let Some(head) = base.strip_suffix(suffix) {
            let head = head.trim_end();
            let head_plural = pluralize_word(head);
            return format!("{head_plural}{suffix}{trailing}");
        }
    }
    if let Some((head, tail)) = base.split_once(" named ") {
        return format!(
            "{} named {}{}",
            pluralize_noun_phrase(head.trim()),
            title_case_card_name_fragment(tail.trim()),
            trailing
        );
    }
    if base.ends_with('s') {
        format!("{base}{trailing}")
    } else {
        format!("{}{}", pluralize_word(base), trailing)
    }
}

pub(super) fn plural_power_toughness_relation_tail(tail: &str) -> Option<&'static str> {
    match tail {
        "power greater than its toughness" => {
            Some("that each have power greater than their toughness")
        }
        "toughness greater than its power" => {
            Some("that each have toughness greater than their power")
        }
        _ => None,
    }
}

pub(crate) fn sacrifice_uses_chosen_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

pub(super) fn filter_uses_chosen_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

pub(super) fn filter_excludes_chosen_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            && constraint.tag.as_str() == tag
    })
}

pub(super) fn destroy_effect_for_choose_compaction(
    effect: &Effect,
) -> Option<&crate::effects::DestroyEffect> {
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some(destroy);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return destroy_effect_for_choose_compaction(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return destroy_effect_for_choose_compaction(&with_id.effect);
    }
    None
}

pub(super) fn is_iterated_player_creature_battlefield_filter(filter: &ObjectFilter) -> bool {
    filter.zone.is_none_or(|zone| zone == Zone::Battlefield)
        && filter.card_types == vec![CardType::Creature]
        && filter.controller == Some(PlayerFilter::IteratedPlayer)
}

pub(super) fn is_iterated_player_nontoken_land_battlefield_filter(filter: &ObjectFilter) -> bool {
    filter.zone.is_none_or(|zone| zone == Zone::Battlefield)
        && filter.card_types == vec![CardType::Land]
        && filter.controller == Some(PlayerFilter::IteratedPlayer)
        && filter.nontoken
}

pub(super) fn describe_for_players_bend_or_break(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any {
        return None;
    }

    let (choose_opponent_effect, choose_pile_effect, destroy_effect, tap_tag_effect, tap_effect) =
        match for_players.effects.as_slice() {
            [choose_opponent, choose_pile, destroy, tap] => {
                (choose_opponent, choose_pile, destroy, None, tap)
            }
            [choose_opponent, choose_pile, destroy, tap_tag, tap] => {
                (choose_opponent, choose_pile, destroy, Some(tap_tag), tap)
            }
            _ => return None,
        };

    let choose_opponent =
        choose_opponent_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    if choose_opponent.chooser != PlayerFilter::IteratedPlayer
        || choose_opponent.filter != PlayerFilter::Opponent
        || choose_opponent.tag.as_str() != "divvy_opponent"
        || choose_opponent.random
    {
        return None;
    }

    let choose_pile = choose_pile_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_pile.tag.as_str() != "divvy_chosen"
        || choose_pile.chooser != PlayerFilter::TaggedPlayer(choose_opponent.tag.clone())
        || choose_primary_zone(choose_pile) != Some(Zone::Battlefield)
        || choose_pile.is_search
        || !choose_pile.count.is_any_number()
        || !is_iterated_player_nontoken_land_battlefield_filter(&choose_pile.filter)
    {
        return None;
    }

    let destroy = destroy_effect_for_choose_compaction(destroy_effect)?;
    if !matches!(&destroy.spec, ChooseSpec::Tagged(tag) if tag.as_str() == "divvy_chosen") {
        return None;
    }

    let tap = tap_effect.downcast_ref::<crate::effects::TapEffect>()?;
    let ChooseSpec::All(tap_filter) = &tap.target else {
        return None;
    };
    if let Some(tap_tag_effect) = tap_tag_effect {
        let tap_tag = tap_tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
        if tap_tag.filter != *tap_filter
            || tap_tag.zone.is_some()
            || !tap_tag.additional_zones.is_empty()
        {
            return None;
        }
    }
    if !is_iterated_player_nontoken_land_battlefield_filter(tap_filter)
        || !filter_excludes_chosen_tag(tap_filter, "divvy_chosen")
    {
        return None;
    }

    Some(
        "Each player separates all nontoken lands they control into two piles. For each player, one of their piles is chosen by one of their opponents of their choice. Destroy all lands in the chosen piles. Tap all lands in the other piles."
            .to_string(),
    )
}

pub(super) fn describe_for_players_may_choose_then_destroy_chosen(
    for_players: &crate::effects::ForPlayersEffect,
    destroy: &crate::effects::DestroyEffect,
) -> Option<String> {
    if for_players.effects.len() != 1 {
        return None;
    }
    let may = for_players.effects[0].downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.is_some() || may.effects.len() != 1 {
        return None;
    }
    let choose = may.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.is_search
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.chooser != PlayerFilter::IteratedPlayer
    {
        return None;
    }
    let ChooseSpec::All(destroy_filter) = &destroy.spec else {
        return None;
    };
    if !filter_uses_chosen_tag(destroy_filter, choose.tag.as_str()) {
        return None;
    }

    let subject = match for_players.filter {
        PlayerFilter::Any => "Each player",
        PlayerFilter::Opponent => "Each opponent",
        _ => return None,
    };
    let chosen = describe_choose_selection(choose);
    Some(format!(
        "{subject} may choose {chosen}. Destroy each permanent chosen this way"
    ))
}

pub(crate) fn describe_for_players_choose_types_then_sacrifice_rest(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let (tail, choose_effects) = for_players.effects.split_last()?;
    let sacrifice = sacrifice_view(tail)?;
    if sacrifice.player != &PlayerFilter::IteratedPlayer {
        return None;
    }
    let Value::Count(count_filter) = sacrifice.count else {
        return None;
    };
    if count_filter != sacrifice.filter {
        return None;
    }

    let mut chooses = Vec::new();
    for effect in choose_effects {
        let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        chooses.push(choose);
    }
    if chooses.len() < 2 {
        return None;
    }

    let keep_tag = chooses.first()?.tag.as_str().to_string();
    let has_sacrifice_keep_guard = sacrifice
        .filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                && constraint.tag.as_str() == keep_tag
        });
    if !has_sacrifice_keep_guard {
        return None;
    }

    let choose_has_common_keep_shape = |choose: &crate::effects::ChooseObjectsEffect| {
        choose_primary_zone(choose) == Some(Zone::Battlefield)
            && !choose.is_search
            && choose.chooser == PlayerFilter::IteratedPlayer
            && choose.tag.as_str() == keep_tag
            && choose.filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                    && constraint.tag.as_str() == keep_tag
            })
    };

    let party_roles = [
        crate::types::Subtype::Cleric,
        crate::types::Subtype::Rogue,
        crate::types::Subtype::Warrior,
        crate::types::Subtype::Wizard,
    ];
    let chosen_party_roles = chooses
        .iter()
        .filter_map(|choose| {
            (choose_has_common_keep_shape(choose)
                && choose.count == ChoiceCount::up_to(1)
                && choose.filter.card_types.as_slice() == [CardType::Creature]
                && choose.filter.subtypes.len() == 1)
                .then(|| choose.filter.subtypes[0])
        })
        .collect::<Vec<_>>();
    let sacrifice_is_party_complement = sacrifice.filter.card_types.as_slice()
        == [CardType::Creature]
        && sacrifice.filter.subtypes.is_empty()
        && sacrifice.filter.controller == Some(PlayerFilter::IteratedPlayer);
    if chooses.len() == party_roles.len()
        && chosen_party_roles.len() == party_roles.len()
        && party_roles
            .iter()
            .all(|role| chosen_party_roles.contains(role))
        && sacrifice_is_party_complement
    {
        return Some(match for_players.filter {
            PlayerFilter::Any => {
                "Each player chooses a party from among creatures they control, then sacrifices the rest"
                    .to_string()
            }
            PlayerFilter::Opponent => {
                "Each opponent chooses a party from among creatures they control, then sacrifices the rest"
                    .to_string()
            }
            PlayerFilter::You => {
                "You choose a party from among creatures you control, then sacrifice the rest"
                    .to_string()
            }
            _ => return None,
        });
    }

    let mut chosen_types = Vec::new();
    for choose in chooses {
        if !choose_has_common_keep_shape(choose) || !choose.count.is_single() {
            return None;
        }
        if choose.filter.card_types.len() != 1 {
            return None;
        }
        let card_type = *choose.filter.card_types.iter().next()?;
        let phrase = with_indefinite_article(describe_card_type_word_local(card_type));
        if !chosen_types.iter().any(|existing| existing == &phrase) {
            chosen_types.push(phrase);
        }
    }
    if chosen_types.len() < 2 {
        return None;
    }

    let list = join_with_and(&chosen_types);
    let (subject, choose_verb, sacrifice_verb, controls) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "chooses", "sacrifices", "they control"),
        PlayerFilter::Opponent => ("Each opponent", "chooses", "sacrifices", "they control"),
        PlayerFilter::You => ("You", "choose", "sacrifice", "you control"),
        _ => return None,
    };
    Some(format!(
        "{subject} {choose_verb} {list} from among permanents {controls}, then {sacrifice_verb} the rest"
    ))
}

pub(super) fn sacrifice_count_tracks_chosen_set(
    sacrifice: SacrificeView<'_>,
    choose: &crate::effects::ChooseObjectsEffect,
) -> bool {
    matches!(
        sacrifice.count,
        Value::Count(count_filter)
            if filter_uses_chosen_tag(count_filter, choose.tag.as_str())
    )
}

pub(super) fn describe_sacrifice_choice_kind(
    choose: &crate::effects::ChooseObjectsEffect,
) -> String {
    let mut filter = choose.filter.clone();
    if choose_primary_zone(choose) == Some(Zone::Battlefield) {
        filter.zone = None;
    }
    if filter.controller.as_ref() == Some(&choose.chooser)
        || filter.controller == Some(PlayerFilter::IteratedPlayer)
    {
        filter.controller = None;
    }

    strip_leading_article(&filter.description()).to_string()
}

pub(super) fn describe_counted_sacrifice_choice_selection(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<String> {
    let kind = describe_sacrifice_choice_kind(choose);
    let plural = pluralize_noun_phrase(&kind);

    if choose.count.is_any_number() {
        return Some(format!("any number of {plural}"));
    }
    if choose.count.is_dynamic_x() {
        let count = describe_runtime_choice_count(choose)
            .unwrap_or_else(|| describe_choice_count(&choose.count));
        return Some(format!("{count} {plural}"));
    }

    match (choose.count.min, choose.count.max) {
        (1, Some(1)) => Some(with_indefinite_article(&kind)),
        (0, Some(1)) => Some(format!("up to one {kind}")),
        (0, Some(max)) => {
            let count = number_word(max as i32).unwrap_or_else(|| max.to_string());
            Some(format!("up to {count} {plural}"))
        }
        (min, Some(max)) if min == max => {
            let count = number_word(max as i32).unwrap_or_else(|| max.to_string());
            Some(format!("{count} {plural}"))
        }
        (min, Some(max)) => Some(format!("{min} to {max} {plural}")),
        (min, None) => Some(format!("at least {min} {plural}")),
    }
}

pub(crate) fn describe_for_players_choose_then_sacrifice(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = sacrifice_view(&for_players.effects[1])?;
    if choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.is_search
        || choose.chooser != PlayerFilter::IteratedPlayer
        || sacrifice.player != &PlayerFilter::IteratedPlayer
        || !sacrifice_uses_chosen_tag(sacrifice.filter, choose.tag.as_str())
        || !(matches!(sacrifice.count, Value::Fixed(value) if choose_exact_count(choose) == Some(*value as usize))
            || sacrifice_count_tracks_chosen_set(sacrifice, choose))
    {
        return None;
    }

    let (subject, verb, possessive) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "sacrifices", "their"),
        PlayerFilter::Opponent => ("Each opponent", "sacrifices", "their"),
        PlayerFilter::You => ("You", "sacrifice", "your"),
        _ => return None,
    };
    if let Some(chosen) = describe_greatest_power_choice_filter(&choose.filter) {
        let chosen = with_indefinite_article(&chosen);
        return Some(format!("{subject} {verb} {chosen}"));
    }
    let chosen = describe_counted_sacrifice_choice_selection(choose)?;
    Some(format!("{subject} {verb} {chosen} of {possessive} choice"))
}

pub(super) fn describe_for_players_choose_then_exile(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if let Some(exile) = unwrap_basic_tag_wrappers(&for_players.effects[1])
        .downcast_ref::<crate::effects::ExileEffect>()
    {
        if choose_primary_zone(choose) == Some(Zone::Library)
            && choose.bottom_only
            && !choose.top_only
            && !choose.is_search
            && choose.count.is_single()
            && choose.chooser == PlayerFilter::IteratedPlayer
            && choose.filter.zone == Some(Zone::Library)
            && choose.filter.controller.is_none()
            && choose.filter.owner.is_none()
            && choose.filter.card_types.is_empty()
            && choose.filter.tagged_constraints.is_empty()
            && exile_uses_chosen_tag(&exile.spec, choose.tag.as_str())
        {
            let subject = match for_players.filter {
                PlayerFilter::Any => "each player's",
                PlayerFilter::Opponent => "each opponent's",
                PlayerFilter::You => "your",
                _ => return None,
            };
            let face_down = if exile.face_down { " face down" } else { "" };
            return Some(format!(
                "Exile the bottom card of {subject} library{face_down}"
            ));
        }
    }
    let move_to_zone = for_players.effects[1].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if choose_primary_zone(choose) == Some(Zone::Hand)
        && choose.additional_zones.contains(&Zone::Battlefield)
        && !choose.is_search
        && choose.count.is_single()
        && choose.chooser == PlayerFilter::IteratedPlayer
        && choose_filter_is_iterated_hand_card_or_permanent(choose)
        && move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str())
    {
        let (subject, object) = match for_players.filter {
            PlayerFilter::Any => (
                "Each player",
                "a card from their hand or a permanent they control",
            ),
            PlayerFilter::Opponent => (
                "Each opponent",
                "a card from their hand or a permanent they control",
            ),
            PlayerFilter::You => ("You", "a card from your hand or a permanent you control"),
            _ => return None,
        };
        let verb = if subject == "You" { "exile" } else { "exiles" };
        return Some(format!("{subject} {verb} {object}"));
    }
    if choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.is_search
        || !choose.count.is_single()
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || !move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str())
    {
        return None;
    }

    let (subject, choose_verb, exile_verb) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "chooses", "exiles"),
        PlayerFilter::Opponent => ("Each opponent", "chooses", "exiles"),
        PlayerFilter::You => ("You", "choose", "exile"),
        _ => return None,
    };
    let mut selected_filter = choose.filter.clone();
    selected_filter.zone = None;
    let selection = selected_filter
        .description()
        .replace("that player controls", "they control");
    Some(format!(
        "{subject} {choose_verb} {selection} and {exile_verb} it"
    ))
}

pub(super) fn describe_for_players_controls_no_lose_game(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 1 {
        return None;
    }
    let conditional = for_players.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let lose_game = conditional.if_true[0].downcast_ref::<crate::effects::LoseTheGameEffect>()?;
    if lose_game.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let Condition::Not(inner) = &conditional.condition else {
        return None;
    };
    let Condition::PlayerControls { player, filter } = inner.as_ref() else {
        return None;
    };
    if player != &PlayerFilter::IteratedPlayer
        || !filter.supertypes.contains(&Supertype::Legendary)
        || !filter.card_types.contains(&CardType::Creature)
        || !filter.card_types.contains(&CardType::Planeswalker)
    {
        return None;
    }
    match for_players.filter {
        PlayerFilter::Opponent => Some(
            "Each opponent who doesn't control a legendary creature or planeswalker loses the game"
                .to_string(),
        ),
        PlayerFilter::Any => Some(
            "Each player who doesn't control a legendary creature or planeswalker loses the game"
                .to_string(),
        ),
        _ => None,
    }
}

pub(super) fn describe_for_players_bottom_library_exile_then_look_cast(
    for_players: &crate::effects::ForPlayersEffect,
    look: &crate::effects::LookAtObjectsEffect,
    grant: &crate::effects::GrantPlayTaggedEffect,
) -> Option<String> {
    let exile_clause = describe_for_players_choose_then_exile(for_players)?;
    if !exile_clause.contains("the bottom card")
        || look.viewer != PlayerFilter::You
        || look.subject != PlayerFilter::You
        || grant.player != PlayerFilter::You
        || grant.duration != crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
        || grant.allow_land
        || !grant.allow_any_color_for_cast
    {
        return None;
    }
    let look_tag = look
        .filter
        .tagged_constraints
        .iter()
        .find_map(|constraint| {
            (constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
                .then_some(&constraint.tag)
        })?;
    if look.filter.zone != Some(Zone::Exile)
        || look
            .filter
            .controller
            .as_ref()
            .is_some_and(|controller| *controller != PlayerFilter::You)
        || look_tag != &grant.tag
    {
        return None;
    }
    let grant_filter = grant.filter.as_ref()?;
    let is_permanent_spell_filter = grant_filter.card_types
        == vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Planeswalker,
            CardType::Battle,
        ];
    if !is_permanent_spell_filter {
        return None;
    }

    Some(format!(
        "{exile_clause}. For as long as those cards remain exiled, you may look at them, you may cast permanent spells from among them, and you may spend mana as though it were mana of any color to cast those spells"
    ))
}

pub(super) fn describe_for_players_choose_then_move_to_battlefield(
    for_players: &crate::effects::ForPlayersEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    let subject = match for_players.filter {
        PlayerFilter::Any => "Each player",
        PlayerFilter::Opponent => "Each opponent",
        _ => return None,
    };
    if for_players.effects.len() != 1
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
    {
        return None;
    }
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let you_choose_for_each_player = choose.chooser == PlayerFilter::You
        && choose.filter.owner == Some(PlayerFilter::IteratedPlayer);
    if choose.is_search || !move_to_battlefield_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }
    if choose.chooser != PlayerFilter::IteratedPlayer && !you_choose_for_each_player {
        return None;
    }

    let primary_zone = choose_primary_zone(choose)?;
    if you_choose_for_each_player {
        let choice_location = match primary_zone {
            Zone::Graveyard => "in that player's graveyard".to_string(),
            _ => return None,
        };
        let chosen = describe_choose_selection(choose);
        let tapped = if move_to_zone.enters_tapped {
            " tapped"
        } else {
            ""
        };
        let attacking = if move_to_zone.enters_attacking {
            " and attacking"
        } else {
            ""
        };
        return Some(format!(
            "For each player, choose {chosen} {choice_location}. Put those cards onto the battlefield{tapped}{attacking} under your control"
        ));
    }

    let choice_location = match primary_zone {
        Zone::Hand => describe_choose_zone_origin(choose, "hand"),
        Zone::Graveyard => describe_choose_zone_location(choose, "graveyard"),
        Zone::Library => {
            if choose.top_only {
                match choose.filter.owner.as_ref() {
                    Some(PlayerFilter::IteratedPlayer) => {
                        "from the top of their library".to_string()
                    }
                    Some(owner) => format!(
                        "from the top of {} library",
                        describe_possessive_player_filter(owner)
                    ),
                    None => "from the top of a library".to_string(),
                }
            } else {
                describe_choose_zone_origin(choose, "library")
            }
        }
        _ => return None,
    };
    let chosen = describe_choose_selection(choose);
    let tapped = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let attacking = if move_to_zone.enters_attacking {
        " and attacking"
    } else {
        ""
    };
    Some(format!(
        "{subject} chooses {chosen} {choice_location}. Put those cards onto the battlefield{tapped}{attacking} under your control"
    ))
}

pub(super) fn apply_continuous_is_forever_tagged(
    apply: &crate::effects::ApplyContinuousEffect,
    tag: &crate::TagKey,
) -> bool {
    apply.until == Until::Forever
        && apply.condition.is_none()
        && apply.runtime_modifications.is_empty()
        && apply_continuous_targets_tag(apply, tag)
}

pub(super) fn apply_continuous_grants_decayed(
    apply: &crate::effects::ApplyContinuousEffect,
) -> bool {
    let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
        return false;
    };
    if ability.id() != crate::static_abilities::StaticAbilityId::KeywordMarker
        || !ability.display().eq_ignore_ascii_case("decayed")
    {
        return false;
    }

    apply.additional_modifications.iter().all(|modification| {
        matches!(
            modification,
            crate::continuous::Modification::AddAbility(ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::CantBlock
        ) || matches!(
            modification,
            crate::continuous::Modification::AddAbilityGeneric(_)
        )
    })
}

pub(super) fn describe_for_players_choose_move_then_characteristics(
    effects: &[&Effect],
) -> Option<String> {
    let [
        for_players_effect,
        move_effect,
        first_apply_effect,
        second_apply_effect,
        ability_effect,
    ] = effects
    else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let base = describe_for_players_choose_then_move_to_battlefield(for_players, move_to_zone)?;
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let tag = &choose.tag;

    let first_apply = tagged_apply_continuous_effect(first_apply_effect)?;
    let second_apply = tagged_apply_continuous_effect(second_apply_effect)?;
    let ability_apply = tagged_apply_continuous_effect(ability_effect)?;
    if !apply_continuous_is_forever_tagged(first_apply, tag)
        || !apply_continuous_is_forever_tagged(second_apply, tag)
        || !apply_continuous_is_forever_tagged(ability_apply, tag)
        || !first_apply.additional_modifications.is_empty()
        || !second_apply.additional_modifications.is_empty()
        || !apply_continuous_grants_decayed(ability_apply)
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

    let subtype_words = subtypes
        .iter()
        .map(|subtype| subtype.display_name())
        .collect::<Vec<_>>()
        .join(" ");
    let subtype_words = pluralize_noun_phrase(&subtype_words);
    let color_words = describe_token_color_words(colors, false);
    Some(format!(
        "{base}. They're {color_words} {subtype_words} in addition to their other colors and types and they gain decayed"
    ))
}

pub(super) fn describe_for_players_may_choose_then_move_to_battlefield(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let subject = match for_players.filter {
        PlayerFilter::Any => "Each player",
        PlayerFilter::Opponent => "Each opponent",
        _ => return None,
    };
    let [may_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [choose_effect, move_effect] = may.effects.as_slice() else {
        return None;
    };
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| *decider != PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let move_to_zone = unwrap_tag_wrapped_effect(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if choose.is_search
        || choose.chooser != PlayerFilter::IteratedPlayer
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || !move_to_battlefield_uses_chosen_tag(move_to_zone, choose.tag.as_str())
    {
        return None;
    }

    let mut choice = choose.filter.description();
    for location in [
        " in that player's hand",
        " in their hand",
        " in a hand",
        " in hand",
    ] {
        choice = choice.replace(location, "");
    }
    let choice = with_indefinite_article(strip_leading_article(&choice).trim());
    let tapped = if move_to_zone.enters_tapped {
        " tapped"
    } else {
        ""
    };
    Some(format!(
        "{subject} may put {choice} from their hand onto the battlefield{tapped}"
    ))
}

pub(crate) fn describe_for_players_split_piles_then_choose_sacrifice(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Opponent || for_players.effects.len() != 2 {
        return None;
    }

    let split = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    describe_split_pile_choice_effect(split, &for_players.effects[1])
}

pub(crate) fn describe_for_players_split_piles_then_choose_sacrifice_pair(
    split_for_players: &crate::effects::ForPlayersEffect,
    choice_for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if split_for_players.filter != PlayerFilter::Opponent
        || choice_for_players.filter != PlayerFilter::Opponent
        || split_for_players.effects.len() != 1
        || choice_for_players.effects.len() != 1
    {
        return None;
    }

    let split =
        split_for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    describe_split_pile_choice_effect(split, &choice_for_players.effects[0])
}

pub(crate) fn describe_for_players_split_piles_then_choose_restriction(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }

    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let cant = for_players.effects[1].downcast_ref::<crate::effects::CantEffect>()?;
    let (filter, sentence_text) = match &cant.restriction {
        crate::effect::Restriction::Attack(filter) => (
            filter,
            "Only creatures in the chosen piles can attack this turn.",
        ),
        crate::effect::Restriction::Block(filter) => (
            filter,
            "Only creatures in the chosen piles can block this turn.",
        ),
        _ => return None,
    };

    if choose.tag.as_str() != "divvy_chosen"
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.is_search
        || !choose.count.is_any_number()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !is_iterated_player_creature_battlefield_filter(&choose.filter)
        || cant.duration != crate::effect::Until::EndOfTurn
        || !is_iterated_player_creature_battlefield_filter(filter)
        || !filter_excludes_chosen_tag(filter, choose.tag.as_str())
    {
        return None;
    }

    let player_filter_text = describe_for_each_player_filter(&for_players.filter);
    let each_player = strip_leading_article(&player_filter_text);
    Some(format!(
        "For each {each_player}, separate all creatures that player controls into two piles and that player chooses one. {sentence_text}"
    ))
}

pub(crate) fn describe_split_piles_then_choose_attack_or_block_restriction(
    choose_effect: &Effect,
    cant_effect: &Effect,
) -> Option<String> {
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let (filter, verb) = match &cant.restriction {
        crate::effect::Restriction::Attack(filter) => (filter, "attack"),
        crate::effect::Restriction::Block(filter) => (filter, "block"),
        _ => return None,
    };

    if choose.tag.as_str() != "divvy_chosen"
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.is_search
        || !choose.count.is_any_number()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || !is_iterated_player_creature_battlefield_filter(&choose.filter)
        || cant.duration != crate::effect::Until::EndOfTurn
        || !is_iterated_player_creature_battlefield_filter(filter)
        || !filter_excludes_chosen_tag(filter, choose.tag.as_str())
    {
        return None;
    }

    Some(format!(
        "Separate all creatures that player controls into two piles. Only creatures in the pile of their choice can {verb} this turn"
    ))
}

pub(super) fn describe_split_pile_choice_effect(
    split: &crate::effects::ChooseObjectsEffect,
    pile_choice_effect: &Effect,
) -> Option<String> {
    if split.tag.as_str() != "divvy_pile"
        || split.chooser != PlayerFilter::IteratedPlayer
        || choose_primary_zone(split) != Some(Zone::Battlefield)
        || split.is_search
        || !split.count.is_any_number()
        || !is_iterated_player_creature_battlefield_filter(&split.filter)
    {
        return None;
    }

    let (main_sacrifice, alternative_sacrifice) = if let Some(pile_choice) =
        pile_choice_effect.downcast_ref::<crate::effects::ChooseModeEffect>()
    {
        if !matches!(pile_choice.choose_count, Value::Fixed(1))
            || !matches!(pile_choice.min_choose_count, Value::Fixed(1))
            || pile_choice.modes.len() != 2
            || pile_choice.modes[0].effects.len() != 1
            || pile_choice.modes[1].effects.len() != 1
        {
            return None;
        }
        (
            sacrifice_view(&pile_choice.modes[0].effects[0])?,
            sacrifice_view(&pile_choice.modes[1].effects[0])?,
        )
    } else {
        let pile_choice =
            pile_choice_effect.downcast_ref::<crate::effects::UnlessActionEffect>()?;
        if pile_choice.player != PlayerFilter::You
            || pile_choice.effects.len() != 1
            || pile_choice.alternative.len() != 1
        {
            return None;
        }
        (
            sacrifice_view(&pile_choice.effects[0])?,
            sacrifice_view(&pile_choice.alternative[0])?,
        )
    };
    let sacrifices_chosen_pile =
        sacrifice_uses_chosen_tag(main_sacrifice.filter, split.tag.as_str())
            && filter_excludes_chosen_tag(alternative_sacrifice.filter, split.tag.as_str());
    let sacrifices_other_pile =
        filter_excludes_chosen_tag(main_sacrifice.filter, split.tag.as_str())
            && sacrifice_uses_chosen_tag(alternative_sacrifice.filter, split.tag.as_str());
    if !sacrifices_chosen_pile && !sacrifices_other_pile {
        return None;
    }
    for sacrifice in [main_sacrifice, alternative_sacrifice] {
        if sacrifice.player != &PlayerFilter::IteratedPlayer
            || !matches!(sacrifice.count, Value::Count(count_filter) if count_filter == sacrifice.filter)
            || !is_iterated_player_creature_battlefield_filter(sacrifice.filter)
        {
            return None;
        }
    }

    Some(
        "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile."
            .to_string(),
    )
}

pub(crate) fn describe_choose_then_sacrifice(
    choose: &crate::effects::ChooseObjectsEffect,
    sacrifice: SacrificeView<'_>,
) -> Option<String> {
    let choose_is_any_number = choose.count.is_any_number();
    let choose_exact = if choose_is_any_number {
        None
    } else {
        choose.count.max.filter(|max| *max == choose.count.min)
    };
    let sacrifice_count = match sacrifice.count {
        Value::Fixed(value) if *value > 0 => Some(*value as usize),
        _ => None,
    };
    let sacrifice_any_number = matches!(
        sacrifice.count,
        Value::Count(count_filter) if count_filter == sacrifice.filter
    );
    if choose_primary_zone(choose).is_some_and(|zone| zone != Zone::Battlefield)
        || choose.is_search
        || sacrifice.player != &choose.chooser
        || !sacrifice_uses_chosen_tag(sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    let player = describe_player_filter(&choose.chooser);
    let verb = player_verb(&player, "sacrifice", "sacrifices");
    let refers_to_triggering_object = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && matches!(constraint.tag.as_str(), "triggering" | "damaged")
    });
    let refers_to_created_token = choose.filter.token
        && choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && (constraint.tag.as_str().starts_with("created_")
                    || crate::cards::is_sentence_helper_tag(constraint.tag.as_str(), "created"))
        });
    // "An opponent sacrifices a creature of their choice": the chooser
    // controlling the chosen object is implicit, so elide the controller.
    let chooser_controls_chosen = choose.filter.controller.as_ref() == Some(&choose.chooser)
        && choose.chooser != PlayerFilter::You;
    let chosen = if chooser_controls_chosen {
        let mut chosen_filter = choose.filter.clone();
        chosen_filter.controller = None;
        chosen_filter.description()
    } else if choose.chooser == PlayerFilter::You
        && choose.filter.controller == Some(PlayerFilter::You)
    {
        let mut chosen_filter = choose.filter.clone();
        chosen_filter.controller = None;
        if chosen_filter.zone == Some(Zone::Battlefield) {
            chosen_filter.zone = None;
        }
        chosen_filter.description()
    } else {
        choose.filter.description()
    };
    if sacrifice_count_tracks_chosen_set(sacrifice, choose) {
        let selection = describe_counted_sacrifice_choice_selection(choose)?;
        let choice_suffix =
            if sacrifice.player != &PlayerFilter::You && choose.chooser == *sacrifice.player {
                " of their choice"
            } else {
                ""
            };
        return Some(format!("{player} {verb} {selection}{choice_suffix}"));
    }
    if choose_is_any_number && sacrifice_any_number {
        let chosen = pluralize_noun_phrase(strip_leading_article(&chosen));
        return Some(format!("{player} {verb} any number of {chosen}"));
    }

    if choose_is_any_number {
        return None;
    }

    let sacrifice_count = sacrifice_count?;
    if choose_exact != Some(sacrifice_count) {
        return None;
    }

    if sacrifice_count == 1 {
        if refers_to_triggering_object {
            return Some(format!("{player} {verb} it"));
        }
        if refers_to_created_token {
            return Some(format!("{player} {verb} that token"));
        }
        if let Some(chosen) = describe_greatest_power_choice_filter(&choose.filter) {
            return Some(format!(
                "{player} {verb} {}",
                with_indefinite_article(&chosen)
            ));
        }
        if chooser_controls_chosen {
            let chosen_kind = with_indefinite_article(strip_leading_article(&chosen));
            return Some(format!("{player} {verb} {chosen_kind} of their choice"));
        }
        if let Some(rest) = chosen.strip_prefix(&format!("{player}'s ")) {
            let chosen_kind = with_indefinite_article(rest);
            return Some(format!("{player} {verb} {chosen_kind} of their choice"));
        }
        let chosen = with_indefinite_article(&chosen);
        Some(format!("{player} {verb} {chosen}"))
    } else {
        let count_text =
            number_word(sacrifice_count as i32).unwrap_or_else(|| sacrifice_count.to_string());
        let chosen = pluralize_noun_phrase(&chosen);
        Some(format!("{player} {verb} {count_text} {chosen}"))
    }
}

pub(super) fn describe_greatest_power_choice_filter(filter: &ObjectFilter) -> Option<String> {
    let Some(crate::filter::Comparison::EqualExpr(value)) = filter.power.as_ref() else {
        return None;
    };
    let Value::GreatestPower(among_filter) = value.as_ref() else {
        return None;
    };
    if filter.card_types != [CardType::Creature]
        || among_filter.card_types != [CardType::Creature]
        || filter.controller != among_filter.controller
    {
        return None;
    }
    let among = match filter.controller.as_ref() {
        Some(PlayerFilter::You) => "among creatures you control".to_string(),
        Some(PlayerFilter::Opponent) => "among creatures an opponent controls".to_string(),
        Some(PlayerFilter::NotYou) => "among creatures your opponents control".to_string(),
        Some(PlayerFilter::IteratedPlayer) => "among creatures that player controls".to_string(),
        Some(PlayerFilter::TaggedPlayer(_)) => "among creatures that player controls".to_string(),
        Some(controller) => format!(
            "among creatures {} controls",
            describe_player_filter(controller)
        ),
        None => "among creatures on the battlefield".to_string(),
    };
    Some(format!("creature with the greatest power {among}"))
}

pub(super) fn describe_sacrifice_effect(sacrifice: SacrificeView<'_>) -> String {
    let player = describe_player_filter(sacrifice.player);
    let verb = player_verb(&player, "sacrifice", "sacrifices");
    if let Value::Count(count_filter) = sacrifice.count
        && count_filter == sacrifice.filter
    {
        let mut noun = sacrifice.filter.description();
        if let Some(rest) = noun.strip_prefix("target player's ") {
            noun = rest.to_string();
        } else if let Some(rest) = noun.strip_prefix("that player's ") {
            noun = rest.to_string();
        } else if let Some(rest) = noun.strip_prefix("the active player's ") {
            noun = rest.to_string();
        }
        if let Some(rest) = noun.strip_prefix("a ") {
            noun = rest.to_string();
        } else if let Some(rest) = noun.strip_prefix("an ") {
            noun = rest.to_string();
        }
        let subject = pluralize_noun_phrase(&noun);
        if matches!(sacrifice.player, PlayerFilter::You) {
            return format!("Sacrifice all {subject}");
        }
        return format!("{player} {verb} all {subject}");
    }
    if matches!(sacrifice.count, Value::Fixed(value) if *value == 1) {
        let description = sacrifice.filter.description();
        if matches!(sacrifice.player, PlayerFilter::You)
            && filter_is_exactly_one_tagged_object(sacrifice.filter)
        {
            return "Sacrifice it".to_string();
        }
        if sacrifice.filter.token && filter_is_tagged_it(sacrifice.filter) {
            return format!("{player} {verb} that token");
        }
        if let Some(chosen) = describe_greatest_power_choice_filter(sacrifice.filter) {
            return format!("{player} {verb} {}", with_indefinite_article(&chosen));
        }
        // A non-you sacrificer controlling the sacrificed object is implicit;
        // elide the controller and keep the "of their choice" surface.
        if sacrifice.player != &PlayerFilter::You
            && sacrifice.filter.controller.as_ref() == Some(sacrifice.player)
        {
            let mut stripped = sacrifice.filter.clone();
            stripped.controller = None;
            return format!(
                "{player} {verb} {} of their choice",
                with_indefinite_article(strip_leading_article(&stripped.description()))
            );
        }
        if matches!(
            sacrifice.player,
            PlayerFilter::Any | PlayerFilter::Opponent | PlayerFilter::IteratedPlayer
        ) && let Some(rest) = description.strip_suffix(" that player controls")
        {
            return format!(
                "{player} {verb} {} of their choice",
                with_indefinite_article(rest)
            );
        }
        if let Some(rest) = description.strip_prefix("target player's ") {
            return format!("{player} {verb} {}", with_indefinite_article(rest));
        }
        if let Some(rest) = description.strip_prefix("that player's ") {
            return format!("{player} {verb} {}", with_indefinite_article(rest));
        }
        if let Some(rest) = description.strip_prefix("the active player's ") {
            return format!("{player} {verb} {}", with_indefinite_article(rest));
        }
    }
    if let Value::Fixed(value) = sacrifice.count
        && *value > 1
    {
        let mut noun = sacrifice.filter.description();
        if let Some(rest) = noun.strip_prefix("target player's ") {
            noun = rest.to_string();
        } else if let Some(rest) = noun.strip_prefix("that player's ") {
            noun = rest.to_string();
        } else if let Some(rest) = noun.strip_prefix("the active player's ") {
            noun = rest.to_string();
        }
        if let Some(rest) = noun.strip_prefix("a ") {
            noun = rest.to_string();
        } else if let Some(rest) = noun.strip_prefix("an ") {
            noun = rest.to_string();
        }
        let count_text = number_word(*value).unwrap_or_else(|| value.to_string());
        return format!(
            "{player} {verb} {count_text} {}",
            pluralize_noun_phrase(&noun)
        );
    }
    format!(
        "{} {} {} {}",
        player,
        verb,
        describe_object_count(sacrifice.count),
        sacrifice.filter.description()
    )
}

pub(crate) fn describe_choose_then_destroy(
    choose: &crate::effects::ChooseObjectsEffect,
    destroy: &crate::effects::DestroyEffect,
) -> Option<String> {
    if choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.is_search
        || !choose.count.is_single()
    {
        return None;
    }
    let destroys_chosen = match &destroy.spec {
        ChooseSpec::Tagged(tag) => tag.as_str() == choose.tag.as_str(),
        ChooseSpec::Iterated => true,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter_uses_chosen_tag(filter, choose.tag.as_str())
        }
        _ => false,
    };
    if !destroys_chosen {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let description = choose.filter.description();
    let chosen = if choose.filter.controller == Some(PlayerFilter::IteratedPlayer)
        && choose.filter.card_types == vec![CardType::Creature]
        && choose_primary_zone(choose) == Some(Zone::Battlefield)
    {
        "a creature they control".to_string()
    } else if let Some(rest) = description.strip_prefix("target player's ") {
        format!("a {} they control", rest)
    } else if let Some(rest) = description.strip_prefix("that player's ") {
        format!("a {} they control", rest)
    } else {
        with_indefinite_article(&description)
    };
    let destroyed = if chosen.contains("creature") {
        "that creature"
    } else {
        "it"
    };
    Some(format!(
        "{} {choose_verb} {chosen}. Destroy {destroyed}",
        capitalize_first(&chooser)
    ))
}

pub(crate) fn describe_choose_then_for_each_copy(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if choose.is_search || choose.count.is_single() {
        return None;
    }
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }
    let create_copy =
        for_each.effects[0].downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    if !matches!(create_copy.target, ChooseSpec::Iterated)
        && !matches!(create_copy.target, ChooseSpec::Tagged(ref tag) if tag == &choose.tag)
    {
        return None;
    }
    if create_copy.controller != PlayerFilter::You
        || create_copy.enters_tapped
        || create_copy.has_haste
        || create_copy.enters_attacking
        || create_copy.attack_target_mode.is_some()
        || create_copy.exile_at_end_of_combat
        || create_copy.sacrifice_at_next_end_step
        || create_copy.exile_at_next_end_step
        || create_copy.pt_adjustment.is_some()
        || !create_copy.added_card_types.is_empty()
        || !create_copy.added_subtypes.is_empty()
        || !create_copy.removed_supertypes.is_empty()
        || create_copy.set_base_power_toughness.is_some()
        || create_copy.set_colors.is_some()
        || create_copy.set_card_types.is_some()
        || create_copy.set_subtypes.is_some()
        || !create_copy.granted_static_abilities.is_empty()
    {
        return None;
    }

    let selected = if choose.count.min == 0
        && choose.count.max.is_none()
        && !choose.count.dynamic_x
        && !choose.count.up_to_x
        && !choose.count.random
        && choose.filter.token
        && choose.filter.distinct_names
        && choose.filter.controller == Some(PlayerFilter::You)
        && choose.filter.card_types.len() == 2
        && choose.filter.card_types.contains(&CardType::Artifact)
        && choose.filter.card_types.contains(&CardType::Creature)
    {
        "any number of artifact tokens and/or creature tokens you control with different names"
            .to_string()
    } else {
        describe_choose_spec(&ChooseSpec::Object(choose.filter.clone()).with_count(choose.count))
    };
    let copy_action = match create_copy.count {
        Value::Fixed(1) => "create a token that's a copy of it".to_string(),
        _ => format!(
            "create {} tokens that are copies of it",
            describe_value(&create_copy.count)
        ),
    };
    Some(format!(
        "Choose {selected}. For each of them, {copy_action}"
    ))
}

pub(crate) fn describe_choose_then_cant_pile_restriction(
    choose: &crate::effects::ChooseObjectsEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let (filter, restriction_text) = match &cant.restriction {
        crate::effect::Restriction::Attack(filter) => (filter, "can't attack this turn"),
        crate::effect::Restriction::Block(filter) => (filter, "can't block this turn"),
        _ => return None,
    };
    if cant.duration != crate::effect::Until::EndOfTurn {
        return None;
    }
    if choose_primary_zone(choose) != Some(Zone::Battlefield) || choose.is_search {
        return None;
    }
    let references_choose_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == choose.tag.as_str()
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    | crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            )
    });
    if !references_choose_tag {
        return None;
    }

    let choose_desc = choose.filter.description();
    let base = strip_leading_article(&choose_desc);
    let plural = pluralize_noun_phrase(base);
    let count_text = |n: usize| number_word(n as i32).unwrap_or_else(|| n.to_string());
    let sentence = if choose.count.is_up_to_dynamic_x() {
        format!("Up to X target {plural} {restriction_text}")
    } else if choose.count.is_dynamic_x() {
        format!("X target {plural} {restriction_text}")
    } else {
        match (choose.count.min, choose.count.max) {
            (0, Some(max)) => {
                if max == 1 {
                    format!("Up to one target {base} {restriction_text}")
                } else {
                    format!(
                        "Up to {} target {plural} {restriction_text}",
                        count_text(max),
                    )
                }
            }
            (min, Some(max)) if min == max => {
                if min == 1 {
                    format!("Target {base} {restriction_text}")
                } else {
                    format!("{} target {plural} {restriction_text}", count_text(min))
                }
            }
            (0, None) => format!("Any number of target {plural} {restriction_text}"),
            (min, None) => format!("At least {min} target {plural} {restriction_text}"),
            (min, Some(max)) => format!("{min} to {max} target {plural} {restriction_text}"),
        }
    };
    Some(sentence)
}

pub(crate) fn describe_additional_combat_then_chosen_attack_or_block_restriction(
    additional_phases: &crate::effects::AdditionalPhasesEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    if additional_phases.phases != [crate::effects::AdditionalPhase::Combat]
        || cant.duration != crate::effect::Until::EndOfCombat
    {
        return None;
    }

    let (filter, verb) = match &cant.restriction {
        crate::effect::Restriction::Attack(filter) => (filter, "attack"),
        crate::effect::Restriction::Block(filter) => (filter, "block"),
        _ => return None,
    };
    let references_chosen_tag = filter.tagged_constraints.iter().all(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    }) && filter.tagged_constraints.iter().any(|constraint| {
        let tag = constraint.tag.as_str();
        matches!(tag, "__it__" | "it")
            || tag.starts_with("targeted_")
            || tag.starts_with("untapped_")
            || tag.starts_with("counters_")
    });
    if !references_chosen_tag {
        return None;
    }
    let mut base_filter = filter.clone();
    base_filter.tagged_constraints.clear();
    if base_filter != ObjectFilter::creature() {
        return None;
    }

    Some(format!(
        "After this main phase, there is an additional combat phase. Only the chosen creatures can {verb} during that combat phase"
    ))
}

pub(crate) fn describe_for_each_chosen_put_counters_then_gain_keywords(
    for_each: &crate::effects::ForEachObject,
    grant_effect: &Effect,
) -> Option<String> {
    let tag = for_each
        .filter
        .tagged_constraints
        .iter()
        .find_map(|constraint| {
            (constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
                .then_some(&constraint.tag)
        })?;
    let [put_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let put = put_effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if !matches!(put.target, ChooseSpec::Iterated) || put.target_count.is_some() || put.distributed
    {
        return None;
    }

    let apply = grant_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .and_then(|tagged| {
            tagged
                .effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        })
        .or_else(|| grant_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())?;
    if apply.until != crate::effect::Until::EndOfTurn
        || apply.condition.is_some()
        || !apply.runtime_modifications.is_empty()
        || !matches!(
            apply.target_spec.as_ref(),
            Some(ChooseSpec::Tagged(found))
                if found == tag || found.as_str().starts_with("counters_")
        )
    {
        return None;
    }

    fn keyword_label(ability: &crate::static_abilities::StaticAbility) -> Option<String> {
        Some(
            match ability.id() {
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
            }
            .to_string(),
        )
    }

    let mut keywords = Vec::new();
    let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
        return None;
    };
    keywords.push(keyword_label(ability)?);
    for modification in &apply.additional_modifications {
        let crate::continuous::Modification::AddAbility(ability) = modification else {
            return None;
        };
        keywords.push(keyword_label(ability)?);
    }
    if keywords.is_empty() {
        return None;
    }

    Some(format!(
        "Put {} on each of them. They gain {} until end of turn",
        describe_put_counter_phrase(&put.amount, put.counter_type),
        join_with_and(&keywords),
    ))
}

pub(crate) fn describe_tagged_target_then_cant_restriction(
    tagged: &crate::effects::TaggedEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if let crate::effect::Restriction::BlockSpecificAttacker { blockers, attacker } =
        &cant.restriction
        && attacker.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == tagged.tag.as_str()
        })
    {
        if cant.duration != crate::effect::Until::EndOfTurn {
            return None;
        }
        let subject = capitalize_first(&describe_choose_spec(&target_only.target));
        if let Some(allowed) = describe_except_by_subtype_blockers(blockers) {
            return Some(format!(
                "{subject} can't be blocked this turn except by {allowed}"
            ));
        }
        let blockers = pluralize_noun_phrase(strip_leading_article(&blockers.description()));
        return Some(format!(
            "{subject} can't be blocked by {blockers} this turn"
        ));
    }
    if let crate::effect::Restriction::BeCountered(filter) = &cant.restriction {
        if !matches!(
            cant.duration,
            crate::effect::Until::Forever | crate::effect::Until::EndOfTurn
        ) {
            return None;
        }
        if !filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == tagged.tag.as_str()
        }) {
            return None;
        }

        let subject = capitalize_first(&describe_choose_spec(&target_only.target));
        let suffix = if cant.duration == crate::effect::Until::EndOfTurn {
            " this turn"
        } else {
            ""
        };
        return Some(format!("{subject} can't be countered{suffix}"));
    }
    if cant.duration != crate::effect::Until::EndOfTurn {
        return None;
    }
    let (filter, restriction_text) = match &cant.restriction {
        crate::effect::Restriction::Block(filter) => (filter, "can't block this turn"),
        crate::effect::Restriction::BeBlocked(filter) => (filter, "can't be blocked this turn"),
        crate::effect::Restriction::MustBeBlocked(filter) => {
            (filter, "must be blocked this turn if able")
        }
        _ => return None,
    };
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tagged.tag.as_str()
    }) {
        return None;
    }

    let subject = capitalize_first(&describe_choose_spec(&target_only.target));
    Some(format!("{subject} {restriction_text}"))
}

pub(super) fn describe_except_by_subtype_blockers(blockers: &ObjectFilter) -> Option<String> {
    if blockers.excluded_subtypes.is_empty() {
        return None;
    }

    let mut expected = ObjectFilter::creature();
    for subtype in &blockers.excluded_subtypes {
        expected = expected.without_subtype(*subtype);
    }
    if *blockers != expected {
        return None;
    }

    let allowed = blockers
        .excluded_subtypes
        .iter()
        .map(|subtype| pluralize_noun_phrase(&subtype.to_string()))
        .collect::<Vec<_>>();
    Some(join_with_and(&allowed))
}

pub(crate) fn describe_damage_then_self_skip_next_untap(
    deal: &crate::effects::DealDamageEffect,
    tagged: &crate::effects::TaggedEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let ChooseSpec::Tagged(tag) = target_only.target.base() else {
        return None;
    };
    if tag.as_str() != "triggering" {
        return None;
    }
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != crate::effect::Until::ControllersNextUntapStep {
        return None;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == "triggering"
    }) {
        return None;
    }

    let target_text = describe_choose_spec(&deal.target);
    Some(format!(
        "This creature deals {} damage to {target_text} and doesn't untap during your next untap step",
        describe_value(&deal.amount)
    ))
}

pub(crate) fn describe_damage_then_source_skip_next_untap(
    deal: &crate::effects::DealDamageEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != crate::effect::Until::ControllersNextUntapStep || !filter.source {
        return None;
    }

    let target_text = describe_choose_spec(&deal.target);
    Some(format!(
        "This creature deals {} damage to {target_text} and doesn't untap during your next untap step",
        describe_value(&deal.amount)
    ))
}

pub(crate) fn tap_uses_chosen_tag(spec: &ChooseSpec, tag: &str) -> bool {
    matches!(spec.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

pub(crate) fn describe_choose_then_tap_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    tap: &crate::effects::TapEffect,
) -> Option<String> {
    if choose_primary_zone(choose) != Some(Zone::Battlefield) || choose.is_search {
        return None;
    }
    if !tap_uses_chosen_tag(&tap.target, choose.tag.as_str()) {
        return None;
    }

    if choose.count.is_single() {
        return Some(format!(
            "Tap {}",
            with_indefinite_article(&choose.filter.description())
        ));
    }

    let exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    let count_text = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
    Some(format!(
        "Tap {} {}",
        count_text,
        pluralize_noun_phrase(&choose.filter.description())
    ))
}

pub(crate) fn exile_uses_chosen_tag(spec: &ChooseSpec, tag: &str) -> bool {
    matches!(spec.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

pub(crate) fn move_to_exile_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    move_to_zone.zone == Zone::Exile
        // Some parser lowerings route the chosen object through a tagged
        // for-each wrapper and leave the move target as `Iterated`.
        && match move_to_zone.target.base() {
            ChooseSpec::Iterated => true,
            ChooseSpec::Tagged(t) => t.as_str() == tag,
            _ => false,
        }
}

pub(crate) fn describe_for_each_filter(filter: &ObjectFilter) -> String {
    let mut base_filter = filter.clone();
    base_filter.controller = None;

    let description = base_filter.description();
    let mut base = strip_indefinite_article(&description).to_string();
    let has_sacrificed_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && matches!(
                tag_action_from_name(constraint.tag.as_str()),
                Some("sacrificed")
            )
    });
    if let Some(rest) = base.strip_prefix("another ") {
        base = format!("other {rest}");
    }
    if let Some(rest) = base.strip_prefix("permanent ")
        && matches!(filter.zone, None | Some(Zone::Battlefield))
        && !filter.chosen_creature_type
        && !filter.chosen_card_type
    {
        if filter.controller.is_some() {
            base = rest.to_string();
        } else {
            base = format!("{rest} on the battlefield");
        }
    }
    if let Some(action) = describe_tagged_this_way_action(filter) {
        if action == "exiled" {
            if let Some(head) = base.strip_suffix(" in exile") {
                base = head.trim().to_string();
            } else if let Some((head, tail)) = base.split_once(" in exile ") {
                base = format!("{} {}", head.trim(), tail.trim());
            }
        } else if action == "revealed" {
            if let Some(head) = base.strip_suffix(" permanent") {
                base = format!("{} card", head.trim());
            } else if let Some(head) = base.strip_suffix(" permanents") {
                base = format!("{} cards", head.trim());
            }
        }
        base = format!("{base} {action} this way");
    }
    if has_sacrificed_tag && !base.to_ascii_lowercase().starts_with("the sacrificed ") {
        base = format!("the sacrificed {}", base.trim_start_matches("the ").trim());
    }

    if let Some(controller) = &filter.controller {
        if matches!(controller, PlayerFilter::You) {
            return format!("{base} you control");
        }
        return format!("{base} {} controls", describe_player_filter(controller));
    }
    base
}
