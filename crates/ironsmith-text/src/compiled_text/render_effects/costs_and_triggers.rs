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
    let enter_verb = match zone_trigger.this_object_subject_number {
        crate::triggers::zone_changes::TriggerSubjectNumber::Singular => "enters",
        crate::triggers::zone_changes::TriggerSubjectNumber::Plural => "enter",
    };
    if triggered_has_you_difference_draw(triggered) {
        return line;
    }

    let Some((start, prefix_len)) = [
        format!("When {surface} {enter_verb},"),
        format!("When {surface} {enter_verb} the battlefield,"),
    ]
    .into_iter()
    .find_map(|prefix| {
        line.find(prefix.as_str())
            .filter(|start| *start == 0 || line[..*start].ends_with(": "))
            .map(|start| (start, prefix.len()))
    }) else {
        for generic_subject in [subject, "this creature", "this permanent", "this artifact"] {
            let generic_prefix = format!("When {generic_subject} {enter_verb},");
            if let Some(start) = line
                .find(generic_prefix.as_str())
                .filter(|start| *start == 0 || line[..*start].ends_with(": "))
            {
                let rest = &line[start + generic_prefix.len()..];
                return format!("{}When {surface} {enter_verb},{rest}", &line[..start]);
            }
        }
        return line;
    };
    let rest = &line[start + prefix_len..];
    if surface.contains('/') {
        return format!("{}When {surface} {enter_verb},{rest}", &line[..start]);
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
    format!("{}When {surface} {enter_verb},{rest}", &line[..start])
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
    if triggered.trigger.saga_chapters().is_some()
        && let Some(label) = presentation_label.display_prefix()
    {
        let label = label.trim();
        if label.is_empty() {
            return line;
        }
        if let Some((chapters, rest)) = line.split_once(" — ") {
            if rest == label || rest.starts_with(&format!("{label} — ")) {
                return line;
            }
            return format!("{chapters} — {label} — {rest}");
        }
        return format!("{line} — {label}");
    }
    let line = if let PresentationLabel::AbilityWord(label) = presentation_label {
        let marker = format!("{} — ", label.trim());
        let lower = line.to_ascii_lowercase();
        let marker_lower = marker.to_ascii_lowercase();
        // The trigger surface can receive the ability word before its
        // resolution text is appended. If a structural resolution renderer
        // also knows the same ability word, retain the authored leading copy
        // and remove the later duplicate.
        let search_start = if lower.starts_with(&marker_lower) {
            marker.len()
        } else {
            0
        };
        if let Some(relative_index) = lower[search_start..].find(&marker_lower) {
            let index = search_start + relative_index;
            let prefix = &line[..index];
            let suffix = &line[index + marker.len()..];
            let suffix = if prefix.ends_with(", ") {
                lowercase_first(suffix)
            } else {
                suffix.to_string()
            };
            format!("{prefix}{suffix}")
        } else {
            line
        }
    } else {
        line
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
        PresentationLabel::Keyword(PresentationKeyword::Firebending(amount)) => {
            format!("Firebending {amount}")
        }
        PresentationLabel::AbilityWord(label)
            if label
                .trim()
                .strip_prefix(
                    ironsmith_core::static_ability_model::STATION_THRESHOLD_STATIC_LABEL_PREFIX,
                )
                .is_some_and(|threshold| threshold.parse::<i32>().is_ok()) =>
        {
            let threshold = label
                .trim()
                .strip_prefix(
                    ironsmith_core::static_ability_model::STATION_THRESHOLD_STATIC_LABEL_PREFIX,
                )
                .expect("station presentation prefix checked above");
            format!("{threshold}+ | {line}")
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

pub(super) fn deduplicate_triggered_presentation_label(
    triggered: &crate::ability::TriggeredAbility,
    line: String,
) -> String {
    let Some(PresentationLabel::AbilityWord(label)) = triggered.presentation_label.as_ref() else {
        return line;
    };
    let marker = format!("{} — ", label.trim());
    let marker_lower = marker.to_ascii_lowercase();
    let lower = line.to_ascii_lowercase();
    let Some(first) = lower.find(&marker_lower) else {
        return line;
    };
    let after_first = first + marker.len();
    let Some(relative_second) = lower[after_first..].find(&marker_lower) else {
        return line;
    };
    let second = after_first + relative_second;
    let prefix = &line[..second];
    let suffix = &line[second + marker.len()..];
    let suffix = if prefix.ends_with(", ") {
        lowercase_first(suffix)
    } else {
        suffix.to_string()
    };
    format!("{prefix}{suffix}")
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
    if filter.token && !filter.nontoken {
        return "this token";
    }
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
        _ if card_types.is_empty()
            && !filter.subtypes.is_empty()
            && filter
                .subtypes
                .iter()
                .all(|subtype| subtype.is_creature_type()) =>
        {
            "this creature"
        }
        _ => "this permanent",
    }
}

#[cfg(test)]
#[test]
fn creature_subtype_only_grant_uses_a_creature_self_subject() {
    let time_lord = ObjectFilter::default().with_subtype(Subtype::TimeLord);
    assert_eq!(
        granted_ability_self_subject_for_filter(&time_lord),
        "this creature"
    );

    let aura = ObjectFilter::default().with_subtype(Subtype::Aura);
    assert_eq!(
        granted_ability_self_subject_for_filter(&aura),
        "this permanent",
        "a noncreature subtype must not inherit the creature death surface"
    );
}

pub(crate) fn granted_ability_self_subject_for_choose_spec(spec: &ChooseSpec) -> &'static str {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            granted_ability_self_subject_for_filter(filter)
        }
        ChooseSpec::Tagged(tag) if tag.as_str().contains("copied") => "this permanent",
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
        // Result IDs and affected-object tags make a discard usable by later
        // effects, but they do not change the authored cost surface.
        let transparent_effect = structural_unwrap_render_wrappers(effect);
        if let Some(discard) = transparent_effect.downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(text) = describe_simple_discard_cost(discard)
        {
            return text;
        }
        if let Some(sacrifice) = sacrifice_view(transparent_effect)
            && sacrifice.player == &PlayerFilter::You
        {
            if let Value::Fixed(1) = sacrifice.count {
                let mut filter = sacrifice.filter.clone();
                if filter.controller == Some(PlayerFilter::You) {
                    filter.controller = None;
                }
                let subject = with_indefinite_article(strip_leading_article(&filter.description()));
                return format!("Sacrifice {subject}");
            }
            return normalize_cost_phrase(&describe_sacrifice_effect(sacrifice));
        }
        if let Some(remove) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>()
            && matches!(remove.target.base(), ChooseSpec::Source)
        {
            return format!(
                "Remove {} from this source",
                describe_put_counter_phrase(&remove.count, remove.counter_type)
            );
        }
        if let Some(compact) = describe_effect_cost_program(effect) {
            return compact;
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

fn describe_effect_cost_program(effect: &Effect) -> Option<String> {
    let sequence = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if !matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Sequential | ironsmith_core::SequenceSurface::Coordinated
    ) {
        return None;
    }

    let mut compacted_choice = false;
    let mut parts = Vec::new();
    let mut index = 0usize;
    while index < sequence.effects.len() {
        let member = structural_unwrap_render_wrappers(&sequence.effects[index]);
        if let Some(choose) = member.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && choose_exact_count(choose) == Some(1)
            && choose.count_value.is_none()
            && choose.aggregate_constraint.is_none()
            && choose.chooser == PlayerFilter::You
            && choose.filter.untapped
            && let Some(next) = sequence.effects.get(index + 1)
            && let Some(tap) =
                structural_unwrap_render_wrappers(next).downcast_ref::<crate::effects::TapEffect>()
            && choose_spec_references_exact_tag(&tap.target, &choose.tag)
        {
            parts.push(format!("Tap {}", describe_choose_selection(choose)));
            compacted_choice = true;
            index += 2;
            continue;
        }
        if let Some(choose) = member.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(next) = sequence.effects.get(index + 1)
            && let Some(sacrifice) = sacrifice_view(structural_unwrap_render_wrappers(next))
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(normalize_cost_phrase(&compact));
            compacted_choice = true;
            index += 2;
            continue;
        }
        if let Some(choose) = member.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(next) = sequence.effects.get(index + 1)
            && let Some(exile) = structural_unwrap_render_wrappers(next)
                .downcast_ref::<crate::effects::ExileEffect>()
            && let Some(compact) = describe_choose_then_exile(choose, exile)
        {
            let compact = compact
                .replace("instants or sorcery cards", "instant and/or sorcery cards")
                .replace("instant or sorcery cards", "instant and/or sorcery cards");
            parts.push(normalize_cost_phrase(&compact));
            compacted_choice = true;
            index += 2;
            continue;
        }
        if let Some(choose) = member.downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(next) = sequence.effects.get(index + 1)
            && let Some(return_to_hand) = structural_unwrap_render_wrappers(next)
                .downcast_ref::<crate::effects::ReturnToHandEffect>()
            && let Some(compact) = describe_choose_then_return_to_hand_cost(choose, return_to_hand)
        {
            parts.push(normalize_cost_phrase(&compact));
            compacted_choice = true;
            index += 2;
            continue;
        }
        if member
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .is_some()
            || member.0.as_cost_executable().is_none()
        {
            return None;
        }
        if let Some(discard) = member.downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(text) = describe_simple_discard_cost(discard)
        {
            parts.push(text);
        } else {
            parts.push(normalize_cost_phrase(&describe_effect(member)));
        }
        index += 1;
    }
    compacted_choice.then(|| {
        for part in parts.iter_mut().skip(1) {
            *part = lowercase_first(part);
        }
        join_with_and(&parts)
    })
}

#[cfg(test)]
#[test]
fn coordinated_effect_cost_hides_choose_sacrifice_scaffolding() {
    let tag = crate::TagKey::from("sacrificed_0");
    let effect = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
        Effect::discard(1),
        Effect::new(crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::creature()
                .you_control()
                .in_zone(Zone::Battlefield),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            tag.clone(),
        )),
        Effect::sacrifice_player(ObjectFilter::tagged(tag), 1, PlayerFilter::You),
    ]));

    assert_eq!(
        describe_effect_cost_program(&effect).as_deref(),
        Some("Discard a card and sacrifice a creature")
    );
}

#[cfg(test)]
#[test]
fn morph_return_cost_hides_choice_scaffolding_and_requires_the_same_tag() {
    const LINE: &str = "Morph—Return a Bird you control to its owner's hand.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Raven Guild Initiate")
            .card_types(vec![CardType::Creature])
            .parse_text(LINE)
            .expect("nonmana morph cost should parse");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [LINE]
    );

    let tag = TagKey::from("return_cost_0");
    let choose = Effect::new(crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::default()
            .with_subtype(crate::types::Subtype::Bird)
            .you_control()
            .in_zone(Zone::Battlefield),
        ChoiceCount::exactly(1),
        PlayerFilter::You,
        tag.clone(),
    ));
    let cost_program = |return_tag: TagKey| {
        Effect::new(crate::effects::SequenceEffect::new(vec![
            choose.clone(),
            Effect::new(crate::effects::ReturnToHandEffect::with_spec(
                ChooseSpec::Tagged(return_tag),
            )),
        ]))
    };
    assert_eq!(
        describe_effect_cost_program(&cost_program(tag)).as_deref(),
        Some("Return a Bird you control to its owner's hand")
    );
    assert!(describe_effect_cost_program(&cost_program(TagKey::from("other"))).is_none());
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
    let costs = activated.mana_cost.as_all()?;
    describe_loyalty_activation_prefix(costs)
        .or_else(|| (activated.is_loyalty_ability() && costs.is_empty()).then(|| "0".to_string()))
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
    if discard.random || discard.player != PlayerFilter::You {
        return None;
    }
    // Tags on activation-cost discards only preserve the discarded objects for
    // possible follow-up references; they do not change the authored cost.
    let Value::Fixed(count) = discard.count else {
        return None;
    };
    let count = count.max(0) as u32;
    if count == 1
        && let Some(filter) = discard.card_filter.as_ref()
        && filter.colors.is_some()
        && let Some(filter_text) = describe_simple_hand_card_filter(filter)
    {
        return Some(format!("Discard {filter_text}"));
    }
    let (card_type, subtype, supertypes, name_filter, other_filter) = match &discard.card_filter {
        None => (None, None, Vec::new(), None, false),
        Some(filter) if !filter.any_of.is_empty() => {
            if let Some(filter_text) = describe_discard_any_of_filter(filter) {
                return Some(if count == 1 {
                    format!("Discard {filter_text}")
                } else {
                    let count = number_word(count as i32).unwrap_or_else(|| count.to_string());
                    format!("Discard {count} {filter_text}s")
                });
            }
            return None;
        }
        Some(filter) if filter.card_types.len() <= 1 => {
            let expected = ObjectFilter {
                zone: Some(Zone::Hand),
                card_types: filter.card_types.clone(),
                subtypes: filter.subtypes.clone(),
                supertypes: filter.supertypes.clone(),
                name: filter.name.clone(),
                other: filter.other,
                ..Default::default()
            };
            if filter != &expected || filter.subtypes.len() > 1 {
                return None;
            }
            (
                filter.card_types.first().copied(),
                filter.subtypes.first().copied(),
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

    if supertypes.is_empty() && card_type.is_none() && subtype.is_none() {
        return Some(if count == 1 {
            "Discard a card".to_string()
        } else {
            let count = number_word(count as i32).unwrap_or_else(|| count.to_string());
            format!("Discard {count} cards")
        });
    }

    let mut descriptors: Vec<String> = supertypes
        .iter()
        .map(|supertype| supertype.name().to_string())
        .collect();
    if let Some(subtype) = subtype {
        descriptors.push(subtype.display_name());
    }
    if let Some(card_type) = card_type {
        descriptors.push(describe_card_type_word_local(card_type).to_string());
    }
    let type_text = with_indefinite_article(&format!("{} card", descriptors.join(" ")));
    Some(if count == 1 {
        format!("Discard {type_text}")
    } else {
        let count = number_word(count as i32).unwrap_or_else(|| count.to_string());
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
    if !matches!(filter.owner.as_ref(), None | Some(PlayerFilter::You)) {
        return None;
    }
    let mut expected = ObjectFilter {
        zone: filter.zone,
        owner: filter.owner.clone(),
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
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
    {
        let color_names = crate::color::Color::ALL
            .into_iter()
            .filter(|color| colors.contains(*color))
            .map(|color| color.name().to_string())
            .collect::<Vec<_>>();
        if color_names.is_empty() {
            return None;
        }
        return Some(with_indefinite_article(&format!(
            "{} card",
            join_with_or(&color_names)
        )));
    }
    None
}

#[cfg(test)]
mod colored_discard_cost_tests {
    use super::*;

    fn red_or_green_hand_card(owner: PlayerFilter) -> ObjectFilter {
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(owner)
            .with_colors(crate::color::ColorSet::RED.union(crate::color::ColorSet::GREEN))
    }

    #[test]
    fn transparent_result_id_preserves_the_exact_color_choice() {
        let discard = Effect::new(crate::effects::DiscardEffect::new_with_filter(
            Value::Fixed(1),
            PlayerFilter::You,
            false,
            Some(red_or_green_hand_card(PlayerFilter::You)),
        ));
        let cost = crate::costs::Cost::try_from_runtime_effect(Effect::with_id(7, discard))
            .expect("discard should be a valid cost effect");

        assert_eq!(
            describe_cost_component(&cost),
            "Discard a red or green card"
        );
    }

    #[test]
    fn color_surface_rejects_a_different_owner_or_extra_type_constraint() {
        assert_eq!(
            describe_simple_hand_card_filter(&red_or_green_hand_card(PlayerFilter::Opponent)),
            None
        );
        assert_eq!(
            describe_simple_hand_card_filter(
                &red_or_green_hand_card(PlayerFilter::You).with_type(CardType::Creature)
            ),
            None
        );
    }
}

pub(super) fn is_grandeur_activation_cost(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.mana_cost.as_all().is_some_and(|costs| {
        costs.iter().any(|cost| {
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
    describe_dynamic_mana_cost_with_target(dynamic, None)
}

fn describe_dynamic_mana_cost_with_target(
    dynamic: &ironsmith_core::DynamicManaCost,
    enclosing_target: Option<&ChooseSpec>,
) -> String {
    if dynamic.source_mana_cost
        && dynamic.x_value.is_none()
        && dynamic.additional_generic.is_none()
        && dynamic.multiplier.is_none()
    {
        return "its mana cost".to_string();
    }
    if matches!(
        dynamic.display_hint,
        ironsmith_core::DynamicManaDisplayHint::ManaEqualTo
    ) && let Some(value) = dynamic.additional_generic.as_ref()
    {
        return format!(
            "mana equal to {}",
            describe_value_with_enclosing_target(value, enclosing_target)
        );
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
        let (standalone, appended) = match additional {
            Value::Scaled(value, multiplier) if *multiplier > 0 => {
                let each = describe_payment_each_value(value);
                (
                    format!("{{{multiplier}}} for each {each}"),
                    format!("plus an additional {{{multiplier}}} for each {each}"),
                )
            }
            Value::CountScaled(filter, multiplier) if *multiplier > 0 => {
                let each = describe_for_each_filter(filter);
                (
                    format!("{{{multiplier}}} for each {each}"),
                    format!("plus an additional {{{multiplier}}} for each {each}"),
                )
            }
            Value::Fixed(amount) => (
                format!("{{{amount}}}"),
                format!("plus an additional {{{amount}}}"),
            ),
            value => {
                let each = describe_payment_each_value(value);
                (
                    format!("{{1}} for each {each}"),
                    format!("plus an additional {{1}} for each {each}"),
                )
            }
        };
        if text.is_empty() {
            text = standalone;
        } else {
            text = format!("{text} {appended}");
        }
    }
    if let Some(x_value) = dynamic.x_value.as_ref() {
        if text.is_empty() {
            text = "{X}".to_string();
        }
        text = format!(
            "{text}, where X is {}",
            describe_value_with_enclosing_target(x_value, enclosing_target)
        );
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
    describe_total_cost_payment_with_target(cost, None)
}

pub(super) fn describe_total_cost_payment_for_same_sole_target(
    cost: &crate::cost::TotalCost,
    enclosing_target: &ChooseSpec,
) -> String {
    describe_total_cost_payment_with_target(cost, Some(enclosing_target))
}

/// Render a destroy instruction whose controller may pay a dynamic life cost
/// and whose no-regeneration rider was authored as a following sentence.
///
/// Dynamic life payments are executable cost effects rather than the fixed
/// `Cost::Life` variant. Prove the payer, destroyed-object tag, characteristic
/// basis, and authored rider before restoring the payment and sentence
/// surfaces.
pub(super) fn describe_destroy_unless_controller_pays_toughness_life(
    unless_pays: &crate::effects::UnlessPaysEffect,
) -> Option<String> {
    if unless_pays.leading_surface || unless_pays.before_delayed_step {
        return None;
    }
    let [destroy_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    let tagged = destroy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let destroy = tagged
        .effect
        .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()?;
    if !destroy.creature_destroyed_this_way_surface
        || !destroy.spec.is_target()
        || !destroy.spec.count().is_single()
        || !matches!(
            &unless_pays.player,
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
                if tag == &tagged.tag
        )
    {
        return None;
    }

    let [cost] = unless_pays.cost.costs() else {
        return None;
    };
    let lose = cost
        .effect_ref()?
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let Value::ToughnessOf(basis) = lose.amount.unhinted() else {
        return None;
    };
    if !matches!(
        lose.player.unhinted(),
        ChooseSpec::Player(PlayerFilter::You)
    ) || basis.unhinted() != destroy.spec.unhinted()
    {
        return None;
    }

    Some(format!(
        "Destroy {} unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated",
        describe_choose_spec(&destroy.spec)
    ))
}

/// Render a target that is simultaneously a damage source, the antecedent for
/// a source-relative characteristic, and the object offered as an unless
/// sacrifice.
///
/// This is deliberately structural: the shared generated tag proves that all
/// four references denote the same object, so the compact Oracle pronouns are
/// safe for any card with this shape.
pub(super) fn describe_target_source_damage_unless_referential_sacrifice(
    unless_pays: &crate::effects::UnlessPaysEffect,
) -> Option<String> {
    let [source_declaration, damage_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    let tagged_source = source_declaration.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target_only = tagged_source
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let source_tag = &tagged_source.tag;
    let ChooseSpec::Target(target_inner) = &target_only.target else {
        return None;
    };
    if target_only.target.count() != ChoiceCount::exactly(1)
        || !matches!(target_inner.base(), ChooseSpec::Object(_))
    {
        return None;
    }

    let with_source = damage_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(
        with_source.source.base(),
        ChooseSpec::Tagged(tag) if tag == source_tag
    ) {
        return None;
    }
    let damage = with_source
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;

    fn characteristic_for_source<'a>(value: &'a Value, source_tag: &TagKey) -> Option<&'a str> {
        let value = match value {
            Value::SurfaceHinted { value, .. } => value.as_ref(),
            value => value,
        };
        let same_source = |spec: &ChooseSpec| {
            matches!(spec.base(), ChooseSpec::Source)
                || matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == source_tag)
        };
        match value {
            Value::ManaValueOf(spec) if same_source(spec) => Some("mana value"),
            Value::PowerOf(spec) if same_source(spec) => Some("power"),
            Value::ToughnessOf(spec) if same_source(spec) => Some("toughness"),
            _ => None,
        }
    }

    fn controller_is_source(filter: &PlayerFilter, source_tag: &TagKey) -> bool {
        match filter {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target) => true,
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
            | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag)) => {
                tag == source_tag
            }
            PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
                controller_is_source(inner, source_tag)
            }
            _ => false,
        }
    }

    let ChooseSpec::Player(recipient) = damage.target.base() else {
        return None;
    };
    if !controller_is_source(recipient, source_tag)
        || !controller_is_source(&unless_pays.player, source_tag)
    {
        return None;
    }

    let [cost] = unless_pays.cost.costs() else {
        return None;
    };
    let cost_effect = unwrap_basic_tag_wrappers(cost.effect_ref()?);
    let tagged_source_filter =
        ObjectFilter::tagged(source_tag.clone()).controlled_by(PlayerFilter::You);
    let sacrifices_tagged_source = cost_effect
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()
        .is_some_and(|sacrifice| {
            matches!(
                sacrifice.target.base(),
                ChooseSpec::Tagged(tag) if tag == source_tag
            )
        })
        || cost_effect
            .downcast_ref::<crate::effects::SacrificeEffect>()
            .is_some_and(|sacrifice| {
                sacrifice.filter == tagged_source_filter
                    && sacrifice.count == Value::Fixed(1)
                    && sacrifice.player == PlayerFilter::You
            });
    if !sacrifices_tagged_source {
        return None;
    }

    let characteristic = characteristic_for_source(&damage.amount, source_tag)?;
    Some(format!(
        "{} deals damage equal to its {characteristic} to its controller unless that player sacrifices it",
        describe_choose_spec(&target_only.target)
    ))
}

fn describe_total_cost_payment_with_target(
    cost: &crate::cost::TotalCost,
    enclosing_target: Option<&ChooseSpec>,
) -> String {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            let parts = describe_cost_component_parts_with_target(costs, enclosing_target)
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
            .map(|branch| describe_total_cost_payment_with_target(branch, enclosing_target))
            .map(|part| part.strip_prefix("Pay ").unwrap_or(&part).to_string())
            .collect::<Vec<_>>()
            .join(" or "),
    }
}

pub(super) fn describe_payment_each_value(value: &Value) -> String {
    if let Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query) =
        value.unhinted()
    {
        return describe_prior_effect_metric_basis(query, false);
    }

    match value {
        Value::Count(filter) => describe_for_each_filter(filter),
        Value::CountScaled(filter, _) => describe_for_each_filter(filter),
        Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query) => {
            describe_prior_effect_metric_basis(query, false)
        }
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
    describe_cost_component_parts_with_target(costs, None)
}

fn tagged_behold_cost(effect: &Effect) -> Option<(&crate::TagKey, &crate::effects::BeholdEffect)> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_behold_cost(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        let behold = unwrap_basic_tag_wrappers(&tagged.effect)
            .downcast_ref::<crate::effects::BeholdEffect>()?;
        return Some((&tagged.tag, behold));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        let behold = unwrap_basic_tag_wrappers(&tagged.effect)
            .downcast_ref::<crate::effects::BeholdEffect>()?;
        return Some((&tagged.tag, behold));
    }
    None
}

fn exile_cost_uses_tag(effect: &Effect, expected_tag: &crate::TagKey) -> bool {
    let effect = unwrap_basic_tag_wrappers(effect);
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return move_to_zone.zone == Zone::Exile
            && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == expected_tag);
    }
    effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .is_some_and(|exile| {
            !exile.face_down
                && matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == expected_tag)
        })
}

fn describe_behold_then_exile_cost(
    behold_cost: &crate::costs::Cost,
    exile_cost: &crate::costs::Cost,
) -> Option<String> {
    let (tag, behold) = tagged_behold_cost(behold_cost.effect_ref()?)?;
    if behold.count != 1 || !exile_cost_uses_tag(exile_cost.effect_ref()?, tag) {
        return None;
    }
    Some(format!(
        "Behold {} and exile it",
        with_indefinite_article(&behold.subtype.to_string())
    ))
}

fn describe_cost_component_parts_with_target(
    costs: &[crate::costs::Cost],
    enclosing_target: Option<&ChooseSpec>,
) -> Vec<String> {
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < costs.len() {
        if idx + 2 < costs.len()
            && costs[idx].is_sacrifice_self()
            && let Some(choose) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(sacrifice) = costs[idx + 2].effect_ref().and_then(sacrifice_view)
            && let Some(chosen) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            let chosen = normalize_cost_phrase(&chosen);
            if let Some(chosen) = chosen.strip_prefix("Sacrifice ") {
                let chosen = if choose.filter.controller == Some(PlayerFilter::You)
                    && !chosen.contains(" you control")
                {
                    format!("{chosen} you control")
                } else {
                    chosen.to_string()
                };
                parts.push(format!("Sacrifice this source and {chosen}"));
                idx += 3;
                continue;
            }
        }
        if idx + 1 < costs.len()
            && costs[idx + 1].is_sacrifice_self()
            && let Some(sacrifice) = costs[idx].effect_ref().and_then(sacrifice_view)
            && sacrifice.player == &PlayerFilter::You
            && matches!(sacrifice.count, Value::Fixed(count) if *count > 0)
        {
            let mut display_filter = sacrifice.filter.clone();
            if display_filter.controller == Some(PlayerFilter::You) {
                display_filter.controller = None;
            }
            let chosen = normalize_cost_phrase(&describe_sacrifice_effect(SacrificeView {
                filter: &display_filter,
                count: sacrifice.count,
                player: sacrifice.player,
            }));
            if let Some(chosen) = chosen.strip_prefix("Sacrifice ") {
                parts.push(format!("Sacrifice {chosen} and this source"));
                idx += 2;
                continue;
            }
        }
        if idx + 1 < costs.len()
            && let Some(compact) = describe_behold_then_exile_cost(&costs[idx], &costs[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if let Some((compact, consumed)) =
            describe_exile_source_and_other_objects_cost(&costs[idx..])
        {
            parts.push(compact);
            idx += consumed;
            continue;
        }
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
            && let Some(reveal) = costs[idx + 1].effect_ref().and_then(|effect| {
                unwrap_basic_tag_wrappers(effect)
                    .downcast_ref::<crate::effects::RevealTaggedEffect>()
            })
            && let Some(compact) = describe_choose_then_reveal_from_hand_cost(choose, reveal)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(move_to_zone) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
            && let Some(compact) =
                describe_put_opponent_owned_exiled_card_into_graveyard_cost(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(move_to_zone) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
            && let Some(compact) =
                describe_choose_then_put_on_bottom_of_library_cost(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(move_to_zone) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
            && let Some(compact) =
                describe_choose_then_put_on_top_of_library_cost(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
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
        if let Some(dynamic) = costs[idx].dynamic_mana_cost_ref() {
            parts.push(describe_dynamic_mana_cost_with_target(
                dynamic,
                enclosing_target,
            ));
        } else {
            parts.push(describe_cost_component(&costs[idx]));
        }
        idx += 1;
    }
    parts
}

fn describe_put_opponent_owned_exiled_card_into_graveyard_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Exile)
        || choose.filter.owner != Some(PlayerFilter::Opponent)
        || !choose.count.is_single()
        || move_to_zone.zone != Zone::Graveyard
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    let mut noun = strip_leading_article(&filter.description()).to_string();
    if noun == "object" || noun == "permanent" {
        noun = "card".to_string();
    } else if !noun.split_whitespace().any(|word| {
        matches!(
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()),
            "card" | "cards"
        )
    }) {
        noun.push_str(" card");
    }

    Some(format!(
        "Put {} an opponent owns from exile into that player's graveyard",
        with_indefinite_article(&noun)
    ))
}

fn describe_choose_then_put_on_top_of_library_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || !matches!(choose.filter.owner, None | Some(PlayerFilter::You))
        || !choose.count.is_single()
        || move_to_zone.zone != Zone::Library
        || !move_to_zone.to_top
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    let mut noun = strip_leading_article(&filter.description()).to_string();
    if noun == "object" || noun == "permanent" {
        noun = "card".to_string();
    } else if !noun
        .split_whitespace()
        .any(|word| matches!(word, "card" | "cards"))
    {
        noun.push_str(" card");
    }
    Some(format!(
        "Put {} from your hand on top of your library",
        with_indefinite_article(&noun)
    ))
}

fn describe_choose_then_put_on_bottom_of_library_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    let exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    if exact == 0
        || choose.is_search
        || choose.count.dynamic_x
        || choose.count.up_to_x
        || choose.count.random
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Graveyard)
        || choose.filter.owner != Some(PlayerFilter::You)
        || move_to_zone
            != &crate::effects::MoveToZoneEffect::to_bottom_of_library(ChooseSpec::Tagged(
                choose.tag.clone(),
            ))
    {
        return None;
    }

    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    if filter != ObjectFilter::default() {
        return None;
    }

    let object = if exact == 1 {
        "a card".to_string()
    } else {
        let count = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
        format!("{count} cards")
    };
    Some(format!(
        "Put {object} from your graveyard on the bottom of your library"
    ))
}

#[cfg(test)]
mod graveyard_to_library_cost_tests {
    use super::*;

    fn cost_pair() -> (
        crate::effects::ChooseObjectsEffect,
        crate::effects::MoveToZoneEffect,
    ) {
        let tag = TagKey::from("library_cost");
        let choose = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
            ChoiceCount::exactly(3),
            PlayerFilter::You,
            tag.clone(),
        );
        let move_to_library =
            crate::effects::MoveToZoneEffect::to_bottom_of_library(ChooseSpec::Tagged(tag));
        (choose, move_to_library)
    }

    #[test]
    fn compacts_graveyard_cards_moved_to_the_bottom_as_one_cost() {
        let (choose, move_to_library) = cost_pair();
        assert_eq!(
            describe_choose_then_put_on_bottom_of_library_cost(&choose, &move_to_library)
                .as_deref(),
            Some("Put three cards from your graveyard on the bottom of your library")
        );
    }

    #[test]
    fn rejects_changed_tag_zone_or_destination_cost_pairs() {
        let (choose, move_to_library) = cost_pair();

        let wrong_tag = crate::effects::MoveToZoneEffect::to_bottom_of_library(ChooseSpec::Tagged(
            TagKey::from("other"),
        ));
        assert!(describe_choose_then_put_on_bottom_of_library_cost(&choose, &wrong_tag).is_none());

        let mut wrong_zone = choose.clone();
        wrong_zone.filter.zone = Some(Zone::Hand);
        assert!(
            describe_choose_then_put_on_bottom_of_library_cost(&wrong_zone, &move_to_library)
                .is_none()
        );

        let wrong_destination = crate::effects::MoveToZoneEffect::to_top_of_library(
            ChooseSpec::Tagged(choose.tag.clone()),
        );
        assert!(
            describe_choose_then_put_on_bottom_of_library_cost(&choose, &wrong_destination)
                .is_none()
        );
    }
}

fn describe_choose_then_reveal_from_hand_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    reveal: &crate::effects::RevealTaggedEffect,
) -> Option<String> {
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose_primary_zone(choose) != Some(Zone::Hand)
        || !matches!(choose.filter.owner, None | Some(PlayerFilter::You))
        || choose.tag != reveal.tag
        || choose.count.dynamic_x
        || choose.count.random
    {
        return None;
    }
    let exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    if exact == 0 {
        return None;
    }

    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    let mut noun = strip_leading_article(&filter.description()).to_string();
    if !noun.split_whitespace().any(|word| {
        matches!(
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()),
            "card" | "cards"
        )
    }) {
        noun.push_str(" card");
    }

    let object = if exact == 1 {
        with_indefinite_article(&noun)
    } else {
        let count = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
        format!("{count} {}", pluralize_noun_phrase(&noun))
    };
    Some(format!("Reveal {object} from your hand"))
}

#[cfg(test)]
mod reveal_from_hand_cost_tests {
    use super::*;

    #[test]
    fn compacts_typed_colorless_creature_choice_and_reveal_into_one_cost() {
        let tag = TagKey::from("revealed_cost");
        let choose = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::creature()
                .colorless()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Hand);
        let reveal = crate::effects::RevealTaggedEffect::new(tag);

        assert_eq!(
            describe_choose_then_reveal_from_hand_cost(&choose, &reveal).as_deref(),
            Some("Reveal a colorless creature card from your hand")
        );
    }
}

#[cfg(test)]
mod opponent_owned_exile_to_graveyard_cost_tests {
    use super::*;

    fn processor_cost_pair() -> (
        crate::effects::ChooseObjectsEffect,
        crate::effects::MoveToZoneEffect,
    ) {
        let tag = TagKey::from("graveyard_cost");
        let choose = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Exile)
                .owned_by(PlayerFilter::Opponent),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Exile);
        let move_to_graveyard =
            crate::effects::MoveToZoneEffect::to_graveyard(ChooseSpec::Tagged(tag));
        (choose, move_to_graveyard)
    }

    #[test]
    fn compacts_opponent_owned_exile_to_owner_graveyard_cost() {
        let (choose, move_to_graveyard) = processor_cost_pair();

        assert_eq!(
            describe_put_opponent_owned_exiled_card_into_graveyard_cost(
                &choose,
                &move_to_graveyard,
            )
            .as_deref(),
            Some("Put a card an opponent owns from exile into that player's graveyard")
        );
    }

    #[test]
    fn does_not_compact_near_miss_cost_pairs() {
        let (choose, move_to_graveyard) = processor_cost_pair();

        let mut wrong_owner = choose.clone();
        wrong_owner.filter.owner = Some(PlayerFilter::You);
        assert!(
            describe_put_opponent_owned_exiled_card_into_graveyard_cost(
                &wrong_owner,
                &move_to_graveyard,
            )
            .is_none()
        );

        let mut wrong_zone = choose.clone();
        wrong_zone.filter.zone = Some(Zone::Hand);
        assert!(
            describe_put_opponent_owned_exiled_card_into_graveyard_cost(
                &wrong_zone,
                &move_to_graveyard,
            )
            .is_none()
        );

        let wrong_tag = crate::effects::MoveToZoneEffect::to_graveyard(ChooseSpec::Tagged(
            TagKey::from("other_cost"),
        ));
        assert!(
            describe_put_opponent_owned_exiled_card_into_graveyard_cost(&choose, &wrong_tag)
                .is_none()
        );

        let wrong_destination = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(choose.tag.clone()),
            Zone::Hand,
            false,
        );
        assert!(
            describe_put_opponent_owned_exiled_card_into_graveyard_cost(
                &choose,
                &wrong_destination,
            )
            .is_none()
        );
    }
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

fn describe_exile_source_and_other_objects_cost(
    costs: &[crate::costs::Cost],
) -> Option<(String, usize)> {
    fn typed_source_surface(surface: &crate::target::SourceReferenceSurface) -> String {
        let text = surface.display_text();
        let crate::target::SourceReferenceSurface::ThisPermanentType(_) = surface else {
            return text;
        };
        let Some(noun) = text.strip_prefix("this ") else {
            return text;
        };
        let authored_subtype = [
            crate::types::SubtypeFamily::Land,
            crate::types::SubtypeFamily::Creature,
            crate::types::SubtypeFamily::Artifact,
            crate::types::SubtypeFamily::Enchantment,
            crate::types::SubtypeFamily::Spell,
            crate::types::SubtypeFamily::Planeswalker,
            crate::types::SubtypeFamily::Battle,
        ]
        .into_iter()
        .flat_map(crate::types::SubtypeFamily::all_subtypes)
        .find(|subtype| subtype.display_name().eq_ignore_ascii_case(noun));
        authored_subtype
            .map(|subtype| format!("this {}", subtype.display_name()))
            .unwrap_or(text)
    }

    let [
        source_choose_cost,
        source_exile_cost,
        other_choose_cost,
        other_exile_cost,
        ..,
    ] = costs
    else {
        return None;
    };
    let source_choose = source_choose_cost
        .effect_ref()?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let source_exile = source_exile_cost
        .effect_ref()?
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let other_choose = other_choose_cost
        .effect_ref()?
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let other_exile = other_exile_cost
        .effect_ref()?
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let source_surface = source_choose.filter.source_surface.as_ref()?;
    if !source_choose.filter.source
        || source_choose.chooser != PlayerFilter::You
        || choose_exact_count(source_choose) != Some(1)
        || choose_primary_zone(source_choose) != Some(Zone::Battlefield)
        || !exile_uses_chosen_tag(&source_exile.spec, source_choose.tag.as_str())
        || other_choose.chooser != PlayerFilter::You
        || choose_exact_count(other_choose).is_none_or(|count| count == 0)
        || choose_primary_zone(other_choose) != Some(Zone::Battlefield)
        || !other_choose.filter.other
        || other_choose.filter.controller != Some(PlayerFilter::You)
        || !exile_uses_chosen_tag(&other_exile.spec, other_choose.tag.as_str())
    {
        return None;
    }
    let other = normalize_cost_phrase(&describe_choose_then_exile(other_choose, other_exile)?);
    let other = other.strip_prefix("Exile ")?;
    Some((
        format!("Exile {} and {other}", typed_source_surface(source_surface)),
        4,
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

pub(crate) fn describe_total_cost(cost: &crate::cost::TotalCost) -> String {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => describe_cost_list(costs),
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            // Waterbend's expanded tap branches are the executable payment
            // model; the authored keyword is the public cost surface.
            if let Some(generic) = waterbend_generic_from_branches(branches) {
                return format!("Waterbend {{{generic}}}");
            }
            branches
                .iter()
                .map(describe_total_cost)
                .collect::<Vec<_>>()
                .join(" or ")
        }
    }
}

pub(super) fn waterbend_generic_from_branches(branches: &[crate::cost::TotalCost]) -> Option<u32> {
    branches.iter().find_map(|branch| {
        let ironsmith_core::TotalCostKind::All(costs) = branch.kind() else {
            return None;
        };
        costs.iter().find_map(|cost| {
            let effect = &cost.downcast_ref::<crate::costs::CostEffect>()?.effect;
            let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            choose
                .tag
                .as_str()
                .strip_prefix("waterbend_cost_")?
                .parse::<u32>()
                .ok()
        })
    })
}

pub(super) fn describe_total_cost_with_trailing_x_definition(
    cost: &crate::cost::TotalCost,
) -> (String, Option<String>) {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            describe_cost_list_with_trailing_x_definition(costs)
        }
        ironsmith_core::TotalCostKind::OneOf(_) => (describe_total_cost(cost), None),
    }
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
            describe_counter_type(*counter_type),
            plural
        ));
    }

    Some(format!(
        "Double the number of {} counters on each {}",
        describe_counter_type(*counter_type),
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
    if choose.family != crate::types::SubtypeFamily::Creature
        || !choose.excluded_subtypes.is_empty()
        || !matches!(choose.chooser, PlayerFilter::You)
    {
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
    if choose.family != crate::types::SubtypeFamily::Creature
        || !choose.excluded_subtypes.is_empty()
        || !matches!(choose.chooser, PlayerFilter::You)
    {
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
        // A set defined only by a "<verbed> this way" tag is a back-reference
        // to the objects the previous sentence just acted on; oracle uses the
        // partitive pronoun ("Put a stun counter on each of them").
        if this_way_back_reference_filter(filter) {
            return Some((
                format!(
                    "Put {} on each of them",
                    describe_put_counter_phrase(&put.amount, put.counter_type)
                ),
                filter,
                None,
            ));
        }
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

    if this_way_back_reference_filter(&for_each.filter) {
        return Some((
            format!(
                "Put {} on each of them",
                describe_put_counter_phrase(&put.amount, put.counter_type)
            ),
            &for_each.filter,
            None,
        ));
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

/// True when the filter identifies objects solely by a "<verbed> this way"
/// provenance tag over generic battlefield scaffolding — a back-reference to
/// the set the previous sentence acted on.
pub(super) fn this_way_back_reference_filter(filter: &ObjectFilter) -> bool {
    if describe_tagged_this_way_action(filter).is_none() {
        return false;
    }
    if !filter.card_types.is_empty() {
        return false;
    }
    let mut base = filter.clone();
    base.tagged_constraints.clear();
    base.set_prior_effect_action_surface(None);
    base.zone = None;
    base == ObjectFilter::default()
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

pub(in crate::compiled_text) fn describe_put_counters_then_untap_them(
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
    if filter.card_types.contains(&CardType::Creature)
        && filter.controller == Some(PlayerFilter::You)
        && filter.tapped
    {
        return Some(format!("{put_text}, then untap them"));
    }
    let plural_pronoun = match untap.target.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.has_plural_pronoun_reference_surface()
        }
        _ => false,
    };
    let subject = if plural_pronoun {
        "them"
    } else if filter.card_types.contains(&CardType::Creature) {
        "those creatures"
    } else {
        "them"
    };
    Some(format!("{put_text}. Untap {subject}"))
}

/// Rejoin a counter-placement sentence with an exact-set untap sentence when
/// lowering materialized the authored pronoun through an intermediate
/// `TagMatchingObjectsEffect`.
pub(in crate::compiled_text) fn describe_put_counters_then_tag_matching_untap_them(
    first: &Effect,
    tag_matching: &Effect,
    untap_effect: &Effect,
) -> Option<String> {
    let (put_text, filter, put_tag) =
        if let Some((put_text, filter, Some(put_tag))) = put_counters_each_filter_view(first) {
            (put_text, filter, put_tag)
        } else {
            // Targeted plural sets retain their count on the choose spec rather
            // than lowering as an `each` filter. The producer tag and the two
            // exact-set correlations below still prove that the authored
            // plural pronoun refers to precisely those targets.
            let tagged = first.downcast_ref::<crate::effects::TaggedEffect>()?;
            let put = tagged
                .effect
                .downcast_ref::<crate::effects::PutCountersEffect>()?;
            let count = put.target.count();
            if put.distributed
                || !put.target.is_target()
                || put
                    .target_count
                    .as_ref()
                    .is_some_and(|target_count| target_count != &count)
                || count.max.is_some_and(|max| max <= 1)
                || count.random
            {
                return None;
            }
            let ChooseSpec::Object(filter) = put.target.base() else {
                return None;
            };
            (
                describe_effect(first).trim_end_matches('.').to_string(),
                filter,
                &tagged.tag,
            )
        };
    let tag_matching = tag_matching.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let matches_countered_group = tag_matching
        .filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == *put_tag
        });
    if !matches_countered_group {
        return None;
    }
    let untap = untap_effect.downcast_ref::<crate::effects::UntapEffect>()?;
    if !untap_target_references_tag(untap, &tag_matching.tag) {
        return None;
    }

    let plural_pronoun = tag_matching.filter.has_plural_pronoun_reference_surface()
        || match untap.target.base() {
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
                filter.has_plural_pronoun_reference_surface()
            }
            _ => false,
        };
    let subject = if plural_pronoun {
        "them"
    } else if filter.card_types.contains(&CardType::Creature) {
        "those creatures"
    } else {
        "them"
    };
    Some(format!("{put_text}. Untap {subject}"))
}

/// Render an exact count of the complete creature set tagged by a preceding
/// counter-placement effect as the authored demonstrative back-reference.
///
/// The tag remains the runtime identity of the affected set. This matcher only
/// selects the cleaner "each of those creatures" surface when the consumer
/// adds no predicate beyond that tag and the producer's creature noun.
pub(in crate::compiled_text) fn describe_put_counters_then_gain_life_for_each_of_them(
    first: &Effect,
    second: &Effect,
) -> Option<String> {
    let (put_text, put_filter, Some(put_tag)) = put_counters_each_filter_view(first)? else {
        return None;
    };
    if put_filter.card_types.as_slice() != [CardType::Creature] {
        return None;
    }

    let gain = second.downcast_ref::<crate::effects::GainLifeEffect>()?;
    if !gain.amount.has_surface_hint(ValueSurfaceHint::ForEach) {
        return None;
    }
    let (count_filter, multiplier) = match gain.amount.unhinted() {
        Value::Count(filter) => (filter, 1),
        Value::Scaled(value, multiplier) if *multiplier > 0 => {
            let Value::Count(filter) = value.as_ref() else {
                return None;
            };
            (filter, *multiplier)
        }
        _ => return None,
    };
    if count_filter.card_types.as_slice() != [CardType::Creature] {
        return None;
    }

    let mut remainder = count_filter.clone();
    remainder.zone = None;
    remainder.card_types.clear();
    remainder.set_prior_effect_action_surface(None);
    remainder.tagged_constraints.retain(|constraint| {
        constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
            || constraint.tag != *put_tag
    });
    if remainder != ObjectFilter::default() {
        return None;
    }

    let player = describe_choose_spec(&gain.player);
    Some(format!(
        "{put_text}. {} {} {multiplier} life for each of those creatures",
        capitalize_first(&player),
        player_verb(&player, "gain", "gains")
    ))
}

pub(crate) fn describe_for_each_tagged_this_way_subject(filter: &ObjectFilter) -> Option<String> {
    let action = filter.tagged_constraints.iter().find_map(|constraint| {
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        let tag = constraint.tag.as_str();
        if tag.starts_with("exiled_") || crate::cards::is_sentence_helper_tag(tag, "exiled") {
            Some("exiled")
        } else if tag.starts_with("destroyed_")
            || crate::cards::is_sentence_helper_tag(tag, "destroyed")
        {
            Some("destroyed")
        } else if tag.starts_with("sacrificed_")
            || crate::cards::is_sentence_helper_tag(tag, "sacrificed")
        {
            Some("sacrificed")
        } else if tag.starts_with("revealed_")
            || crate::cards::is_sentence_helper_tag(tag, "revealed")
        {
            Some("revealed")
        } else if tag.starts_with("discarded_")
            || crate::cards::is_sentence_helper_tag(tag, "discarded")
        {
            Some("discarded")
        } else if tag.starts_with("milled_") || crate::cards::is_sentence_helper_tag(tag, "milled")
        {
            Some("milled")
        } else if tag.starts_with("tapped_") || crate::cards::is_sentence_helper_tag(tag, "tapped")
        {
            Some("tapped")
        } else {
            None
        }
    })?;

    if filter.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Those) {
        let mut noun_filter = filter.clone();
        noun_filter.zone = None;
        noun_filter.tagged_constraints.clear();
        noun_filter.set_prior_effect_action_surface(None);
        noun_filter.set_set_quantifier_surface(None);
        let noun = strip_indefinite_article(&noun_filter.description())
            .trim()
            .to_string();
        if noun.is_empty() {
            return None;
        }
        return Some(format!(
            "For each of those {}",
            pluralize_noun_phrase(&noun)
        ));
    }

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

    if filter.has_put_into_graveyard_this_way_surface() {
        Some(format!("For each {subject} put into a graveyard this way"))
    } else {
        Some(format!("For each {subject} {action} this way"))
    }
}

#[cfg(test)]
mod for_each_tagged_set_surface_tests {
    use super::*;

    fn destroyed_permanent_filter() -> ObjectFilter {
        ObjectFilter::permanent()
            .in_zone(Zone::Battlefield)
            .match_tagged(
                TagKey::from("destroyed_0"),
                crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            )
    }

    #[test]
    fn typed_those_surface_distinguishes_set_reference_from_this_way() {
        let ordinary = destroyed_permanent_filter();
        assert_eq!(
            describe_for_each_tagged_this_way_subject(&ordinary).as_deref(),
            Some("For each permanent destroyed this way")
        );

        let mut those = ordinary;
        those.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
        assert_eq!(
            describe_for_each_tagged_this_way_subject(&those).as_deref(),
            Some("For each of those permanents")
        );

        let mut sentence_helper = ObjectFilter::default().match_tagged(
            TagKey::from("__sentence_helper_revealed_l0_s0_e0"),
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        sentence_helper.set_explicit_card_noun(true);
        sentence_helper
            .set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
        assert_eq!(
            describe_for_each_tagged_this_way_subject(&sentence_helper).as_deref(),
            Some("For each of those cards")
        );
    }

    #[test]
    fn public_reveal_each_unless_payment_program_keeps_the_authored_set_surface() {
        let oracle = "Reveal the top three cards of your library. For each of those cards, put that card into your hand unless any opponent pays 3 life. Then exile the rest.";
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Reveal Payment Probe",
        )
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("reveal/offer/rest program should parse");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [oracle]
        );
        let debug = format!("{:#?}", definition.spell_effect);
        assert!(debug.contains("UnlessPaysEffect"), "{debug}");
        assert!(debug.contains("player: Opponent"), "{debug}");
        assert!(debug.contains("LoseLifeEffect"), "{debug}");
    }

    #[test]
    fn typed_graveyard_action_surface_overrides_a_mill_result_tag() {
        let mut filter = ObjectFilter::creature()
            .in_zone(Zone::Graveyard)
            .match_tagged(
                TagKey::from("milled_0"),
                crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            );
        filter.zone = None;
        filter.set_put_into_graveyard_this_way_surface(true);
        assert_eq!(
            describe_for_each_tagged_this_way_subject(&filter).as_deref(),
            Some("For each creature put into a graveyard this way")
        );
    }
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
    // A trailing progressive clause modifies the noun ahead of it; pluralize
    // the noun phrase, not the clause's last word.
    for clause in [
        " attacking you",
        " attacking them",
        " attacking that player",
    ] {
        if let Some(head) = word.strip_suffix(clause)
            && !head.is_empty()
        {
            return format!("{}{clause}", pluralize_word(head));
        }
    }
    // A trailing "-ed this way" participle modifies the noun ahead of it;
    // pluralize the noun phrase, not "way".
    if let Some(head) = word.strip_suffix(" this way") {
        for tail in [" put onto the battlefield", " put there"] {
            if let Some(noun) = head.strip_suffix(tail)
                && !noun.is_empty()
            {
                return format!("{}{tail} this way", pluralize_word(noun));
            }
        }
        if let Some((noun, participle)) = head.rsplit_once(' ')
            && !noun.is_empty()
            && participle.ends_with("ed")
        {
            return format!("{} {participle} this way", pluralize_word(noun));
        }
    }
    if let Some((prefix, last)) = word.rsplit_once(' ')
        && !prefix.is_empty()
        && !last.is_empty()
    {
        // A trailing controller/owner clause is not the noun; pluralize the
        // noun phrase ahead of the clause instead of the verb.
        if matches!(last, "controls" | "control" | "owns" | "own") {
            for clause in [
                " defending player controls",
                " its controller controls",
                " that player controls",
                " that player owns",
                " you don't control",
                " you don't own",
                " you control",
                " you own",
                " they don't control",
                " they control",
                " they own",
                " an opponent controls",
                " an opponent owns",
                " target player controls",
                " target opponent controls",
                " attacking player controls",
                " active player controls",
                " a teammate controls",
                " your team controls",
            ] {
                if let Some(head) = word.strip_suffix(clause) {
                    if head.is_empty() {
                        return word.to_string();
                    }
                    return format!("{}{}", pluralize_word(head), clause);
                }
            }
            return word.to_string();
        }
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
    if lower == "fungus" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Fungi".to_string()
        } else {
            "fungi".to_string()
        };
    }
    if matches!(lower.as_str(), "myr" | "merfolk" | "treefolk" | "equipment") {
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
    // English -o plurals are irregular (Heroes, but Rhinos); enumerate the
    // -es cases that appear as game nouns.
    if matches!(lower.as_str(), "hero" | "potato" | "tomato") {
        return format!("{word}es");
    }
    format!("{word}s")
}

fn and_or_arm_has_shared_terminal_noun(arm: &str) -> bool {
    let arm = strip_indefinite_article(arm).trim();
    if matches!(arm.to_ascii_lowercase().as_str(), "token" | "tokens") {
        // A bare `token` arm is itself one member of the union, as in
        // "Zombies and/or tokens"; it is not a trailing noun shared by the
        // subtype arm.
        return false;
    }
    arm.split_whitespace().any(|word| {
        matches!(
            word.trim_matches(|ch: char| !ch.is_ascii_alphabetic())
                .to_ascii_lowercase()
                .as_str(),
            "card" | "cards" | "spell" | "spells" | "token" | "tokens"
        )
    })
}

fn pluralize_independent_and_or_arms(phrase: &str) -> Option<String> {
    let parts = if let Some((leading, last)) = phrase.rsplit_once(", and/or ") {
        leading
            .split(", ")
            .chain(std::iter::once(last))
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
    } else {
        phrase
            .split(" and/or ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
    };
    if parts.len() < 2 || parts.iter().any(|part| part.contains(" or ")) {
        return None;
    }

    // In phrases such as "instant and/or sorcery card", `card` is the
    // shared noun for both arms. Pluralizing each arm independently would
    // incorrectly turn the modifiers into "instants and/or sorcery cards".
    // When no arm, or multiple arms, contain an explicit shared noun, each
    // arm has its own noun and must be pluralized independently.
    let arms_with_shared_noun = parts
        .iter()
        .map(|arm| and_or_arm_has_shared_terminal_noun(arm))
        .collect::<Vec<_>>();
    let explicit_shared_noun_arms = arms_with_shared_noun.iter().filter(|has| **has).count();
    // A trailing noun can only be shared by earlier arms when it closes the
    // LAST arm ("instant and/or sorcery card"). A noun word inside an earlier
    // arm is that arm's own noun ("spell and/or ability").
    if explicit_shared_noun_arms == 1 && arms_with_shared_noun.last() == Some(&true) {
        return None;
    }

    let plural_parts = parts
        .iter()
        .map(|part| pluralize_noun_phrase(part))
        .collect::<Vec<_>>();
    match plural_parts.as_slice() {
        [] | [_] => None,
        [first, second] => Some(format!("{first} and/or {second}")),
        many => Some(format!(
            "{}, and/or {}",
            many[..many.len() - 1].join(", "),
            many.last()?
        )),
    }
}

pub(crate) fn pluralize_noun_phrase(phrase: &str) -> String {
    let mut base = strip_indefinite_article(phrase).trim();
    let mut trailing = "";
    if let Some(stripped) = base.strip_suffix('.') {
        base = stripped.trim_end();
        trailing = ".";
    }
    if let Some(rest) = base.strip_prefix("another ") {
        return format!("other {}{}", pluralize_noun_phrase(rest), trailing);
    }
    for (relation, plural_relation, predicate_is_adjectival) in [
        (" that isn't ", " that aren't ", true),
        (" that is not ", " that aren't ", true),
        (" that's ", " that are ", false),
        (" that is ", " that are ", false),
    ] {
        if let Some((head, selectors)) = base.split_once(relation) {
            let selectors = strip_indefinite_article(selectors.trim());
            // A participial combat relation is a predicate over the noun on
            // the left, not another noun phrase.  Recursing into its object
            // would pluralize "that player" into "that players".
            let selectors = if predicate_is_adjectival
                || selectors.starts_with("attacking ")
                || selectors.starts_with("blocking ")
            {
                selectors.to_string()
            } else {
                pluralize_noun_phrase(selectors)
            };
            return format!(
                "{}{}{}{}",
                pluralize_noun_phrase(head.trim()),
                plural_relation,
                selectors,
                trailing
            );
        }
    }
    // Here "attacking" / "blocking" can either introduce a relation
    // ("creature attacking you") or be a prenominal adjective
    // ("nontoken attacking creature"). In the latter shape the noun to
    // pluralize is on the right; treating the adjective as a relation yields
    // malformed surfaces such as "nontokens attacking creature".
    for adjective in ["attacking", "blocking"] {
        let marker = format!(" {adjective} ");
        if let Some((modifiers, noun)) = base.split_once(&marker)
            && [
                "creature",
                "creature card",
                "creature token",
                "artifact creature",
                "artifact creature token",
            ]
            .iter()
            .any(|head| noun == *head || noun.starts_with(&format!("{head} ")))
        {
            return format!(
                "{} {adjective} {}{}",
                modifiers.trim(),
                pluralize_noun_phrase(noun),
                trailing
            );
        }
    }
    for relation in [" attacking ", " blocking "] {
        if let Some((head, object)) = base.split_once(relation) {
            return format!(
                "{}{}{}{}",
                pluralize_noun_phrase(head.trim()),
                relation,
                object.trim(),
                trailing
            );
        }
    }
    // Past-participial provenance qualifies the noun to its left. Handle it
    // before the broader `with` qualifier so "card exiled with this source"
    // pluralizes its noun instead of becoming the malformed "card exileds".
    for participle in ["created", "exiled"] {
        let marker = format!(" {participle} ");
        if let Some((head, tail)) = base.split_once(&marker) {
            return format!(
                "{} {participle} {}{}",
                pluralize_noun_phrase(head.trim()),
                tail.trim(),
                trailing
            );
        }
    }
    // Qualifiers delimit the noun phrase before any conjunctions they may
    // contain. In particular, the `or` in "less than or equal to" is not a
    // union of noun-phrase arms and must not recursively pluralize its value.
    if let Some((head, tail)) = base.split_once(" with ") {
        if let Some(relation_tail) = plural_power_toughness_relation_tail(tail.trim()) {
            return format!(
                "{} {}{}",
                pluralize_noun_phrase(head),
                relation_tail,
                trailing
            );
        }
        let tail = tail.trim();
        let plural_tail = if !tail.starts_with("a ") && !tail.starts_with("an ") {
            tail.strip_suffix(" ability")
                .map(|ability| format!("{ability} abilities"))
        } else {
            None
        };
        return format!(
            "{} with {}{}",
            pluralize_noun_phrase(head),
            plural_tail.as_deref().unwrap_or(tail),
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
        " you both own and control",
        " you control but don't own",
        " that player both owns and controls",
    ] {
        if let Some(head) = base.strip_suffix(suffix) {
            return format!(
                "{}{suffix}{trailing}",
                pluralize_noun_phrase(head.trim_end())
            );
        }
    }
    if base.contains("and/or")
        && let Some(pluralized) = pluralize_independent_and_or_arms(base)
    {
        return format!("{pluralized}{trailing}");
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
    if base.contains(" and ") {
        let parts = base
            .split(" and ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let shared_terminal_noun_arms = parts
            .iter()
            .filter(|arm| and_or_arm_has_shared_terminal_noun(arm))
            .count();
        if parts.len() > 1 && shared_terminal_noun_arms != 1 {
            let plural_parts = parts
                .iter()
                .map(|part| pluralize_noun_phrase(part))
                .collect::<Vec<_>>();
            return format!("{}{}", plural_parts.join(" and "), trailing);
        }
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
        " your team controls",
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
        " on the battlefield",
        " revealed this way",
        " of the chosen type",
        " of the chosen color",
        " that aren't of the chosen type",
    ] {
        if let Some(head) = base.strip_suffix(suffix) {
            let head = head.trim_end();
            // The head may itself carry qualifiers ("creature you control")
            // — recurse so the noun pluralizes, not the last qualifier word.
            let head_plural = pluralize_noun_phrase(head);
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
    if base.eq_ignore_ascii_case("fungus") {
        format!("{}{}", pluralize_word(base), trailing)
    } else if base.ends_with('s') {
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

pub(in crate::compiled_text) fn destroy_effect_for_choose_compaction(
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

pub(in crate::compiled_text) fn describe_for_players_may_choose_then_destroy_chosen(
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

/// Render a mandatory quantified choice followed by an action on the exact
/// aggregate selected across all iterations. The shared tag proves that the
/// destroy effect consumes the chosen collection rather than an arbitrary
/// permanent matching the choice filter.
pub(super) fn describe_for_players_choose_then_destroy_chosen_collection(
    for_players: &crate::effects::ForPlayersEffect,
    destroy: &crate::effects::DestroyEffect,
) -> Option<String> {
    if for_players.starting_with_controller
        || for_players.stop_after_first_happened
        || for_players.effects.len() != 1
    {
        return None;
    }
    let choose = structural_unwrap_render_wrappers(&for_players.effects[0])
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.is_search
        || !choose.count.is_single()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
    {
        return None;
    }
    let chooser = match choose.chooser {
        PlayerFilter::You => "you choose",
        PlayerFilter::IteratedPlayer => "that player chooses",
        _ => return None,
    };
    let quantified = match for_players.filter {
        PlayerFilter::Any => "player",
        PlayerFilter::Opponent => "opponent",
        _ => return None,
    };
    if let ChooseSpec::WithCount(spec, count) = &destroy.spec
        && matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        && count.min == 1
        && count.max == Some(1)
        && !count.dynamic_x
        && !count.up_to_x
        && count.random
        && !count.explicit_exactly
    {
        let selection = describe_choose_selection(choose);
        return Some(format!(
            "For each {quantified}, choose {selection}. Destroy one of them chosen at random"
        ));
    }

    let destroy_filter = match destroy.spec.base() {
        ChooseSpec::All(filter) | ChooseSpec::Object(filter) => filter,
        _ => return None,
    };
    if !filter_uses_chosen_tag(destroy_filter, choose.tag.as_str()) {
        return None;
    }

    let mut chosen_kind = destroy_filter.clone();
    chosen_kind.zone = None;
    chosen_kind.tagged_constraints.retain(|constraint| {
        constraint.tag != choose.tag
            || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    });
    if !chosen_kind.tagged_constraints.is_empty() {
        return None;
    }
    let chosen_kind = pluralize_noun_phrase(strip_leading_article(&chosen_kind.description()));
    let selection = describe_choose_selection(choose);
    Some(format!(
        "For each {quantified}, {chooser} {selection}. Destroy the chosen {chosen_kind}"
    ))
}

/// Recognize the executable producer/consumer pair even when lowering adds
/// render-neutral wrappers around either effect. This entry point is shared
/// by ordinary effect-list rendering and branch-local conditional rendering.
pub(in crate::compiled_text) fn describe_for_players_choose_then_destroy_chosen_collection_pair(
    producer: &Effect,
    consumer: &Effect,
) -> Option<String> {
    let for_players = structural_unwrap_render_wrappers(producer)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let destroy = destroy_effect_for_choose_compaction(consumer)?;
    describe_for_players_choose_then_destroy_chosen_collection(for_players, destroy)
}

#[cfg(test)]
mod random_participant_choice_destroy_tests {
    use super::*;

    fn choice_and_destroy(
        destroy_tag: &str,
        random: bool,
    ) -> (
        crate::effects::ForPlayersEffect,
        crate::effects::DestroyEffect,
    ) {
        let choice_tag = TagKey::from("participant_choice");
        let choose = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::permanent()
                .in_zone(Zone::Battlefield)
                .controlled_by(PlayerFilter::IteratedPlayer)
                .without_type(CardType::Land),
            ChoiceCount::exactly(1),
            PlayerFilter::You,
            choice_tag,
        )
        .in_zone(Zone::Battlefield);
        let destroy = crate::effects::DestroyEffect::with_spec(
            ChooseSpec::Tagged(TagKey::from(destroy_tag)).with_count(if random {
                ChoiceCount::exactly(1).at_random()
            } else {
                ChoiceCount::exactly(1)
            }),
        );
        (
            crate::effects::ForPlayersEffect {
                filter: PlayerFilter::Opponent,
                effects: vec![Effect::new(choose)],
                starting_with_controller: false,
                stop_after_first_happened: false,
            },
            destroy,
        )
    }

    #[test]
    fn shared_random_collection_renders_one_of_them() {
        let (players, destroy) = choice_and_destroy("participant_choice", true);
        assert_eq!(
            describe_for_players_choose_then_destroy_chosen_collection(&players, &destroy)
                .as_deref(),
            Some(
                "For each opponent, choose a nonland permanent that player controls. Destroy one of them chosen at random"
            )
        );
    }

    #[test]
    fn changed_tag_or_nonrandom_consumer_does_not_claim_random_correlation() {
        let (players, changed_tag) = choice_and_destroy("other_choice", true);
        assert!(
            describe_for_players_choose_then_destroy_chosen_collection(&players, &changed_tag)
                .is_none()
        );
        let (players, nonrandom) = choice_and_destroy("participant_choice", false);
        assert!(
            describe_for_players_choose_then_destroy_chosen_collection(&players, &nonrandom)
                .is_none()
        );
    }
}

/// Render a per-player choice followed by destruction of that same player's
/// unchosen creatures when both operations live inside one quantified loop.
/// The negative shared-tag constraint proves that "the rest" is exactly the
/// complement of the creature each player selected.
pub(super) fn describe_for_players_choose_creature_then_destroy_rest(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.filter != PlayerFilter::Any
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let [choose_effect, destroy_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::IteratedPlayer
        || choose.is_search
        || choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.count.dynamic_x
        || choose.count.random
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || choose.filter.card_types.as_slice() != [crate::types::CardType::Creature]
    {
        return None;
    }
    let mut plain_choose_filter = choose.filter.clone();
    plain_choose_filter.zone = None;
    plain_choose_filter.controller = None;
    plain_choose_filter.card_types.clear();
    if plain_choose_filter != ObjectFilter::default() {
        return None;
    }

    let destroy = structural_unwrap_render_wrappers(destroy_effect)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(destroy_filter) = &destroy.spec else {
        return None;
    };
    if destroy_filter.controller != Some(PlayerFilter::IteratedPlayer)
        || destroy_filter.zone != Some(Zone::Battlefield)
        || destroy_filter.card_types.as_slice() != [crate::types::CardType::Creature]
        || destroy_filter.tagged_constraints.as_slice()
            != [crate::filter::TaggedObjectConstraint {
                tag: choose.tag.clone(),
                relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
            }]
    {
        return None;
    }
    let mut plain_destroy_filter = destroy_filter.clone();
    plain_destroy_filter.zone = None;
    plain_destroy_filter.controller = None;
    plain_destroy_filter.card_types.clear();
    plain_destroy_filter.tagged_constraints.clear();
    if plain_destroy_filter != ObjectFilter::default() {
        return None;
    }

    Some("Each player chooses a creature they control. Destroy the rest".to_string())
}

/// Render a quantified choice followed by destroying the complement of that
/// exact chosen collection. The shared tag proves which creatures survive;
/// wrapper removal is structural and does not broaden the matched filters.
pub(in crate::compiled_text) fn describe_each_player_choose_creature_then_destroy_others_pair(
    producer: &Effect,
    consumer: &Effect,
) -> Option<String> {
    let for_players = structural_unwrap_render_wrappers(producer)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any {
        return None;
    }
    let [choose_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let chooser = match choose.chooser {
        PlayerFilter::You => "choose",
        PlayerFilter::IteratedPlayer => "that player chooses",
        _ => return None,
    };
    if choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.filter.card_types != vec![CardType::Creature]
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || choose.filter.power != Some(ironsmith_core::FilterComparison::LessThanOrEqual(2))
    {
        return None;
    }
    let destroy = structural_unwrap_render_wrappers(consumer)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(destroy_filter) = &destroy.spec else {
        return None;
    };
    if destroy_filter.card_types != vec![CardType::Creature]
        || !destroy_filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                && constraint.tag == choose.tag
        })
    {
        return None;
    }
    Some(format!(
        "for each player, {chooser} a creature with power 2 or less that player controls. Then destroy all creatures except creatures chosen this way"
    ))
}

pub(crate) fn describe_for_players_choose_types_then_sacrifice_rest(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let effects = if let [effect] = for_players.effects.as_slice()
        && let Some(sequence) = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        for_players.effects.as_slice()
    };
    let (tail, choose_effects) = effects.split_last()?;
    let sacrifice = sacrifice_view(structural_unwrap_render_wrappers(tail))?;
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
        let choose = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        chooses.push(choose);
    }
    if chooses.is_empty() {
        return None;
    }

    let keep_tag = chooses.first()?.tag.clone();
    let has_sacrifice_keep_guard = sacrifice
        .filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                && constraint.tag == keep_tag
        });
    if !has_sacrifice_keep_guard {
        return None;
    }

    if let [choose] = chooses.as_slice() {
        let choose_has_keep_guard = choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                && constraint.tag == keep_tag
        });
        if choose_primary_zone(choose) != Some(Zone::Battlefield)
            || choose.is_search
            || choose.chooser != PlayerFilter::IteratedPlayer
            || choose.aggregate_constraint.is_some()
            || choose.count.random
            || !choose_has_keep_guard
        {
            return None;
        }

        let strip_keep_guard = |filter: &ObjectFilter| {
            let mut filter = filter.clone();
            filter.tagged_constraints.retain(|constraint| {
                constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                    || constraint.tag != keep_tag
            });
            filter
        };
        let choose_base = strip_keep_guard(&choose.filter);
        let sacrifice_base = strip_keep_guard(sacrifice.filter);
        if choose_base != sacrifice_base
            || choose_base.controller != Some(PlayerFilter::IteratedPlayer)
        {
            return None;
        }

        let mut display_choose = (**choose).clone();
        display_choose.filter = choose_base;
        let selection = describe_counted_sacrifice_choice_selection(&display_choose)?;
        let (subject, choose_verb, sacrifice_verb, controls) = match for_players.filter {
            PlayerFilter::Any => ("Each player", "chooses", "sacrifices", "they control"),
            PlayerFilter::Opponent => ("Each opponent", "chooses", "sacrifices", "they control"),
            PlayerFilter::You => ("You", "choose", "sacrifice", "you control"),
            _ => return None,
        };
        return Some(format!(
            "{subject} {choose_verb} {selection} {controls}, then {sacrifice_verb} the rest"
        ));
    }

    let choose_has_common_keep_shape = |choose: &crate::effects::ChooseObjectsEffect| {
        choose_primary_zone(choose) == Some(Zone::Battlefield)
            && !choose.is_search
            && choose.chooser == PlayerFilter::IteratedPlayer
            && choose.tag == keep_tag
            && choose.filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                    && constraint.tag == keep_tag
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
        .filter(|&choose| {
            choose_has_common_keep_shape(choose)
                && choose.count == ChoiceCount::up_to(1)
                && choose.filter.card_types.as_slice() == [CardType::Creature]
                && choose.filter.subtypes.len() == 1
        })
        .map(|choose| choose.filter.subtypes[0])
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
        let card_type = *choose.filter.card_types.first()?;
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

fn choice_count_is_half_rounded_down_of_filter(
    choose: &crate::effects::ChooseObjectsEffect,
) -> bool {
    let Some(Value::HalfRoundedDown(inner)) = choose.count_value.as_ref() else {
        return false;
    };
    let Value::Count(filter) = inner.as_ref() else {
        return false;
    };
    if filter == &choose.filter {
        return true;
    }
    // Lowering may rewrite the chooser-facing filter's controller reference
    // (target back-references) without touching the count basis copy.
    let mut left = filter.clone();
    let mut right = choose.filter.clone();
    left.controller = None;
    right.controller = None;
    left == right
}

fn choice_count_filter_matches(
    count_filter: &ObjectFilter,
    choose: &crate::effects::ChooseObjectsEffect,
) -> bool {
    if count_filter == &choose.filter {
        return true;
    }
    // Player references can be rebound independently on the chooser-facing
    // filter and the copied count basis. The remaining filter structure still
    // proves that both sides count the exact selectable set.
    let mut left = count_filter.clone();
    let mut right = choose.filter.clone();
    left.controller = None;
    right.controller = None;
    left == right
}

fn choice_count_is_half_rounded_up_of_filter(choose: &crate::effects::ChooseObjectsEffect) -> bool {
    let Some(Value::HalfRoundedDown(inner)) = choose.count_value.as_ref() else {
        return false;
    };
    let Value::Add(left, right) = inner.as_ref() else {
        return false;
    };
    matches!(
        (left.as_ref(), right.as_ref()),
        (Value::Count(filter), Value::Fixed(1))
            | (Value::Fixed(1), Value::Count(filter))
            if choice_count_filter_matches(filter, choose)
    )
}

fn choice_count_unit_fraction_of_filter(
    choose: &crate::effects::ChooseObjectsEffect,
) -> Option<(u32, &'static str)> {
    let Value::DividedRoundedDown(inner, divisor) = choose.count_value.as_ref()? else {
        return None;
    };
    let denominator = u32::try_from(*divisor).ok().filter(|value| *value > 1)?;
    let (count_filter, rounding) = match inner.as_ref() {
        Value::Count(filter) => (filter, "down"),
        Value::Add(left, right) => match (left.as_ref(), right.as_ref()) {
            (Value::Count(filter), Value::Fixed(offset))
            | (Value::Fixed(offset), Value::Count(filter))
                if *offset == divisor.saturating_sub(1) =>
            {
                (filter, "up")
            }
            _ => return None,
        },
        _ => return None,
    };
    choice_count_filter_matches(count_filter, choose).then_some((denominator, rounding))
}

fn choice_count_all_except_of_filter(choose: &crate::effects::ChooseObjectsEffect) -> Option<u32> {
    let Value::Add(left, right) = choose.count_value.as_ref()? else {
        return None;
    };
    let (count_filter, offset) = match (left.as_ref(), right.as_ref()) {
        (Value::Count(filter), Value::Fixed(offset))
        | (Value::Fixed(offset), Value::Count(filter)) => (filter, *offset),
        _ => return None,
    };
    let keep_count = offset
        .checked_neg()
        .and_then(|value| u32::try_from(value).ok())?;
    (keep_count > 0 && choice_count_filter_matches(count_filter, choose)).then_some(keep_count)
}

fn unit_fraction_quantifier(denominator: u32) -> Option<String> {
    if denominator == 2 {
        Some("half the".to_string())
    } else {
        ironsmith_core::ordinal_word(denominator).map(|ordinal| format!("a {ordinal} of the"))
    }
}

fn sacrifice_choice_players_match(left: &PlayerFilter, right: &PlayerFilter) -> bool {
    player_filters_refer_to_same_player(left, right)
        || matches!(
            (left, right),
            (
                PlayerFilter::ControllerOf(left_ref),
                PlayerFilter::AliasedControllerOf(right_ref),
            )
                | (
                    PlayerFilter::AliasedControllerOf(left_ref),
                    PlayerFilter::ControllerOf(right_ref),
                )
                | (
                    PlayerFilter::OwnerOf(left_ref),
                    PlayerFilter::AliasedOwnerOf(right_ref),
                )
                | (
                    PlayerFilter::AliasedOwnerOf(left_ref),
                    PlayerFilter::OwnerOf(right_ref),
                ) if left_ref == right_ref
        )
}

pub(super) fn sacrifice_tracks_exact_sentence_helper_chosen_set(
    sacrifice: SacrificeView<'_>,
    choose: &crate::effects::ChooseObjectsEffect,
) -> bool {
    if !crate::cards::is_sentence_helper_tag(choose.tag.as_str(), "sacrificed") {
        return false;
    }

    let chosen_set = ObjectFilter::tagged(choose.tag.clone());
    sacrifice.filter == &chosen_set
        && matches!(sacrifice.count, Value::Count(count_filter) if count_filter == &chosen_set)
}

pub(super) fn describe_sacrifice_choice_kind(
    choose: &crate::effects::ChooseObjectsEffect,
) -> String {
    let mut filter = choose.filter.clone();
    filter.tagged_constraints.retain(|constraint| {
        constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            || constraint.tag != choose.tag
    });
    if choose_primary_zone(choose) == Some(Zone::Battlefield) {
        filter.zone = None;
    }
    if filter
        .controller
        .as_ref()
        .is_some_and(|controller| sacrifice_choice_players_match(controller, &choose.chooser))
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
        (1, None) => Some(format!("one or more {plural}")),
        (min, None) => Some(format!("at least {min} {plural}")),
    }
}

pub(crate) fn describe_for_players_choose_then_sacrifice(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let effects = match for_players.effects.as_slice() {
        [effect] => effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .map(|sequence| sequence.effects.as_slice())
            .unwrap_or(for_players.effects.as_slice()),
        effects => effects,
    };
    let [choose_effect, sacrifice_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifices_exact_choice = sacrifice_effect
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()
        .is_some_and(|sacrifice| {
            matches!(sacrifice.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        })
        || sacrifice_view(sacrifice_effect).is_some_and(|sacrifice| {
            sacrifice.player == &PlayerFilter::IteratedPlayer
                && matches!(sacrifice.count, Value::Fixed(1))
                && filter_is_exactly_tagged(sacrifice.filter, &choose.tag)
        });
    if sacrifices_exact_choice
        && choose_primary_zone(choose) == Some(Zone::Battlefield)
        && !choose.is_search
        && choose.chooser == PlayerFilter::IteratedPlayer
        && choose.count.is_single()
        && choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SharesCardType
        })
    {
        let (subject, sacrifice_verb, possessive) = match for_players.filter {
            PlayerFilter::Any => ("Each player", "sacrifices", "their"),
            PlayerFilter::Opponent => ("Each opponent", "sacrifices", "their"),
            PlayerFilter::You => ("You", "sacrifice", "your"),
            _ => return None,
        };
        let mut kind = choose.filter.clone();
        kind.zone = None;
        kind.controller = None;
        let selection = with_indefinite_article(&kind.description());
        let selection = if let Some((head, tail)) = selection.split_once(" that shares ") {
            format!("{head} of {possessive} choice that shares {tail}")
        } else {
            format!("{selection} of {possessive} choice")
        };
        return Some(format!("{subject} {sacrifice_verb} {selection}"));
    }

    let sacrifice = sacrifice_view(sacrifice_effect)?;
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

    let (subject, verb, actor, possessive) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "sacrifices", "they", "their"),
        PlayerFilter::Opponent => ("Each opponent", "sacrifices", "they", "their"),
        PlayerFilter::You => ("You", "sacrifice", "you", "your"),
        _ => return None,
    };
    if choose.count.is_single()
        && choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SharesCardType
        })
    {
        let mut kind = choose.filter.clone();
        kind.zone = None;
        kind.controller = None;
        let selection = with_indefinite_article(&kind.description());
        let selection = if let Some((head, tail)) = selection.split_once(" that shares ") {
            format!("{head} {actor} control that shares {tail}")
        } else {
            format!("{selection} {actor} control")
        };
        let choose_verb = if subject == "You" {
            "choose"
        } else {
            "chooses"
        };
        return Some(format!("{subject} {choose_verb} {selection} and {verb} it"));
    }
    if let Some(chosen) = describe_greatest_power_choice_filter(&choose.filter) {
        let chosen = with_indefinite_article(&chosen);
        return Some(format!("{subject} {verb} {chosen}"));
    }
    if choose.count.is_dynamic_x()
        && let Some((denominator, rounding)) = choice_count_unit_fraction_of_filter(choose)
        && matches!(
            choose.filter.controller.as_ref(),
            Some(&PlayerFilter::IteratedPlayer)
        )
    {
        let fraction = unit_fraction_quantifier(denominator)?;
        let kind = pluralize_noun_phrase(&describe_sacrifice_choice_kind(choose));
        return Some(format!(
            "{subject} {verb} {fraction} {kind} {actor} control of {possessive} choice, rounded {rounding}"
        ));
    }
    if choose.count.is_dynamic_x()
        && let Some(keep_count) = choice_count_all_except_of_filter(choose)
        && matches!(
            choose.filter.controller.as_ref(),
            Some(&PlayerFilter::IteratedPlayer)
        )
    {
        let kind = pluralize_noun_phrase(&describe_sacrifice_choice_kind(choose));
        let keep_count = number_word(keep_count as i32).unwrap_or_else(|| keep_count.to_string());
        return Some(format!(
            "{subject} {verb} all {kind} {actor} control except for {keep_count}"
        ));
    }
    if choose.count.is_dynamic_x()
        && choice_count_is_half_rounded_up_of_filter(choose)
        && matches!(
            choose.filter.controller.as_ref(),
            Some(&PlayerFilter::IteratedPlayer)
        )
    {
        let kind = pluralize_noun_phrase(&describe_sacrifice_choice_kind(choose));
        return Some(format!(
            "{subject} {verb} half the {kind} {actor} control of {possessive} choice, rounded up"
        ));
    }
    if choose.count.is_dynamic_x() && choice_count_is_half_rounded_down_of_filter(choose) {
        let kind = pluralize_noun_phrase(&describe_sacrifice_choice_kind(choose));
        return Some(format!(
            "{subject} {verb} half the {kind} {actor} control of {possessive} choice, rounded down"
        ));
    }
    let chosen = describe_counted_sacrifice_choice_selection(choose)?;
    Some(format!("{subject} {verb} {chosen} of {possessive} choice"))
}

fn correlated_choice_comparison_filter(filter: &ObjectFilter, tag: &crate::TagKey) -> bool {
    if filter.controller != Some(PlayerFilter::IteratedPlayer) {
        return false;
    }
    let mut stripped = filter.clone();
    stripped.controller = None;
    stripped == ObjectFilter::tagged(tag.clone())
}

fn correlated_choice_slot_label(
    choose: &crate::effects::ChooseObjectsEffect,
    tag: &crate::TagKey,
) -> Option<String> {
    if choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.filter.controller != Some(PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.controller = None;
    filter.tagged_constraints.retain(|constraint| {
        constraint.tag != *tag
            || constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    });
    if !filter.tagged_constraints.is_empty()
        || !filter.no_shared_creature_types_with.is_empty()
        || !filter.any_of.is_empty()
    {
        return None;
    }
    Some(strip_leading_article(&filter.description()).to_string())
}

/// Render two player loops that form one locked-choice/complement procedure.
/// The first loop records every player's chosen set; only after those choices
/// are complete does the second loop perform the sacrifice.
pub(crate) fn describe_split_for_players_choose_then_sacrifice(
    effects: &[Effect],
) -> Option<String> {
    let [choice_loop_effect, sacrifice_loop_effect] = effects else {
        return None;
    };
    let choice_loop = choice_loop_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let sacrifice_loop =
        sacrifice_loop_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if choice_loop.filter != PlayerFilter::Any
        || sacrifice_loop.filter != PlayerFilter::Any
        || choice_loop.starting_with_controller
        || sacrifice_loop.starting_with_controller
        || choice_loop.stop_after_first_happened
        || sacrifice_loop.stop_after_first_happened
        || choice_loop.effects.is_empty()
        || sacrifice_loop.effects.len() != 1
    {
        return None;
    }

    let first_choice =
        choice_loop.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let tag = &first_choice.tag;
    let mut slots = Vec::new();
    for effect in &choice_loop.effects {
        let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        if choose.tag != *tag {
            return None;
        }
        slots.push(correlated_choice_slot_label(choose, tag)?);
    }

    let sacrifice = sacrifice_view(&sacrifice_loop.effects[0])?;
    if sacrifice.player != &PlayerFilter::IteratedPlayer
        || !matches!(sacrifice.count, Value::Count(filter) if filter == sacrifice.filter)
        || sacrifice.filter.controller != Some(PlayerFilter::IteratedPlayer)
        || !sacrifice
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == *tag
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            })
    {
        return None;
    }

    let mut sacrificed = sacrifice.filter.clone();
    sacrificed.controller = None;
    sacrificed.other = false;
    sacrificed.tagged_constraints.retain(|constraint| {
        constraint.tag != *tag
            || constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    });
    let comparisons = std::mem::take(&mut sacrificed.no_shared_creature_types_with);
    if !sacrificed.tagged_constraints.is_empty() {
        return None;
    }
    let sacrificed_kind = pluralize_noun_phrase(strip_leading_article(&sacrificed.description()));
    let comparison_tail = match comparisons.as_slice() {
        [] => String::new(),
        [comparison] if correlated_choice_comparison_filter(comparison, tag) => format!(
            " that don't share a creature type with the chosen {} they control",
            slots.first()?
        ),
        _ => return None,
    };

    let choice_clause = if slots.len() == 1 {
        format!(
            "For each player, you choose {} that player controls",
            with_indefinite_article(&slots[0])
        )
    } else {
        let choices = slots
            .iter()
            .map(|slot| with_indefinite_article(slot))
            .collect::<Vec<_>>();
        format!(
            "For each player, you choose from among the permanents that player controls {}",
            join_with_and(&choices)
        )
    };
    Some(format!(
        "{choice_clause}. Then each player sacrifices all other {sacrificed_kind} they control{comparison_tail}"
    ))
}

pub(in crate::compiled_text) fn describe_for_players_choose_graveyard_then_exile_rest(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.starting_with_controller || for_players.stop_after_first_happened {
        return None;
    }
    let [choose_effect, move_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose_primary_zone(choose) != Some(Zone::Graveyard)
        || choose.filter.zone != Some(Zone::Graveyard)
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose.filter.controller.is_some()
        || choose.chooser != PlayerFilter::IteratedPlayer
        || choose.is_search
        || choose.reveal
        || choose.aggregate_constraint.is_some()
        || !choose.additional_zones.is_empty()
        || choose.top_only
        || choose.bottom_only
        || choose.count.random
    {
        return None;
    }
    let move_to_zone = structural_unwrap_render_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Exile
        || move_to_zone.library_order.is_some()
        || move_to_zone.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Canonical
        || move_to_zone.actor_surface != Some(PlayerFilter::IteratedPlayer)
        || move_to_zone.destination_player_surface.is_some()
        || move_to_zone.exiled_with_source_surface.is_some()
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || !move_to_zone.enters_with_counters.is_empty()
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.enters_transformed
    {
        return None;
    }
    let ChooseSpec::Object(complement) = move_to_zone.target.base() else {
        return None;
    };
    let mut expected_complement = choose.filter.clone();
    expected_complement
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: choose.tag.clone(),
            relation: crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
        });
    if complement != &expected_complement {
        return None;
    }

    let (subject, choose_verb, exile_verb, possessive) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "chooses", "exiles", "their"),
        PlayerFilter::Opponent => ("Each opponent", "chooses", "exiles", "their"),
        PlayerFilter::You => ("You", "choose", "exile", "your"),
        _ => return None,
    };
    let mut display_choose = choose.clone();
    display_choose.filter.zone = None;
    display_choose.filter.owner = None;
    display_choose.zone = None;
    let selection = describe_counted_sacrifice_choice_selection(&display_choose)?;
    Some(format!(
        "{subject} {choose_verb} {selection} in {possessive} graveyard and {exile_verb} the rest"
    ))
}

pub(in crate::compiled_text) fn describe_for_players_choose_then_exile(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if let Some(rendered) = describe_for_players_choose_graveyard_then_exile_rest(for_players) {
        return Some(rendered);
    }
    let effects = match for_players.effects.as_slice() {
        [effect]
            if effect
                .downcast_ref::<crate::effects::SequenceEffect>()
                .is_some_and(|sequence| {
                    sequence.surface == ironsmith_core::SequenceSurface::CommaThen
                        && sequence.result_label.is_none()
                }) =>
        {
            &effect
                .downcast_ref::<crate::effects::SequenceEffect>()?
                .effects
        }
        effects => effects,
    };
    let [choose_effect, exile_effect] = effects else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if let Some(exile) =
        unwrap_basic_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()
        && choose_primary_zone(choose) == Some(Zone::Library)
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
    if let Some(exile) =
        unwrap_basic_tag_wrappers(exile_effect).downcast_ref::<crate::effects::ExileEffect>()
        && !exile.face_down
        && !exile.turn_face_up
        && choose_primary_zone(choose) == Some(Zone::Battlefield)
        && !choose.is_search
        && choose.count.is_single()
        && choose.chooser == PlayerFilter::IteratedPlayer
        && choose.filter.controller == Some(PlayerFilter::IteratedPlayer)
        && exile_uses_chosen_tag(&exile.spec, choose.tag.as_str())
    {
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
        return Some(format!(
            "{subject} {choose_verb} {selection} and {exile_verb} it"
        ));
    }
    let move_to_zone = exile_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if choose_primary_zone(choose) == Some(Zone::Hand)
        && choose.additional_zones.contains(&Zone::Battlefield)
        && !choose.is_search
        && choose.chooser == PlayerFilter::IteratedPlayer
        && choose_filter_is_iterated_hand_card_or_permanent(choose)
        && move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str())
    {
        let (subject, actor, possessive, singular_object) = match for_players.filter {
            PlayerFilter::Any => (
                "Each player",
                "they",
                "their",
                "a card from their hand or a permanent they control",
            ),
            PlayerFilter::Opponent => (
                "Each opponent",
                "they",
                "their",
                "a card from their hand or a permanent they control",
            ),
            PlayerFilter::You => (
                "You",
                "you",
                "your",
                "a card from your hand or a permanent you control",
            ),
            _ => return None,
        };
        let verb = if subject == "You" { "exile" } else { "exiles" };
        if choose.count.is_single() {
            return Some(format!("{subject} {verb} {singular_object}"));
        }
        let count = describe_runtime_choice_count(choose)?;
        let where_clause = describe_runtime_choice_where_clause(choose).unwrap_or_default();
        return Some(format!(
            "{subject} {verb} {count} permanents {actor} control and/or cards from {possessive} hand{where_clause}"
        ));
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

#[cfg(test)]
mod quantified_cross_zone_exile_tests {
    use super::*;

    fn cross_zone_loop(
        surface: ironsmith_core::SequenceSurface,
        move_tag: &str,
        additional_zones: Vec<Zone>,
    ) -> crate::effects::ForPlayersEffect {
        let mut filter = ObjectFilter::default().controlled_by(PlayerFilter::IteratedPlayer);
        filter.any_of = vec![
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::IteratedPlayer),
            ObjectFilter::permanent()
                .in_zone(Zone::Battlefield)
                .controlled_by(PlayerFilter::IteratedPlayer),
        ];
        let tag = TagKey::from("cross_zone_choice");
        let mut choose = crate::effects::ChooseObjectsEffect::new(
            filter,
            ChoiceCount::dynamic_x(),
            PlayerFilter::IteratedPlayer,
            tag,
        )
        .with_count_value(
            Value::counters_on_source_reference(
                Some(crate::object::CounterType::Named("despair".into())),
                Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this enchantment".to_string(),
                )),
            )
            .with_surface_hint(ValueSurfaceHint::WhereXIs),
        );
        choose.zone = Some(Zone::Hand);
        choose.additional_zones = additional_zones;
        let exile = Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(TagKey::from(move_tag)),
            Zone::Exile,
            true,
        ));
        let sequence = Effect::new(crate::effects::SequenceEffect {
            effects: vec![Effect::new(choose), exile],
            surface,
            result_label: None,
        });
        crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![sequence])
    }

    #[test]
    fn comma_then_wrapper_preserves_quantified_cross_zone_exile_surface() {
        let for_players = cross_zone_loop(
            ironsmith_core::SequenceSurface::CommaThen,
            "cross_zone_choice",
            vec![Zone::Battlefield],
        );
        assert_eq!(
            describe_for_players_choose_then_exile(&for_players).as_deref(),
            Some(
                "Each player exiles X permanents they control and/or cards from their hand, where X is the number of despair counters on this enchantment"
            )
        );
    }

    #[test]
    fn changed_wrapper_tag_or_zone_is_not_compacted() {
        let sequential = cross_zone_loop(
            ironsmith_core::SequenceSurface::Sequential,
            "cross_zone_choice",
            vec![Zone::Battlefield],
        );
        assert!(describe_for_players_choose_then_exile(&sequential).is_none());

        let changed_tag = cross_zone_loop(
            ironsmith_core::SequenceSurface::CommaThen,
            "different_choice",
            vec![Zone::Battlefield],
        );
        assert!(describe_for_players_choose_then_exile(&changed_tag).is_none());

        let changed_zone = cross_zone_loop(
            ironsmith_core::SequenceSurface::CommaThen,
            "cross_zone_choice",
            vec![Zone::Graveyard],
        );
        assert!(describe_for_players_choose_then_exile(&changed_zone).is_none());
    }
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

pub(crate) fn describe_for_players_bottom_library_exile_then_look_cast(
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

    let mana_clause = grant.mana_spend_cast_clause("those spells")?;
    Some(format!(
        "{exile_clause}. For as long as those cards remain exiled, you may look at them, you may cast permanent spells from among them, and {mana_clause}"
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

pub(crate) fn describe_for_players_choose_move_then_characteristics(
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

/// The same typed procedure after lowering coalesces the permanent color,
/// subtype, and decayed grants into one continuous effect. Every executable
/// modification is still checked before the internal decayed implementation
/// abilities are collapsed back to the keyword surface.
pub(crate) fn describe_for_players_choose_move_then_combined_characteristics(
    effects: &[&Effect],
) -> Option<String> {
    let [for_players_effect, move_effect, apply_effect] = effects else {
        return None;
    };
    let for_players = for_players_effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let move_to_zone = unwrap_basic_tag_wrappers(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let base = describe_for_players_choose_then_move_to_battlefield(for_players, move_to_zone)?;
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let apply = tagged_apply_continuous_effect(apply_effect)?;
    if !apply_continuous_is_forever_tagged(apply, &choose.tag)
        || !apply.runtime_modifications.is_empty()
    {
        return None;
    }

    let mut colors = None;
    let mut subtypes = None;
    let mut decayed = false;
    let mut cant_block = false;
    let mut decayed_delayed_sacrifice = false;
    for modification in apply
        .modification
        .iter()
        .chain(apply.additional_modifications.iter())
    {
        match modification {
            crate::continuous::Modification::AddColors(value) if colors.is_none() => {
                colors = Some(*value);
            }
            crate::continuous::Modification::AddSubtypes(value) if subtypes.is_none() => {
                subtypes = Some(value);
            }
            crate::continuous::Modification::AddAbility(ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
                    && ability.display().eq_ignore_ascii_case("decayed")
                    && !decayed =>
            {
                decayed = true;
            }
            crate::continuous::Modification::AddAbility(ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::CantBlock
                    && !cant_block =>
            {
                cant_block = true;
            }
            crate::continuous::Modification::AddAbilityGeneric(_) if !decayed_delayed_sacrifice => {
                decayed_delayed_sacrifice = true;
            }
            _ => return None,
        }
    }
    let (Some(colors), Some(subtypes)) = (colors, subtypes) else {
        return None;
    };
    if colors.is_empty()
        || subtypes.is_empty()
        || !decayed
        || !cant_block
        || !decayed_delayed_sacrifice
    {
        return None;
    }

    let subtype_words = pluralize_noun_phrase(
        &subtypes
            .iter()
            .map(|subtype| subtype.display_name())
            .collect::<Vec<_>>()
            .join(" "),
    );
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
    let any_number = choose.count.min == 0
        && choose.count.max.is_none()
        && !choose.count.dynamic_x
        && !choose.count.up_to_x
        && !choose.count.random;
    if choose.is_search
        || choose.chooser != PlayerFilter::IteratedPlayer
        || (!choose.count.is_single() && !any_number)
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
    let choice = if any_number {
        format!(
            "any number of {}",
            pluralize_noun_phrase(strip_leading_article(&choice).trim())
        )
    } else {
        with_indefinite_article(strip_leading_article(&choice).trim())
    };
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

pub(crate) fn describe_target_player_permanent_piles_sacrifice(
    effects: &[Effect],
) -> Option<String> {
    let mut choose = None;
    let mut sacrifice = None;
    let mut saw_target_declaration = false;

    for effect in effects {
        let unwrapped = unwrap_basic_tag_wrappers(effect);
        if let Some(target) = unwrapped.downcast_ref::<crate::effects::TargetOnlyEffect>() {
            if saw_target_declaration
                || target.explicit_declaration
                || target.chooser.is_some()
                || target.target != ChooseSpec::target_player()
            {
                return None;
            }
            saw_target_declaration = true;
            continue;
        }
        if let Some(found) = unwrapped.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            if choose.replace(found).is_some() {
                return None;
            }
            continue;
        }
        if let Some(found) = sacrifice_view_unwrapped(effect) {
            if sacrifice.replace(found).is_some() {
                return None;
            }
            continue;
        }
        return None;
    }

    let choose = choose?;
    let sacrifice = sacrifice?;
    let expected_filter = ObjectFilter::permanent().controlled_by(PlayerFilter::target_player());
    if choose.filter != expected_filter
        || choose.count != ChoiceCount::any_number()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.chooser != PlayerFilter::target_player()
        || choose_primary_zone(choose) != Some(Zone::Battlefield)
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || sacrifice.player != &PlayerFilter::target_player()
        || !filter_is_exactly_tagged(sacrifice.filter, &choose.tag)
        || !matches!(
            sacrifice.count,
            Value::Count(count_filter) if count_filter == sacrifice.filter
        )
    {
        return None;
    }

    Some(
        "Separate all permanents target player controls into two piles. That player sacrifices all permanents in the pile of their choice"
            .to_string(),
    )
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
    let players_match = sacrifice_choice_players_match(sacrifice.player, &choose.chooser);
    let exact_sentence_helper_set =
        sacrifice_tracks_exact_sentence_helper_chosen_set(sacrifice, choose);
    if choose_primary_zone(choose).is_some_and(|zone| zone != Zone::Battlefield)
        || choose.is_search
        || (!players_match && !exact_sentence_helper_set)
        || !sacrifice_uses_chosen_tag(sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    // A sentence-helper choice is parser scaffolding for a sacrifice, not a
    // separately authored choice. Reference annotation can preserve the
    // carried player on the sacrifice while the implicit choice lowers to
    // `You`. Keep that carried actor for the player-or-planeswalker follow-up;
    // standalone costs and as-enters sacrifices retain their implicit `You`.
    let render_player = if !players_match
        && exact_sentence_helper_set
        && sacrifice.player != &PlayerFilter::You
        && sacrifice.player != &PlayerFilter::IteratedPlayer
    {
        sacrifice.player
    } else {
        &choose.chooser
    };
    let player = describe_player_filter(render_player);
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
    let chooser_controls_chosen = choose
        .filter
        .controller
        .as_ref()
        .is_some_and(|controller| sacrifice_choice_players_match(controller, &choose.chooser))
        && choose.chooser != PlayerFilter::You;
    let chosen = if chooser_controls_chosen {
        let mut chosen_filter = choose.filter.clone();
        chosen_filter.controller = None;
        chosen_filter.description()
    } else if choose.chooser == PlayerFilter::You
        && choose.filter.controller == Some(PlayerFilter::You)
        && !choose.filter.other
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
        if choose.count.is_dynamic_x()
            && let Some(keep_count) = choice_count_all_except_of_filter(choose)
        {
            let kind = pluralize_noun_phrase(&describe_sacrifice_choice_kind(choose));
            let controls = choose.filter.controller.as_ref().is_some_and(|controller| {
                sacrifice_choice_players_match(controller, &choose.chooser)
            });
            let control_suffix = if controls {
                if render_player == &PlayerFilter::You {
                    " you control"
                } else {
                    " they control"
                }
            } else {
                ""
            };
            let keep_count =
                number_word(keep_count as i32).unwrap_or_else(|| keep_count.to_string());
            return Some(format!(
                "{player} {verb} all {kind}{control_suffix} except for {keep_count}"
            ));
        }
        let fraction = if choose.count.is_dynamic_x()
            && let Some((denominator, rounded)) = choice_count_unit_fraction_of_filter(choose)
        {
            Some((unit_fraction_quantifier(denominator)?, rounded))
        } else if choose.count.is_dynamic_x() && choice_count_is_half_rounded_up_of_filter(choose) {
            Some(("half the".to_string(), "up"))
        } else if choose.count.is_dynamic_x() && choice_count_is_half_rounded_down_of_filter(choose)
        {
            Some(("half the".to_string(), "down"))
        } else {
            None
        };
        if let Some((fraction, rounded)) = fraction {
            let kind = pluralize_noun_phrase(&describe_sacrifice_choice_kind(choose));
            let controls = choose.filter.controller.as_ref().is_some_and(|controller| {
                sacrifice_choice_players_match(controller, &choose.chooser)
            });
            let control_suffix = if controls {
                if render_player == &PlayerFilter::You {
                    " you control"
                } else {
                    " they control"
                }
            } else {
                ""
            };
            let choice_suffix = if render_player != &PlayerFilter::You {
                " of their choice"
            } else {
                ""
            };
            return Some(format!(
                "{player} {verb} {fraction} {kind}{control_suffix}{choice_suffix}, rounded {rounded}"
            ));
        }
        let selection = describe_counted_sacrifice_choice_selection(choose)?;
        let choice_suffix = if render_player != &PlayerFilter::You {
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
        if choose.filter.has_one_of_tagged_set_surface() {
            return Some(format!("{player} {verb} one of them"));
        }
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
        let authored_each = sacrifice.filter.set_quantifier_surface()
            == Some(ironsmith_core::SetQuantifierSurface::Each);
        let subject = if authored_each {
            noun.strip_prefix("another ")
                .map(|rest| format!("other {rest}"))
                .unwrap_or(noun)
        } else {
            pluralize_noun_phrase(&noun)
        };
        if authored_each {
            if matches!(sacrifice.player, PlayerFilter::You) {
                return format!("Sacrifice each {subject}");
            }
            return format!("{player} {verb} each {subject}");
        }
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
    describe_choose_then_for_each_copy_effects(choose, &for_each.effects)
}

pub(crate) fn describe_choose_any_number_then_remove_counter_from_each(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if choose.is_search
        || choose.count != ChoiceCount::any_number()
        || choose.count_value.is_some()
        || choose.chooser != PlayerFilter::You
        || for_each.tag != choose.tag
    {
        return None;
    }
    let [remove_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let remove = unwrap_basic_tag_wrappers(remove_effect)
        .downcast_ref::<crate::effects::RemoveCountersEffect>()?;
    let removes_iterated = matches!(remove.target.unhinted(), ChooseSpec::Iterated)
        || matches!(
            remove.target.unhinted(),
            ChooseSpec::Tagged(tag) if is_implicit_reference_tag(tag.as_str())
        );
    if !removes_iterated {
        return None;
    }

    let selection =
        describe_choose_spec(&ChooseSpec::Object(choose.filter.clone()).with_count(choose.count));
    let counter_phrase =
        describe_remove_counter_phrase(&remove.count, remove.counter_type, &remove.target);
    Some(format!("Remove {counter_phrase} from each of {selection}"))
}

pub(crate) fn describe_choose_then_for_each_object_copy(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachObject,
) -> Option<String> {
    if !for_each.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == choose.tag
    }) {
        return None;
    }
    describe_choose_then_for_each_copy_effects(choose, &for_each.effects)
}

fn describe_choose_then_for_each_copy_effects(
    choose: &crate::effects::ChooseObjectsEffect,
    effects: &[Effect],
) -> Option<String> {
    if choose.is_search || choose.count.is_single() || effects.len() != 1 {
        return None;
    }
    let create_copy = unwrap_basic_tag_wrappers(&effects[0])
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    let target_matches = matches!(create_copy.target.unhinted(), ChooseSpec::Iterated)
        || matches!(create_copy.target.unhinted(), ChooseSpec::Tagged(tag) if tag == &choose.tag);
    if !target_matches {
        return None;
    }
    let has_unsupported_modifier = create_copy.controller != PlayerFilter::You
        || create_copy.enters_tapped
        || create_copy.has_haste
        || create_copy.loses_soulbond
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
        || create_copy.set_base_power_toughness_value.is_some()
        || create_copy.set_colors.is_some()
        || create_copy.set_card_types.is_some()
        || create_copy.set_subtypes.is_some()
        || !create_copy.granted_static_abilities.is_empty();
    if has_unsupported_modifier {
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
        let leading_duration = cant.duration_surface
            == crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn;
        if let Some(allowed) = describe_except_by_characteristic_blockers(blockers)
            .or_else(|| describe_except_by_subtype_blockers(blockers))
        {
            let body = if leading_duration {
                format!("{subject} can't be blocked except by {allowed}")
            } else {
                format!("{subject} can't be blocked this turn except by {allowed}")
            };
            return Some(if leading_duration {
                format!("Until end of turn, {}", lowercase_first(&body))
            } else {
                body
            });
        }
        let blockers = describe_blocker_union(blockers).unwrap_or_else(|| {
            pluralize_noun_phrase(strip_leading_article(&blockers.description()))
        });
        let body = if leading_duration {
            format!("{subject} can't be blocked by {blockers}")
        } else {
            format!("{subject} can't be blocked by {blockers} this turn")
        };
        return Some(if leading_duration {
            format!("Until end of turn, {}", lowercase_first(&body))
        } else {
            body
        });
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

pub(super) fn describe_except_by_characteristic_blockers(
    blockers: &ObjectFilter,
) -> Option<String> {
    let allowed_types = blockers
        .excluded_card_types
        .iter()
        .copied()
        .filter(|card_type| *card_type != CardType::Creature)
        .collect::<Vec<_>>();
    if allowed_types.is_empty() && blockers.excluded_colors.is_empty() {
        return None;
    }

    let mut expected = ObjectFilter::creature();
    expected.set_union_connective(blockers.union_connective());
    for card_type in &allowed_types {
        expected = expected.without_type(*card_type);
    }
    expected = expected.without_colors(blockers.excluded_colors);
    if *blockers != expected {
        return None;
    }

    let mut allowed = allowed_types
        .into_iter()
        .map(|card_type| format!("{} creatures", describe_card_type_word_local(card_type)))
        .collect::<Vec<_>>();
    allowed.extend(
        crate::color::Color::ALL
            .into_iter()
            .filter(|color| blockers.excluded_colors.contains(*color))
            .map(|color| format!("{} creatures", color.name())),
    );
    let connective = match blockers.union_connective() {
        crate::filter::ObjectFilterUnionConnective::Or => " or ",
        crate::filter::ObjectFilterUnionConnective::AndOr => " and/or ",
    };
    Some(allowed.join(connective))
}

pub(super) fn describe_blocker_union(blockers: &ObjectFilter) -> Option<String> {
    if blockers.any_of.len() < 2 {
        return None;
    }

    let mut expected = ObjectFilter::default();
    expected.any_of = blockers.any_of.clone();
    expected.set_union_connective(blockers.union_connective());
    if *blockers != expected {
        return None;
    }

    let connective = match blockers.union_connective() {
        crate::filter::ObjectFilterUnionConnective::Or => " or ",
        crate::filter::ObjectFilterUnionConnective::AndOr => " and/or ",
    };
    Some(
        blockers
            .any_of
            .iter()
            .map(|branch| pluralize_noun_phrase(strip_leading_article(&branch.description())))
            .collect::<Vec<_>>()
            .join(connective),
    )
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

#[cfg(test)]
mod source_exiled_plural_surface_tests {
    use super::*;

    #[test]
    fn source_exiled_relative_clause_pluralizes_the_card_noun() {
        assert_eq!(
            pluralize_noun_phrase("card exiled with this creature"),
            "cards exiled with this creature"
        );
        assert_eq!(
            pluralize_noun_phrase("cards exiled with this artifact"),
            "cards exiled with this artifact"
        );
    }
}

#[cfg(test)]
mod except_by_blocker_tests {
    use super::*;

    #[test]
    fn characteristic_exclusions_recover_the_allowed_and_or_surface() {
        let mut blockers = ObjectFilter::creature()
            .without_type(CardType::Artifact)
            .without_colors(crate::color::ColorSet::RED);
        blockers.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);

        assert_eq!(
            describe_except_by_characteristic_blockers(&blockers).as_deref(),
            Some("artifact creatures and/or red creatures")
        );
    }

    #[test]
    fn blocker_union_pluralizes_each_independent_arm() {
        let mut blockers = ObjectFilter::default();
        blockers.any_of = vec![
            ObjectFilter::creature().with_power(crate::filter::Comparison::LessThanOrEqual(2)),
            ObjectFilter::default()
                .in_zone(Zone::Battlefield)
                .with_subtype(Subtype::Wall),
        ];
        blockers.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);

        assert_eq!(
            describe_blocker_union(&blockers).as_deref(),
            Some("creatures with power 2 or less and/or Walls")
        );
    }

    #[test]
    fn tagged_target_restriction_keeps_leading_end_of_turn_surface() {
        let tag = TagKey::from("targeted_0");
        let tagged_effect = Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::target(
            ChooseSpec::Object(ObjectFilter::creature()),
        )))
        .tag(tag.clone());
        let tagged = tagged_effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .expect("tag wrapper");
        let cant = crate::effects::CantEffect::until_end_of_turn(
            crate::effect::Restriction::block_specific_attacker(
                ObjectFilter::default()
                    .in_zone(Zone::Battlefield)
                    .with_subtype(Subtype::Wall),
                ObjectFilter::tagged(tag),
            ),
        )
        .with_duration_surface(crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn);

        assert_eq!(
            describe_tagged_target_then_cant_restriction(tagged, &cant).as_deref(),
            Some("Until end of turn, target creature can't be blocked by Walls")
        );
    }
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
    let tag = TagKey::from(tag);
    matches!(spec.base(), ChooseSpec::Tagged(candidate) if candidate == &tag)
        || choose_spec_has_tagged_constraint(spec, &tag)
}

pub(crate) fn move_to_exile_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    let tag = TagKey::from(tag);
    move_to_zone.zone == Zone::Exile
        // Some parser lowerings route the chosen object through a tagged
        // for-each wrapper and leave the move target as `Iterated`.
        && match move_to_zone.target.base() {
            ChooseSpec::Iterated => true,
            ChooseSpec::Tagged(candidate) => candidate == &tag,
            _ => choose_spec_has_tagged_constraint(&move_to_zone.target, &tag),
        }
}

pub(crate) fn describe_milled_graveyard_count_filter(filter: &ObjectFilter) -> Option<String> {
    if filter.zone != Some(Zone::Graveyard) {
        return None;
    }
    let mut matching_tags = filter.tagged_constraints.iter().filter(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && this_way_action_from_tag(&constraint.tag) == Some("milled")
    });
    let matching_tag = matching_tags.next()?;
    if matching_tags.next().is_some() {
        return None;
    }

    let mut card_filter = filter.clone();
    card_filter.zone = None;
    card_filter.owner = None;
    card_filter.controller = None;
    card_filter
        .tagged_constraints
        .retain(|constraint| constraint != matching_tag);
    if !card_filter.tagged_constraints.is_empty() {
        return None;
    }
    let description = card_filter.description();
    let noun = strip_indefinite_article(&description).trim();
    let card = if noun == "permanent" {
        "card".to_string()
    } else if noun.ends_with(" card") {
        noun.to_string()
    } else {
        format!("{noun} card")
    };
    if filter.prior_effect_action_surface() == Some(crate::effect::PriorEffectAction::Milled) {
        return Some(format!("{card} milled this way"));
    }

    let graveyard = match filter.owner {
        Some(PlayerFilter::You) => "your graveyard",
        Some(_) => "their graveyard",
        None => "a graveyard",
    };
    Some(format!("{card} put into {graveyard} this way"))
}

pub(crate) fn describe_for_each_filter(filter: &ObjectFilter) -> String {
    if filter.tagged_constraints.len() == 1
        && filter.tagged_constraints[0].relation
            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        && filter.tagged_constraints[0].tag.as_str() == ironsmith_core::CAST_MODIFIED_CREATURES_TAG
    {
        return "modified creature you controlled as you cast this spell".to_string();
    }
    if let Some(subject) = describe_shared_tagged_attachment_union_count_subject(filter) {
        return subject;
    }
    if let Some(relative) = describe_relative_characteristic_list_filter(filter) {
        return relative;
    }
    if let Some(milled) = describe_milled_graveyard_count_filter(filter) {
        return milled;
    }
    // Controller and owner are one correlated scope on the counted object.
    // Rendering the owner through `ObjectFilter::description` and appending
    // the controller afterward produces ungrammatical inversions such as
    // "permanent you don't own you control." The count-filter renderer
    // already preserves the typed pair and its canonical connective.
    if filter.controller.is_some() && filter.owner.is_some() {
        return describe_for_each_count_filter(filter);
    }
    let mut base_filter = filter.clone();
    base_filter.controller = None;
    let explicit_name = base_filter
        .name
        .take()
        .map(|name| title_case_card_name_fragment(&name));

    let description = base_filter.description();
    let mut base = strip_indefinite_article(&description).to_string();
    if let Some(name) = explicit_name {
        base = if let Some((noun, zone)) = base.split_once(" in ") {
            format!("{noun} named {name} in {zone}")
        } else {
            format!("{base} named {name}")
        };
    }
    if filter.was_dealt_damage_this_turn {
        base = base
            .replace(
                " that was dealt damage this turn",
                " dealt damage this turn",
            )
            .replace(
                " that were dealt damage this turn",
                " dealt damage this turn",
            );
    }
    let has_sacrificed_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && matches!(
                tag_action_from_name(constraint.tag.as_str()),
                Some("sacrificed")
            )
    });
    let has_sacrifice_cost_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str().starts_with("sacrifice_cost_")
    });
    if let Some(rest) = base.strip_prefix("another ") {
        base = format!("other {rest}");
    }
    // A restrictive qualifier needs the noun it restricts: "permanents you
    // control with oil counters on them" cannot shed its head and still read
    // as English ("with oil counters you control on them").
    if let Some(rest) = base.strip_prefix("permanent ")
        && !rest.starts_with("with ")
        && !rest.starts_with("without ")
        && !rest.starts_with("that ")
        && !rest.starts_with("that's ")
        && matches!(filter.zone, None | Some(Zone::Battlefield))
        && !filter.chosen_creature_type
        && !filter.chosen_card_type
        && !filter.has_all_permanent_card_types()
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
    if has_sacrificed_tag
        && !has_sacrifice_cost_tag
        && !base.to_ascii_lowercase().starts_with("the sacrificed ")
    {
        base = format!("the sacrificed {}", base.trim_start_matches("the ").trim());
    }

    if let Some(controller) = &filter.controller {
        let controller_suffix = match controller {
            PlayerFilter::You => "you control".to_string(),
            PlayerFilter::Active => "they control".to_string(),
            // A count/iteration filter ranges over the whole opposing
            // collection, rather than selecting one opponent. Oracle surfaces
            // therefore use the plural possessive ("for each creature your
            // opponents control"), while target filters remain singular.
            PlayerFilter::Opponent => "your opponents control".to_string(),
            PlayerFilter::Target(inner) if inner.relative_target_exclusion_base().is_some() => {
                "another target player controls".to_string()
            }
            _ => format!("{} controls", describe_player_filter(controller)),
        };
        if filter.has_controller_after_qualifiers_surface() {
            return format!("{base} {controller_suffix}");
        }
        const QUALIFIER_BOUNDARIES: &[&str] = &[
            " with ",
            " without ",
            " that ",
            " that's ",
            " of the chosen ",
            " named ",
            " not named ",
            " attached to ",
            " cast by ",
            " put into ",
            " in ",
            " on ",
        ];
        let boundary = QUALIFIER_BOUNDARIES
            .iter()
            .filter_map(|marker| base.find(marker))
            .min();
        if let Some(boundary) = boundary {
            let (head, tail) = base.split_at(boundary);
            return format!("{} {controller_suffix}{tail}", head.trim());
        }
        return format!("{base} {controller_suffix}");
    }
    if count_filter_needs_battlefield_surface(filter, &base) {
        base.push_str(" on the battlefield");
    }
    base
}

#[cfg(test)]
mod for_each_filter_controller_surface_tests {
    use super::*;

    #[test]
    fn controller_precedes_keyword_and_numeric_qualifiers() {
        let keyword = ObjectFilter::creature()
            .controlled_by(PlayerFilter::You)
            .with_static_ability(crate::static_abilities::StaticAbilityId::Vigilance);
        assert_eq!(
            describe_for_each_filter(&keyword),
            "creature you control with vigilance"
        );

        let power = ObjectFilter::creature()
            .controlled_by(PlayerFilter::You)
            .with_power(crate::filter::Comparison::GreaterThanOrEqual(4));
        assert_eq!(
            describe_for_each_filter(&power),
            "creature you control with power 4 or greater"
        );
    }

    #[test]
    fn typed_permanent_noun_precedes_ability_predicate() {
        let mut fading = ObjectFilter::permanent_card()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You)
            .with_ability_marker("fading");
        fading.set_controller_after_qualifiers_surface(true);

        assert!(fading.has_all_permanent_card_types());
        assert_eq!(
            describe_for_each_filter(&fading),
            "permanent with fading you control"
        );
    }

    #[test]
    fn canonical_controller_order_does_not_infer_postpositive_surface() {
        let fading = ObjectFilter::permanent_card()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You)
            .with_ability_marker("fading");

        assert!(!fading.has_controller_after_qualifiers_surface());
        assert_eq!(
            describe_for_each_filter(&fading),
            "permanent you control with fading"
        );
    }

    #[test]
    fn controller_and_inverse_owner_are_one_correlated_scope() {
        let unowned = ObjectFilter::permanent()
            .controlled_by(PlayerFilter::You)
            .owned_by(PlayerFilter::NotYou);

        assert_eq!(
            describe_for_each_filter(&unowned),
            "permanent you control but don't own"
        );
    }

    #[test]
    fn payment_each_unwraps_typed_prior_action_surface() {
        let count = Value::PendingPriorEffectMetric(
            crate::effect::PriorEffectMetricQuery::new(
                crate::effect::EffectMetricSource::AffectedObjects,
                crate::effect::EffectMetric::Count,
            )
            .with_filter(ObjectFilter::default())
            .with_action(crate::effect::PriorEffectAction::Discarded),
        )
        .with_surface_hints([
            ValueSurfaceHint::CardsDiscardedThisWay,
            ValueSurfaceHint::ForEach,
        ]);

        assert_eq!(
            describe_payment_each_value(&count),
            "card discarded this way"
        );
    }
}

pub(crate) fn describe_relative_characteristic_list_filter(
    filter: &ObjectFilter,
) -> Option<String> {
    ironsmith_core::filter_model::describe_relative_characteristic_list_filter(filter)
}
