use super::*;

pub(crate) fn title_case_card_name_fragment(name: &str) -> String {
    let small_words = [
        "a", "an", "and", "as", "at", "but", "by", "for", "from", "in", "of", "or", "the", "to",
        "with",
    ];
    name.split_whitespace()
        .enumerate()
        .map(|(idx, word)| {
            if idx > 0 && small_words.contains(&word) {
                word.to_string()
            } else {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        let mut out = first.to_ascii_uppercase().to_string();
                        out.push_str(chars.as_str());
                        out
                    }
                    None => String::new(),
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn party_size_multiplier(value: &Value) -> Option<(PlayerFilter, i32)> {
    match value {
        Value::PartySize(filter) => Some((filter.clone(), 1)),
        Value::SurfaceHinted { value, .. } => party_size_multiplier(value),
        Value::Scaled(value, factor) => {
            let (filter, mult) = party_size_multiplier(value)?;
            Some((filter, mult * factor))
        }
        Value::Add(left, right) => {
            let (left_filter, left_mult) = party_size_multiplier(left)?;
            let (right_filter, right_mult) = party_size_multiplier(right)?;
            if left_filter == right_filter {
                Some((left_filter, left_mult + right_mult))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn describe_party_size_for_each_basis(value: &Value) -> Option<(i32, String)> {
    let (filter, multiplier) = party_size_multiplier(value)?;
    Some((
        multiplier,
        format!(
            "creature in {} party",
            describe_possessive_player_filter(&filter)
        ),
    ))
}

pub(crate) fn describe_counter_for_each_basis(value: &Value) -> Option<(i32, String)> {
    match value {
        Value::SurfaceHinted { value, .. } => describe_counter_for_each_basis(value),
        Value::Scaled(value, multiplier) => {
            let (inner_multiplier, basis) = describe_counter_for_each_basis(value)?;
            Some((inner_multiplier * *multiplier, basis))
        }
        Value::CountersOnSource(_) | Value::CountersOn(_, _) => {
            let described = describe_value(value);
            let basis = described.strip_prefix("the number of ")?;
            let basis = if let Some(rest) = basis.strip_prefix("counters ") {
                format!("counter {rest}")
            } else {
                basis.replacen(" counters ", " counter ", 1)
            };
            Some((1, basis))
        }
        _ => None,
    }
}

pub(crate) fn spells_cast_this_turn_multiplier(value: &Value) -> Option<(PlayerFilter, i32)> {
    match value {
        Value::SpellsCastThisTurn(filter) => Some((filter.clone(), 1)),
        Value::Scaled(value, factor) => {
            let (filter, mult) = spells_cast_this_turn_multiplier(value)?;
            Some((filter, mult * factor))
        }
        Value::Add(left, right) => {
            let (left_filter, left_mult) = spells_cast_this_turn_multiplier(left)?;
            let (right_filter, right_mult) = spells_cast_this_turn_multiplier(right)?;
            if left_filter == right_filter {
                Some((left_filter, left_mult + right_mult))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn describe_spells_cast_this_turn_each(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::You => "spell you've cast this turn".to_string(),
        PlayerFilter::Opponent => "spell an opponent has cast this turn".to_string(),
        PlayerFilter::Any => "spell cast this turn".to_string(),
        other => format!(
            "spell cast this turn by {}",
            strip_leading_article(&describe_player_filter(other))
        ),
    }
}

pub(crate) fn describe_signed_value(value: &Value) -> String {
    match value {
        Value::Fixed(n) if *n >= 0 => format!("+{n}"),
        Value::Scaled(value, factor) if *factor > 0 => {
            format!(
                "+{}",
                describe_value(&Value::Scaled(value.clone(), *factor))
            )
        }
        Value::X => "+X".to_string(),
        Value::XTimes(factor) if *factor > 0 => {
            if *factor == 1 {
                "+X".to_string()
            } else {
                format!("+{factor}*X")
            }
        }
        Value::EffectValue(_) => "+X".to_string(),
        Value::EffectValueOffset(_, offset) if *offset == 0 => "+X".to_string(),
        Value::EffectValueOffset(_, offset) if *offset > 0 => format!("+X plus {offset}"),
        Value::EffectValueOffset(_, offset) => format!("+X minus {}", -offset),
        Value::Fixed(n) => n.to_string(),
        _ => describe_value(value),
    }
}

pub(crate) fn describe_toughness_delta_with_power_context(
    power: &Value,
    toughness: &Value,
) -> String {
    if matches!(power, Value::Fixed(n) if *n < 0) && matches!(toughness, Value::Fixed(0)) {
        "-0".to_string()
    } else {
        describe_signed_value(toughness)
    }
}

fn is_exactly_one_choice(count: &ChoiceCount) -> bool {
    count.min == 1 && count.max == Some(1) && !count.dynamic_x
}

fn canonical_target_payload(spec: &ChooseSpec) -> Option<&ChooseSpec> {
    match spec.unhinted() {
        ChooseSpec::Target(inner) => canonical_target_payload(inner),
        ChooseSpec::WithCount(inner, count) | ChooseSpec::WithCountValue(inner, count, _)
            if is_exactly_one_choice(count) =>
        {
            canonical_target_payload(inner)
        }
        ChooseSpec::WithCount(_, _) | ChooseSpec::WithCountValue(_, _, _) => None,
        inner => Some(inner),
    }
}

fn canonical_sole_target_inner(spec: &ChooseSpec) -> Option<&ChooseSpec> {
    match spec.unhinted() {
        ChooseSpec::Target(inner) => canonical_target_payload(inner),
        ChooseSpec::WithCount(inner, count) | ChooseSpec::WithCountValue(inner, count, _)
            if is_exactly_one_choice(count) && inner.is_target() =>
        {
            canonical_sole_target_inner(inner)
        }
        _ => None,
    }
}

/// Render a characteristic as an anaphora only when the value carries the
/// canonical unqualified object-target reference and the surrounding action
/// has exactly one object target. A constrained value target remains explicit:
/// it can denote a different target even when its filter resembles the action's
/// target filter.
pub(crate) fn describe_value_for_same_sole_target(
    value: &Value,
    enclosing_target: &ChooseSpec,
) -> Option<String> {
    if !matches!(
        canonical_sole_target_inner(enclosing_target)?,
        ChooseSpec::Object(_)
    ) {
        return None;
    }

    let (value_target, characteristic) = match value.unhinted() {
        Value::PowerOf(spec) => (spec.as_ref(), "power"),
        Value::ToughnessOf(spec) => (spec.as_ref(), "toughness"),
        Value::ManaValueOf(spec) => (spec.as_ref(), "mana value"),
        _ => return None,
    };
    let ChooseSpec::Object(filter) = canonical_sole_target_inner(value_target)? else {
        return None;
    };
    if filter != &ObjectFilter::default() {
        return None;
    }

    Some(format!("its {characteristic}"))
}

pub(crate) fn describe_value_with_enclosing_target(
    value: &Value,
    enclosing_target: Option<&ChooseSpec>,
) -> String {
    enclosing_target
        .and_then(|target| describe_value_for_same_sole_target(value, target))
        .unwrap_or_else(|| describe_value(value))
}

pub(crate) fn possessive_runtime_pt_target(target: &str) -> String {
    let target = target.trim().to_string();
    match target.as_str() {
        "it" => "its".to_string(),
        "they" | "them" => "their".to_string(),
        _ => format!("{target}'s"),
    }
}

pub(crate) fn normalized_runtime_pt_subject(text: &str) -> String {
    let lower = lowercase_first(text.trim());
    let stripped = strip_leading_article(&lower).trim();
    stripped
        .trim_end_matches("'s")
        .trim_end_matches('\'')
        .to_ascii_lowercase()
}

pub(crate) fn runtime_pt_subjects_match(left: &str, right: &str) -> bool {
    let left = normalized_runtime_pt_subject(left);
    let right = normalized_runtime_pt_subject(right);
    left == right || (left.starts_with("this ") && right.starts_with("this "))
}

pub(crate) fn dynamic_runtime_pt_axis_subject_multiplier(
    value: &Value,
    power_axis: bool,
) -> Option<(String, i32)> {
    match value.unhinted() {
        Value::SourcePower if power_axis => Some(("this source".to_string(), 1)),
        Value::SourceToughness if !power_axis => Some(("this source".to_string(), 1)),
        Value::PowerOf(spec) if power_axis => Some((describe_choose_spec(spec), 1)),
        Value::ToughnessOf(spec) if !power_axis => Some((describe_choose_spec(spec), 1)),
        Value::Scaled(inner, multiplier) if *multiplier > 0 => {
            dynamic_runtime_pt_axis_subject_multiplier(inner, power_axis)
                .map(|(subject, base)| (subject, base * *multiplier))
        }
        _ => None,
    }
}

pub(crate) fn describe_dynamic_runtime_pt_scale_action(
    target: &str,
    plural_target: bool,
    power: &Value,
    toughness: &Value,
    until_text: &str,
) -> Option<String> {
    if plural_target {
        return None;
    }

    let power_scale = dynamic_runtime_pt_axis_subject_multiplier(power, true);
    let toughness_scale = dynamic_runtime_pt_axis_subject_multiplier(toughness, false);
    let (subject, multiplier, stat) = match (power_scale, toughness_scale) {
        (Some((power_subject, power_multiplier)), None)
            if matches!(toughness.unhinted(), Value::Fixed(0)) =>
        {
            (power_subject, power_multiplier, "power")
        }
        (None, Some((toughness_subject, toughness_multiplier)))
            if matches!(power.unhinted(), Value::Fixed(0)) =>
        {
            (toughness_subject, toughness_multiplier, "toughness")
        }
        (
            Some((power_subject, power_multiplier)),
            Some((toughness_subject, toughness_multiplier)),
        ) if power_multiplier == toughness_multiplier
            && runtime_pt_subjects_match(&power_subject, &toughness_subject) =>
        {
            (power_subject, power_multiplier, "power and toughness")
        }
        _ => return None,
    };
    if !runtime_pt_subjects_match(&subject, target) {
        return None;
    }

    let verb = match multiplier + 1 {
        2 => "Double",
        3 => "Triple",
        _ => return None,
    };
    Some(format!(
        "{verb} {} {stat} {until_text}",
        possessive_runtime_pt_target(target),
    ))
}

pub(crate) fn describe_dynamic_runtime_pt_with_where_x(
    target: &str,
    plural_target: bool,
    target_spec: Option<&ChooseSpec>,
    power: &Value,
    toughness: &Value,
    until: &Until,
) -> Option<String> {
    if matches!(until, Until::Forever) {
        return None;
    }
    let until_text = describe_until(until);
    if until_text.is_empty() {
        return None;
    }

    if matches!(toughness.unhinted(), Value::Fixed(0))
        && let Some(multiplier) = counters_removed_this_way_multiplier(power)
        && multiplier > 0
    {
        let gets = if plural_target { "get" } else { "gets" };
        return Some(format!(
            "For each counter removed this way, {target} {gets} +{multiplier}/+0 {until_text}"
        ));
    }

    let for_each_axis = |value: &Value| -> Option<(Value, i32)> {
        if !value.has_surface_hint(ValueSurfaceHint::ForEach) {
            return None;
        }
        match value.unhinted() {
            Value::Scaled(inner, multiplier) => Some((inner.as_ref().clone(), *multiplier)),
            _ => Some((
                value
                    .clone()
                    .without_surface_hint(ValueSurfaceHint::ForEach),
                1,
            )),
        }
    };
    let power_for_each = for_each_axis(power);
    let toughness_for_each = for_each_axis(toughness);
    let dynamic_for_each = match (power_for_each, toughness_for_each) {
        (Some((power_basis, power_per)), Some((toughness_basis, toughness_per)))
            if power_basis == toughness_basis =>
        {
            Some((power_basis, power_per, toughness_per))
        }
        (Some((basis, power_per)), None) if matches!(toughness.unhinted(), Value::Fixed(0)) => {
            Some((basis, power_per, 0))
        }
        (None, Some((basis, toughness_per))) if matches!(power.unhinted(), Value::Fixed(0)) => {
            Some((basis, 0, toughness_per))
        }
        _ => None,
    };
    if let Some((basis, power_per, toughness_per)) = dynamic_for_each
        && let Some(each_text) = describe_create_for_each_count(&basis)
    {
        let gets = if plural_target { "get" } else { "gets" };
        let additional = if power
            .has_surface_hint(ValueSurfaceHint::AdditionalPowerToughnessModifier)
            || toughness.has_surface_hint(ValueSurfaceHint::AdditionalPowerToughnessModifier)
        {
            "an additional "
        } else {
            ""
        };
        return Some(format!(
            "{target} {gets} {additional}{}/{} {until_text} for each {each_text}",
            describe_signed_i32(power_per),
            describe_signed_i32(toughness_per),
        ));
    }

    if let Some(text) = describe_dynamic_runtime_pt_scale_action(
        target,
        plural_target,
        power,
        toughness,
        &until_text,
    ) {
        return Some(text);
    }

    let power_text = describe_explicit_where_x_surface(power)
        .map(str::to_string)
        .unwrap_or_else(|| describe_value_with_enclosing_target(power, target_spec));
    let toughness_text = describe_explicit_where_x_surface(toughness)
        .map(str::to_string)
        .unwrap_or_else(|| describe_value_with_enclosing_target(toughness, target_spec));
    let gets = if plural_target { "get" } else { "gets" };

    let power_is_variable = !matches!(power, Value::Fixed(_));
    let toughness_is_variable = !matches!(toughness, Value::Fixed(_));

    if let Some(for_each_text) = describe_basic_land_type_pt_for_each(power, toughness) {
        return Some(format!("{target} {gets} {for_each_text} {until_text}"));
    }

    if matches!((power, toughness), (Value::X, Value::X)) {
        return Some(format!("{target} {gets} +X/+X {until_text}"));
    }
    if matches!((power, toughness), (Value::XTimes(-1), Value::XTimes(-1))) {
        return Some(format!("{target} {gets} -X/-X {until_text}"));
    }
    if let (Value::Scaled(power_inner, -1), Value::Scaled(toughness_inner, -1)) = (power, toughness)
        && power_inner == toughness_inner
    {
        let basis = describe_explicit_where_x_surface(power_inner)
            .map(str::to_string)
            .unwrap_or_else(|| describe_value(power_inner));
        return Some(format!(
            "{target} {gets} -X/-X {until_text}, where X is {basis}"
        ));
    }
    if power_is_variable && toughness_is_variable && power_text == toughness_text {
        return Some(format!(
            "{target} {gets} +X/+X {until_text}, where X is {power_text}"
        ));
    }
    if power_is_variable
        && let Value::Scaled(toughness_inner, -1) = toughness
        && toughness_inner.as_ref() == power
    {
        return Some(format!(
            "{target} {gets} +X/-X {until_text}, where X is {power_text}"
        ));
    }
    if power_is_variable && matches!(toughness, Value::Fixed(0)) {
        return Some(format!(
            "{target} {gets} +X/+0 {until_text}, where X is {power_text}"
        ));
    }
    if toughness_is_variable && matches!(power, Value::Fixed(0)) {
        return Some(format!(
            "{target} {gets} +0/+X {until_text}, where X is {toughness_text}"
        ));
    }

    None
}

pub(crate) fn describe_source_reference_surface_text(
    surface: &crate::target::SourceReferenceSurface,
) -> String {
    match surface {
        crate::target::SourceReferenceSurface::ThisPermanentType(text)
            if text.to_ascii_lowercase().starts_with("this of ") =>
        {
            "this permanent".to_string()
        }
        _ => surface.display_text(),
    }
}

pub(crate) fn apply_continuous_source_reference_text(
    effect: &crate::effects::ApplyContinuousEffect,
) -> String {
    effect
        .source_reference_surface
        .as_ref()
        .map(describe_source_reference_surface_text)
        .unwrap_or_else(|| "this source".to_string())
}

pub(crate) fn describe_basic_land_type_pt_for_each(
    power: &Value,
    toughness: &Value,
) -> Option<String> {
    let power_multiplier = basic_land_types_multiplier(power);
    let toughness_multiplier = basic_land_types_multiplier(toughness);

    let (power_per, toughness_per, filter) = match (power_multiplier, toughness_multiplier) {
        (Some((power_filter, power_per)), Some((toughness_filter, toughness_per))) => {
            if power_filter != toughness_filter {
                return None;
            }
            (power_per, toughness_per, power_filter)
        }
        (Some((filter, power_per)), None) if matches!(toughness, Value::Fixed(0)) => {
            (power_per, 0, filter)
        }
        (None, Some((filter, toughness_per))) if matches!(power, Value::Fixed(0)) => {
            (0, toughness_per, filter)
        }
        _ => return None,
    };

    let each_text =
        describe_basic_land_types_among(filter).replace("basic land types", "basic land type");
    Some(format!(
        "{}/{} for each {each_text}",
        describe_signed_i32(power_per),
        describe_signed_i32(toughness_per)
    ))
}

pub(crate) fn describe_signed_i32(value: i32) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

pub(crate) fn choose_spec_is_plural(spec: &ChooseSpec) -> bool {
    effect_text_shared::choose_spec_is_plural(spec)
}

pub(crate) fn choose_spec_allows_multiple(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => choose_spec_allows_multiple(spec),
        ChooseSpec::Target(inner) => choose_spec_allows_multiple(inner),
        ChooseSpec::All(_) | ChooseSpec::EachPlayer(_) => true,
        ChooseSpec::WithCount(inner, count) => {
            if count.is_dynamic_x() {
                true
            } else if let Some(max) = count.max {
                max > 1 || choose_spec_allows_multiple(inner)
            } else {
                true
            }
        }
        _ => false,
    }
}

pub(crate) fn choose_spec_dynamic_count_value_where_clause(spec: &ChooseSpec) -> Option<String> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } | ChooseSpec::Target(spec) => {
            choose_spec_dynamic_count_value_where_clause(spec)
        }
        ChooseSpec::WithCountValue(_, count, value) if count.is_dynamic_x() => {
            let basis = if value.has_surface_hint(ValueSurfaceHint::PriorEffectResult) {
                "the result".to_string()
            } else {
                describe_value(value)
            };
            Some(format!(", where X is {basis}"))
        }
        _ => None,
    }
}

pub(crate) fn owner_hand_phrase_for_spec(spec: &ChooseSpec) -> &'static str {
    if choose_spec_is_plural(spec) {
        "their owners' hands"
    } else {
        "its owner's hand"
    }
}

pub(crate) fn owner_library_phrase_for_spec(spec: &ChooseSpec) -> &'static str {
    if choose_spec_is_plural(spec) {
        "their owners' libraries"
    } else {
        "its owner's library"
    }
}

pub(crate) fn describe_put_counter_phrase(count: &Value, counter_type: CounterType) -> String {
    let counter_name = counter_type.description().into_owned();
    if let Value::SurfaceHinted { value, hints } = count
        && hints.contains(&ValueSurfaceHint::UpTo)
    {
        let inner = describe_put_counter_phrase(value, counter_type);
        return format!("up to {inner}");
    }
    if count.has_surface_hint(ValueSurfaceHint::EqualTo) {
        let amount = count
            .clone()
            .without_surface_hint(ValueSurfaceHint::EqualTo);
        return format!(
            "a number of {counter_name} counters equal to {}",
            describe_value(&amount)
        );
    }
    match count.unhinted() {
        Value::Fixed(1) => with_indefinite_article(&format!("{counter_name} counter")),
        Value::Fixed(n) if *n > 1 => {
            let n = *n as usize;
            let amount = number_word(n as i32).unwrap_or_else(|| n.to_string());
            format!("{amount} {counter_name} counters")
        }
        _ => format!(
            "{} {counter_name} counters",
            describe_effect_count_backref(count).unwrap_or_else(|| describe_value(count))
        ),
    }
}

pub(crate) fn describe_apply_continuous_target(
    effect: &crate::effects::ApplyContinuousEffect,
) -> (String, bool) {
    let targets_source = effect
        .target_spec
        .as_ref()
        .map(|spec| matches!(spec.unhinted(), ChooseSpec::Source))
        .unwrap_or_else(|| matches!(effect.target, crate::continuous::EffectTarget::Source));
    if targets_source && let Some(surface) = effect.source_reference_surface.as_ref() {
        return (describe_source_reference_surface_text(surface), false);
    }
    if let crate::continuous::EffectTarget::Filter(filter) = &effect.target
        && effect.runtime_modifications.iter().any(|modification| {
            matches!(
                modification,
                crate::effects::continuous::RuntimeModification::CopyOf { .. }
            )
        })
        && let Some(chosen) = filter
            .source_surface
            .as_ref()
            .map(crate::target::SourceReferenceSurface::display_text)
            .filter(|surface| surface.starts_with("the chosen "))
        && filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject)
    {
        let mut base_filter = filter.clone();
        base_filter
            .tagged_constraints
            .retain(|constraint| constraint.relation != TaggedOpbjectRelation::IsNotTaggedObject);
        base_filter.source_surface = None;
        let base = strip_leading_article(&base_filter.description()).to_string();
        return (format!("Each {base} other than {chosen}"), false);
    }
    if effect.target_spec.is_none()
        && let crate::continuous::EffectTarget::Filter(filter) = &effect.target
        && let Some(subject) = describe_attached_and_related_creatures_filter(filter)
    {
        return (subject, true);
    }
    let (mut target, mut plural) = effect_text_shared::describe_apply_continuous_target(
        effect,
        describe_choose_spec,
        |filter| pluralize_noun_phrase(&filter.description()),
    );
    if let crate::continuous::EffectTarget::Filter(filter) = &effect.target
        && !target.contains(" this way")
        && let Some(action) = prior_effect_action_for_filter(filter)
    {
        target = format!(
            "{target} {} this way",
            describe_prior_effect_action_clause(action)
        );
    }
    target = if plural || target.contains("creatures that shares ") {
        target
            .replace(" that shares ", " that share ")
            .replace(" that object", " it")
    } else {
        target
    };
    match effect.set_quantifier_surface {
        Some(ironsmith_core::SetQuantifierSurface::All) => {
            if !target.to_ascii_lowercase().starts_with("all ") {
                target = format!("all {target}");
            }
            plural = true;
        }
        Some(ironsmith_core::SetQuantifierSurface::Each)
            if effect
                .target_spec
                .as_ref()
                .is_some_and(|spec| matches!(spec.base(), ChooseSpec::Tagged(_))) =>
        {
            target = "each of them".to_string();
            plural = false;
        }
        Some(ironsmith_core::SetQuantifierSurface::Each)
            if effect.target_spec.is_some() && plural =>
        {
            target.push_str(" each");
        }
        Some(ironsmith_core::SetQuantifierSurface::Each) => {
            if let crate::continuous::EffectTarget::Filter(filter) = &effect.target {
                let description = describe_relative_characteristic_list_filter(filter)
                    .unwrap_or_else(|| filter.description());
                let description = strip_indefinite_article(&description);
                let description = if filter.other {
                    description
                        .strip_prefix("another ")
                        .map(|rest| format!("other {rest}"))
                        .unwrap_or_else(|| description.to_string())
                } else {
                    description.to_string()
                };
                target = format!("Each {description}");
                plural = false;
            }
        }
        None => {}
    }
    (target, plural)
}

pub(crate) fn describe_attached_and_related_creatures_filter(
    filter: &ObjectFilter,
) -> Option<String> {
    let [first, second] = filter.any_of.as_slice() else {
        return None;
    };

    for (attached, related) in [(first, second), (second, first)] {
        let [attached_constraint] = attached.tagged_constraints.as_slice() else {
            continue;
        };
        if attached_constraint.relation != TaggedOpbjectRelation::IsTaggedObject
            || !matches!(attached_constraint.tag.as_str(), "enchanted" | "equipped")
        {
            continue;
        }

        let tag = &attached_constraint.tag;
        if related.tagged_constraints.len() != 2
            || !related.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == tag.as_str()
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
            || !related.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == tag.as_str()
                    && constraint.relation == TaggedOpbjectRelation::SharesSubtypeWithTagged
            })
        {
            continue;
        }

        let mut attached_base = attached.clone();
        attached_base.tagged_constraints.clear();
        let mut related_base = related.clone();
        related_base.tagged_constraints.clear();
        if attached_base != related_base
            || attached_base.card_types.as_slice() != [CardType::Creature]
        {
            continue;
        }

        let mut outer_base = filter.clone();
        outer_base.any_of.clear();
        let outer_zone = outer_base.zone.take();
        if outer_base != ObjectFilter::default()
            || outer_zone.is_some_and(|zone| attached_base.zone != Some(zone))
        {
            continue;
        }

        let attached_subject = attached.description();
        let related_subject =
            pluralize_noun_phrase(strip_leading_article(&related_base.description()));
        return Some(format!(
            "{attached_subject} and other {related_subject} that share a creature type with it"
        ));
    }

    None
}

pub(crate) fn source_generic_ability_grant_target_surface(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if !matches!(effect.target, crate::continuous::EffectTarget::Source) {
        return None;
    }
    let crate::continuous::Modification::AddAbilityGeneric(ability) =
        effect.modification.as_ref()?
    else {
        return None;
    };
    if !matches!(
        ability.kind,
        crate::ability::AbilityKind::Triggered(_) | crate::ability::AbilityKind::Activated(_)
    ) {
        return None;
    }
    effect
        .source_reference_surface
        .as_ref()
        .map(describe_source_reference_surface_text)
}

pub(crate) fn granted_ability_self_subject_for_apply_continuous(
    effect: &crate::effects::ApplyContinuousEffect,
) -> &str {
    let targets_source = effect
        .target_spec
        .as_ref()
        .map(|spec| matches!(spec.unhinted(), ChooseSpec::Source))
        .unwrap_or_else(|| matches!(effect.target, crate::continuous::EffectTarget::Source));
    if targets_source
        && let Some(crate::target::SourceReferenceSurface::ThisPermanentType(text)) =
            effect.source_reference_surface.as_ref()
    {
        return text;
    }
    if let Some(spec) = &effect.target_spec {
        return granted_ability_self_subject_for_choose_spec(spec);
    }
    match &effect.target {
        crate::continuous::EffectTarget::Filter(filter) => {
            granted_ability_self_subject_for_filter(filter)
        }
        crate::continuous::EffectTarget::AllCreatures => "this creature",
        crate::continuous::EffectTarget::AllPermanents
        | crate::continuous::EffectTarget::Specific(_)
        | crate::continuous::EffectTarget::Source
        | crate::continuous::EffectTarget::AttachedTo(_) => "this permanent",
    }
}

pub(crate) fn describe_apply_continuous_clauses(
    effect: &crate::effects::ApplyContinuousEffect,
    plural_target: bool,
) -> Vec<String> {
    let gets = if plural_target { "get" } else { "gets" };
    let has = if plural_target { "have" } else { "has" };
    let gains = if plural_target { "gain" } else { "gains" };
    let loses = if plural_target { "lose" } else { "loses" };
    let add_ability_verb = if effect.condition == Some(Condition::SourceIsTapped)
        && matches!(effect.until, Until::SourceUntaps)
    {
        has
    } else {
        gains
    };
    let self_subject = granted_ability_self_subject_for_apply_continuous(effect);
    let has_copy_runtime = effect.runtime_modifications.iter().any(|runtime| {
        matches!(
            runtime,
            crate::effects::continuous::RuntimeModification::CopyOf { .. }
        )
    });

    let mut clauses = Vec::new();

    if let Some(enchant_target) = describe_becomes_aura_enchantment_clause(effect) {
        let verb = if plural_target { "become" } else { "becomes" };
        return vec![format!("{verb} an Aura with enchant {enchant_target}")];
    }

    let mut push_modification = |modification: &crate::continuous::Modification| match modification
    {
        crate::continuous::Modification::ModifyPowerToughness { power, toughness } => {
            let toughness_text = if *power < 0 && *toughness == 0 {
                "-0".to_string()
            } else {
                describe_signed_i32(*toughness)
            };
            clauses.push(format!(
                "{gets} {}/{}",
                describe_signed_i32(*power),
                toughness_text
            ));
        }
        crate::continuous::Modification::ModifyPower(value) => {
            clauses.push(format!("{gets} {} power", describe_signed_i32(*value)));
        }
        crate::continuous::Modification::ModifyToughness(value) => {
            clauses.push(format!("{gets} {} toughness", describe_signed_i32(*value)));
        }
        crate::continuous::Modification::SwitchPowerToughness => {
            clauses.push("switches power and toughness".to_string());
        }
        crate::continuous::Modification::SetColors(colors) => {
            clauses.push(format!(
                "becomes {}",
                describe_token_color_words(*colors, false)
            ));
        }
        crate::continuous::Modification::AddColors(colors) => {
            let verb = if plural_target { "become" } else { "becomes" };
            let other_colors = if plural_target {
                "their other colors"
            } else {
                "its other colors"
            };
            clauses.push(format!(
                "{verb} {} in addition to {other_colors}",
                describe_token_color_words(*colors, false)
            ));
        }
        crate::continuous::Modification::AddCardTypes(card_types) => {
            let mut words: Vec<String> = card_types
                .iter()
                .map(|card_type| describe_card_type_word_local(*card_type).to_string())
                .collect();
            if words.is_empty() {
                return;
            }

            let descriptor = if plural_target {
                if let Some(last) = words.last_mut() {
                    *last = pluralize_word(last);
                }
                words.join(" ")
            } else {
                with_indefinite_article(&words.join(" "))
            };
            let other_types = if plural_target {
                "their other types"
            } else {
                "its other types"
            };
            let verb = if plural_target { "become" } else { "becomes" };
            clauses.push(format!("{verb} {descriptor} in addition to {other_types}"));
        }
        crate::continuous::Modification::SetCardTypes(card_types) => {
            let mut words: Vec<String> = card_types
                .iter()
                .map(|card_type| describe_card_type_word_local(*card_type).to_string())
                .collect();
            if words.is_empty() {
                return;
            }
            let descriptor = if plural_target {
                if let Some(last) = words.last_mut() {
                    *last = pluralize_word(last);
                }
                words.join(" ")
            } else {
                with_indefinite_article(&words.join(" "))
            };
            let verb = if plural_target { "become" } else { "becomes" };
            clauses.push(format!("{verb} {descriptor}"));
        }
        crate::continuous::Modification::AddSubtypes(subtypes) => {
            let mut words: Vec<String> = subtypes
                .iter()
                .map(|subtype| subtype.to_string().to_ascii_lowercase())
                .collect();
            if words.is_empty() {
                return;
            }

            let descriptor = if plural_target {
                if let Some(last) = words.last_mut() {
                    *last = pluralize_word(last);
                }
                words.join(" ")
            } else {
                with_indefinite_article(&words.join(" "))
            };
            let other_types = if plural_target {
                "their other types"
            } else {
                "its other types"
            };
            let verb = if plural_target { "become" } else { "becomes" };
            clauses.push(format!("{verb} {descriptor} in addition to {other_types}"));
        }
        crate::continuous::Modification::AddAllSubtypesOfFamily(family) => {
            if *family == crate::types::SubtypeFamily::Creature {
                clauses.push(format!("{gains} all creature types"));
            }
        }
        crate::continuous::Modification::RemoveAllSubtypesOfFamily(family) => {
            if *family == crate::types::SubtypeFamily::Creature {
                clauses.push(format!("{loses} all creature types"));
            }
        }
        crate::continuous::Modification::RemoveCardTypes(card_types) => {
            let words = card_types
                .iter()
                .map(|card_type| describe_card_type_word_local(*card_type).to_string())
                .collect::<Vec<_>>();
            if words.is_empty() {
                return;
            }
            let descriptor = join_with_or(&words);
            if plural_target {
                clauses.push(format!("aren't {}", pluralize_noun_phrase(&descriptor)));
            } else {
                clauses.push(format!("isn't {}", with_indefinite_article(&descriptor)));
            }
        }
        crate::continuous::Modification::MakeColorless => {
            clauses.push("becomes colorless".to_string());
        }
        crate::continuous::Modification::SetPowerToughness {
            power,
            toughness,
            sublayer,
        } => {
            let verb = if *sublayer == crate::continuous::PtSublayer::Setting {
                has
            } else {
                gets
            };
            if power.unhinted() == toughness.unhinted()
                && power.has_surface_hint(ValueSurfaceHint::WhereXIs)
            {
                clauses.push(format!("{verb} base power and toughness X/X"));
                return;
            }
            clauses.push(format!(
                "{verb} base power and toughness {}/{}",
                describe_value(power),
                describe_value(toughness)
            ));
        }
        crate::continuous::Modification::SetPower { value, sublayer } => {
            let verb = if *sublayer == crate::continuous::PtSublayer::Setting {
                has
            } else {
                gets
            };
            clauses.push(format!("{verb} base power {}", describe_value(value)));
        }
        crate::continuous::Modification::SetToughness { value, sublayer } => {
            let verb = if *sublayer == crate::continuous::PtSublayer::Setting {
                has
            } else {
                gets
            };
            clauses.push(format!("{verb} base toughness {}", describe_value(value)));
        }
        crate::continuous::Modification::AddAbility(ability) => {
            if let Some(inline) = ability.granted_inline_ability() {
                clauses.push(format!(
                    "{add_ability_verb} {}",
                    describe_inline_ability_with_self_subject(inline, self_subject)
                ));
            } else {
                clauses.push(format!(
                    "{add_ability_verb} {}",
                    lowercase_first(&ability.display())
                ));
            }
        }
        crate::continuous::Modification::RemoveAbility(ability) => {
            if let Some(inline) = ability.granted_inline_ability() {
                clauses.push(format!("{loses} {}", describe_inline_ability(inline)));
            } else {
                clauses.push(format!("{loses} {}", lowercase_first(&ability.display())));
            }
        }
        crate::continuous::Modification::RemoveAllAbilities => {
            clauses.push(format!("{loses} all abilities"));
        }
        crate::continuous::Modification::AddAbilityGeneric(ability) => {
            if matches!(
                ability.kind,
                crate::ability::AbilityKind::Triggered(_)
                    | crate::ability::AbilityKind::Activated(_)
            ) {
                let self_subject = if self_subject == "this spell" {
                    "this creature"
                } else {
                    self_subject
                };
                let mut ability_text = capitalize_first(
                    &describe_inline_ability_with_self_subject(ability, self_subject),
                )
                .replace(". otherwise,", ". Otherwise,");
                if self_subject != "this spell" {
                    ability_text = replace_this_spell_self_reference(ability_text, self_subject);
                }
                ability_text = normalize_granted_triggered_ability_surface(ability_text);
                if matches!(effect.until, Until::EndOfTurn) {
                    ability_text =
                        normalize_temporary_granted_trigger_surface(ability_text, ability);
                }
                let grant_verb = add_ability_verb;
                clauses.push(format!("{grant_verb} \"{ability_text}\""));
            } else {
                clauses.push(format!(
                    "{gains} {}",
                    describe_inline_ability_with_self_subject(ability, self_subject)
                ));
            }
        }
        crate::continuous::Modification::DoesntUntap => {
            clauses.push("can't untap".to_string());
        }
        _ => {}
    };

    // Type-SETTING surface: RemoveAllSubtypesOfFamily(Creature) paired with
    // AddSubtypes renders as the oracle's plain "becomes a Bird Giant"
    // (CR 205.1b replacement), not "in addition to its other types".
    let set_creature_subtypes = match (
        &effect.modification,
        effect.additional_modifications.as_slice(),
    ) {
        (
            Some(crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                crate::types::SubtypeFamily::Creature,
            )),
            [crate::continuous::Modification::AddSubtypes(subtypes)],
        ) => Some(subtypes),
        _ => None,
    };
    if let Some(subtypes) = set_creature_subtypes {
        let mut words: Vec<String> = subtypes.iter().map(ToString::to_string).collect();
        if plural_target {
            if let Some(last) = words.last_mut() {
                *last = pluralize_word(last);
            }
            clauses.push(format!("become {}", words.join(" ")));
        } else {
            clauses.push(format!(
                "becomes {}",
                with_indefinite_article(&words.join(" "))
            ));
        }
    } else {
        if let Some(modification) = &effect.modification {
            push_modification(modification);
        }
        for modification in &effect.additional_modifications {
            if has_copy_runtime
                && matches!(
                    modification,
                    crate::continuous::Modification::RemoveSupertypes(_)
                        | crate::continuous::Modification::AddCardTypes(_)
                        | crate::continuous::Modification::SetCardTypes(_)
                        | crate::continuous::Modification::AddSubtypes(_)
                        | crate::continuous::Modification::SetSubtypes(_)
                        | crate::continuous::Modification::RemoveAllSubtypesOfFamily(_)
                        | crate::continuous::Modification::SetPowerToughness { .. }
                        | crate::continuous::Modification::AddAbility(_)
                        | crate::continuous::Modification::AddAbilityGeneric(_)
                )
            {
                continue;
            }
            push_modification(modification);
        }
    }
    for runtime in &effect.runtime_modifications {
        match runtime {
            crate::effects::continuous::RuntimeModification::CopyOf { source, .. } => {
                let verb = if plural_target { "become" } else { "becomes" };
                clauses.push(format!("{verb} a copy of {}", describe_choose_spec(source)));
            }
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            } => {
                if let Some(for_each_text) = describe_basic_land_type_pt_for_each(power, toughness)
                {
                    clauses.push(format!("{gets} {for_each_text}"));
                } else if power.unhinted() == toughness.unhinted()
                    && power.has_surface_hint(ValueSurfaceHint::WhereXIs)
                {
                    clauses.push(format!("{gets} +X/+X"));
                } else {
                    clauses.push(format!(
                        "{gets} {}/{}",
                        describe_signed_value(power),
                        describe_toughness_delta_with_power_context(power, toughness)
                    ));
                }
            }
            crate::effects::continuous::RuntimeModification::ModifyPower { value } => {
                clauses.push(format!("{gets} {} power", describe_signed_value(value)));
            }
            crate::effects::continuous::RuntimeModification::ModifyToughness { value } => {
                clauses.push(format!("{gets} {} toughness", describe_signed_value(value)));
            }
            crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController => {
                clauses.push("changes controller to this effect's controller".to_string());
            }
            crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player) => {
                clauses.push(format!(
                    "changes controller to {}",
                    describe_player_filter(player)
                ));
            }
            crate::effects::continuous::RuntimeModification::RemoveAllAbilities => {
                clauses.push(if plural_target {
                    "lose all abilities".to_string()
                } else {
                    "loses all abilities".to_string()
                });
            }
            crate::effects::continuous::RuntimeModification::RemoveThisAbility => {
                clauses.push("loses this ability".to_string());
            }
            crate::effects::continuous::RuntimeModification::SetAuraAttachmentFilter(_) => {
                clauses.push("has enchant restriction".to_string());
            }
        }
    }

    if clauses.len() > 1 {
        let shared_gain_prefix = if plural_target { "gain " } else { "gains " };
        if clauses
            .iter()
            .all(|clause| clause.starts_with(shared_gain_prefix))
        {
            let gained = clauses
                .iter()
                .map(|clause| clause[shared_gain_prefix.len()..].to_string())
                .collect::<Vec<_>>();
            return vec![format!("{shared_gain_prefix}{}", join_with_and(&gained))];
        }
    }

    clauses
}

pub(crate) fn describe_becomes_aura_enchantment_clause(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if effect.condition.is_some() || !matches!(effect.until, Until::Forever) {
        return None;
    }
    let Some(crate::continuous::Modification::AddCardTypes(card_types)) = &effect.modification
    else {
        return None;
    };
    if card_types.as_slice() != [CardType::Enchantment] {
        return None;
    }
    let mut removes_non_enchantment_types = false;
    let mut adds_aura_subtype = false;
    for modification in &effect.additional_modifications {
        match modification {
            crate::continuous::Modification::RemoveCardTypes(card_types) => {
                removes_non_enchantment_types = card_types.contains(&CardType::Artifact)
                    && card_types.contains(&CardType::Battle)
                    && card_types.contains(&CardType::Creature)
                    && card_types.contains(&CardType::Kindred)
                    && card_types.contains(&CardType::Land)
                    && card_types.contains(&CardType::Planeswalker);
            }
            crate::continuous::Modification::AddSubtypes(subtypes) => {
                adds_aura_subtype = subtypes.as_slice() == [Subtype::Aura];
            }
            _ => return None,
        }
    }
    if !removes_non_enchantment_types || !adds_aura_subtype {
        return None;
    }
    let [crate::effects::continuous::RuntimeModification::SetAuraAttachmentFilter(filter)] =
        effect.runtime_modifications.as_slice()
    else {
        return None;
    };
    Some(describe_aura_attachment_filter_inline(filter))
}

pub(crate) fn describe_aura_attachment_filter_inline(
    filter: &crate::object::AuraAttachmentFilter,
) -> String {
    match filter {
        crate::object::AuraAttachmentFilter::Object(filter) => {
            strip_leading_article(&filter.description()).to_string()
        }
        crate::object::AuraAttachmentFilter::Player(player) => {
            strip_leading_article(&describe_player_filter(player)).to_string()
        }
    }
}

pub(crate) fn describe_copy_exception_tail(
    name_override: &Option<String>,
    name_override_surface: &Option<crate::target::SourceReferenceSurface>,
    add_supertypes: &[crate::types::Supertype],
    preserve_source_abilities: bool,
    additional_modifications: &[crate::continuous::Modification],
) -> Option<String> {
    let mut parts = Vec::new();
    let mut granted_abilities = Vec::new();
    if let Some(name) = name_override_surface
        .as_ref()
        .map(crate::target::SourceReferenceSurface::display_text)
        .or_else(|| name_override.clone())
    {
        parts.push(format!("its name is {name}"));
    }
    for supertype in add_supertypes {
        parts.push(format!("it's {} in addition to its other types", supertype));
    }
    for modification in additional_modifications {
        match modification {
            crate::continuous::Modification::RemoveSupertypes(supertypes) => {
                for supertype in supertypes {
                    parts.push(format!("it isn't {}", supertype));
                }
            }
            crate::continuous::Modification::SetPowerToughness {
                power,
                toughness,
                sublayer: crate::continuous::PtSublayer::Setting,
            } => parts.push(format!(
                "it's {}/{}",
                describe_value(power),
                describe_value(toughness)
            )),
            crate::continuous::Modification::AddAbility(ability) => {
                granted_abilities.push(lowercase_first(&ability.display()));
            }
            crate::continuous::Modification::AddAbilityGeneric(ability) => {
                let rendered = describe_inline_ability_with_self_subject(ability, "this creature");
                granted_abilities.push(lowercase_first(&normalize_common_semantic_phrasing(
                    &rendered,
                )));
            }
            _ => {}
        }
    }
    if preserve_source_abilities {
        granted_abilities.push("this ability".to_string());
    }
    if !granted_abilities.is_empty() {
        parts.push(format!("it has {}", join_with_and(&granted_abilities)));
    }

    match parts.len() {
        0 => None,
        1 => parts.pop(),
        2 => Some(format!("{} and {}", parts[0], parts[1])),
        _ => {
            let last = parts.pop().expect("nonempty parts");
            Some(format!("{}, and {}", parts.join(", "), last))
        }
    }
}

pub(crate) fn describe_apply_continuous_tail(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if effect.condition == Some(Condition::SourceIsTapped)
        && matches!(effect.until, Until::SourceUntaps)
    {
        return Some("for as long as this source remains tapped".to_string());
    }

    let mut tail_parts = Vec::new();
    if let Some(condition) = &effect.condition
        && matches!(effect.until, Until::ThisLeavesTheBattlefield)
    {
        tail_parts.push(format!(
            "while {}",
            lowercase_first(&describe_condition(condition))
        ));
    }
    if !matches!(effect.until, Until::Forever) {
        tail_parts.push(describe_until(&effect.until));
    }
    for runtime in &effect.runtime_modifications {
        if let crate::effects::continuous::RuntimeModification::CopyOf {
            preserve_source_abilities,
            name_override,
            name_override_surface,
            add_supertypes,
            copy_exception_surface,
            ..
        } = runtime
            && let Some(exception_tail) = copy_exception_surface.clone().or_else(|| {
                describe_copy_exception_tail(
                    name_override,
                    name_override_surface,
                    add_supertypes,
                    *preserve_source_abilities,
                    &effect.additional_modifications,
                )
            })
        {
            tail_parts.push(format!("except {exception_tail}"));
        }
    }
    if tail_parts.is_empty() {
        None
    } else {
        Some(tail_parts.join(", "))
    }
}

pub(crate) fn apply_continuous_preserves_source_abilities(
    effect: &crate::effects::ApplyContinuousEffect,
) -> bool {
    effect.runtime_modifications.iter().any(|runtime| {
        matches!(
            runtime,
            crate::effects::continuous::RuntimeModification::CopyOf {
                preserve_source_abilities: true,
                ..
            }
        )
    })
}

pub(crate) fn describe_doesnt_untap_apply_continuous_effect(
    effect: &crate::effects::ApplyContinuousEffect,
    target: &str,
    plural_target: bool,
) -> Option<String> {
    if !matches!(
        effect.modification,
        Some(crate::continuous::Modification::DoesntUntap)
    ) || !effect.additional_modifications.is_empty()
        || !effect.runtime_modifications.is_empty()
    {
        return None;
    }

    let target =
        if target == "permanent" && matches!(effect.target_spec, Some(ChooseSpec::Tagged(_))) {
            "it"
        } else {
            target
        };
    let mut text = if plural_target {
        format!("{target} don't untap during their controllers' untap steps")
    } else {
        format!("{target} doesn't untap during its controller's untap step")
    };
    if let Some(tail) = describe_apply_continuous_tail(effect) {
        text.push(' ');
        text.push_str(&tail);
    }
    Some(normalize_each_other_continuous_subject(text))
}

pub(crate) fn choose_spec_land_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_is_land_kind(filter).then_some(filter)
        }
        _ => None,
    }
}

pub(crate) fn object_filter_is_land_kind(filter: &ObjectFilter) -> bool {
    filter.card_types.contains(&CardType::Land)
        || filter.subtypes.iter().any(Subtype::is_land_subtype)
}

pub(crate) fn choose_spec_prefers_land_addition_surface(
    spec: &ChooseSpec,
    dynamic_equal_pt: bool,
    plural_target: bool,
) -> bool {
    let ChooseSpec::Object(filter) = spec.base() else {
        return false;
    };
    filter.card_types.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
        && filter.subtypes.iter().any(Subtype::is_land_subtype)
        && (dynamic_equal_pt || plural_target)
}

pub(crate) fn choose_spec_guarantees_artifact(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => {
            hints.iter().any(|hint| {
                matches!(
                    hint,
                    crate::target::ChooseSpecSurfaceHint::SourceReference(
                        crate::target::SourceReferenceSurface::ThisPermanentType(text)
                    ) if text.eq_ignore_ascii_case("this artifact")
                )
            }) || choose_spec_guarantees_artifact(spec)
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_guarantees_artifact(inner),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.card_types.contains(&CardType::Artifact)
                && !filter.excluded_card_types.contains(&CardType::Artifact)
        }
        _ => false,
    }
}

pub(crate) fn plural_non_target_land_animation_target(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if !matches!(effect.target, crate::continuous::EffectTarget::Filter(_)) {
        return None;
    }
    let Some(ChooseSpec::Object(filter)) = &effect.target_spec else {
        return None;
    };
    if !object_filter_is_land_kind(filter) {
        return None;
    }
    if filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && matches!(constraint.tag.as_str(), "enchanted" | "equipped")
    }) {
        return None;
    }

    let description = filter.description();
    let rest = strip_leading_article(&description).trim();
    if rest.is_empty() {
        return None;
    }
    Some(format!("all {}", pluralize_noun_phrase(rest)))
}

pub(crate) fn describe_apply_continuous_animation_effect(
    effect: &crate::effects::ApplyContinuousEffect,
    target: &str,
    plural_target: bool,
) -> Option<String> {
    let (card_types, replaces_other_types) = match &effect.modification {
        Some(crate::continuous::Modification::AddCardTypes(card_types)) => (card_types, false),
        Some(crate::continuous::Modification::SetCardTypes(card_types)) => (card_types, true),
        _ => return None,
    };
    if !card_types.contains(&CardType::Creature) || !effect.runtime_modifications.is_empty() {
        return None;
    }

    let mut power = None;
    let mut toughness = None;
    let mut colors = None;
    let mut subtypes = Vec::new();
    let mut ability_text = Vec::new();
    let mut has_quoted_generic_ability = false;
    for modification in &effect.additional_modifications {
        match modification {
            crate::continuous::Modification::SetPowerToughness {
                power: candidate_power,
                toughness: candidate_toughness,
                sublayer,
            } if *sublayer == crate::continuous::PtSublayer::Setting => {
                power = Some(candidate_power);
                toughness = Some(candidate_toughness);
            }
            crate::continuous::Modification::SetColors(candidate_colors) => {
                colors = Some(*candidate_colors);
            }
            crate::continuous::Modification::AddSubtypes(candidate_subtypes) => {
                subtypes.extend(candidate_subtypes.iter().copied());
            }
            crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                crate::types::SubtypeFamily::Creature,
            ) => {}
            crate::continuous::Modification::AddAllSubtypesOfFamily(
                crate::types::SubtypeFamily::Creature,
            ) => {
                ability_text.push("all creature types".to_string());
            }
            crate::continuous::Modification::AddAbility(ability) => {
                ability_text.push(lowercase_first(&ability.display()));
            }
            crate::continuous::Modification::AddAbilityGeneric(ability) => {
                has_quoted_generic_ability = true;
                let mut rendered = capitalize_first(&describe_inline_ability_with_self_subject(
                    ability,
                    "this creature",
                ))
                .replace(". otherwise,", ". Otherwise,");
                rendered = replace_this_spell_self_reference(rendered, "this creature");
                rendered = normalize_granted_triggered_ability_surface(rendered);
                rendered = rendered.replace(", where X is X", "");
                if matches!(effect.until, Until::EndOfTurn) {
                    rendered = normalize_temporary_granted_trigger_surface(rendered, ability);
                }
                if !rendered.ends_with('.') && !rendered.ends_with('!') && !rendered.ends_with('?')
                {
                    rendered.push('.');
                }
                ability_text.push(format!("\"{rendered}\""));
            }
            _ => return None,
        }
    }

    let returned_permanent_animation = effect.until == Until::Forever
        && matches!(
            effect.target_spec.as_ref(),
            Some(ChooseSpec::Tagged(tag)) if tag.as_str().starts_with("returned_")
        );
    let returned_artifact_creature_animation = returned_permanent_animation
        && !replaces_other_types
        && effect.type_retention_surface.is_none()
        && card_types.contains(&CardType::Artifact);
    let explicitly_still_a_land = matches!(
        effect.type_retention_surface,
        Some(ironsmith_core::TypeRetentionSurface::StillALand)
    );
    let explicitly_in_addition = matches!(
        effect.type_retention_surface,
        Some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes)
    );
    let mut preserves_land_types = !replaces_other_types
        && (explicitly_still_a_land
            || effect
                .target_spec
                .as_ref()
                .and_then(choose_spec_land_filter)
                .is_some()
            || target.eq_ignore_ascii_case("this land"));
    let (target_text, plural_target) =
        if let Some(target_text) = plural_non_target_land_animation_target(effect) {
            (target_text, true)
        } else if returned_artifact_creature_animation {
            ("they".to_string(), true)
        } else if returned_permanent_animation {
            ("those permanents".to_string(), true)
        } else {
            (target.to_string(), plural_target)
        };
    preserves_land_types = preserves_land_types || target_text.eq_ignore_ascii_case("this land");

    let mut descriptor = Vec::new();
    if let Some(colors) = colors {
        descriptor.push(describe_token_color_words(colors, false));
    }
    if !subtypes.is_empty() {
        descriptor.push(
            subtypes
                .iter()
                .map(|subtype| {
                    let subtype = subtype.to_string();
                    if returned_artifact_creature_animation {
                        subtype
                    } else {
                        subtype.to_ascii_lowercase()
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    let adds_named_types = !subtypes.is_empty();
    let extra_card_types = card_types
        .iter()
        .copied()
        .filter(|card_type| *card_type != CardType::Creature)
        .map(|card_type| describe_card_type_word_local(card_type).to_string())
        .collect::<Vec<_>>();
    if !extra_card_types.is_empty() {
        descriptor.push(extra_card_types.join(" "));
    }
    let adds_named_types = adds_named_types || !extra_card_types.is_empty();
    descriptor.push(if plural_target {
        "creatures".to_string()
    } else {
        "creature".to_string()
    });

    let noun_phrase = descriptor
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let dynamic_equal_pt = if let (Some(power), Some(toughness)) = (power, toughness) {
        power == toughness && !matches!(power, Value::Fixed(_))
    } else {
        false
    };
    let dynamic_equal_pt_uses_mana_value =
        if let (Some(power), Some(toughness)) = (power, toughness) {
            power == toughness
                && matches!(power.unhinted(), Value::ManaValueOf(_))
                && !power.has_surface_hint(ValueSurfaceHint::WhereXIs)
        } else {
            false
        };
    let pt_where_clause = if let (Some(power), _) = (power, toughness) {
        (dynamic_equal_pt && !dynamic_equal_pt_uses_mana_value && !matches!(power, Value::X))
            .then(|| describe_value(power))
    } else {
        None
    };

    let mut text = if let (Some(power), Some(toughness)) = (power, toughness) {
        if dynamic_equal_pt_uses_mana_value {
            let pt_noun_phrase = format!(
                "{noun_phrase} with power and toughness each equal to {}",
                describe_value(power)
            );
            if returned_permanent_animation {
                format!("{target_text} are {pt_noun_phrase}")
            } else if plural_target {
                format!("{target_text} become {pt_noun_phrase}")
            } else {
                format!(
                    "{target_text} becomes {}",
                    with_indefinite_article(&pt_noun_phrase)
                )
            }
        } else {
            let pt = if dynamic_equal_pt {
                "X/X".to_string()
            } else {
                format!("{}/{}", describe_value(power), describe_value(toughness))
            };
            let pt_noun_phrase = if dynamic_equal_pt {
                format!("{pt} {noun_phrase}")
            } else if returned_artifact_creature_animation {
                format!("{pt} {noun_phrase}")
            } else {
                format!("{noun_phrase} with base power and toughness {pt}")
            };
            if returned_permanent_animation {
                format!("{target_text} are {pt_noun_phrase}")
            } else if plural_target {
                format!("{target_text} become {pt_noun_phrase}")
            } else {
                format!(
                    "{target_text} becomes {}",
                    with_indefinite_article(&pt_noun_phrase)
                )
            }
        }
    } else if power.is_none() && toughness.is_none() {
        if plural_target {
            format!("{target_text} become {noun_phrase}")
        } else {
            format!(
                "{target_text} becomes {}",
                with_indefinite_article(&noun_phrase)
            )
        }
    } else {
        return None;
    };
    if !ability_text.is_empty() {
        let ability_connector = if text.contains(" with base power and toughness ")
            || text.contains(" with power and toughness each equal to ")
        {
            " and "
        } else {
            " with "
        };
        text.push_str(ability_connector);
        text.push_str(&join_with_and(&ability_text));
    }
    let adds_artifact_type = card_types
        .iter()
        .any(|card_type| *card_type == CardType::Artifact);
    let target_is_guaranteed_artifact = effect
        .target_spec
        .as_ref()
        .is_some_and(choose_spec_guarantees_artifact);
    let artifact_type_is_redundant = target_is_guaranteed_artifact && adds_artifact_type;
    let land_addition_surface = preserves_land_types
        && adds_named_types
        && !dynamic_equal_pt_uses_mana_value
        && effect.target_spec.as_ref().is_some_and(|spec| {
            choose_spec_prefers_land_addition_surface(spec, dynamic_equal_pt, plural_target)
        });
    let render_as_addition_to_other_types = !replaces_other_types
        && !explicitly_still_a_land
        && !returned_artifact_creature_animation
        && (explicitly_in_addition
            || returned_permanent_animation
            || (!preserves_land_types && !ability_text.is_empty() && !has_quoted_generic_ability)
            || land_addition_surface);
    if render_as_addition_to_other_types && !artifact_type_is_redundant {
        if plural_target {
            text.push_str(" in addition to their other types");
        } else {
            text.push_str(" in addition to its other types");
        }
    }
    text = apply_continuous_text_with_tail(
        text,
        describe_apply_continuous_tail(effect),
        has_quoted_generic_ability,
    );
    if let Some(where_clause) = pt_where_clause {
        text.push_str(", where X is ");
        text.push_str(&where_clause);
    }
    if preserves_land_types && !render_as_addition_to_other_types {
        if plural_target {
            text.push_str(". They're still lands");
        } else {
            text.push_str(". It's still a land");
        }
    }
    let text = capitalize_first(&text);
    Some(if returned_artifact_creature_animation {
        text.replacen("They are ", "They're ", 1)
    } else {
        text
    })
}

pub(crate) fn describe_apply_continuous_effect(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    let (mut target, plural_target) = describe_apply_continuous_target(effect);
    if let Some(surface) = source_generic_ability_grant_target_surface(effect) {
        target = surface;
    }
    if let Some(text) = describe_dies_return_counter_grant(effect, &target) {
        return Some(text);
    }
    if let Some(text) = describe_attack_block_if_able_apply_continuous(effect, &target) {
        return Some(text);
    }
    if let Some(text) = describe_apply_continuous_animation_effect(effect, &target, plural_target) {
        return Some(text);
    }
    if matches!(effect.target, crate::continuous::EffectTarget::Source)
        && effect.additional_modifications.is_empty()
        && effect.runtime_modifications.is_empty()
        && matches!(effect.until, Until::Forever)
        && let Some(crate::continuous::Modification::AddAbility(ability)) = &effect.modification
        && ability.id() == crate::static_abilities::StaticAbilityId::CanAttackAsThoughHaste
        && let Some(Condition::Not(inner)) = &effect.condition
        && matches!(
            inner.as_ref(),
            Condition::ObjectEnteredBattlefieldThisTurn(filter) if *filter == ObjectFilter::source()
        )
    {
        return Some(
            "This creature can attack as though it had haste unless it entered this turn"
                .to_string(),
        );
    }
    if effect.modification.is_none()
        && effect.additional_modifications.is_empty()
        && matches!(
            effect.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
    {
        if let Some(text) = describe_gain_control_target_player_creatures(effect) {
            return Some(text);
        }
        let mut text = format!("Gain control of {target}");
        if effect.condition == Some(Condition::SourceIsTapped)
            && matches!(effect.until, Until::SourceUntaps)
        {
            let source = apply_continuous_source_reference_text(effect);
            text.push_str(&format!(
                " for as long as you control {source} and {source} remains tapped"
            ));
        } else if !matches!(effect.until, Until::Forever) {
            text.push(' ');
            text.push_str(&describe_until(&effect.until));
        }
        return Some(text);
    }
    if effect.modification.is_none()
        && effect.additional_modifications.is_empty()
        && let [crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)] =
            effect.runtime_modifications.as_slice()
    {
        let controller_text = match player {
            PlayerFilter::MostLifeTied => "the player with the most life".to_string(),
            PlayerFilter::LowestLifeTied => "the player with the lowest life total".to_string(),
            _ => describe_player_filter(player),
        };
        let mut text = format!("{controller_text} gains control of {target}");
        if matches!(player, PlayerFilter::LowestLifeTied)
            && matches!(effect.until, Until::Forever)
            && matches!(effect.target, crate::continuous::EffectTarget::Source)
        {
            text.push_str(". If two or more players are tied for lowest life total, you choose one of them, and that player gains control of ");
            text.push_str(&target);
            return Some(text);
        }
        if !matches!(effect.until, Until::Forever) {
            text.push(' ');
            text.push_str(&describe_until(&effect.until));
        }
        return Some(text);
    }
    if effect.additional_modifications.is_empty()
        && matches!(
            effect.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
        && let Some(crate::continuous::Modification::AddAbility(ability)) = &effect.modification
        && ability.id() == crate::static_abilities::StaticAbilityId::Haste
    {
        let mut text = format!("Gain control of {target}");
        if !matches!(effect.until, Until::Forever) {
            text.push(' ');
            text.push_str(&describe_until(&effect.until));
        }
        text.push_str(". ");
        text.push_str(&capitalize_first(&target));
        text.push_str(" gains haste");
        if !matches!(effect.until, Until::Forever) {
            text.push(' ');
            text.push_str(&describe_until(&effect.until));
        }
        return Some(text);
    }
    if effect.modification.is_none()
        && effect.additional_modifications.is_empty()
        && let [
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            },
        ] = effect.runtime_modifications.as_slice()
        && let Some(text) = describe_dynamic_runtime_pt_with_where_x(
            target.as_str(),
            plural_target,
            effect.target_spec.as_ref(),
            power,
            toughness,
            &effect.until,
        )
    {
        return Some(text);
    }
    if let Some(text) =
        describe_doesnt_untap_apply_continuous_effect(effect, &target, plural_target)
    {
        return Some(text);
    }
    if matches!(
        effect.modification.as_ref(),
        Some(crate::continuous::Modification::SwitchPowerToughness)
    ) && effect.additional_modifications.is_empty()
        && effect.runtime_modifications.is_empty()
    {
        let mut text = format!(
            "Switch {} power and toughness",
            possessive_runtime_pt_target(&target)
        );
        if !matches!(effect.until, Until::Forever) {
            text.push(' ');
            text.push_str(&describe_until(&effect.until));
        }
        return Some(text);
    }

    let clauses = describe_apply_continuous_clauses(effect, plural_target);
    if clauses.is_empty() {
        return None;
    }

    let quoted_granted_ability = clauses.iter().any(|clause| clause.contains('"'));
    let mut text = format!("{target} {}", join_with_and(&clauses));
    text = apply_continuous_text_with_tail(
        text,
        describe_apply_continuous_tail(effect),
        quoted_granted_ability,
    );
    if !text.contains("where X is ")
        && let Some(where_x) = effect.runtime_modifications.iter().find_map(|runtime| {
            let crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            } = runtime
            else {
                return None;
            };
            (power.unhinted() == toughness.unhinted()
                && power.has_surface_hint(ValueSurfaceHint::WhereXIs))
            .then(|| describe_where_x_basis(power))
            .flatten()
        })
    {
        text.push_str(", where X is ");
        text.push_str(&where_x);
    }
    if !text.contains("where X is ")
        && let Some(where_x) = effect
            .modification
            .iter()
            .chain(effect.additional_modifications.iter())
            .find_map(|modification| {
                let crate::continuous::Modification::SetPowerToughness {
                    power, toughness, ..
                } = modification
                else {
                    return None;
                };
                (power.unhinted() == toughness.unhinted()
                    && power.has_surface_hint(ValueSurfaceHint::WhereXIs))
                .then(|| describe_value(power))
            })
    {
        text.push_str(", where X is ");
        text.push_str(&where_x);
    }
    if let Some(spec) = effect.target_spec.as_ref()
        && let Some(where_clause) = choose_spec_dynamic_count_value_where_clause(spec)
        && !text.contains("where X is ")
    {
        text.push_str(&where_clause);
    }
    let text = if effect.set_quantifier_surface == Some(ironsmith_core::SetQuantifierSurface::Each)
    {
        text
    } else {
        normalize_each_other_continuous_subject(text)
    };
    if let Some(condition) = &effect.condition
        && !matches!(effect.until, Until::ThisLeavesTheBattlefield)
        && !(effect.condition == Some(Condition::SourceIsTapped)
            && matches!(effect.until, Until::SourceUntaps))
    {
        return Some(format!(
            "If {}, {}",
            describe_condition(condition),
            lowercase_first(&text)
        ));
    }
    Some(text)
}

pub(crate) fn apply_continuous_text_with_tail(
    mut text: String,
    tail: Option<String>,
    quoted_granted_ability: bool,
) -> String {
    let Some(tail) = tail else {
        return text;
    };
    if tail == "until end of turn" && quoted_granted_ability {
        return format!("Until end of turn, {}", lowercase_first(&text));
    }
    text.push(' ');
    text.push_str(&tail);
    text
}

pub(crate) fn normalize_each_other_continuous_subject(text: String) -> String {
    let text = if text.contains("creatures that shares ") {
        text.replace("creatures that shares ", "creatures that share ")
            .replace("that object", "it")
    } else {
        text
    };
    let (prefix, rest) = if let Some(rest) = text.strip_prefix("Each other ") {
        ("Other", rest)
    } else if let Some(rest) = text.strip_prefix("each other ") {
        ("other", rest)
    } else {
        return text;
    };

    for (singular_verb, plural_verb) in [
        (" gets ", " get "),
        (" gains ", " gain "),
        (" has ", " have "),
        (" loses ", " lose "),
    ] {
        if let Some((subject, tail)) = rest.split_once(singular_verb) {
            let tail = tail
                .replace(" and gets ", " and get ")
                .replace(" and gains ", " and gain ")
                .replace(" and has ", " and have ")
                .replace(" and loses ", " and lose ");
            return format!(
                "{prefix} {}{plural_verb}{tail}",
                pluralize_noun_phrase(subject)
            );
        }
    }

    text
}

pub(crate) fn describe_dies_return_counter_grant(
    effect: &crate::effects::ApplyContinuousEffect,
    target: &str,
) -> Option<String> {
    if effect.until != Until::EndOfTurn
        || effect.condition.is_some()
        || !effect.additional_modifications.is_empty()
        || !effect.runtime_modifications.is_empty()
    {
        return None;
    }
    let crate::continuous::Modification::AddAbilityGeneric(ability) =
        effect.modification.as_ref()?
    else {
        return None;
    };
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if triggered.trigger.display() != "When this creature dies" {
        return None;
    }
    let effects = triggered.effects.flattened_default_effects();
    let has_return = effects.iter().any(|effect| {
        let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
            return false;
        };
        let inner = tagged
            .effect
            .downcast_ref::<crate::effects::TaggedEffect>()
            .map(|nested| nested.effect.as_ref())
            .unwrap_or(tagged.effect.as_ref());
        let Some(move_to_zone) = inner.downcast_ref::<crate::effects::MoveToZoneEffect>() else {
            return false;
        };
        move_to_zone.zone == Zone::Battlefield
            && move_to_zone.enters_tapped
            && matches!(
                move_to_zone.battlefield_controller,
                crate::effects::BattlefieldController::Owner
            )
    });
    if !has_return {
        return None;
    }
    let has_counter = effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::PutCountersEffect>()
            .is_some_and(|put| {
                put.counter_type == CounterType::PlusOnePlusOne
                    && put.amount == Value::Fixed(1)
                    && matches!(put.target, ChooseSpec::Tagged(_))
            })
    });
    if !has_counter {
        return None;
    }
    Some(format!(
        "{target} gains \"When this creature dies, return it to the battlefield tapped under its owner's control with a +1/+1 counter on it\" until end of turn"
    ))
}

pub(crate) fn describe_attack_block_if_able_apply_continuous(
    effect: &crate::effects::ApplyContinuousEffect,
    target: &str,
) -> Option<String> {
    if effect.until != Until::EndOfTurn
        || effect.condition.is_some()
        || !effect.runtime_modifications.is_empty()
    {
        return None;
    }

    let mut has_must_attack = false;
    let mut has_must_block = false;
    let mut saw_ability = false;
    let mut visit_modification = |modification: &crate::continuous::Modification| match modification
    {
        crate::continuous::Modification::AddAbility(ability) => {
            saw_ability = true;
            match ability.id() {
                crate::static_abilities::StaticAbilityId::MustAttack => has_must_attack = true,
                crate::static_abilities::StaticAbilityId::MustBlock => has_must_block = true,
                _ => return false,
            }
            true
        }
        _ => false,
    };

    if let Some(modification) = &effect.modification
        && !visit_modification(modification)
    {
        return None;
    }
    for modification in &effect.additional_modifications {
        if !visit_modification(modification) {
            return None;
        }
    }
    if !saw_ability {
        return None;
    }

    match (has_must_attack, has_must_block) {
        (true, true) => Some(format!("{target} attacks or blocks this turn if able")),
        (true, false) => Some(format!("{target} attacks this turn if able")),
        (false, true) => Some(format!("{target} blocks this turn if able")),
        (false, false) => None,
    }
}

pub(crate) fn describe_color_subtype_addition_pair(
    first: &crate::effects::ApplyContinuousEffect,
    second: &crate::effects::ApplyContinuousEffect,
    target: &str,
    plural_target: bool,
) -> Option<String> {
    if !first.additional_modifications.is_empty()
        || !second.additional_modifications.is_empty()
        || !first.runtime_modifications.is_empty()
        || !second.runtime_modifications.is_empty()
    {
        return None;
    }

    let (colors, subtypes) = match (&first.modification, &second.modification) {
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
        .map(|subtype| subtype.to_string().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let descriptor = format!(
        "{} {subtype_words}",
        describe_token_color_words(colors, false)
    );
    let descriptor = if plural_target {
        pluralize_noun_phrase(&descriptor)
    } else {
        with_indefinite_article(&descriptor)
    };
    let verb = if plural_target { "become" } else { "becomes" };
    let other = if plural_target {
        "their other colors and types"
    } else {
        "its other colors and types"
    };
    let mut text = format!("{target} {verb} {descriptor} in addition to {other}");
    if let Some(tail) = describe_apply_continuous_tail(first) {
        text.push(' ');
        text.push_str(&tail);
    }
    Some(normalize_each_other_continuous_subject(text))
}

pub(crate) fn describe_compact_apply_continuous_pair(
    first: &crate::effects::ApplyContinuousEffect,
    second: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if first.target != second.target
        || first.target_spec != second.target_spec
        || first.until != second.until
        || first.condition != second.condition
        || apply_continuous_preserves_source_abilities(first)
            != apply_continuous_preserves_source_abilities(second)
    {
        return None;
    }

    let (target, plural_target) = describe_apply_continuous_target(first);
    if let Some(text) = describe_color_subtype_addition_pair(first, second, &target, plural_target)
    {
        return Some(text);
    }
    let mut clauses = describe_apply_continuous_clauses(first, plural_target);
    clauses.extend(describe_apply_continuous_clauses(second, plural_target));
    if clauses.is_empty() {
        return None;
    }

    let quoted_granted_ability = clauses.iter().any(|clause| clause.contains('"'));
    let text = apply_continuous_text_with_tail(
        format!("{target} {}", join_with_and(&clauses)),
        describe_apply_continuous_tail(first),
        quoted_granted_ability,
    );
    Some(normalize_each_other_continuous_subject(text))
}

pub(crate) fn describe_compact_tagged_apply_continuous_pair(
    first_effect: &Effect,
    second_effect: &Effect,
) -> Option<String> {
    let tagged = first_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if !is_implicit_reference_tag(tagged.tag.as_str()) {
        return None;
    }

    let first = tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let second = if let Some(apply) =
        second_effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
    {
        apply
    } else if let Some(tagged) = second_effect.downcast_ref::<crate::effects::TaggedEffect>()
        && is_implicit_reference_tag(tagged.tag.as_str())
    {
        tagged
            .effect
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?
    } else {
        return None;
    };

    let first_spec = first.target_spec.as_ref()?;
    let second_spec = second.target_spec.as_ref()?;
    if !choose_spec_references_tag(second_spec, tagged.tag.as_str()) {
        return None;
    }
    if choose_spec_references_tag(first_spec, tagged.tag.as_str()) {
        return None;
    }
    if first.target != second.target
        || apply_continuous_preserves_source_abilities(first)
            != apply_continuous_preserves_source_abilities(second)
    {
        return None;
    }

    let (target, plural_target) = describe_apply_continuous_target(first);
    if let Some(text) = describe_color_subtype_addition_pair(first, second, &target, plural_target)
    {
        return Some(text);
    }
    if tagged_apply_pair_preserves_animated_land(first, second)
        && (first.until == second.until || matches!(second.until, Until::Forever))
        && (first.condition == second.condition || second.condition.is_none())
        && let Some(text) =
            describe_apply_continuous_animation_effect(first, &target, plural_target)
    {
        return Some(text);
    }
    if first.until != second.until || first.condition != second.condition {
        return None;
    }

    let mut clauses = describe_apply_continuous_clauses(first, plural_target);
    clauses.extend(describe_apply_continuous_clauses(second, plural_target));
    if clauses.is_empty() {
        return None;
    }

    let quoted_granted_ability = clauses.iter().any(|clause| clause.contains('"'));
    let text = apply_continuous_text_with_tail(
        format!("{target} {}", join_with_and(&clauses)),
        describe_apply_continuous_tail(first),
        quoted_granted_ability,
    );
    Some(normalize_each_other_continuous_subject(text))
}

pub(crate) fn tagged_apply_pair_preserves_animated_land(
    first: &crate::effects::ApplyContinuousEffect,
    second: &crate::effects::ApplyContinuousEffect,
) -> bool {
    matches!(
        &first.modification,
        Some(crate::continuous::Modification::AddCardTypes(card_types))
            if card_types.contains(&CardType::Creature)
    ) && matches!(
        &second.modification,
        Some(crate::continuous::Modification::AddCardTypes(card_types))
            if card_types.len() == 1 && card_types.contains(&CardType::Land)
    ) && second.additional_modifications.is_empty()
        && second.runtime_modifications.is_empty()
        && first
            .target_spec
            .as_ref()
            .and_then(choose_spec_land_filter)
            .is_some()
}

pub(crate) fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate.as_str() == tag,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_tag(inner, tag)
        }
        _ => false,
    }
}

pub(crate) fn describe_attached_object_for_tag(tag: &str, spec: Option<&ChooseSpec>) -> String {
    let default = match tag {
        "enchanted" => "enchanted permanent",
        "equipped" => "equipped creature",
        _ => "attached object",
    };

    if tag != "enchanted" {
        return default.to_string();
    }

    let spec = match spec {
        Some(ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _)) => Some(inner.as_ref()),
        other => other,
    };

    let Some(ChooseSpec::Object(filter) | ChooseSpec::All(filter)) = spec else {
        return default.to_string();
    };
    let references_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    });
    if !references_tag {
        return default.to_string();
    }

    if filter.subtypes.contains(&Subtype::Equipment) {
        return format!("{tag} equipment");
    }

    if filter.card_types.len() == 1 && filter.all_card_types.is_empty() {
        return format!(
            "enchanted {}",
            describe_card_type_word_local(filter.card_types[0])
        );
    }

    default.to_string()
}

pub(crate) fn describe_tag_attached_then_tap_or_untap(
    tag_attached: &crate::effects::TagAttachedToSourceEffect,
    next: &Effect,
) -> Option<String> {
    let tag = tag_attached.tag.as_str();
    if !matches!(tag, "enchanted" | "equipped") {
        return None;
    }

    if let Some(tap) = next.downcast_ref::<crate::effects::TapEffect>()
        && choose_spec_references_tag(&tap.target, tag)
    {
        let attached_object = describe_attached_object_for_tag(tag, Some(&tap.target));
        return Some(format!("Tap {attached_object}"));
    }
    if let Some(untap) = next.downcast_ref::<crate::effects::UntapEffect>()
        && choose_spec_references_tag(&untap.target, tag)
    {
        let attached_object = describe_attached_object_for_tag(tag, Some(&untap.target));
        return Some(format!("Untap {attached_object}"));
    }
    None
}

pub(crate) fn describe_tag_attached_then_unattach(
    tag_attached: &crate::effects::TagAttachedToSourceEffect,
    next: &Effect,
) -> Option<String> {
    let tag = tag_attached.tag.as_str();
    if !matches!(tag, "enchanted" | "equipped") {
        return None;
    }
    let unattach = next.downcast_ref::<crate::effects::UnattachObjectsEffect>()?;
    if !choose_spec_filter_references_tag(&unattach.objects, tag)
        && !choose_spec_references_tag(&unattach.objects, tag)
    {
        return None;
    }
    Some(format!(
        "Unattach {}",
        describe_attached_object_for_tag(tag, Some(&unattach.objects))
    ))
}

pub(crate) fn describe_gain_control_target_player_creatures(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if !matches!(effect.until, Until::Forever) {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(filter) = &effect.target else {
        return None;
    };
    if filter.zone != Some(Zone::Battlefield)
        || filter.card_types != [CardType::Creature]
        || filter.controller.is_none()
    {
        return None;
    }
    let Some(PlayerFilter::Target(player)) = &filter.controller else {
        return None;
    };
    let player = describe_player_filter(&PlayerFilter::Target(player.clone()));
    Some(format!("Gain control of all creatures {player} controls"))
}

pub(crate) fn object_filter_references_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == tag
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
    }) || filter
        .any_of
        .iter()
        .any(|candidate| object_filter_references_tag(candidate, tag))
}

pub(crate) fn choose_spec_filter_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_references_tag(filter, tag)
        }
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_filter_references_tag(inner, tag)
        }
        _ => false,
    }
}

pub(crate) fn describe_tag_attached_then_double_power(
    tag_attached: &crate::effects::TagAttachedToSourceEffect,
    next: &Effect,
) -> Option<String> {
    let tag = tag_attached.tag.as_str();
    if tag != "equipped" {
        return None;
    }
    let apply = next.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::EndOfTurn
        || apply.condition.is_some()
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || apply.runtime_modifications.len() != 1
        || !apply
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_filter_references_tag(spec, tag))
    {
        return None;
    }
    let crate::effects::RuntimeModification::ModifyPowerToughness { power, toughness } =
        &apply.runtime_modifications[0]
    else {
        return None;
    };
    if *toughness != Value::Fixed(0) {
        return None;
    }
    let Value::PowerOf(power_spec) = power else {
        return None;
    };
    if !choose_spec_filter_references_tag(power_spec, tag) {
        return None;
    }
    let attached_object = describe_attached_object_for_tag(tag, apply.target_spec.as_ref());
    Some(format!(
        "Double {attached_object}'s power until end of turn"
    ))
}

pub(crate) fn is_generated_internal_tag(tag: &str) -> bool {
    effect_text_shared::is_generated_internal_tag(tag)
}

pub(crate) fn is_implicit_reference_tag(tag: &str) -> bool {
    effect_text_shared::is_implicit_reference_tag(tag)
}

pub(crate) fn is_aura_only_filter(filter: &ObjectFilter) -> bool {
    if filter.subtypes.as_slice() != [Subtype::Aura] {
        return false;
    }
    if !filter.card_types.is_empty() && filter.card_types.as_slice() != [CardType::Enchantment] {
        return false;
    }
    let mut bare = filter.clone();
    bare.subtypes.clear();
    if bare.card_types.as_slice() == [CardType::Enchantment] {
        bare.card_types.clear();
    }
    bare == ObjectFilter::default()
}

pub(crate) fn describe_attached_object_color_condition(
    tag: &TagKey,
    filter: &ObjectFilter,
) -> Option<String> {
    let subject = match tag.as_str() {
        "enchanted" => "enchanted creature",
        "equipped" => "equipped creature",
        _ => return None,
    };
    let colors = filter.colors?;
    let mut bare = filter.clone();
    bare.colors = None;
    if bare != ObjectFilter::default() {
        return None;
    }
    let color_text = describe_token_color_words(colors, false);
    if color_text.is_empty() {
        return None;
    }
    Some(format!("{subject} is {color_text}"))
}

pub(crate) fn describe_attached_object_type_condition(
    tag: &TagKey,
    filter: &ObjectFilter,
) -> Option<String> {
    let subject = match tag.as_str() {
        "enchanted" => "enchanted creature",
        "equipped" => "equipped creature",
        _ => return None,
    };

    if filter.subtypes.len() == 1 {
        let subtype = format!("{:?}", filter.subtypes[0]).to_ascii_lowercase();
        let mut bare = filter.clone();
        bare.subtypes.clear();
        if bare == ObjectFilter::default() {
            return Some(format!(
                "{subject} is {}",
                with_indefinite_article(&subtype)
            ));
        }
    }

    if filter.card_types.len() == 1 {
        let card_type = filter.card_types[0].to_string().to_ascii_lowercase();
        let mut bare = filter.clone();
        bare.card_types.clear();
        if bare == ObjectFilter::default() {
            return Some(format!(
                "{subject} is {}",
                with_indefinite_article(&card_type)
            ));
        }
    }

    // Subtype list on the attached object ("enchanted creature is a Wolf or
    // Werewolf" — Howl of the Hunt).
    if !filter.subtypes.is_empty() && filter.card_types.len() <= 1 {
        let mut bare = filter.clone();
        bare.subtypes.clear();
        bare.card_types.clear();
        bare.type_or_subtype_union = false;
        bare.zone = None;
        if bare == ObjectFilter::default() {
            let names = filter
                .subtypes
                .iter()
                .map(|subtype| format!("{subtype:?}"))
                .collect::<Vec<_>>()
                .join(" or ");
            return Some(format!("{subject} is {}", with_indefinite_article(&names)));
        }
    }

    // Tap state on the attached object ("enchanted creature is untapped" —
    // Narcolepsy).
    if filter.tapped != filter.untapped && filter.card_types.len() <= 1 {
        let mut bare = filter.clone();
        bare.tapped = false;
        bare.untapped = false;
        bare.card_types.clear();
        bare.zone = None;
        if bare == ObjectFilter::default() {
            let state = if filter.untapped {
                "untapped"
            } else {
                "tapped"
            };
            return Some(format!("{subject} is {state}"));
        }
    }

    None
}

pub(crate) fn describe_until(until: &Until) -> String {
    match until {
        Until::Forever => "forever".to_string(),
        Until::EndOfTurn => "until end of turn".to_string(),
        Until::YourNextTurn => "until your next turn".to_string(),
        Until::YourNextTurnEnd => "until the end of your next turn".to_string(),
        Until::YourNextUpkeep => "until your next upkeep".to_string(),
        Until::ControllersNextUntapStep => "during its controller's next untap step".to_string(),
        Until::EndOfCombat => "until end of combat".to_string(),
        Until::ThisLeavesTheBattlefield => {
            "for as long as this source remains on the battlefield".to_string()
        }
        Until::SourceUntaps => "for as long as this source remains tapped".to_string(),
        Until::YouStopControllingThis => "for as long as you control this source".to_string(),
        Until::TurnsPass(turns) => format!("for {} turn(s)", describe_value(turns)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntapRestrictionSubject {
    text: String,
    plural: bool,
    controller_is_you: bool,
}

impl UntapRestrictionSubject {
    pub(crate) fn singular(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            plural: false,
            controller_is_you: false,
        }
    }

    pub(crate) fn plural(text: impl Into<String>, controller_is_you: bool) -> Self {
        Self {
            text: text.into(),
            plural: true,
            controller_is_you,
        }
    }

    pub(crate) fn source(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            plural: false,
            controller_is_you: true,
        }
    }

    pub(crate) fn controlled_by_you(text: impl Into<String>, plural: bool) -> Self {
        Self {
            text: text.into(),
            plural,
            controller_is_you: true,
        }
    }
}

pub(crate) fn untap_restriction_filter_noun(filter: &ObjectFilter) -> &'static str {
    match filter.card_types.as_slice() {
        [CardType::Artifact] => "artifact",
        [CardType::Battle] => "battle",
        [CardType::Creature] => "creature",
        [CardType::Enchantment] => "enchantment",
        [CardType::Land] => "land",
        [CardType::Planeswalker] => "planeswalker",
        _ => "permanent",
    }
}

fn default_untap_restriction_subject(filter: &ObjectFilter) -> UntapRestrictionSubject {
    if !filter.any_of.is_empty() {
        let subjects = filter
            .any_of
            .iter()
            .map(|branch| {
                let description = branch.description();
                let description = strip_indefinite_article(&description)
                    .trim()
                    .trim_end_matches(" on the battlefield")
                    .trim();
                format!("each {description}")
            })
            .collect::<Vec<_>>();
        let text = capitalize_first(&join_with_and(&subjects));
        let controlled_by_you = filter
            .any_of
            .iter()
            .all(|branch| branch.controller == Some(PlayerFilter::You));
        return if controlled_by_you {
            UntapRestrictionSubject::controlled_by_you(text, false)
        } else {
            UntapRestrictionSubject::singular(text)
        };
    }

    if filter.source {
        let subject = filter
            .source_surface
            .as_ref()
            .map(describe_source_reference_surface_text)
            .unwrap_or_else(|| format!("this {}", untap_restriction_filter_noun(filter)));
        return UntapRestrictionSubject::source(capitalize_first(&subject));
    }

    let tagged = filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        .collect::<Vec<_>>();
    if let [constraint] = tagged.as_slice() {
        let text = if matches!(constraint.tag.as_str(), "__it__" | "it") {
            "It".to_string()
        } else {
            format!("That {}", untap_restriction_filter_noun(filter))
        };
        return if filter.controller == Some(PlayerFilter::You) {
            UntapRestrictionSubject::controlled_by_you(text, false)
        } else {
            UntapRestrictionSubject::singular(text)
        };
    }

    let description = filter.description();
    let description = strip_indefinite_article(&description)
        .trim()
        .trim_end_matches(" on the battlefield")
        .trim();
    if filter.controller == Some(PlayerFilter::You) {
        return UntapRestrictionSubject::plural(
            capitalize_first(&pluralize_relative_object_phrase(description)),
            true,
        );
    }

    UntapRestrictionSubject::singular(format!("Each {description}"))
}

pub(crate) fn describe_untap_restriction_for_subject(
    cant: &crate::effects::CantEffect,
    subject: UntapRestrictionSubject,
) -> Option<String> {
    let crate::effect::Restriction::Untap(_) = &cant.restriction else {
        return None;
    };
    if !matches!(
        cant.duration,
        Until::Forever
            | Until::ControllersNextUntapStep
            | Until::ThisLeavesTheBattlefield
            | Until::SourceUntaps
            | Until::YouStopControllingThis
    ) {
        return None;
    }

    let verb = if subject.plural {
        "don't untap"
    } else {
        "doesn't untap"
    };
    let controller_step = if subject.controller_is_you {
        "your untap step"
    } else if subject.plural {
        "their controllers' untap steps"
    } else {
        "its controller's untap step"
    };
    let controller_next_step = if subject.controller_is_you {
        "your next untap step"
    } else if subject.plural {
        "their controllers' next untap steps"
    } else {
        "its controller's next untap step"
    };

    let mut text = match cant.duration {
        Until::ControllersNextUntapStep => {
            format!("{} {verb} during {controller_next_step}", subject.text)
        }
        _ => format!("{} {verb} during {controller_step}", subject.text),
    };
    if matches!(
        cant.duration,
        Until::ThisLeavesTheBattlefield | Until::SourceUntaps | Until::YouStopControllingThis
    ) {
        text.push(' ');
        text.push_str(&describe_until(&cant.duration));
    }
    Some(text)
}

pub(crate) fn describe_untap_restriction_oracle(
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let crate::effect::Restriction::Untap(filter) = &cant.restriction else {
        return None;
    };
    describe_untap_restriction_for_subject(cant, default_untap_restriction_subject(filter))
}

pub(crate) fn describe_damage_filter(filter: &crate::prevention::DamageFilter) -> String {
    let mut parts = Vec::new();
    if filter.combat_only {
        parts.push("combat damage".to_string());
    } else if filter.noncombat_only {
        parts.push("noncombat damage".to_string());
    } else {
        parts.push("all damage".to_string());
    }

    if let Some(source_filter) = &filter.from_source {
        let mut source_text = if source_filter.zone == Some(Zone::Battlefield)
            && source_filter.controller == Some(PlayerFilter::Opponent)
        {
            let mut bare = source_filter.clone();
            bare.controller = None;
            let subject = pluralize_noun_phrase(
                strip_indefinite_article(&bare.description())
                    .trim()
                    .trim_end_matches(" on the battlefield")
                    .trim(),
            );
            format!("{subject} your opponents control")
        } else {
            source_filter.description()
        };
        if let Some(stripped) = source_text.strip_suffix(" permanent") {
            source_text = stripped.to_string();
        }
        parts.push(format!("from {source_text} sources"));
    }
    if let Some(source_types) = &filter.from_card_types
        && !source_types.is_empty()
    {
        let text = source_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>()
            .join(" or ");
        parts.push(format!("from {text} sources"));
    }
    if let Some(source_colors) = &filter.from_colors
        && !source_colors.is_empty()
    {
        let text = source_colors
            .iter()
            .map(|color| color.name().to_string())
            .collect::<Vec<_>>()
            .join(" or ");
        parts.push(format!("from {text} sources"));
    }
    if filter.from_specific_source.is_some() {
        parts.push("from that source".to_string());
    }
    if filter.excluded_specific_source.is_some() {
        parts.push("other than that source".to_string());
    }

    parts.join(" ")
}

pub(crate) fn describe_prevention_damage_source(filter: &ObjectFilter, chosen: bool) -> String {
    let description = filter.description();
    let bare = strip_indefinite_article(&description).trim();
    if bare.is_empty() {
        return if chosen {
            "a source of your choice".to_string()
        } else {
            "a source".to_string()
        };
    }

    let creature_only = filter.card_types.len() == 1
        && filter.card_types[0] == CardType::Creature
        && filter.all_card_types.is_empty();
    if creature_only {
        if chosen {
            if let Some((head, tail)) = bare.split_once(" with ") {
                return format!(
                    "{} of your choice with {tail}",
                    with_indefinite_article(head)
                );
            }
            return format!("{} of your choice", with_indefinite_article(bare));
        }
        return with_indefinite_article(bare);
    }

    let source = with_indefinite_article(&format!("{bare} source"));
    if chosen {
        format!("{source} of your choice")
    } else {
        source
    }
}

fn describe_prevention_matching_permanents(filter: &ObjectFilter) -> String {
    let surface = describe_for_each_filter(filter);
    let surface = surface
        .strip_suffix(" on the battlefield")
        .unwrap_or(&surface);
    pluralize_noun_phrase(surface)
}

pub(crate) fn describe_prevention_target(target: &crate::prevention::PreventionTarget) -> String {
    match target {
        crate::prevention::PreventionTarget::Player(_) => "that player".to_string(),
        crate::prevention::PreventionTarget::Permanent(_) => "that permanent".to_string(),
        crate::prevention::PreventionTarget::PermanentsMatching(filter) => {
            if filter.source {
                filter.description()
            } else {
                describe_prevention_matching_permanents(filter)
            }
        }
        crate::prevention::PreventionTarget::YouAndPermanentsMatching(filter) => format!(
            "you and {}",
            describe_prevention_matching_permanents(filter)
        ),
        crate::prevention::PreventionTarget::Players => "players".to_string(),
        crate::prevention::PreventionTarget::You => "you".to_string(),
        crate::prevention::PreventionTarget::YouAndPermanentsYouControl => {
            "you and permanents you control".to_string()
        }
        crate::prevention::PreventionTarget::All => "all players and permanents".to_string(),
    }
}

pub(crate) fn describe_restriction(restriction: &crate::effect::Restriction) -> String {
    match restriction {
        crate::effect::Restriction::AdditionalLandPlays(filter, count) => {
            if *count == 1 {
                format!(
                    "{} may play an additional land",
                    describe_player_set_filter(filter)
                )
            } else {
                format!(
                    "{} may play {} additional lands",
                    describe_player_set_filter(filter),
                    count
                )
            }
        }
        crate::effect::Restriction::GainLife(filter) => {
            format!("{} can't gain life", describe_player_set_filter(filter))
        }
        crate::effect::Restriction::LoseUnspentMana(filter, color) => {
            let subject = describe_player_set_filter(filter);
            let dont = if subject.eq_ignore_ascii_case("you") {
                "don't"
            } else {
                "doesn't"
            };
            let subject = if subject.eq_ignore_ascii_case("each player") {
                "players don't".to_string()
            } else {
                format!("{subject} {dont}")
            };
            match color {
                Some(color) => format!(
                    "{subject} lose unspent {} mana as steps and phases end",
                    color.name()
                ),
                None => format!("{subject} lose unspent mana as steps and phases end"),
            }
        }
        crate::effect::Restriction::SearchLibraries(filter) => {
            format!(
                "{} can't search libraries",
                describe_player_set_filter(filter)
            )
        }
        crate::effect::Restriction::CastSpellsMatching(filter, spell_filter) => format!(
            "{} can't cast {}",
            describe_player_set_filter(filter),
            describe_cast_ban_spell_filter(spell_filter)
        ),
        crate::effect::Restriction::CastSpellsOnlyAsSorcery(filter) => format!(
            "{} can cast spells only any time they could cast a sorcery",
            describe_player_set_filter(filter)
        ),
        crate::effect::Restriction::ActivateNonManaAbilities(filter) => {
            format!(
                "{} can't activate non-mana abilities",
                describe_player_set_filter(filter)
            )
        }
        crate::effect::Restriction::ActivateAbilitiesOf(filter) => {
            let description = filter.description();
            let subject = description
                .strip_prefix("target ")
                .unwrap_or(description.as_str());
            let subject = if subject.eq_ignore_ascii_case("permanent")
                && filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
            {
                "that permanent"
            } else {
                subject
            };
            format!("activated abilities of {} can't be activated", subject)
        }
        crate::effect::Restriction::ActivateTapAbilitiesOf(filter) => {
            let description = filter.description();
            let subject = description
                .strip_prefix("target ")
                .unwrap_or(description.as_str());
            let subject = if subject.eq_ignore_ascii_case("permanent")
                && filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
            {
                "that permanent"
            } else {
                subject
            };
            format!(
                "activated abilities with {{T}} in their costs of {} can't be activated",
                subject
            )
        }
        crate::effect::Restriction::ActivateNonManaAbilitiesOf(filter) => {
            let description = filter.description();
            let subject = description
                .strip_prefix("target ")
                .unwrap_or(description.as_str());
            let subject = if subject.eq_ignore_ascii_case("permanent")
                && filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
            {
                "that permanent"
            } else {
                subject
            };
            format!(
                "non-mana activated abilities of {} can't be activated",
                subject
            )
        }
        crate::effect::Restriction::CastMoreThanOneSpellEachTurn(filter, spell_filter) => format!(
            "{} can't cast more than one {} each turn",
            describe_player_set_filter(filter),
            describe_cast_limit_spell_filter(spell_filter)
        ),
        crate::effect::Restriction::DrawCards(filter) => {
            format!("{} can't draw cards", describe_player_set_filter(filter))
        }
        crate::effect::Restriction::DrawExtraCards(filter) => {
            format!(
                "{} can't draw extra cards",
                describe_player_set_filter(filter)
            )
        }
        crate::effect::Restriction::PoisonCounters(filter) => {
            format!(
                "{} can't get poison counters",
                describe_player_set_filter(filter)
            )
        }
        crate::effect::Restriction::LoseLife(filter) => {
            format!("{} can't lose life", describe_player_set_filter(filter))
        }
        crate::effect::Restriction::DamageCauseLifeLoss(filter) => match filter {
            PlayerFilter::You => "damage doesn't cause you to lose life".to_string(),
            PlayerFilter::Any => "damage doesn't cause players to lose life".to_string(),
            PlayerFilter::IteratedPlayer => {
                "damage doesn't cause that player to lose life".to_string()
            }
            _ => format!(
                "damage doesn't cause {} to lose life",
                describe_player_filter(filter)
            ),
        },
        crate::effect::Restriction::ChangeLifeTotal(filter) => {
            format!(
                "{} can't have life total changed",
                describe_player_set_filter(filter)
            )
        }
        crate::effect::Restriction::LoseGame(filter) => {
            format!("{} can't lose the game", describe_player_set_filter(filter))
        }
        crate::effect::Restriction::WinGame(filter) => {
            format!("{} can't win the game", describe_player_set_filter(filter))
        }
        crate::effect::Restriction::BecomeMonarch(filter) => {
            format!(
                "{} can't become the monarch",
                describe_player_set_filter(filter)
            )
        }
        crate::effect::Restriction::PreventDamage => "damage can't be prevented".to_string(),
        crate::effect::Restriction::Attack(filter) => {
            format!("{} can't attack", filter.description())
        }
        crate::effect::Restriction::AttackPlayerOrPlaneswalkersControlledBy {
            attackers,
            player,
        } => {
            let attacker_text = if attackers
                == &crate::target::ObjectFilter::creature()
                    .controlled_by(PlayerFilter::IteratedPlayer)
            {
                "creatures that player controls".to_string()
            } else {
                attackers.description()
            };
            let planeswalker_controller = match player {
                PlayerFilter::You => "you control".to_string(),
                PlayerFilter::Opponent => "your opponents control".to_string(),
                PlayerFilter::Any => "players control".to_string(),
                PlayerFilter::IteratedPlayer => "that player controls".to_string(),
                PlayerFilter::Target(inner) if inner.as_ref() == &PlayerFilter::You => {
                    "you control".to_string()
                }
                _ => format!("{} controls", describe_player_filter(player)),
            };
            format!(
                "{} can't attack {} or planeswalkers {}",
                attacker_text,
                describe_player_filter(player),
                planeswalker_controller
            )
        }
        crate::effect::Restriction::AttackAlone(filter) => {
            format!("{} can't attack alone", filter.description())
        }
        crate::effect::Restriction::Block(filter) => {
            format!("{} can't block", filter.description())
        }
        crate::effect::Restriction::BlockSpecificAttacker { blockers, attacker } => {
            format!(
                "{} can't block {}",
                blockers.description(),
                attacker.description()
            )
        }
        crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } => {
            if blockers.description().eq_ignore_ascii_case("creature")
                && attacker.description().eq_ignore_ascii_case("creature")
            {
                return "All creatures able to block target creature do so".to_string();
            }
            format!(
                "{} must block {} if able",
                blockers.description(),
                attacker.description()
            )
        }
        crate::effect::Restriction::MustBeBlocked(filter) => {
            format!("{} must be blocked if able", filter.description())
        }
        crate::effect::Restriction::BlockAlone(filter) => {
            format!("{} can't block alone", filter.description())
        }
        crate::effect::Restriction::Untap(filter) => {
            format!("{} can't untap", filter.description())
        }
        crate::effect::Restriction::BeBlocked(filter) => {
            format!("{} can't be blocked", filter.description())
        }
        crate::effect::Restriction::BeDestroyed(filter) => {
            format!("{} can't be destroyed", filter.description())
        }
        crate::effect::Restriction::BeRegenerated(filter) => {
            let subject = describe_prior_effect_tagged_filter_surface(filter)
                .map(|subject| capitalize_first(&subject))
                .unwrap_or_else(|| filter.description());
            format!("{subject} can't be regenerated")
        }
        crate::effect::Restriction::BeSacrificed(filter) => {
            format!("{} can't be sacrificed", filter.description())
        }
        crate::effect::Restriction::HaveCountersPlaced(filter) => {
            format!("counters can't be placed on {}", filter.description())
        }
        crate::effect::Restriction::BeTargeted(filter) => {
            format!("{} can't be targeted", filter.description())
        }
        crate::effect::Restriction::BeTargetedFrom(filter, source_filter) => {
            if let Some(source_description) = describe_spell_targeting_source_filter(source_filter)
            {
                return format!(
                    "{} can't be the target of {}",
                    filter.description(),
                    source_description
                );
            }
            let source_description = describe_hexproof_from_filter(source_filter);
            format!(
                "{} can't be the target of {} spells or abilities from {} sources",
                filter.description(),
                source_description,
                source_description
            )
        }
        crate::effect::Restriction::BeTargetedPlayer(filter) => {
            format!("{} can't be targeted", describe_player_set_filter(filter))
        }
        crate::effect::Restriction::BeTargetedPlayerFrom(player, source_filter) => {
            let opponent_sources_only =
                source_filter.controller == Some(crate::target::PlayerFilter::Opponent) && {
                    let mut stripped = source_filter.clone();
                    stripped.controller = None;
                    stripped == ObjectFilter::default()
                };
            if opponent_sources_only {
                return format!("{} have hexproof", describe_player_set_filter(player));
            }
            let source_description = describe_hexproof_from_filter(source_filter);
            format!(
                "{} have hexproof from {}",
                describe_player_set_filter(player),
                source_description
            )
        }
        crate::effect::Restriction::BeCountered(filter) => {
            format!("{} can't be countered", filter.description())
        }
        crate::effect::Restriction::Transform(filter) => {
            format!("{} can't transform", filter.description())
        }
        crate::effect::Restriction::PhaseOut(filter) => {
            format!("{} can't phase out", filter.description())
        }
        crate::effect::Restriction::AttackOrBlock(filter) => {
            format!("{} can't attack or block", filter.description())
        }
        crate::effect::Restriction::AttackOrBlockAlone(filter) => {
            format!("{} can't attack or block alone", filter.description())
        }
    }
}

pub(crate) fn describe_spell_targeting_source_filter(
    source_filter: &ObjectFilter,
) -> Option<String> {
    if source_filter.zone != Some(Zone::Stack)
        || source_filter.stack_kind != Some(StackObjectKind::Spell)
    {
        return None;
    }

    let mut rest = source_filter.clone();
    rest.zone = None;
    rest.stack_kind = None;
    if rest.subtypes == [crate::types::Subtype::Aura]
        && (rest.card_types.is_empty() || rest.card_types == [crate::types::CardType::Enchantment])
    {
        rest.subtypes.clear();
        rest.card_types.clear();
        if rest == ObjectFilter::default() {
            return Some("Aura spells".to_string());
        }
    }

    let description = source_filter.description();
    description
        .strip_suffix(" spell")
        .map(|description| format!("{description} spells"))
}

pub(crate) fn describe_hexproof_from_filter(filter: &ObjectFilter) -> String {
    if !filter.any_of.is_empty() {
        return filter
            .any_of
            .iter()
            .map(describe_hexproof_from_filter)
            .collect::<Vec<_>>()
            .join(" or ");
    }

    if is_exactly_all_magic_colors_filter(filter) {
        return "each color".to_string();
    }

    let description = filter.description();
    description
        .strip_suffix(" permanent")
        .or_else(|| description.strip_suffix(" spell"))
        .or_else(|| description.strip_suffix(" source"))
        .unwrap_or(description.as_str())
        .to_string()
}

pub(crate) fn is_exactly_all_magic_colors_filter(filter: &ObjectFilter) -> bool {
    let mut expected = ObjectFilter::default();
    expected.colors = Some(all_magic_colors());
    filter == &expected
}

pub(crate) fn all_magic_colors() -> crate::color::ColorSet {
    crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN)
}

pub(crate) fn describe_sacrifice_cost_object_condition(
    tag: &crate::tag::TagKey,
    filter: &ObjectFilter,
) -> Option<String> {
    if !tag.as_str().starts_with("sacrifice_cost_") {
        return None;
    }
    let colors = filter.colors?;
    if colors.is_empty() || filter.card_types.len() != 1 {
        return None;
    }
    let mut rest = filter.clone();
    rest.colors = None;
    rest.card_types.clear();
    if rest != ObjectFilter::default() {
        return None;
    }

    Some(format!(
        "the sacrificed {} was {}",
        describe_card_type_word_local(filter.card_types[0]),
        describe_token_color_words(colors, false)
    ))
}

pub(crate) fn describe_comparison(cmp: &Comparison) -> String {
    match cmp {
        Comparison::GreaterThan(n) => format!("is greater than {n}"),
        Comparison::GreaterThanOrEqual(n) => format!("is at least {n}"),
        Comparison::Equal(n) => format!("is equal to {n}"),
        Comparison::LessThan(n) => format!("is less than {n}"),
        Comparison::LessThanOrEqual(n) => format!("is at most {n}"),
        Comparison::NotEqual(n) => format!("is not equal to {n}"),
        Comparison::OneOf(values) => format!(
            "is one of {}",
            values
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Comparison::BetweenInclusive(min, max) => {
            format!("is between {min} and {max} inclusive")
        }
    }
}

pub(crate) fn basic_land_types_multiplier(value: &Value) -> Option<(&ObjectFilter, i32)> {
    match value {
        Value::BasicLandTypesAmong(filter) => Some((filter, 1)),
        Value::Scaled(value, factor) => {
            let (filter, mult) = basic_land_types_multiplier(value)?;
            Some((filter, mult * factor))
        }
        Value::Add(left, right) => {
            let (left_filter, left_mult) = basic_land_types_multiplier(left)?;
            let (right_filter, right_mult) = basic_land_types_multiplier(right)?;
            if left_filter == right_filter {
                Some((left_filter, left_mult + right_mult))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn describe_basic_land_type_scope(filter: &ObjectFilter) -> String {
    let lands = describe_for_each_filter(filter);
    if lands == "land" {
        return "lands".to_string();
    }
    if let Some(rest) = lands.strip_prefix("land ") {
        return format!("lands {rest}");
    }
    if let Some(rest) = lands.strip_prefix("a land ") {
        return format!("lands {rest}");
    }
    lands
}

pub(crate) fn describe_basic_land_types_among(filter: &ObjectFilter) -> String {
    format!(
        "basic land types among {}",
        describe_basic_land_type_scope(filter)
    )
}

pub(crate) fn describe_colors_that_sacrificed_object_was(filter: &ObjectFilter) -> Option<String> {
    if filter.card_types.len() != 1 {
        return None;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && (constraint.tag.as_str().starts_with("sacrificed_")
                || constraint.tag.as_str().starts_with("sacrifice_cost_"))
    }) {
        return None;
    }

    let mut rest = filter.clone();
    rest.card_types.clear();
    rest.tagged_constraints.clear();
    if rest.zone == Some(Zone::Battlefield) {
        rest.zone = None;
    }
    if rest != ObjectFilter::default() {
        return None;
    }

    Some(format!(
        "colors that {} was",
        describe_card_type_word_local(filter.card_types[0])
    ))
}

pub(crate) fn describe_colors_among(filter: &ObjectFilter) -> String {
    if let Some(text) = describe_colors_that_sacrificed_object_was(filter) {
        return text;
    }
    format!("colors among {}", describe_for_each_filter(filter))
}

fn describe_prior_result_active_action(action: crate::effect::PriorEffectAction) -> &'static str {
    match action {
        crate::effect::PriorEffectAction::Cast => "cast",
        crate::effect::PriorEffectAction::Chosen => "choose",
        crate::effect::PriorEffectAction::Connived => "connive",
        crate::effect::PriorEffectAction::Countered => "counter",
        crate::effect::PriorEffectAction::CountersPut => "put counters on",
        crate::effect::PriorEffectAction::DealtDamage => "deal damage to",
        crate::effect::PriorEffectAction::Destroyed => "destroy",
        crate::effect::PriorEffectAction::Discarded => "discard",
        crate::effect::PriorEffectAction::Drawn => "draw",
        crate::effect::PriorEffectAction::Exiled => "exile",
        crate::effect::PriorEffectAction::Goaded => "goad",
        crate::effect::PriorEffectAction::Milled => "mill",
        crate::effect::PriorEffectAction::PhasedOut => "phase out",
        crate::effect::PriorEffectAction::Prevented => "prevent",
        crate::effect::PriorEffectAction::PutOntoBattlefield => "put onto the battlefield",
        crate::effect::PriorEffectAction::Removed => "remove",
        crate::effect::PriorEffectAction::Returned => "return",
        crate::effect::PriorEffectAction::Revealed => "reveal",
        crate::effect::PriorEffectAction::Sacrificed => "sacrifice",
        crate::effect::PriorEffectAction::Searched => "search for",
        crate::effect::PriorEffectAction::Shuffled => "shuffle",
        crate::effect::PriorEffectAction::Tapped => "tap",
    }
}

fn describe_prior_effect_result_surface(
    surface: &crate::effect::PriorEffectResultSurface,
) -> String {
    if surface.quantifier == crate::effect::PriorEffectResultQuantifier::ActionOnly {
        return match (surface.actor, surface.action) {
            (
                crate::effect::PriorEffectResultActor::You,
                crate::effect::PriorEffectAction::Searched,
            ) => "you search your library this way".to_string(),
            (
                crate::effect::PriorEffectResultActor::It,
                crate::effect::PriorEffectAction::Connived,
            ) => "it connives this way".to_string(),
            (
                crate::effect::PriorEffectResultActor::Passive,
                crate::effect::PriorEffectAction::Prevented,
            ) => "damage is prevented this way".to_string(),
            (
                crate::effect::PriorEffectResultActor::Passive,
                crate::effect::PriorEffectAction::Removed,
            ) => "one or more counters are removed this way".to_string(),
            (
                crate::effect::PriorEffectResultActor::Passive,
                crate::effect::PriorEffectAction::Countered,
            ) => "an ability is countered this way".to_string(),
            (actor, action) => {
                let actor = match actor {
                    crate::effect::PriorEffectResultActor::Passive => "the prior action",
                    crate::effect::PriorEffectResultActor::You => "you",
                    crate::effect::PriorEffectResultActor::ThatPlayer => "that player",
                    crate::effect::PriorEffectResultActor::It => "it",
                };
                format!(
                    "{actor} {} this way",
                    describe_prior_result_active_action(action)
                )
            }
        };
    }
    let mut filter = surface.filter.clone();
    filter.zone = None;
    filter.tagged_constraints.clear();
    filter.set_prior_effect_action_surface(None);
    let base = strip_leading_article(&filter.description())
        .trim()
        .to_string();
    let object = match surface.quantifier {
        crate::effect::PriorEffectResultQuantifier::One => with_indefinite_article(&base),
        crate::effect::PriorEffectResultQuantifier::OneOrMore => {
            format!("one or more {}", pluralize_relative_object_phrase(&base))
        }
        crate::effect::PriorEffectResultQuantifier::ActionOnly => String::new(),
    };
    let actor = match surface.actor {
        crate::effect::PriorEffectResultActor::Passive => None,
        crate::effect::PriorEffectResultActor::You => Some("you"),
        crate::effect::PriorEffectResultActor::ThatPlayer => Some("that player"),
        crate::effect::PriorEffectResultActor::It => Some("it"),
    };
    if let Some(actor) = actor {
        let action = describe_prior_result_active_action(surface.action);
        if object.is_empty() {
            return format!("{actor} {action} this way");
        }
        return format!("{actor} {action} {object} this way");
    }

    let copula = if surface.quantifier == crate::effect::PriorEffectResultQuantifier::OneOrMore {
        "are"
    } else {
        "is"
    };
    format!(
        "{object} {copula} {} this way",
        describe_prior_effect_action(surface.action)
    )
}

pub(crate) fn describe_effect_predicate(predicate: &EffectPredicate) -> String {
    match predicate {
        EffectPredicate::Succeeded => "succeeded".to_string(),
        EffectPredicate::Failed => "failed".to_string(),
        EffectPredicate::Happened => "happened".to_string(),
        EffectPredicate::DidNotHappen => "that doesn't happen".to_string(),
        EffectPredicate::SearchedLibrary => "you search your library this way".to_string(),
        EffectPredicate::HappenedNotReplaced => "happened and was not replaced".to_string(),
        EffectPredicate::ExcessDamageDealt => "excess damage was dealt this way".to_string(),
        EffectPredicate::DealtDamageToPlayer => "a player is dealt damage this way".to_string(),
        EffectPredicate::AffectedObjectMatchesCardType { card_type, negated } => {
            let relation = if *negated { "isn't" } else { "is" };
            format!(
                "the affected object {relation} {}",
                card_type.name().to_ascii_lowercase()
            )
        }
        EffectPredicate::PriorEffectResult(surface) => {
            describe_prior_effect_result_surface(surface)
        }
        EffectPredicate::Value(cmp) => format!("its count {}", describe_comparison(cmp)),
        EffectPredicate::Chosen => "was chosen".to_string(),
        EffectPredicate::WasDeclined => "was declined".to_string(),
    }
}

pub(crate) fn tag_action_from_name(tag: &str) -> Option<&'static str> {
    let base = tag.split('_').next().unwrap_or(tag);
    match base {
        "sacrifice" => Some("sacrificed"),
        "sacrificed" => Some("sacrificed"),
        "destroyed" => Some("destroyed"),
        "damaged" => Some("dealt damage"),
        "counters" => Some("that had counters put on them"),
        "exiled" => Some("exiled"),
        "discarded" => Some("discarded"),
        "revealed" => Some("revealed"),
        "returned" => Some("returned"),
        "countered" => Some("countered"),
        "died" => Some("died"),
        "milled" => Some("milled"),
        "goaded" => Some("goaded"),
        "phased" => Some("that phased out"),
        "prevented" => Some("prevented"),
        "shuffled" => Some("shuffled"),
        "tapped" => Some("tapped"),
        "moved" => Some("put"),
        _ => None,
    }
}

pub(crate) fn this_way_action_from_tag(tag: &TagKey) -> Option<&'static str> {
    if let Some(action) = tag_action_from_name(tag.as_str()) {
        return Some(action);
    }
    for (helper_action, rendered) in [
        ("revealed", "revealed"),
        ("destroyed", "destroyed"),
        ("exiled", "exiled"),
        ("discarded", "discarded"),
        ("sacrificed", "sacrificed"),
        ("milled", "milled"),
        ("returned", "returned"),
        ("countered", "countered"),
        ("died", "died"),
    ] {
        if crate::cards::is_sentence_helper_tag(tag.as_str(), helper_action) {
            return Some(rendered);
        }
    }
    None
}

pub(crate) fn describe_player_tagged_object_text(tag: &TagKey, filter: &ObjectFilter) -> String {
    let action = this_way_action_from_tag(tag);
    let card_context = tag.as_str().starts_with("discarded_")
        || tag.as_str().starts_with("exiled_")
        || tag.as_str().starts_with("revealed_")
        || matches!(action, Some("revealed" | "milled" | "discarded"))
        // A type exclusion such as "noncreature card" is broader than a
        // noncreature permanent. Preserve that card-set surface when the
        // remembered result is in exile.
        || (action == Some("exiled")
            && !filter.excluded_card_types.is_empty());
    if card_context
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
        let type_phrase = join_with_or(&words);
        let described_type = with_indefinite_article(&type_phrase);
        let desc = filter.description();
        let bare_desc = strip_leading_article(&desc);
        let bare_type = strip_leading_article(&described_type);
        if let Some(rest) = bare_desc.strip_prefix(bare_type) {
            return format!("{described_type} card{rest}");
        }
        return with_indefinite_article(&format!("{type_phrase} card"));
    }
    if card_context
        && filter.card_types.is_empty()
        && filter.excluded_card_types.len() == 1
        && filter.zone.is_none()
        && filter.controller.is_none()
        && filter.owner.is_none()
        && filter.subtypes.is_empty()
        && filter.any_of.is_empty()
        && filter.tagged_constraints.is_empty()
    {
        let excluded = describe_card_type_word_local(filter.excluded_card_types[0]);
        return with_indefinite_article(&format!("non{excluded} card"));
    }

    let desc = filter.description();
    let stripped = strip_leading_article(&desc).to_ascii_lowercase();
    if card_context && stripped == "land" {
        return "a land card".to_string();
    }
    if card_context && stripped == "creature" {
        return "a creature card".to_string();
    }
    if filter.subtypes.len() == 1
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.any_of.is_empty()
        && filter.tagged_constraints.is_empty()
    {
        match filter.subtypes[0] {
            Subtype::Equipment => return "an Equipment".to_string(),
            Subtype::Aura => return "an Aura".to_string(),
            _ => {}
        }
    }
    with_indefinite_article(&desc)
}

pub(crate) fn is_owned_player_zone(zone: Option<Zone>) -> bool {
    matches!(
        zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Command)
    )
}

pub(crate) fn describe_owned_player_zone_filter(
    player: &PlayerFilter,
    filter: &ObjectFilter,
) -> String {
    let mut described = filter.clone();
    if described.owner.is_none() {
        described.owner = Some(player.clone());
    }
    described.description()
}

pub(crate) fn describe_player_relative_condition(condition: &Condition) -> Option<String> {
    match condition {
        Condition::PlayerTappedLandForManaThisTurn { player } => {
            if *player != PlayerFilter::IteratedPlayer {
                return None;
            }
            Some("tapped a land for mana this turn".to_string())
        }
        Condition::PlayerHadLandEnterBattlefieldThisTurn { player } => {
            if *player != PlayerFilter::IteratedPlayer {
                return None;
            }
            Some("had a land enter the battlefield under their control this turn".to_string())
        }
        Condition::PlayerDescendedThisTurn { player } => {
            if *player != PlayerFilter::IteratedPlayer {
                return None;
            }
            Some("descended this turn".to_string())
        }
        Condition::ValueComparison {
            left: Value::MaxCardsDrawnThisTurn(PlayerFilter::IteratedPlayer),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count),
        } => {
            let count_text = small_number_word(*count as u32).unwrap_or_else(|| count.to_string());
            Some(format!("drew {count_text} or more cards this turn"))
        }
        Condition::ValueComparison {
            left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count),
        } => {
            let count_text = small_number_word(*count as u32).unwrap_or_else(|| count.to_string());
            Some(format!(
                "had {count_text} or more lands enter the battlefield under their control this turn"
            ))
        }
        Condition::PlayerTaggedObjectMatches {
            player,
            tag,
            filter,
        } => {
            if *player != PlayerFilter::IteratedPlayer {
                return None;
            }
            let action = tag_action_from_name(tag.as_str())?;
            let object_text = with_indefinite_article(&filter.description());
            Some(format!("{action} {object_text} this way"))
        }
        Condition::PlayerTaggedObjectEnteredBattlefieldThisTurn { player, tag } => {
            if *player != PlayerFilter::IteratedPlayer {
                return None;
            }
            let action = tag_action_from_name(tag.as_str())?;
            Some(format!("{action} it this way"))
        }
        Condition::SourceIsInZone(zone) => Some(match zone {
            Zone::Hand => "this card is in your hand".to_string(),
            Zone::Graveyard => "this card is in your graveyard".to_string(),
            Zone::Library => "this card is in your library".to_string(),
            Zone::Exile => "this card is in exile".to_string(),
            Zone::Command => "this card is in the command zone".to_string(),
            _ => return None,
        }),
        _ => None,
    }
}

pub(crate) fn spell_cast_this_turn_condition_filter(
    condition: &Condition,
) -> Option<(&PlayerFilter, &ObjectFilter)> {
    let Condition::ValueComparison {
        left:
            Value::SpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source: false,
            },
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    } = condition
    else {
        return None;
    };
    Some((player, filter))
}

pub(crate) fn describe_spell_cast_condition_object(filter: &ObjectFilter) -> String {
    let described = describe_for_each_filter(filter);
    if described == "card in your hand" || described == "spell in your hand" {
        return "a spell from your hand".to_string();
    }
    if let Some(base) = described.strip_suffix(" card in your hand") {
        return with_indefinite_article(&format!("{base} spell from your hand"));
    }
    if let Some(base) = described.strip_suffix(" spell in your hand") {
        return with_indefinite_article(&format!("{base} spell from your hand"));
    }
    with_indefinite_article(&described)
}

pub(crate) fn describe_both_spell_cast_condition(
    left: &Condition,
    right: &Condition,
) -> Option<String> {
    let (left_player, left_filter) = spell_cast_this_turn_condition_filter(left)?;
    let (right_player, right_filter) = spell_cast_this_turn_condition_filter(right)?;
    if left_player != right_player {
        return None;
    }
    let left_spell = describe_spell_cast_condition_object(left_filter);
    let right_spell = describe_spell_cast_condition_object(right_filter);
    let opener = match left_player {
        PlayerFilter::You => "you've cast".to_string(),
        player => {
            let subject = describe_player_filter(player);
            format!("{} {} cast", subject, player_verb(&subject, "have", "has"))
        }
    };
    Some(format!(
        "{opener} both {left_spell} and {right_spell} this turn"
    ))
}

pub(crate) fn describe_missing_counter_spell_cast_gate(
    left: &Condition,
    right: &Condition,
) -> Option<String> {
    describe_missing_counter_spell_cast_gate_ordered(left, right)
        .or_else(|| describe_missing_counter_spell_cast_gate_ordered(right, left))
}

pub(crate) fn describe_missing_counter_spell_cast_gate_ordered(
    spell_condition: &Condition,
    counter_condition: &Condition,
) -> Option<String> {
    let Condition::SourceHasNoCounter(counter_type) = counter_condition else {
        return None;
    };
    let Condition::Not(inner) = spell_condition else {
        return None;
    };
    if !matches!(
        inner.as_ref(),
        Condition::ValueComparison {
            left: Value::SpellsCastThisTurnMatching {
                exclude_source: false,
                ..
            },
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(1),
        }
    ) {
        return None;
    }
    Some(format!(
        "{} and this creature doesn't have {} on it",
        describe_condition(spell_condition),
        with_indefinite_article(&format!("{} counter", counter_type.description()))
    ))
}

pub(crate) fn happily_permanents_you_control_filter() -> ObjectFilter {
    ObjectFilter::permanent().you_control()
}

pub(crate) fn happily_cards_in_your_graveyard_filter() -> ObjectFilter {
    ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
}

pub(crate) fn happily_card_type_scope_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![
        happily_permanents_you_control_filter(),
        happily_cards_in_your_graveyard_filter(),
    ];
    filter
}

pub(crate) fn describe_happily_scope(filter: &ObjectFilter) -> Option<&'static str> {
    if *filter == happily_permanents_you_control_filter() {
        return Some("permanents you control");
    }
    if *filter == happily_card_type_scope_filter() {
        return Some("permanents you control and/or cards in your graveyard");
    }
    None
}

pub(crate) fn describe_happily_value_comparison(
    left: &Value,
    operator: crate::effect::ValueComparisonOperator,
    right: &Value,
) -> Option<String> {
    match (left, operator, right) {
        (
            Value::ColorsAmong(filter),
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            Value::Fixed(count),
        ) if *count == 5 && *filter == happily_permanents_you_control_filter() => {
            Some("there are five colors among permanents you control".to_string())
        }
        (
            Value::CardTypesAmong(filter),
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            Value::Fixed(count),
        ) => {
            let scope = describe_happily_scope(filter)?;
            let count_text = small_number_word(*count as u32).unwrap_or_else(|| count.to_string());
            Some(format!(
                "there are {count_text} or more card types among {scope}"
            ))
        }
        (
            Value::LifeTotal(PlayerFilter::You),
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            Value::StartingLifeTotal(PlayerFilter::You),
        ) => {
            Some("your life total is greater than or equal to your starting life total".to_string())
        }
        _ => None,
    }
}

pub(crate) fn collect_and_conditions<'a>(condition: &'a Condition, out: &mut Vec<&'a Condition>) {
    if let Condition::And(left, right) = condition {
        collect_and_conditions(left, out);
        collect_and_conditions(right, out);
    } else {
        out.push(condition);
    }
}

pub(crate) fn describe_happily_ever_after_condition(condition: &Condition) -> Option<String> {
    let mut conditions = Vec::new();
    collect_and_conditions(condition, &mut conditions);
    let [first, second, third] = conditions.as_slice() else {
        return None;
    };

    let first = match first {
        Condition::ValueComparison {
            left,
            operator,
            right,
        } => describe_happily_value_comparison(left, *operator, right)?,
        _ => return None,
    };
    let second = match second {
        Condition::ValueComparison {
            left,
            operator,
            right,
        } => describe_happily_value_comparison(left, *operator, right)?,
        _ => return None,
    };
    let third = match third {
        Condition::ValueComparison {
            left,
            operator,
            right,
        } => describe_happily_value_comparison(left, *operator, right)?,
        _ => return None,
    };

    Some(format!("{first}, {second}, and {third}"))
}

pub(crate) fn pluralize_relative_object_phrase(phrase: &str) -> String {
    let mut plural = pluralize_noun_phrase(phrase);
    for (singular, plural_noun) in [
        ("artifact", "artifacts"),
        ("battle", "battles"),
        ("card", "cards"),
        ("creature", "creatures"),
        ("enchantment", "enchantments"),
        ("land", "lands"),
        ("permanent", "permanents"),
        ("planeswalker", "planeswalkers"),
        ("spell", "spells"),
    ] {
        if plural == format!("{singular} you don't controls") {
            plural = format!("{plural_noun} you don't control");
        }
        if plural == format!("{singular} you controls") {
            plural = format!("{plural_noun} you control");
        }
        if plural == format!("{singular} an opponent controlss") {
            plural = format!("{plural_noun} an opponent controls");
        }
        if plural == format!("target {singular} an opponent controlss") {
            plural = format!("{plural_noun} target opponent controls");
        }
        if plural == format!("{singular} target opponent controlss") {
            plural = format!("{plural_noun} target opponent controls");
        }
        if plural == format!("{singular} that player controlss") {
            plural = format!("{plural_noun} that player controls");
        }
        if plural == format!("{singular} target player controlss") {
            plural = format!("{plural_noun} target player controls");
        }
        plural = plural.replace(
            &format!(" {singular} you don't controls"),
            &format!(" {plural_noun} you don't control"),
        );
        plural = plural.replace(
            &format!(" {singular} you controls"),
            &format!(" {plural_noun} you control"),
        );
        plural = plural.replace(
            &format!(" {singular} an opponent controlss"),
            &format!(" {plural_noun} an opponent controls"),
        );
        plural = plural.replace(
            &format!(" target {singular} an opponent controlss"),
            &format!(" {plural_noun} target opponent controls"),
        );
        plural = plural.replace(
            &format!(" {singular} target opponent controlss"),
            &format!(" {plural_noun} target opponent controls"),
        );
        plural = plural.replace(
            &format!(" {singular} that player controlss"),
            &format!(" {plural_noun} that player controls"),
        );
        plural = plural.replace(
            &format!(" {singular} target player controlss"),
            &format!(" {plural_noun} target player controls"),
        );
    }
    plural = plural.replace(" that was dealt damage ", " that were dealt damage ");
    plural
}
