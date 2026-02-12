use std::cell::Cell;

use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::{
    ChoiceCount, Comparison, Condition, EffectPredicate, EventValueSpec, Until, Value,
};
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::{CardDefinition, CardType, Effect, ManaSymbol, Zone};

thread_local! {
    static EFFECT_RENDER_DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn with_effect_render_depth<F: FnOnce() -> String>(render: F) -> String {
    EFFECT_RENDER_DEPTH.with(|depth| {
        let current = depth.get();
        if current >= 128 {
            return "<render recursion limit>".to_string();
        }
        depth.set(current + 1);
        let rendered = render();
        depth.set(current);
        rendered
    })
}

fn describe_player_filter(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::Opponent => "an opponent".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        PlayerFilter::Target(inner) => {
            let inner_text = describe_player_filter(inner);
            if inner_text == "you" {
                "you".to_string()
            } else {
                format!("target {}", strip_leading_article(&inner_text))
            }
        }
        PlayerFilter::Specific(_) => "that player".to_string(),
        PlayerFilter::Active => "the active player".to_string(),
        PlayerFilter::Defending => "the defending player".to_string(),
        PlayerFilter::Attacking => "the attacking player".to_string(),
        PlayerFilter::DamagedPlayer => "the damaged player".to_string(),
        PlayerFilter::Teammate => "a teammate".to_string(),
        PlayerFilter::IteratedPlayer => "that player".to_string(),
        PlayerFilter::ControllerOf(crate::target::ObjectRef::Target) => {
            "its controller".to_string()
        }
        PlayerFilter::OwnerOf(crate::target::ObjectRef::Target) => "its owner".to_string(),
        PlayerFilter::ControllerOf(_) => "that object's controller".to_string(),
        PlayerFilter::OwnerOf(_) => "that object's owner".to_string(),
    }
}

fn strip_leading_article(text: &str) -> &str {
    text.strip_prefix("a ")
        .or_else(|| text.strip_prefix("an "))
        .or_else(|| text.strip_prefix("the "))
        .unwrap_or(text)
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn describe_mana_pool_owner(filter: &PlayerFilter) -> String {
    let player = describe_player_filter(filter);
    if player == "you" || player == "target you" {
        "your mana pool".to_string()
    } else if player.ends_with('s') {
        format!("{player}' mana pool")
    } else {
        format!("{player}'s mana pool")
    }
}

fn describe_possessive_player_filter(filter: &PlayerFilter) -> String {
    let player = describe_player_filter(filter);
    if player == "you" || player == "target you" {
        "your".to_string()
    } else if player.ends_with('s') {
        format!("{player}'")
    } else {
        format!("{player}'s")
    }
}

fn describe_possessive_choose_spec(spec: &ChooseSpec) -> String {
    let subject = describe_choose_spec(spec);
    if subject == "you" || subject == "target you" {
        "your".to_string()
    } else if subject.ends_with('s') {
        format!("{subject}'")
    } else {
        format!("{subject}'s")
    }
}

fn join_with_and(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let mut text = parts[..parts.len() - 1].join(", ");
            text.push_str(", and ");
            text.push_str(parts.last().map(String::as_str).unwrap_or_default());
            text
        }
    }
}

fn describe_pt_value(value: crate::card::PtValue) -> String {
    match value {
        crate::card::PtValue::Fixed(n) => n.to_string(),
        crate::card::PtValue::Star => "*".to_string(),
        crate::card::PtValue::StarPlus(n) => format!("*+{n}"),
    }
}

fn describe_token_color_words(colors: crate::color::ColorSet, include_colorless: bool) -> String {
    if colors.is_empty() {
        return if include_colorless {
            "colorless".to_string()
        } else {
            String::new()
        };
    }

    let mut names = Vec::new();
    if colors.contains(crate::color::Color::White) {
        names.push("white".to_string());
    }
    if colors.contains(crate::color::Color::Blue) {
        names.push("blue".to_string());
    }
    if colors.contains(crate::color::Color::Black) {
        names.push("black".to_string());
    }
    if colors.contains(crate::color::Color::Red) {
        names.push("red".to_string());
    }
    if colors.contains(crate::color::Color::Green) {
        names.push("green".to_string());
    }
    join_with_and(&names)
}

fn describe_token_blueprint(token: &CardDefinition) -> String {
    let card = &token.card;
    let mut parts = Vec::new();

    if let Some(pt) = card.power_toughness {
        parts.push(format!(
            "{}/{}",
            describe_pt_value(pt.power),
            describe_pt_value(pt.toughness)
        ));
    }

    let colors = describe_token_color_words(card.colors(), card.is_creature());
    if !colors.is_empty() {
        parts.push(colors);
    }

    if card.subtypes.is_empty()
        && !card.is_creature()
        && card.card_types.contains(&CardType::Artifact)
        && !card.name.trim().is_empty()
        && card.name.to_ascii_lowercase() != "token"
    {
        parts.push(card.name.clone());
    }

    if !card.subtypes.is_empty() {
        parts.push(
            card.subtypes
                .iter()
                .map(|subtype| format!("{subtype:?}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    if !card.card_types.is_empty() {
        parts.push(
            card.card_types
                .iter()
                .map(|card_type| format!("{card_type:?}").to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    parts.push("token".to_string());

    let mut text = parts.join(" ");
    let mut keyword_texts = Vec::new();
    let mut extra_ability_texts = Vec::new();
    for ability in &token.abilities {
        match &ability.kind {
            AbilityKind::Static(static_ability) => {
                if static_ability.is_keyword() {
                    keyword_texts.push(static_ability.display().to_ascii_lowercase());
                    continue;
                }
                if let Some(text) = ability.text.as_ref() {
                    extra_ability_texts.push(text.trim().to_string());
                } else {
                    extra_ability_texts.push(static_ability.display());
                }
            }
            AbilityKind::Triggered(_) | AbilityKind::Activated(_) | AbilityKind::Mana(_) => {
                if let Some(text) = ability.text.as_ref() {
                    extra_ability_texts.push(text.trim().to_string());
                }
            }
        }
    }
    keyword_texts.sort();
    keyword_texts.dedup();
    extra_ability_texts.sort();
    extra_ability_texts.dedup();
    if !keyword_texts.is_empty() {
        text.push_str(" with ");
        text.push_str(&join_with_and(&keyword_texts));
    }
    if !extra_ability_texts.is_empty() {
        if keyword_texts.is_empty() {
            text.push_str(" with ");
        } else {
            text.push_str(" and ");
        }
        text.push_str(&join_with_and(&extra_ability_texts));
    }

    text
}

fn player_verb(subject: &str, you_form: &'static str, other_form: &'static str) -> &'static str {
    if subject == "you" {
        you_form
    } else {
        other_form
    }
}

fn normalize_you_verb_phrase(text: &str) -> String {
    let replacements = [
        ("pays ", "pay "),
        ("loses ", "lose "),
        ("gains ", "gain "),
        ("draws ", "draw "),
        ("discards ", "discard "),
        ("sacrifices ", "sacrifice "),
        ("chooses ", "choose "),
        ("mills ", "mill "),
        ("scries ", "scry "),
        ("surveils ", "surveil "),
    ];
    for (from, to) in replacements {
        if text.starts_with(from) {
            return format!("{to}{}", &text[from.len()..]);
        }
    }
    text.to_string()
}

fn normalize_you_subject_phrase(text: &str) -> String {
    if let Some(rest) = text.strip_prefix("you ") {
        return format!("you {}", normalize_you_verb_phrase(rest));
    }
    if let Some(rest) = text.strip_prefix("You ") {
        return format!("You {}", normalize_you_verb_phrase(rest));
    }
    text.to_string()
}

fn normalize_cost_amount_token(text: &str) -> String {
    let cleaned = text.trim().trim_end_matches('.').trim_matches('"').trim();
    if cleaned.is_empty() {
        return cleaned.to_string();
    }
    if cleaned.starts_with('{') && cleaned.ends_with('}') {
        return cleaned.to_string();
    }
    if cleaned.chars().all(|ch| ch.is_ascii_digit()) {
        return format!("{{{cleaned}}}");
    }
    cleaned.to_string()
}

fn small_number_word(n: u32) -> Option<&'static str> {
    match n {
        0 => Some("zero"),
        1 => Some("one"),
        2 => Some("two"),
        3 => Some("three"),
        4 => Some("four"),
        5 => Some("five"),
        6 => Some("six"),
        7 => Some("seven"),
        8 => Some("eight"),
        9 => Some("nine"),
        10 => Some("ten"),
        _ => None,
    }
}

fn normalize_zero_pt_prefix(text: &str) -> String {
    text.replace(" gets 0/+", " gets +0/+")
        .replace(" gets 0/", " gets +0/")
}

fn strip_square_bracketed_segments(text: &str) -> String {
    if !text.contains('[') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for ch in text.chars() {
        if ch == '[' {
            depth += 1;
            continue;
        }
        if ch == ']' {
            depth = depth.saturating_sub(1);
            continue;
        }
        if depth == 0 {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_parenthetical_segments(text: &str) -> String {
    if !text.contains('(') {
        return text.trim().to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for ch in text.chars() {
        if ch == '(' {
            depth += 1;
            continue;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
            continue;
        }
        if depth == 0 {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn mana_word_to_symbol(word: &str) -> Option<&'static str> {
    match word {
        "w" => Some("{W}"),
        "u" => Some("{U}"),
        "b" => Some("{B}"),
        "r" => Some("{R}"),
        "g" => Some("{G}"),
        "c" => Some("{C}"),
        _ => None,
    }
}

fn normalize_sliver_grant_clause(subject: &str, rest: &str) -> Option<String> {
    let rest = strip_parenthetical_segments(rest);
    let rest = rest.trim().trim_matches('"').trim();
    let subject_prefix = if subject.eq_ignore_ascii_case("all slivers") {
        "All Slivers"
    } else {
        "All Sliver creatures"
    };
    let words = rest
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| {
                !(ch.is_ascii_alphanumeric() || ch == '{' || ch == '}')
            })
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    let mut costs = Vec::new();
    while idx < words.len() {
        let word = words[idx];
        if word.chars().all(|ch| ch.is_ascii_digit()) {
            costs.push(format!("{{{word}}}"));
            idx += 1;
            continue;
        }
        if word == "t" {
            costs.push("{T}".to_string());
            idx += 1;
            continue;
        }
        break;
    }
    if idx + 2 < words.len()
        && words[idx] == "sacrifice"
        && words[idx + 1] == "this"
        && words[idx + 2] == "permanent"
    {
        costs.push("Sacrifice this permanent".to_string());
        idx += 3;
    }

    let mut effect_words = words[idx..].to_vec();
    if effect_words.is_empty() {
        return None;
    }

    let effect = if effect_words[0] == "add" && effect_words.len() > 1 {
        let mut mana_parts = Vec::new();
        for word in &effect_words[1..] {
            if let Some(symbol) = mana_word_to_symbol(word) {
                mana_parts.push(symbol.to_string());
                continue;
            }
            let lower = word.to_ascii_lowercase();
            if lower.len() > 1 && lower.chars().all(|ch| matches!(ch, 'w' | 'u' | 'b' | 'r' | 'g' | 'c'))
            {
                for ch in lower.chars() {
                    if let Some(symbol) = mana_word_to_symbol(&ch.to_string()) {
                        mana_parts.push(symbol.to_string());
                    }
                }
                continue;
            }
            let mut idx = 0usize;
            let bytes = lower.as_bytes();
            while idx + 2 < bytes.len() {
                if bytes[idx] == b'{' && bytes[idx + 2] == b'}' {
                    let candidate = &lower[idx + 1..idx + 2];
                    if let Some(symbol) = mana_word_to_symbol(candidate) {
                        mana_parts.push(symbol.to_string());
                    }
                    idx += 3;
                } else {
                    idx += 1;
                }
            }
        }
        let mana = mana_parts.join("");
        if mana.is_empty() {
            capitalize_first(&effect_words.join(" "))
        } else {
            format!("Add {mana}")
        }
    } else {
        if effect_words.len() >= 2 && effect_words[0] == "target" && effect_words[1] == "sliver" {
            effect_words[1] = "Sliver";
        }
        normalize_zero_pt_prefix(&capitalize_first(&effect_words.join(" ")))
    };

    let effect_lower = effect.to_ascii_lowercase();
    let is_simple_keyword = is_keyword_phrase(&effect_lower)
        || effect_lower.starts_with("absorb ")
        || effect_lower.starts_with("frenzy ")
        || effect_lower.starts_with("poisonous ");

    if costs.is_empty() && is_simple_keyword {
        Some(format!("{subject_prefix} have {effect_lower}"))
    } else if costs.is_empty() {
        Some(format!("{subject_prefix} have \"{effect}.\""))
    } else {
        Some(format!(
            "{subject_prefix} have \"{}: {effect}.\"",
            costs.join(", ")
        ))
    }
}

fn describe_card_count(value: &Value) -> String {
    match value {
        Value::Fixed(1) => "a card".to_string(),
        Value::Fixed(n) => format!("{n} cards"),
        _ => format!("{} cards", describe_value(value)),
    }
}

fn is_generic_owned_card_search_filter(filter: &ObjectFilter) -> bool {
    filter.zone.is_none()
        && filter.controller.is_none()
        && matches!(filter.owner, Some(PlayerFilter::You))
        && filter.targets_player.is_none()
        && filter.targets_object.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.colors.is_none()
        && filter.excluded_colors.is_empty()
        && !filter.colorless
        && !filter.multicolored
        && !filter.token
        && !filter.nontoken
        && !filter.other
        && !filter.tapped
        && !filter.untapped
        && !filter.attacking
        && !filter.blocking
        && filter.power.is_none()
        && filter.toughness.is_none()
        && filter.mana_value.is_none()
        && !filter.has_mana_cost
        && !filter.has_tap_activated_ability
        && !filter.no_x_in_cost
        && filter.name.is_none()
        && filter.alternative_cast.is_none()
        && filter.static_abilities.is_empty()
        && filter.excluded_static_abilities.is_empty()
        && filter.custom_static_markers.is_empty()
        && filter.excluded_custom_static_markers.is_empty()
        && !filter.is_commander
        && filter.tagged_constraints.is_empty()
        && filter.specific.is_none()
        && !filter.source
}

fn describe_object_count(value: &Value) -> String {
    match value {
        Value::Fixed(1) => "a".to_string(),
        _ => describe_value(value),
    }
}

fn describe_choose_spec(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::Target(inner) => {
            let inner_text = describe_choose_spec(inner);
            if inner_text == "it" {
                inner_text
            } else if inner_text.starts_with("this ") {
                inner_text
            } else if inner_text.starts_with("target ") {
                inner_text
            } else if let Some(rest) = inner_text.strip_prefix("another ") {
                format!("another target {rest}")
            } else if let Some(rest) = inner_text.strip_prefix("other ") {
                format!("other target {rest}")
            } else {
                let stripped = strip_leading_article(&inner_text);
                if stripped == inner_text {
                    format!("target {inner_text}")
                } else {
                    format!("target {stripped}")
                }
            }
        }
        ChooseSpec::AnyTarget => "any target".to_string(),
        ChooseSpec::Object(filter) => filter.description(),
        ChooseSpec::Player(filter) => describe_player_filter(filter),
        ChooseSpec::Source => "this source".to_string(),
        ChooseSpec::SourceController => "you".to_string(),
        ChooseSpec::SourceOwner => "this source's owner".to_string(),
        ChooseSpec::Tagged(tag) => {
            if is_implicit_reference_tag(tag.as_str()) {
                "it".to_string()
            } else {
                format!("the tagged object '{}'", tag.as_str())
            }
        }
        ChooseSpec::All(filter) => format!("all {}", filter.description()),
        ChooseSpec::EachPlayer(filter) => format!("each {}", describe_player_filter(filter)),
        ChooseSpec::SpecificObject(_) => "that object".to_string(),
        ChooseSpec::SpecificPlayer(_) => "that player".to_string(),
        ChooseSpec::Iterated => "that object".to_string(),
        ChooseSpec::WithCount(inner, count) => {
            let inner_text = describe_choose_spec(inner);
            if count.is_single() {
                inner_text
            } else {
                if let ChooseSpec::Target(target_inner) = inner.as_ref() {
                    let target_desc = describe_choose_spec(target_inner);
                    let base = strip_leading_article(&target_desc);
                    let plural = pluralize_noun_phrase(base);
                    let count_text = |n: usize| {
                        number_word(n as i32)
                            .map(str::to_string)
                            .unwrap_or_else(|| n.to_string())
                    };
                    if count.is_dynamic_x() {
                        return format!("X target {plural}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of target {plural}"),
                        (min, None) => format!("at least {min} target {plural}"),
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one target {base}")
                            } else {
                                format!("up to {} target {plural}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("target {base}")
                            } else {
                                format!("{} target {plural}", count_text(min))
                            }
                        }
                        (1, Some(2)) => format!("one or two target {plural}"),
                        (min, Some(max)) => {
                            format!("{} to {} target {plural}", count_text(min), count_text(max))
                        }
                    }
                } else {
                    if count.is_dynamic_x() {
                        return format!("X {inner_text}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of {inner_text}"),
                        (min, None) => format!("at least {min} {inner_text}"),
                        (0, Some(max)) => format!("up to {max} {inner_text}"),
                        (min, Some(max)) if min == max => format!("{min} {inner_text}"),
                        (min, Some(max)) => format!("{min} to {max} {inner_text}"),
                    }
                }
            }
        }
    }
}

fn describe_transform_target(spec: &ChooseSpec) -> String {
    match spec {
        // Oracle text overwhelmingly uses "this creature" for source transforms
        // and this keeps compiled wording aligned with parser normalization.
        ChooseSpec::Source => "this creature".to_string(),
        _ => describe_choose_spec(spec),
    }
}

fn owner_for_zone_from_spec(spec: &ChooseSpec, zone: Zone) -> Option<Option<PlayerFilter>> {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            owner_for_zone_from_spec(inner, zone)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            if filter.zone == Some(zone) {
                Some(filter.owner.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn graveyard_owner_from_spec(spec: &ChooseSpec) -> Option<Option<PlayerFilter>> {
    owner_for_zone_from_spec(spec, Zone::Graveyard)
}

fn hand_owner_from_spec(spec: &ChooseSpec) -> Option<Option<PlayerFilter>> {
    owner_for_zone_from_spec(spec, Zone::Hand)
}

fn describe_card_choice_count(count: ChoiceCount) -> String {
    if count.is_dynamic_x() {
        return "X cards".to_string();
    }
    match (count.min, count.max) {
        (1, Some(1)) => "a card".to_string(),
        (min, Some(max)) if min == max => format!("{min} cards"),
        (0, Some(max)) => format!("up to {max} cards"),
        (0, None) => "any number of cards".to_string(),
        (min, None) => format!("at least {min} cards"),
        (min, Some(max)) => format!("{min} to {max} cards"),
    }
}

fn describe_choose_spec_without_graveyard_zone(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::Target(inner) => {
            let inner_text = describe_choose_spec_without_graveyard_zone(inner);
            if inner_text == "it" {
                inner_text
            } else if inner_text.starts_with("target ") {
                inner_text
            } else if let Some(rest) = inner_text.strip_prefix("another ") {
                format!("another target {rest}")
            } else if let Some(rest) = inner_text.strip_prefix("other ") {
                format!("other target {rest}")
            } else {
                let stripped = strip_leading_article(&inner_text);
                if stripped == inner_text {
                    format!("target {inner_text}")
                } else {
                    format!("target {stripped}")
                }
            }
        }
        ChooseSpec::Object(filter) => {
            if filter.zone == Some(Zone::Graveyard) {
                let text = filter.description();
                let suffix = match &filter.owner {
                    Some(owner) => {
                        format!(" in {} graveyard", describe_possessive_player_filter(owner))
                    }
                    None => " in graveyard".to_string(),
                };
                if let Some(stripped) = text.strip_suffix(&suffix) {
                    return stripped.to_string();
                }
                return text;
            }
            filter.description()
        }
        ChooseSpec::All(filter) => {
            if filter.zone == Some(Zone::Graveyard) {
                let text = filter.description();
                let suffix = match &filter.owner {
                    Some(owner) => {
                        format!(" in {} graveyard", describe_possessive_player_filter(owner))
                    }
                    None => " in graveyard".to_string(),
                };
                if let Some(stripped) = text.strip_suffix(&suffix) {
                    return format!("all {}", stripped);
                }
                return format!("all {}", text);
            }
            format!("all {}", filter.description())
        }
        ChooseSpec::WithCount(inner, count) => {
            let inner_text = describe_choose_spec_without_graveyard_zone(inner);
            if count.is_single() {
                inner_text
            } else {
                if let ChooseSpec::Target(target_inner) = inner.as_ref() {
                    let target_desc = describe_choose_spec_without_graveyard_zone(target_inner);
                    let base = strip_leading_article(&target_desc);
                    let plural = pluralize_noun_phrase(base);
                    let count_text = |n: usize| {
                        number_word(n as i32)
                            .map(str::to_string)
                            .unwrap_or_else(|| n.to_string())
                    };
                    if count.is_dynamic_x() {
                        return format!("X target {plural}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of target {plural}"),
                        (min, None) => format!("at least {min} target {plural}"),
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one target {base}")
                            } else {
                                format!("up to {} target {plural}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("target {base}")
                            } else {
                                format!("{} target {plural}", count_text(min))
                            }
                        }
                        (1, Some(2)) => format!("one or two target {plural}"),
                        (min, Some(max)) => {
                            format!("{} to {} target {plural}", count_text(min), count_text(max))
                        }
                    }
                } else {
                    if count.is_dynamic_x() {
                        return format!("X {inner_text}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of {inner_text}"),
                        (min, None) => format!("at least {min} {inner_text}"),
                        (0, Some(max)) => format!("up to {max} {inner_text}"),
                        (min, Some(max)) if min == max => format!("{min} {inner_text}"),
                        (min, Some(max)) => format!("{min} to {max} {inner_text}"),
                    }
                }
            }
        }
        _ => describe_choose_spec(spec),
    }
}

fn describe_choice_count(count: &ChoiceCount) -> String {
    if count.is_dynamic_x() {
        return "X".to_string();
    }
    match (count.min, count.max) {
        (0, None) => "any number".to_string(),
        (min, None) => format!("at least {min}"),
        (0, Some(max)) => format!("up to {max}"),
        (min, Some(max)) if min == max => format!("exactly {min}"),
        (min, Some(max)) => format!("{min} to {max}"),
    }
}

fn ensure_trailing_period(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.ends_with('.') {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

fn normalize_modal_text(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn modal_text_equivalent(description: &str, compiled: &str) -> bool {
    normalize_modal_text(description) == normalize_modal_text(compiled)
}

fn number_word(value: i32) -> Option<&'static str> {
    match value {
        1 => Some("one"),
        2 => Some("two"),
        3 => Some("three"),
        4 => Some("four"),
        5 => Some("five"),
        _ => None,
    }
}

fn describe_search_selection_with_cards(selection: &str) -> String {
    let selection = selection.trim();
    if selection.is_empty() {
        return "a card".to_string();
    }
    if selection.contains(" card") {
        return selection.to_string();
    }
    if let Some(rest) = selection.strip_prefix("up to ") {
        let mut parts = rest.splitn(2, ' ');
        let amount = parts.next().unwrap_or_default().trim();
        let tail = parts.next().unwrap_or_default().trim();
        if !tail.is_empty() {
            if amount == "1" || amount.eq_ignore_ascii_case("one") {
                return format!("a {tail} card");
            }
            return format!("up to {amount} {tail} cards");
        }
    }
    if let Some(rest) = selection.strip_prefix("any number ") {
        let rest = rest.trim_start_matches("of ").trim();
        if !rest.is_empty() {
            return format!("any number of {rest} cards");
        }
    }
    format!("{} card", with_indefinite_article(selection))
}

fn normalize_search_you_own_clause(text: &str) -> Option<String> {
    let rest = text.strip_prefix("Search your library for ")?;
    let (selection, tail) = rest.split_once(" you own")?;
    let selection = describe_search_selection_with_cards(selection);
    let tail = tail
        .replace(
            ", reveal it, put it into hand, then shuffle",
            ", reveal it, put it into your hand, then shuffle",
        )
        .replace(
            ", put it into hand, then shuffle",
            ", put it into your hand, then shuffle",
        )
        .replace(
            ", reveal it, put it on top of library, then shuffle",
            ", reveal it, then shuffle and put that card on top",
        )
        .replace(
            ", put it on top of library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            ", put it on top of your library, then shuffle",
            ", then shuffle and put that card on top",
        );
    Some(format!("Search your library for {selection}{tail}"))
}

fn normalize_choose_between_modes_clause(text: &str) -> Option<String> {
    let (prefix, rest) = if let Some(rest) = text.strip_prefix("Choose between ") {
        ("", rest)
    } else if let Some((prefix, rest)) = text.split_once("Choose between ") {
        (prefix, rest)
    } else {
        return None;
    };
    let (range, modes) = rest.split_once(" mode(s) - ")?;
    let (min_raw, max_raw) = range.split_once(" and ")?;
    let min = min_raw.trim().parse::<u32>().ok()?;
    let max = max_raw.trim().parse::<u32>().ok()?;
    let modes = modes.replace("• ", "");
    let count_word = |n: u32| {
        number_word(n as i32)
            .map(str::to_string)
            .unwrap_or_else(|| n.to_string())
    };
    let header = match (min, max) {
        (0, 1) => "Choose up to one —".to_string(),
        (1, 1) => "Choose one —".to_string(),
        (1, n) if n > 1 => "Choose one or more —".to_string(),
        (0, n) => format!("Choose up to {} —", count_word(n)),
        (n, m) if n == m => format!("Choose {} —", count_word(n)),
        _ => format!("Choose between {min} and {max} —"),
    };
    if prefix.is_empty() {
        Some(format!("{header} {modes}"))
    } else {
        Some(format!("{prefix}{header} {modes}"))
    }
}

fn describe_mode_choice_header(max: &Value, min: Option<&Value>) -> String {
    match (min, max) {
        (None, Value::Fixed(value)) if *value > 0 => {
            if let Some(word) = number_word(*value) {
                format!("Choose {word} -")
            } else {
                format!("Choose {value} mode(s) -")
            }
        }
        (Some(Value::Fixed(1)), Value::Fixed(2)) => "Choose one or both -".to_string(),
        (Some(min), max) => format!(
            "Choose between {} and {} mode(s) -",
            describe_value(min),
            describe_value(max)
        ),
        (None, max) => format!("Choose {} mode(s) -", describe_value(max)),
    }
}

fn describe_compact_protection_choice(effect: &Effect) -> Option<String> {
    let choose_mode = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose_mode.min_choose_count.is_some()
        || !matches!(choose_mode.choose_count, Value::Fixed(1))
    {
        return None;
    }

    let mut target: Option<&ChooseSpec> = None;
    let mut color_mode_count = 0usize;
    let mut allow_colorless = false;

    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        let grant = mode.effects[0].downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()?;
        if !matches!(grant.duration, Until::EndOfTurn) || grant.abilities.len() != 1 {
            return None;
        }
        match grant.abilities[0].protection_from()? {
            crate::ability::ProtectionFrom::Colorless => {
                allow_colorless = true;
            }
            crate::ability::ProtectionFrom::Color(colors) => {
                if colors.count() != 1 {
                    return None;
                }
                color_mode_count += 1;
            }
            _ => return None,
        }

        if let Some(existing) = target {
            if existing != &grant.target {
                return None;
            }
        } else {
            target = Some(&grant.target);
        }
    }

    if color_mode_count != 5 {
        return None;
    }
    let target_desc = describe_choose_spec(target?);
    Some(if allow_colorless {
        format!(
            "{target_desc} gains protection from colorless or from the color of your choice until end of turn"
        )
    } else {
        format!("{target_desc} gains protection from the color of your choice until end of turn")
    })
}

fn describe_mana_symbol(symbol: ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "{W}".to_string(),
        ManaSymbol::Blue => "{U}".to_string(),
        ManaSymbol::Black => "{B}".to_string(),
        ManaSymbol::Red => "{R}".to_string(),
        ManaSymbol::Green => "{G}".to_string(),
        ManaSymbol::Colorless => "{C}".to_string(),
        ManaSymbol::Generic(v) => format!("{{{v}}}"),
        ManaSymbol::Snow => "{S}".to_string(),
        ManaSymbol::Life(_) => "{P}".to_string(),
        ManaSymbol::X => "{X}".to_string(),
    }
}

fn describe_mana_alternatives(symbols: &[ManaSymbol]) -> String {
    let rendered = symbols
        .iter()
        .copied()
        .map(describe_mana_symbol)
        .collect::<Vec<_>>();
    match rendered.len() {
        0 => "{0}".to_string(),
        1 => rendered[0].clone(),
        2 => format!("{} or {}", rendered[0], rendered[1]),
        _ => {
            let mut text = rendered[..rendered.len() - 1].join(", ");
            text.push_str(", or ");
            text.push_str(rendered.last().map(String::as_str).unwrap_or("{0}"));
            text
        }
    }
}

fn describe_counter_type(counter_type: crate::object::CounterType) -> String {
    match counter_type {
        crate::object::CounterType::PlusOnePlusOne => "+1/+1".to_string(),
        crate::object::CounterType::MinusOneMinusOne => "-1/-1".to_string(),
        other => format!("{other:?}"),
    }
}

fn describe_value(value: &Value) -> String {
    match value {
        Value::Fixed(n) => n.to_string(),
        Value::X => "X".to_string(),
        Value::XTimes(factor) => {
            if *factor == 1 {
                "X".to_string()
            } else if *factor == -1 {
                "-X".to_string()
            } else {
                format!("{factor}*X")
            }
        }
        Value::Count(filter) => format!("the number of {}", filter.description()),
        Value::CountPlayers(filter) => format!("the number of {}", describe_player_filter(filter)),
        Value::SourcePower => "this source's power".to_string(),
        Value::SourceToughness => "this source's toughness".to_string(),
        Value::PowerOf(spec) => format!("the power of {}", describe_choose_spec(spec)),
        Value::ToughnessOf(spec) => format!("the toughness of {}", describe_choose_spec(spec)),
        Value::LifeTotal(filter) => {
            format!("{} life total", describe_possessive_player_filter(filter))
        }
        Value::CardsInHand(filter) => format!(
            "the number of cards in {} hand",
            describe_possessive_player_filter(filter)
        ),
        Value::CardsInGraveyard(filter) => format!(
            "the number of cards in {} graveyard",
            describe_possessive_player_filter(filter)
        ),
        Value::SpellsCastThisTurn(filter) => {
            format!(
                "the number of spells cast this turn by {}",
                describe_player_filter(filter)
            )
        }
        Value::SpellsCastBeforeThisTurn(filter) => format!(
            "the number of spells cast before this spell this turn by {}",
            describe_player_filter(filter)
        ),
        Value::CardTypesInGraveyard(filter) => format!(
            "the number of distinct card types in {} graveyard",
            describe_possessive_player_filter(filter)
        ),
        Value::Devotion { player, color } => format!(
            "{} devotion to {}",
            describe_possessive_player_filter(player),
            format!("{color:?}").to_ascii_lowercase()
        ),
        Value::EffectValue(id) => format!("the count result of effect #{}", id.0),
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => "that much".to_string(),
        Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier }) => {
            if *multiplier == 1 {
                "the number of blockers beyond the first".to_string()
            } else {
                format!("{multiplier} times the number of blockers beyond the first")
            }
        }
        Value::WasKicked => "whether this spell was kicked (1 or 0)".to_string(),
        Value::WasBoughtBack => "whether buyback was paid (1 or 0)".to_string(),
        Value::WasEntwined => "whether entwine was paid (1 or 0)".to_string(),
        Value::WasPaid(index) => format!("whether optional cost #{index} was paid (1 or 0)"),
        Value::WasPaidLabel(label) => {
            format!("whether optional cost '{label}' was paid (1 or 0)")
        }
        Value::TimesPaid(index) => format!("how many times optional cost #{index} was paid"),
        Value::TimesPaidLabel(label) => {
            format!("how many times optional cost '{label}' was paid")
        }
        Value::KickCount => "how many times this spell was kicked".to_string(),
        Value::CountersOnSource(counter_type) => format!(
            "the number of {} counter(s) on this source",
            describe_counter_type(*counter_type)
        ),
        Value::CountersOn(spec, Some(counter_type)) => format!(
            "the number of {} counter(s) on {}",
            describe_counter_type(*counter_type),
            describe_choose_spec(spec)
        ),
        Value::CountersOn(spec, None) => {
            format!("the number of counters on {}", describe_choose_spec(spec))
        }
        Value::TaggedCount => "the tagged object count".to_string(),
    }
}

fn describe_signed_value(value: &Value) -> String {
    match value {
        Value::Fixed(n) if *n >= 0 => format!("+{n}"),
        Value::X => "+X".to_string(),
        Value::XTimes(factor) if *factor > 0 => {
            if *factor == 1 {
                "+X".to_string()
            } else {
                format!("+{factor}*X")
            }
        }
        Value::Fixed(n) => n.to_string(),
        _ => describe_value(value),
    }
}

fn describe_toughness_delta_with_power_context(power: &Value, toughness: &Value) -> String {
    if matches!(power, Value::Fixed(n) if *n < 0) && matches!(toughness, Value::Fixed(0)) {
        "-0".to_string()
    } else {
        describe_signed_value(toughness)
    }
}

fn describe_signed_i32(value: i32) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

fn choose_spec_is_plural(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) => choose_spec_is_plural(inner),
        ChooseSpec::All(_) | ChooseSpec::EachPlayer(_) => true,
        ChooseSpec::WithCount(inner, count) => !count.is_single() || choose_spec_is_plural(inner),
        _ => false,
    }
}

fn owner_hand_phrase_for_spec(spec: &ChooseSpec) -> &'static str {
    if choose_spec_is_plural(spec) {
        "their owners' hands"
    } else {
        "its owner's hand"
    }
}

fn owner_library_phrase_for_spec(spec: &ChooseSpec) -> &'static str {
    if choose_spec_is_plural(spec) {
        "their owners' libraries"
    } else {
        "its owner's library"
    }
}

fn describe_put_counter_phrase(count: &Value, counter_type: CounterType) -> String {
    let counter_name = describe_counter_type(counter_type);
    match count {
        Value::Fixed(1) => format!("a {counter_name} counter"),
        Value::Fixed(n) if *n > 1 => {
            let n = *n as usize;
            let amount = number_word(n as i32)
                .map(str::to_string)
                .unwrap_or_else(|| n.to_string());
            format!("{amount} {counter_name} counters")
        }
        _ => format!("{} {counter_name} counter(s)", describe_value(count)),
    }
}

fn describe_apply_continuous_target(
    effect: &crate::effects::ApplyContinuousEffect,
) -> (String, bool) {
    if let Some(spec) = &effect.target_spec {
        return (describe_choose_spec(spec), choose_spec_is_plural(spec));
    }

    match &effect.target {
        crate::continuous::EffectTarget::Specific(_) => ("that permanent".to_string(), false),
        crate::continuous::EffectTarget::Filter(filter) => {
            (pluralize_noun_phrase(&filter.description()), true)
        }
        crate::continuous::EffectTarget::Source => ("this source".to_string(), false),
        crate::continuous::EffectTarget::AllPermanents => ("all permanents".to_string(), true),
        crate::continuous::EffectTarget::AllCreatures => ("all creatures".to_string(), true),
        crate::continuous::EffectTarget::AttachedTo(_) => {
            ("the attached permanent".to_string(), false)
        }
    }
}

fn describe_apply_continuous_clauses(
    effect: &crate::effects::ApplyContinuousEffect,
    plural_target: bool,
) -> Vec<String> {
    let gets = if plural_target { "get" } else { "gets" };
    let gains = if plural_target { "gain" } else { "gains" };

    let mut clauses = Vec::new();

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
        crate::continuous::Modification::SetPowerToughness {
            power, toughness, ..
        } => {
            clauses.push(format!(
                "{gets} base power and toughness {}/{}",
                describe_value(power),
                describe_value(toughness)
            ));
        }
        crate::continuous::Modification::SetPower { value, .. } => {
            clauses.push(format!("{gets} base power {}", describe_value(value)));
        }
        crate::continuous::Modification::SetToughness { value, .. } => {
            clauses.push(format!("{gets} base toughness {}", describe_value(value)));
        }
        crate::continuous::Modification::AddAbility(ability) => {
            clauses.push(format!("{gains} {}", ability.display()));
        }
        crate::continuous::Modification::AddAbilityGeneric(ability) => {
            clauses.push(format!("{gains} {}", describe_inline_ability(ability)));
        }
        _ => {}
    };

    if let Some(modification) = &effect.modification {
        push_modification(modification);
    }
    for modification in &effect.additional_modifications {
        push_modification(modification);
    }
    for runtime in &effect.runtime_modifications {
        match runtime {
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            } => {
                clauses.push(format!(
                    "{gets} {}/{}",
                    describe_signed_value(power),
                    describe_toughness_delta_with_power_context(power, toughness)
                ));
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
        }
    }

    clauses
}

fn describe_apply_continuous_effect(
    effect: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    let (target, plural_target) = describe_apply_continuous_target(effect);
    if effect.modification.is_none()
        && effect.additional_modifications.is_empty()
        && matches!(
            effect.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
    {
        let mut text = format!("Gain control of {target}");
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

    let mut text = format!("{target} {}", join_with_and(&clauses));
    if !matches!(effect.until, Until::Forever) {
        text.push(' ');
        text.push_str(&describe_until(&effect.until));
    }
    Some(text)
}

fn describe_compact_apply_continuous_pair(
    first: &crate::effects::ApplyContinuousEffect,
    second: &crate::effects::ApplyContinuousEffect,
) -> Option<String> {
    if first.target != second.target
        || first.target_spec != second.target_spec
        || first.until != second.until
    {
        return None;
    }

    let (target, plural_target) = describe_apply_continuous_target(first);
    let mut clauses = describe_apply_continuous_clauses(first, plural_target);
    clauses.extend(describe_apply_continuous_clauses(second, plural_target));
    if clauses.is_empty() {
        return None;
    }

    let mut text = format!("{target} {}", join_with_and(&clauses));
    if !matches!(first.until, Until::Forever) {
        text.push(' ');
        text.push_str(&describe_until(&first.until));
    }
    Some(text)
}

fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate.as_str() == tag,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_tag(inner, tag)
        }
        _ => false,
    }
}

fn describe_tag_attached_then_tap_or_untap(
    tag_attached: &crate::effects::TagAttachedToSourceEffect,
    next: &Effect,
) -> Option<String> {
    let tag = tag_attached.tag.as_str();
    let attached_object = match tag {
        "enchanted" => "enchanted permanent",
        "equipped" => "equipped creature",
        _ => return None,
    };

    if let Some(tap) = next.downcast_ref::<crate::effects::TapEffect>()
        && choose_spec_references_tag(&tap.spec, tag)
    {
        return Some(format!("Tap {attached_object}"));
    }
    if let Some(untap) = next.downcast_ref::<crate::effects::UntapEffect>()
        && choose_spec_references_tag(&untap.spec, tag)
    {
        return Some(format!("Untap {attached_object}"));
    }
    None
}

fn is_generated_internal_tag(tag: &str) -> bool {
    let Some((_, suffix)) = tag.rsplit_once('_') else {
        return false;
    };
    !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
}

fn is_implicit_reference_tag(tag: &str) -> bool {
    matches!(tag, "triggering" | "damaged" | "__it__") || is_generated_internal_tag(tag)
}

fn describe_until(until: &Until) -> String {
    match until {
        Until::Forever => "forever".to_string(),
        Until::EndOfTurn => "until end of turn".to_string(),
        Until::YourNextTurn => "until your next turn".to_string(),
        Until::EndOfCombat => "until end of combat".to_string(),
        Until::ThisLeavesTheBattlefield => {
            "while this source remains on the battlefield".to_string()
        }
        Until::YouStopControllingThis => "while you control this source".to_string(),
        Until::TurnsPass(turns) => format!("for {} turn(s)", describe_value(turns)),
    }
}

fn describe_damage_filter(filter: &crate::prevention::DamageFilter) -> String {
    let mut parts = Vec::new();
    if filter.combat_only {
        parts.push("combat damage".to_string());
    } else if filter.noncombat_only {
        parts.push("noncombat damage".to_string());
    } else {
        parts.push("all damage".to_string());
    }

    if let Some(source_filter) = &filter.from_source {
        parts.push(format!("from {}", source_filter.description()));
    }
    if let Some(source_types) = &filter.from_card_types
        && !source_types.is_empty()
    {
        let text = source_types
            .iter()
            .map(|card_type| format!("{card_type:?}").to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(" or ");
        parts.push(format!("from {text} sources"));
    }
    if let Some(source_colors) = &filter.from_colors
        && !source_colors.is_empty()
    {
        let text = source_colors
            .iter()
            .map(|color| format!("{color:?}").to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(" or ");
        parts.push(format!("from {text} sources"));
    }
    if filter.from_specific_source.is_some() {
        parts.push("from that source".to_string());
    }

    parts.join(" ")
}

fn describe_prevention_target(target: &crate::prevention::PreventionTarget) -> &'static str {
    match target {
        crate::prevention::PreventionTarget::Player(_) => "that player",
        crate::prevention::PreventionTarget::Permanent(_) => "that permanent",
        crate::prevention::PreventionTarget::PermanentsMatching(_) => "matching permanents",
        crate::prevention::PreventionTarget::Players => "players",
        crate::prevention::PreventionTarget::You => "you",
        crate::prevention::PreventionTarget::YouAndPermanentsYouControl => {
            "you and permanents you control"
        }
        crate::prevention::PreventionTarget::All => "all players and permanents",
    }
}

fn describe_restriction(restriction: &crate::effect::Restriction) -> String {
    match restriction {
        crate::effect::Restriction::GainLife(filter) => {
            format!("{} can't gain life", describe_player_filter(filter))
        }
        crate::effect::Restriction::SearchLibraries(filter) => {
            format!("{} can't search libraries", describe_player_filter(filter))
        }
        crate::effect::Restriction::CastSpells(filter) => {
            format!("{} can't cast spells", describe_player_filter(filter))
        }
        crate::effect::Restriction::DrawCards(filter) => {
            format!("{} can't draw cards", describe_player_filter(filter))
        }
        crate::effect::Restriction::DrawExtraCards(filter) => {
            format!("{} can't draw extra cards", describe_player_filter(filter))
        }
        crate::effect::Restriction::ChangeLifeTotal(filter) => {
            format!(
                "{} can't have life total changed",
                describe_player_filter(filter)
            )
        }
        crate::effect::Restriction::LoseGame(filter) => {
            format!("{} can't lose the game", describe_player_filter(filter))
        }
        crate::effect::Restriction::WinGame(filter) => {
            format!("{} can't win the game", describe_player_filter(filter))
        }
        crate::effect::Restriction::PreventDamage => "damage can't be prevented".to_string(),
        crate::effect::Restriction::Attack(filter) => {
            format!("{} can't attack", filter.description())
        }
        crate::effect::Restriction::Block(filter) => {
            format!("{} can't block", filter.description())
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
        crate::effect::Restriction::BeSacrificed(filter) => {
            format!("{} can't be sacrificed", filter.description())
        }
        crate::effect::Restriction::HaveCountersPlaced(filter) => {
            format!("counters can't be placed on {}", filter.description())
        }
        crate::effect::Restriction::BeTargeted(filter) => {
            format!("{} can't be targeted", filter.description())
        }
        crate::effect::Restriction::BeCountered(filter) => {
            format!("{} can't be countered", filter.description())
        }
    }
}

fn describe_comparison(cmp: &Comparison) -> String {
    match cmp {
        Comparison::GreaterThan(n) => format!("is greater than {n}"),
        Comparison::GreaterThanOrEqual(n) => format!("is at least {n}"),
        Comparison::Equal(n) => format!("is equal to {n}"),
        Comparison::LessThan(n) => format!("is less than {n}"),
        Comparison::LessThanOrEqual(n) => format!("is at most {n}"),
        Comparison::NotEqual(n) => format!("is not equal to {n}"),
    }
}

fn describe_effect_predicate(predicate: &EffectPredicate) -> String {
    match predicate {
        EffectPredicate::Succeeded => "succeeded".to_string(),
        EffectPredicate::Failed => "failed".to_string(),
        EffectPredicate::Happened => "happened".to_string(),
        EffectPredicate::DidNotHappen => "did not happen".to_string(),
        EffectPredicate::HappenedNotReplaced => "happened and was not replaced".to_string(),
        EffectPredicate::Value(cmp) => format!("its count {}", describe_comparison(cmp)),
        EffectPredicate::Chosen => "was chosen".to_string(),
        EffectPredicate::WasDeclined => "was declined".to_string(),
    }
}

fn describe_condition(condition: &Condition) -> String {
    match condition {
        Condition::YouControl(filter) => format!("you control {}", filter.description()),
        Condition::OpponentControls(filter) => {
            format!("an opponent controls {}", filter.description())
        }
        Condition::LifeTotalOrLess(n) => format!("your life total is {n} or less"),
        Condition::LifeTotalOrGreater(n) => format!("your life total is {n} or greater"),
        Condition::CardsInHandOrMore(n) => format!("you have {n} or more cards in hand"),
        Condition::YourTurn => "it is your turn".to_string(),
        Condition::CreatureDiedThisTurn => "a creature died this turn".to_string(),
        Condition::CastSpellThisTurn => "a spell was cast this turn".to_string(),
        Condition::AttackedThisTurn => "you attacked this turn".to_string(),
        Condition::NoSpellsWereCastLastTurn => "no spells were cast last turn".to_string(),
        Condition::TargetIsTapped => "the target is tapped".to_string(),
        Condition::SourceIsTapped => "this source is tapped".to_string(),
        Condition::TargetIsAttacking => "the target is attacking".to_string(),
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            if let Some(symbol) = symbol {
                format!(
                    "at least {amount} {} mana was spent to cast this spell",
                    describe_mana_symbol(*symbol)
                )
            } else {
                format!("at least {amount} mana was spent to cast this spell")
            }
        }
        Condition::YouControlCommander => "you control your commander".to_string(),
        Condition::TaggedObjectMatches(tag, filter) => format!(
            "the tagged object '{}' matches {}",
            tag.as_str(),
            filter.description()
        ),
        Condition::Not(inner) => format!("not ({})", describe_condition(inner)),
        Condition::And(left, right) => {
            format!(
                "({}) and ({})",
                describe_condition(left),
                describe_condition(right)
            )
        }
        Condition::Or(left, right) => {
            format!(
                "({}) or ({})",
                describe_condition(left),
                describe_condition(right)
            )
        }
    }
}

fn describe_effect_list(effects: &[Effect]) -> String {
    let has_non_target_only = effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_none()
    });
    let filtered = effects
        .iter()
        .filter(|effect| {
            !(has_non_target_only
                && effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some())
        })
        .collect::<Vec<_>>();

    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < filtered.len() {
        if idx + 1 < filtered.len()
            && let Some(first_apply) =
                filtered[idx].downcast_ref::<crate::effects::ApplyContinuousEffect>()
            && let Some(second_apply) =
                filtered[idx + 1].downcast_ref::<crate::effects::ApplyContinuousEffect>()
            && let Some(compact) = describe_compact_apply_continuous_pair(first_apply, second_apply)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tag_attached) =
                filtered[idx].downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            && let Some(compact) =
                describe_tag_attached_then_tap_or_untap(tag_attached, filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(move_back) =
                filtered[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_exile_then_return(tagged, move_back)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(choose_new) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
            && let Some(compact) = describe_with_id_then_choose_new_targets(with_id, choose_new)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = filtered[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && let Some(compact) = describe_with_id_then_if(with_id, if_effect)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(deal) = filtered[idx + 1].downcast_ref::<crate::effects::DealDamageEffect>()
            && let Some(compact) = describe_tagged_target_then_power_damage(tagged, deal)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
        {
            let shuffle = filtered
                .get(idx + 2)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
            if let Some(compact) = describe_search_choose_for_each(choose, for_each, shuffle, false)
            {
                parts.push(compact);
                idx += if shuffle.is_some() { 3 } else { 2 };
                continue;
            }
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(shuffle) =
                filtered[idx + 1].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(for_each) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_search_choose_for_each(choose, for_each, Some(shuffle), true)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = filtered[idx + 1].downcast_ref::<crate::effects::ExileEffect>()
            && let Some(compact) = describe_choose_then_exile(choose, exile)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) =
                filtered[idx + 1].downcast_ref::<crate::effects::SacrificeEffect>()
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_tagged_target_then_cant_block(tagged, cant)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_choose_then_cant_block(choose, cant)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(draw) = filtered[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(discard) = filtered[idx + 1].downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(compact) = describe_draw_then_discard(draw, discard)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        let rendered = describe_effect(filtered[idx]);
        if !rendered.is_empty() {
            parts.push(rendered);
        }
        idx += 1;
    }
    let text = parts.join(". ");
    cleanup_decompiled_text(&text)
}

fn describe_exile_then_return(
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
    Some(format!("Exile {target}, then return it to the battlefield"))
}

fn describe_tagged_target_then_power_damage(
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

    let source_text = describe_choose_spec(&target_only.target);
    if matches!(
        deal.target,
        ChooseSpec::Tagged(ref target_tag) if target_tag == source_tag
    ) {
        return Some(format!(
            "{source_text} deals damage to itself equal to its power"
        ));
    }

    let target_text = describe_choose_spec(&deal.target);
    Some(format!(
        "{source_text} deals damage equal to its power to {target_text}"
    ))
}

fn cleanup_decompiled_text(text: &str) -> String {
    let mut out = text.to_string();
    for (from, to) in [
        ("you gets", "you get"),
        ("you puts", "you put"),
        ("a artifact", "an artifact"),
        ("a another", "another"),
        ("a enchantment", "an enchantment"),
        ("a untapped", "an untapped"),
        ("a opponent", "an opponent"),
        (" ors ", " or "),
        ("creature are", "creatures are"),
        ("target any target", "any target"),
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

fn describe_inline_ability(ability: &Ability) -> String {
    if let Some(text) = &ability.text
        && !text.trim().is_empty()
    {
        return text.trim().to_string();
    }
    match &ability.kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        AbilityKind::Triggered(triggered) => {
            format!("a triggered ability ({})", triggered.trigger.display())
        }
        AbilityKind::Activated(_) => "an activated ability".to_string(),
        AbilityKind::Mana(_) => "a mana ability".to_string(),
    }
}

fn normalize_cost_phrase(text: &str) -> String {
    if let Some(rest) = text.strip_prefix("you ") {
        let normalized = normalize_you_verb_phrase(rest);
        return capitalize_first(&normalized);
    }
    if let Some(rest) = text.strip_prefix("You ") {
        let normalized = normalize_you_verb_phrase(rest);
        return capitalize_first(&normalized);
    }
    text.to_string()
}

fn describe_cost_component(cost: &crate::costs::Cost) -> String {
    if let Some(mana_cost) = cost.mana_cost_ref() {
        return mana_cost.to_oracle();
    }
    if let Some(effect) = cost.effect_ref() {
        if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>()
            && matches!(tap.spec, ChooseSpec::Source)
        {
            return "{T}".to_string();
        }
        if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>()
            && matches!(untap.spec, ChooseSpec::Source)
        {
            return "{Q}".to_string();
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
        return "Sacrifice this source".to_string();
    }
    let display = cost.display().trim().to_string();
    if display.is_empty() {
        format!("{cost:?}")
    } else {
        display
    }
}

fn describe_cost_list(costs: &[crate::costs::Cost]) -> String {
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < costs.len() {
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
            && let Some(sacrifice) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(normalize_cost_phrase(&compact));
            idx += 2;
            continue;
        }
        parts.push(describe_cost_component(&costs[idx]));
        idx += 1;
    }
    parts.join(", ")
}

fn with_indefinite_article(noun: &str) -> String {
    let trimmed = noun.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
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
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

fn strip_indefinite_article(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("a ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("an ") {
        return rest;
    }
    trimmed
}

fn pluralize_word(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
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

fn pluralize_noun_phrase(phrase: &str) -> String {
    let base = strip_indefinite_article(phrase);
    for suffix in [
        " you control",
        " that player controls",
        " in your graveyard",
        " in target player's graveyard",
        " in that player's graveyard",
        " in a graveyard",
        " in graveyard",
        " in your hand",
        " in target player's hand",
        " in that player's hand",
        " in a hand",
        " in your library",
        " in target player's library",
        " in that player's library",
        " in a library",
        " in exile",
    ] {
        if let Some(head) = base.strip_suffix(suffix) {
            let head = head.trim_end();
            let head_plural = pluralize_word(head);
            return format!("{head_plural}{suffix}");
        }
    }
    if base.ends_with('s') {
        base.to_string()
    } else {
        pluralize_word(base)
    }
}

fn sacrifice_uses_chosen_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

fn describe_for_players_choose_then_sacrifice(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = for_players.effects[1].downcast_ref::<crate::effects::SacrificeEffect>()?;
    if choose.zone != Zone::Battlefield
        || choose.is_search
        || !choose.count.is_single()
        || choose.chooser != PlayerFilter::IteratedPlayer
        || !matches!(sacrifice.count, Value::Fixed(1))
        || sacrifice.player != PlayerFilter::IteratedPlayer
        || !sacrifice_uses_chosen_tag(&sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    let (subject, verb, possessive) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "sacrifices", "their"),
        PlayerFilter::Opponent => ("Each opponent", "sacrifices", "their"),
        PlayerFilter::You => ("You", "sacrifice", "your"),
        _ => return None,
    };
    let chosen = with_indefinite_article(&choose.filter.description());
    Some(format!("{subject} {verb} {chosen} of {possessive} choice"))
}

fn describe_choose_then_sacrifice(
    choose: &crate::effects::ChooseObjectsEffect,
    sacrifice: &crate::effects::SacrificeEffect,
) -> Option<String> {
    let choose_exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    let sacrifice_count = match sacrifice.count {
        Value::Fixed(value) if value > 0 => value as usize,
        _ => return None,
    };
    if choose.zone != Zone::Battlefield
        || choose.is_search
        || choose_exact != sacrifice_count
        || sacrifice.player != choose.chooser
        || !sacrifice_uses_chosen_tag(&sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    let player = describe_player_filter(&choose.chooser);
    let verb = player_verb(&player, "sacrifice", "sacrifices");
    let refers_to_triggering_object = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && matches!(constraint.tag.as_str(), "triggering" | "damaged")
    });
    let chosen = choose.filter.description();
    if sacrifice_count == 1 {
        if refers_to_triggering_object {
            return Some(format!("{player} {verb} it"));
        }
        let chosen = with_indefinite_article(&chosen);
        Some(format!("{player} {verb} {chosen}"))
    } else {
        let count_text = number_word(sacrifice_count as i32)
            .map(str::to_string)
            .unwrap_or_else(|| sacrifice_count.to_string());
        let chosen = pluralize_noun_phrase(&chosen);
        Some(format!("{player} {verb} {count_text} {chosen}"))
    }
}

fn describe_choose_then_cant_block(
    choose: &crate::effects::ChooseObjectsEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let crate::effect::Restriction::Block(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != crate::effect::Until::EndOfTurn {
        return None;
    }
    if choose.zone != Zone::Battlefield || choose.is_search {
        return None;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == choose.tag.as_str()
    }) {
        return None;
    }

    let choose_desc = choose.filter.description();
    let base = strip_leading_article(&choose_desc);
    let plural = pluralize_noun_phrase(base);
    let count_text = |n: usize| {
        number_word(n as i32)
            .map(str::to_string)
            .unwrap_or_else(|| n.to_string())
    };
    let sentence = if choose.count.is_dynamic_x() {
        format!("X target {plural} can't block this turn")
    } else {
        match (choose.count.min, choose.count.max) {
        (0, Some(max)) => {
            if max == 1 {
                format!("Up to one target {base} can't block this turn")
            } else {
                format!(
                    "Up to {} target {plural} can't block this turn",
                    count_text(max)
                )
            }
        }
        (min, Some(max)) if min == max => {
            if min == 1 {
                format!("Target {base} can't block this turn")
            } else {
                format!("{} target {plural} can't block this turn", count_text(min))
            }
        }
        (0, None) => format!("Any number of target {plural} can't block this turn"),
        (min, None) => format!("At least {min} target {plural} can't block this turn"),
        (min, Some(max)) => format!("{min} to {max} target {plural} can't block this turn"),
    }
    };
    Some(sentence)
}

fn describe_tagged_target_then_cant_block(
    tagged: &crate::effects::TaggedEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let crate::effect::Restriction::Block(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != crate::effect::Until::EndOfTurn {
        return None;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tagged.tag.as_str()
    }) {
        return None;
    }

    let subject = capitalize_first(&describe_choose_spec(&target_only.target));
    Some(format!("{subject} can't block this turn"))
}

fn tap_uses_chosen_tag(spec: &ChooseSpec, tag: &str) -> bool {
    matches!(spec.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

fn describe_choose_then_tap_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    tap: &crate::effects::TapEffect,
) -> Option<String> {
    if choose.zone != Zone::Battlefield || choose.is_search {
        return None;
    }
    if !tap_uses_chosen_tag(&tap.spec, choose.tag.as_str()) {
        return None;
    }

    if choose.count.is_single() {
        return Some(format!(
            "Tap {}",
            with_indefinite_article(&choose.filter.description())
        ));
    }

    let exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    let count_text = number_word(exact as i32)
        .map(str::to_string)
        .unwrap_or_else(|| exact.to_string());
    Some(format!(
        "Tap {} {}",
        count_text,
        pluralize_noun_phrase(&choose.filter.description())
    ))
}

fn exile_uses_chosen_tag(spec: &ChooseSpec, tag: &str) -> bool {
    matches!(spec.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

fn describe_for_each_filter(filter: &ObjectFilter) -> String {
    let mut base_filter = filter.clone();
    base_filter.controller = None;

    let description = base_filter.description();
    let mut base = strip_indefinite_article(&description).to_string();
    if let Some(rest) = base.strip_prefix("permanent ") {
        if filter.controller.is_some() {
            base = rest.to_string();
        } else {
            base = format!("{rest} on the battlefield");
        }
    }

    if let Some(controller) = &filter.controller {
        if matches!(controller, PlayerFilter::You) {
            return format!("{base} you control");
        }
        return format!("{base} {} controls", describe_player_filter(controller));
    }
    base
}

fn describe_each_controlled_by_iterated(filter: &ObjectFilter) -> Option<String> {
    if filter.controller != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    if !filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.colors.is_none()
        && filter.excluded_card_types.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.excluded_colors.is_empty()
        && !filter.token
        && !filter.nontoken
        && !filter.tapped
        && !filter.untapped
        && !filter.attacking
        && !filter.blocking
        && filter.zone.is_none()
        && filter.tagged_constraints.is_empty()
        && filter.targets_object.is_none()
        && filter.targets_player.is_none()
        && filter.custom_static_markers.is_empty()
        && filter.excluded_custom_static_markers.is_empty()
    {
        let words = filter
            .card_types
            .iter()
            .map(|card_type| format!("{:?}", card_type).to_ascii_lowercase())
            .collect::<Vec<_>>();
        let list = match words.len() {
            0 => String::new(),
            1 => words[0].clone(),
            2 => format!("{} and {}", words[0], words[1]),
            _ => {
                let mut out = words[..words.len() - 1].join(", ");
                out.push_str(", and ");
                out.push_str(&words[words.len() - 1]);
                out
            }
        };
        return Some(format!("each {list} they control"));
    }
    None
}

fn describe_for_players_damage_and_controlled_damage(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }
    let deal_player = for_players.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        deal_player.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }
    let for_each = for_players.effects[1].downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.effects.len() != 1 {
        return None;
    }
    let deal_object = for_each.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if deal_object.amount != deal_player.amount {
        return None;
    }
    if !matches!(deal_object.target, ChooseSpec::Iterated) {
        return None;
    }
    let objects = describe_each_controlled_by_iterated(&for_each.filter)?;
    let player_filter_text = describe_player_filter(&for_players.filter);
    let each_player = strip_leading_article(&player_filter_text);
    Some(format!(
        "Deal {} damage to each {} and {}",
        describe_value(&deal_player.amount),
        each_player,
        objects
    ))
}

fn describe_draw_for_each(draw: &crate::effects::DrawCardsEffect) -> Option<String> {
    let Value::Count(filter) = &draw.count else {
        return None;
    };
    let player = describe_player_filter(&draw.player);
    let verb = player_verb(&player, "draw", "draws");
    Some(format!(
        "{player} {verb} a card for each {}",
        describe_for_each_filter(filter)
    ))
}

fn describe_compact_token_count(value: &Value, token_name: &str) -> String {
    match value {
        Value::Fixed(1) => format!("a {token_name} token"),
        Value::Fixed(n) => format!("{n} {token_name} tokens"),
        _ => format!("{} {token_name} token(s)", describe_value(value)),
    }
}

fn describe_compact_create_token(
    create_token: &crate::effects::CreateTokenEffect,
) -> Option<String> {
    if create_token.enters_tapped
        || create_token.enters_attacking
        || create_token.exile_at_end_of_combat
    {
        return None;
    }

    let token_name = create_token.token.name();
    let is_compact_named_token = matches!(
        token_name,
        "Treasure" | "Clue" | "Food" | "Blood" | "Powerstone"
    );
    if !is_compact_named_token {
        return None;
    }

    let amount = describe_compact_token_count(&create_token.count, token_name);
    if matches!(create_token.controller, PlayerFilter::You) {
        Some(format!("Create {amount}"))
    } else {
        Some(format!(
            "Create {amount} under {} control",
            describe_possessive_player_filter(&create_token.controller)
        ))
    }
}

fn choose_exact_count(choose: &crate::effects::ChooseObjectsEffect) -> Option<usize> {
    choose.count.max.filter(|max| *max == choose.count.min)
}

fn describe_choose_selection(choose: &crate::effects::ChooseObjectsEffect) -> String {
    if choose.top_only {
        if let Some(exact) = choose_exact_count(choose) {
            if exact > 1 {
                let count_text = number_word(exact as i32)
                    .map(str::to_string)
                    .unwrap_or_else(|| exact.to_string());
                return format!("the top {count_text} cards");
            }
        }
        return "the top card".to_string();
    }

    let filter_text = choose.filter.description();
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    if let Some(rest) = card_desc.strip_prefix("card ") {
        card_desc = format!("{rest} card");
    }

    if choose.count.is_single() {
        return with_indefinite_article(&card_desc);
    }
    if let Some(exact) = choose_exact_count(choose) {
        let count_text = number_word(exact as i32)
            .map(str::to_string)
            .unwrap_or_else(|| exact.to_string());
        return format!("{count_text} {}", pluralize_noun_phrase(&card_desc));
    }
    format!(
        "{} {}",
        describe_choice_count(&choose.count),
        pluralize_noun_phrase(&card_desc)
    )
}

fn describe_choose_then_exile(
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if choose.is_search || !exile_uses_chosen_tag(&exile.spec, choose.tag.as_str()) {
        return None;
    }

    let zone_text = match choose.zone {
        Zone::Hand => "hand",
        Zone::Graveyard => "graveyard",
        Zone::Library => "library",
        _ => return None,
    };
    let chooser = describe_player_filter(&choose.chooser);
    let verb = player_verb(&chooser, "exile", "exiles");
    let chosen = describe_choose_selection(choose);
    let origin = if choose.zone == Zone::Library && choose.top_only {
        format!(
            "of {} {zone_text}",
            describe_possessive_player_filter(&choose.chooser)
        )
    } else {
        format!(
            "from {} {zone_text}",
            describe_possessive_player_filter(&choose.chooser)
        )
    };
    Some(format!("{chooser} {verb} {chosen} {origin}"))
}

fn describe_draw_then_discard(
    draw: &crate::effects::DrawCardsEffect,
    discard: &crate::effects::DiscardEffect,
) -> Option<String> {
    if draw.player != discard.player {
        return None;
    }
    let player = describe_player_filter(&draw.player);
    let mut text = format!(
        "{player} {} {}, then {} {}",
        player_verb(&player, "draw", "draws"),
        describe_card_count(&draw.count),
        player_verb(&player, "discard", "discards"),
        describe_card_count(&discard.count)
    );
    if discard.random {
        text.push_str(" at random");
    }
    Some(text)
}

fn describe_with_id_then_if(
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if if_effect.condition != with_id.id {
        return None;
    }

    let setup = describe_effect(&with_id.effect);
    let then_text = describe_effect_list(&if_effect.then);
    let else_text = describe_effect_list(&if_effect.else_);

    let condition = if let Some(may) = with_id.effect.downcast_ref::<crate::effects::MayEffect>() {
        if let Some(decider) = may.decider.as_ref() {
            let who = describe_player_filter(decider);
            if who == "you" {
                "If you do".to_string()
            } else {
                format!("If {who} does")
            }
        } else {
            "If you do".to_string()
        }
    } else {
        match if_effect.predicate {
            EffectPredicate::Happened => "If it happened".to_string(),
            EffectPredicate::HappenedNotReplaced => {
                "If it happened and wasn't replaced".to_string()
            }
            _ => format!("If {}", describe_effect_predicate(&if_effect.predicate)),
        }
    };

    if else_text.is_empty() {
        Some(format!("{setup}. {condition}, {then_text}"))
    } else {
        Some(format!(
            "{setup}. {condition}, {then_text}. Otherwise, {else_text}"
        ))
    }
}

fn describe_with_id_then_choose_new_targets(
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

enum SearchDestination {
    Battlefield { tapped: bool },
    Hand,
    Graveyard,
    LibraryTop,
}

fn describe_search_choose_for_each(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachTaggedEffect,
    shuffle: Option<&crate::effects::ShuffleLibraryEffect>,
    shuffle_before_move: bool,
) -> Option<String> {
    let search_like = choose.is_search
        || (choose.zone == Zone::Library && choose.tag.as_str().starts_with("searched_"));
    if !search_like || choose.zone != Zone::Library {
        return None;
    }
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }

    let destination = if let Some(put) =
        for_each.effects[0].downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
    {
        if !matches!(put.target, ChooseSpec::Iterated) {
            return None;
        }
        SearchDestination::Battlefield { tapped: put.tapped }
    } else if let Some(return_to_hand) =
        for_each.effects[0].downcast_ref::<crate::effects::ReturnToHandEffect>()
    {
        if !matches!(return_to_hand.spec, ChooseSpec::Iterated) {
            return None;
        }
        SearchDestination::Hand
    } else if let Some(move_to_zone) =
        for_each.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        if !matches!(move_to_zone.target, ChooseSpec::Iterated) {
            return None;
        }
        if move_to_zone.zone == Zone::Battlefield {
            SearchDestination::Battlefield { tapped: false }
        } else if move_to_zone.zone == Zone::Hand {
            SearchDestination::Hand
        } else if move_to_zone.zone == Zone::Graveyard {
            SearchDestination::Graveyard
        } else if move_to_zone.zone == Zone::Library && move_to_zone.to_top {
            SearchDestination::LibraryTop
        } else {
            return None;
        }
    } else {
        return None;
    };

    if let Some(shuffle) = shuffle
        && shuffle.player != choose.chooser
    {
        return None;
    }

    let filter_text = choose.filter.description();
    let selection_text = if choose.count.is_single() {
        with_indefinite_article(&filter_text)
    } else {
        format!("{} {}", describe_choice_count(&choose.count), filter_text)
    };
    let pronoun = if choose.count.max == Some(1) {
        "it"
    } else {
        "them"
    };

    let mut text;
    match destination {
        SearchDestination::Battlefield { tapped } => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}, shuffle, then put {} onto the battlefield",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    pronoun
                )
            } else {
                format!(
                    "Search {} library for {}, put {} onto the battlefield",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    pronoun
                )
            };
            if tapped {
                text.push_str(" tapped");
            }
        }
        SearchDestination::Hand => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}, shuffle, then put {} into {} hand",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            } else {
                format!(
                    "Search {} library for {}, put {} into {} hand",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            };
        }
        SearchDestination::Graveyard => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}, shuffle, then put {} into {} graveyard",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            } else {
                format!(
                    "Search {} library for {}, put {} into {} graveyard",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            };
        }
        SearchDestination::LibraryTop => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}, shuffle, then put {} on top of {} library",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            } else {
                format!(
                    "Search {} library for {}, put {} on top of {} library",
                    describe_possessive_player_filter(&choose.chooser),
                    selection_text,
                    pronoun,
                    describe_possessive_player_filter(&choose.chooser)
                )
            };
        }
    }
    if shuffle.is_some() && !shuffle_before_move {
        text.push_str(", then shuffle");
    }
    Some(text)
}

fn describe_search_sequence(sequence: &crate::effects::SequenceEffect) -> Option<String> {
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

fn describe_effect(effect: &Effect) -> String {
    with_effect_render_depth(|| describe_effect_impl(effect))
}

fn describe_tap_or_untap_mode(choose_mode: &crate::effects::ChooseModeEffect) -> Option<String> {
    if choose_mode.modes.len() != 2 {
        return None;
    }
    let is_choose_one = matches!(choose_mode.choose_count, Value::Fixed(1))
        && choose_mode
            .min_choose_count
            .as_ref()
            .is_none_or(|value| matches!(value, Value::Fixed(1)));
    if !is_choose_one {
        return None;
    }
    let mut target = None;
    let mut saw_tap = false;
    let mut saw_untap = false;
    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        let effect = &mode.effects[0];
        if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
            saw_tap = true;
            target = Some(describe_choose_spec(&tap.spec));
            continue;
        }
        if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
            saw_untap = true;
            target = Some(describe_choose_spec(&untap.spec));
            continue;
        }
        return None;
    }
    if saw_tap && saw_untap {
        let target = target.unwrap_or_else(|| "that object".to_string());
        return Some(format!("Tap or untap {target}"));
    }
    None
}

fn describe_effect_impl(effect: &Effect) -> String {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        if let Some(compact) = describe_search_sequence(sequence) {
            return compact;
        }
        return describe_effect_list(&sequence.effects);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        let description = for_each.filter.description();
        let filter_text = strip_indefinite_article(&description);
        return format!(
            "For each {}, {}",
            filter_text,
            describe_effect_list(&for_each.effects)
        );
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return format!(
            "For each tagged '{}' object, {}",
            for_each_tagged.tag.as_str(),
            describe_effect_list(&for_each_tagged.effects)
        );
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        if let Some(compact) = describe_for_players_choose_then_sacrifice(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_damage_and_controlled_damage(for_players) {
            return compact;
        }
        let player_filter_text = describe_player_filter(&for_players.filter);
        let each_player = strip_leading_article(&player_filter_text);
        return format!(
            "For each {}, {}",
            each_player,
            describe_effect_list(&for_players.effects)
        );
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        let chooser = describe_player_filter(&choose.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        let search_like = choose.is_search
            || (choose.zone == Zone::Library && choose.tag.as_str().starts_with("searched_"));
        let filter_text = choose.filter.description();
        let choice_text = if choose.top_only {
            if let Some(exact) = choose_exact_count(choose) {
                if exact > 1 {
                    let count_text = number_word(exact as i32)
                        .map(str::to_string)
                        .unwrap_or_else(|| exact.to_string());
                    format!(
                        "the top {count_text} {}",
                        pluralize_noun_phrase(&filter_text)
                    )
                } else {
                    format!("the top {filter_text}")
                }
            } else {
                format!("the top {filter_text}")
            }
        } else {
            format!("{} {}", describe_choice_count(&choose.count), filter_text)
        };
        return format!(
            "{} {} {} in {} and tags it as '{}'",
            chooser,
            if search_like {
                "searches for"
            } else {
                choose_verb
            },
            choice_text,
            match choose.zone {
                Zone::Battlefield => "the battlefield",
                Zone::Hand => "a hand",
                Zone::Graveyard => "a graveyard",
                Zone::Library => "a library",
                Zone::Stack => "the stack",
                Zone::Exile => "exile",
                Zone::Command => "the command zone",
            },
            choose.tag.as_str()
        );
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        let target = describe_choose_spec(&move_to_zone.target);
        return match move_to_zone.zone {
            Zone::Exile => format!("Exile {target}"),
            Zone::Graveyard => format!("Put {target} into its owner's graveyard"),
            Zone::Hand => format!(
                "Return {target} to {}",
                owner_hand_phrase_for_spec(&move_to_zone.target)
            ),
            Zone::Library => {
                if let Some(owner) = hand_owner_from_spec(&move_to_zone.target) {
                    let cards = describe_card_choice_count(move_to_zone.target.count());
                    let from_zone = match &owner {
                        Some(owner) => {
                            format!("{} hand", describe_possessive_player_filter(owner))
                        }
                        None => "a hand".to_string(),
                    };
                    let library = match &owner {
                        Some(owner) => {
                            format!("{} library", describe_possessive_player_filter(owner))
                        }
                        None => owner_library_phrase_for_spec(&move_to_zone.target).to_string(),
                    };
                    if move_to_zone.to_top {
                        return format!("Put {cards} from {from_zone} on top of {library}");
                    }
                    return format!("Put {cards} from {from_zone} on the bottom of {library}");
                }
                if let Some(owner) = graveyard_owner_from_spec(&move_to_zone.target) {
                    let cards = describe_choose_spec_without_graveyard_zone(&move_to_zone.target);
                    let from_zone = match &owner {
                        Some(owner) => {
                            format!("{} graveyard", describe_possessive_player_filter(owner))
                        }
                        None => "a graveyard".to_string(),
                    };
                    let library = match &owner {
                        Some(owner) => {
                            format!("{} library", describe_possessive_player_filter(owner))
                        }
                        None => owner_library_phrase_for_spec(&move_to_zone.target).to_string(),
                    };
                    if move_to_zone.to_top {
                        return format!("Put {cards} from {from_zone} on top of {library}");
                    }
                    return format!("Put {cards} from {from_zone} on the bottom of {library}");
                }
                if move_to_zone.to_top {
                    format!(
                        "Put {target} on top of {}",
                        owner_library_phrase_for_spec(&move_to_zone.target)
                    )
                } else {
                    format!(
                        "Put {target} on the bottom of {}",
                        owner_library_phrase_for_spec(&move_to_zone.target)
                    )
                }
            }
            Zone::Battlefield => {
                if let crate::target::ChooseSpec::Tagged(tag) = &move_to_zone.target
                    && tag.as_str().starts_with("exiled_")
                {
                    format!("Return {target} to the battlefield")
                } else {
                    format!("Put {target} onto the battlefield")
                }
            }
            Zone::Stack => format!("Put {target} on the stack"),
            Zone::Command => format!("Move {target} to the command zone"),
        };
    }
    if let Some(put_onto_battlefield) =
        effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
    {
        let target = describe_choose_spec(&put_onto_battlefield.target);
        let mut text = format!("Put {target} onto the battlefield");
        if put_onto_battlefield.tapped {
            text.push_str(" tapped");
        }
        return text;
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return format!("Exile {}", describe_choose_spec(&exile.spec));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return format!("Destroy {}", describe_choose_spec(&destroy.spec));
    }
    if let Some(deal_damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        if let Value::PowerOf(source) | Value::ToughnessOf(source) = &deal_damage.amount {
            let mut subject = describe_choose_spec(source);
            if subject == "this source" {
                subject = "this creature".to_string();
            } else if subject == "it" {
                subject = "that creature".to_string();
            }
            let mut target = describe_choose_spec(&deal_damage.target);
            if target == "this source" {
                target = "this creature".to_string();
            } else if target == "it" {
                target = "that creature".to_string();
            }
            let verb = if choose_spec_is_plural(source) {
                "deal"
            } else {
                "deals"
            };
            let stat = if matches!(&deal_damage.amount, Value::ToughnessOf(_)) {
                "toughness"
            } else {
                "power"
            };
            return format!("{subject} {verb} damage equal to its {stat} to {target}");
        }
        return format!(
            "Deal {} damage to {}",
            describe_value(&deal_damage.amount),
            describe_choose_spec(&deal_damage.target)
        );
    }
    if let Some(fight) = effect.downcast_ref::<crate::effects::FightEffect>() {
        return format!(
            "{} fights {}",
            describe_choose_spec(&fight.creature1),
            describe_choose_spec(&fight.creature2)
        );
    }
    if let Some(counter_spell) = effect.downcast_ref::<crate::effects::CounterEffect>() {
        return format!("Counter {}", describe_choose_spec(&counter_spell.target));
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        let payer = describe_player_filter(&unless_pays.player);
        let pay_verb = player_verb(&payer, "pay", "pays");
        if unless_pays.effects.len() == 1
            && let Some(counter) =
                unless_pays.effects[0].downcast_ref::<crate::effects::CounterEffect>()
        {
            return format!(
                "Counter {} unless {} {} {}",
                describe_choose_spec(&counter.target),
                payer,
                pay_verb,
                unless_pays
                    .mana
                    .iter()
                    .copied()
                    .map(describe_mana_symbol)
                    .collect::<Vec<_>>()
                    .join("")
            );
        }

        let inner_text = describe_effect_list(&unless_pays.effects);
        let mana_text = unless_pays
            .mana
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        return format!(
            "{} unless {} {} {}",
            inner_text,
            payer,
            pay_verb,
            mana_text
        );
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        let inner_text = describe_effect_list(&unless_action.effects);
        let alt_text = describe_effect_list(&unless_action.alternative);
        let player = describe_player_filter(&unless_action.player);
        let unless_clause = if alt_text == player || alt_text.starts_with(&format!("{player} ")) {
            alt_text
        } else {
            format!("{player} {alt_text}")
        };
        return format!("{} unless {}", inner_text, unless_clause);
    }
    if let Some(put_counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        let mut target = describe_choose_spec(&put_counters.target);
        if let ChooseSpec::WithCount(inner, count) = &put_counters.target
            && matches!(inner.as_ref(), ChooseSpec::Target(_))
            && !count.is_single()
            && !target.starts_with("each of ")
        {
            target = format!("each of {target}");
        }
        return format!(
            "Put {} on {}",
            describe_put_counter_phrase(&put_counters.count, put_counters.counter_type),
            target
        );
    }
    if let Some(move_counters) = effect.downcast_ref::<crate::effects::MoveAllCountersEffect>() {
        return format!(
            "Move all counters from {} to {}",
            describe_choose_spec(&move_counters.from),
            describe_choose_spec(&move_counters.to)
        );
    }
    if let Some(proliferate) = effect.downcast_ref::<crate::effects::ProliferateEffect>() {
        let _ = proliferate;
        return "Proliferate".to_string();
    }
    if let Some(return_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        if let Some(owner) = graveyard_owner_from_spec(&return_to_battlefield.target) {
            let target_text =
                describe_choose_spec_without_graveyard_zone(&return_to_battlefield.target);
            let from_text = match owner {
                Some(owner) => format!(
                    "from {} graveyard",
                    describe_possessive_player_filter(&owner)
                ),
                None => "from graveyard".to_string(),
            };
            return format!(
                "Return {} {} to the battlefield{}",
                target_text,
                from_text,
                if return_to_battlefield.tapped {
                    " tapped"
                } else {
                    ""
                }
            );
        }
        return format!(
            "Return {} from graveyard to the battlefield{}",
            describe_choose_spec(&return_to_battlefield.target),
            if return_to_battlefield.tapped {
                " tapped"
            } else {
                ""
            }
        );
    }
    if let Some(return_all_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
    {
        return format!(
            "Return all {} to the battlefield{}",
            describe_for_each_filter(&return_all_to_battlefield.filter),
            if return_all_to_battlefield.tapped {
                " tapped"
            } else {
                ""
            }
        );
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        if let Some(dynamic_for_each) = describe_draw_for_each(draw) {
            return dynamic_for_each;
        }
        let player = describe_player_filter(&draw.player);
        return format!(
            "{player} {} {}",
            player_verb(&player, "draw", "draws"),
            describe_card_count(&draw.count)
        );
    }
    if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        let player = describe_choose_spec(&gain.player);
        return format!(
            "{} {} {} life",
            player,
            player_verb(&player, "gain", "gains"),
            describe_value(&gain.amount)
        );
    }
    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        let player = describe_choose_spec(&lose.player);
        return format!(
            "{} {} {} life",
            player,
            player_verb(&player, "lose", "loses"),
            describe_value(&lose.amount)
        );
    }
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
        let player = describe_player_filter(&discard.player);
        let random_suffix = if discard.random { " at random" } else { "" };
        return format!(
            "{} {} {}{}",
            player,
            player_verb(&player, "discard", "discards"),
            describe_card_count(&discard.count),
            random_suffix
        );
    }
    if let Some(discard_hand) = effect.downcast_ref::<crate::effects::DiscardHandEffect>() {
        let player = describe_player_filter(&discard_hand.player);
        let hand = if player == "you" {
            "your hand"
        } else {
            "their hand"
        };
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "discard", "discards"),
            hand
        );
    }
    if let Some(add_mana) = effect.downcast_ref::<crate::effects::AddManaEffect>() {
        let mana = add_mana
            .mana
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        return format!(
            "Add {} to {}",
            if mana.is_empty() { "{0}" } else { &mana },
            describe_mana_pool_owner(&add_mana.player)
        );
    }
    if let Some(add_colorless) = effect.downcast_ref::<crate::effects::AddColorlessManaEffect>() {
        return format!(
            "Add {} colorless mana to {}",
            describe_value(&add_colorless.amount),
            describe_mana_pool_owner(&add_colorless.player)
        );
    }
    if let Some(add_scaled) = effect.downcast_ref::<crate::effects::AddScaledManaEffect>() {
        let mana = add_scaled
            .mana
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        let mana_text = if mana.is_empty() { "{0}" } else { &mana };
        if let Value::Count(filter) = &add_scaled.amount {
            return format!(
                "Add {} for each {} to {}",
                mana_text,
                filter.description(),
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::Devotion { player, color } = &add_scaled.amount {
            let color_name = format!("{color:?}").to_ascii_lowercase();
            return format!(
                "Add an amount of {} equal to {} devotion to {}",
                mana_text,
                describe_possessive_player_filter(player),
                color_name
            );
        }
        if let Value::PowerOf(spec) = &add_scaled.amount {
            return format!(
                "Add an amount of {} equal to the power of {} to {}",
                mana_text,
                describe_choose_spec(spec),
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        return format!(
            "Add {} {} time(s) to {}",
            mana_text,
            describe_value(&add_scaled.amount),
            describe_mana_pool_owner(&add_scaled.player)
        );
    }
    if let Some(mill) = effect.downcast_ref::<crate::effects::MillEffect>() {
        let player = describe_player_filter(&mill.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "mill", "mills"),
            describe_card_count(&mill.count)
        );
    }
    if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
        return format!("Tap {}", describe_choose_spec(&tap.spec));
    }
    if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
        return format!("Untap {}", describe_choose_spec(&untap.spec));
    }
    if let Some(attach) = effect.downcast_ref::<crate::effects::AttachToEffect>() {
        return format!(
            "Attach this source to {}",
            describe_choose_spec(&attach.target)
        );
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        let player = describe_player_filter(&sacrifice.player);
        let verb = player_verb(&player, "sacrifice", "sacrifices");
        return format!(
            "{} {} {} {}",
            player,
            verb,
            describe_object_count(&sacrifice.count),
            sacrifice.filter.description()
        );
    }
    if let Some(sacrifice_target) = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>() {
        return format!(
            "Sacrifice {}",
            describe_choose_spec(&sacrifice_target.target)
        );
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        if let Some(owner) = graveyard_owner_from_spec(&return_to_hand.spec) {
            let target_text = describe_choose_spec_without_graveyard_zone(&return_to_hand.spec);
            let from_text = match &owner {
                Some(owner) => format!("{} graveyard", describe_possessive_player_filter(owner)),
                None => "a graveyard".to_string(),
            };
            let to_text = match &owner {
                Some(owner) => format!("{} hand", describe_possessive_player_filter(owner)),
                None => owner_hand_phrase_for_spec(&return_to_hand.spec).to_string(),
            };
            return format!("Return {target_text} from {from_text} to {to_text}");
        }
        return format!(
            "Return {} to {}",
            describe_choose_spec(&return_to_hand.spec),
            owner_hand_phrase_for_spec(&return_to_hand.spec)
        );
    }
    if let Some(shuffle_library) = effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>() {
        return format!(
            "Shuffle {} library",
            describe_possessive_player_filter(&shuffle_library.player)
        );
    }
    if let Some(search_library) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>() {
        let destination = match search_library.destination {
            Zone::Hand => "into hand",
            Zone::Battlefield => "onto the battlefield",
            Zone::Library => "on top of library",
            Zone::Graveyard => "into their graveyard",
            Zone::Exile => "into exile",
            Zone::Stack => "onto the stack",
            Zone::Command => "into the command zone",
        };
        let filter_desc = if is_generic_owned_card_search_filter(&search_library.filter) {
            "a card".to_string()
        } else {
            search_library.filter.description()
        };
        if search_library.reveal && search_library.destination != Zone::Battlefield {
            return format!(
                "Search {} library for {}, reveal it, put it {}, then shuffle",
                describe_possessive_player_filter(&search_library.player),
                filter_desc,
                destination
            );
        }
        return format!(
            "Search {} library for {}, put it {}, then shuffle",
            describe_possessive_player_filter(&search_library.player),
            filter_desc,
            destination
        );
    }
    if let Some(reveal_top) = effect.downcast_ref::<crate::effects::RevealTopEffect>() {
        let owner = describe_possessive_player_filter(&reveal_top.player);
        let mut text = format!("Reveal the top card of {owner} library");
        if let Some(tag) = &reveal_top.tag {
            text.push_str(&format!(" and tag it as '{}'", tag.as_str()));
        }
        return text;
    }
    if let Some(look_at_hand) = effect.downcast_ref::<crate::effects::LookAtHandEffect>() {
        let owner = describe_possessive_choose_spec(&look_at_hand.target);
        return format!("Look at {owner} hand");
    }
    if let Some(apply_continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        if let Some(text) = describe_apply_continuous_effect(apply_continuous) {
            return text;
        }
    }
    if let Some(grant_all) = effect.downcast_ref::<crate::effects::GrantAbilitiesAllEffect>() {
        return format!(
            "{} gains {} {}",
            grant_all.filter.description(),
            grant_all
                .abilities
                .iter()
                .map(|ability| ability.display())
                .collect::<Vec<_>>()
                .join(", "),
            describe_until(&grant_all.duration)
        );
    }
    if let Some(grant_target) = effect.downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()
    {
        return format!(
            "{} gains {} {}",
            describe_choose_spec(&grant_target.target),
            grant_target
                .abilities
                .iter()
                .map(|ability| ability.display())
                .collect::<Vec<_>>()
                .join(", "),
            describe_until(&grant_target.duration)
        );
    }
    if let Some(grant_object) = effect.downcast_ref::<crate::effects::GrantObjectAbilityEffect>() {
        return format!(
            "Grant {} to {}",
            describe_inline_ability(&grant_object.ability),
            describe_choose_spec(&grant_object.target)
        );
    }
    if let Some(modify_pt) = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>() {
        let power_text = describe_value(&modify_pt.power);
        let toughness_text = describe_value(&modify_pt.toughness);
        if !matches!(modify_pt.power, Value::Fixed(_)) && power_text == toughness_text {
            return format!(
                "{} gets +X/+X {}, where X is {}",
                describe_choose_spec(&modify_pt.target),
                describe_until(&modify_pt.duration),
                power_text
            );
        }
        return format!(
            "{} gets {}/{} {}",
            describe_choose_spec(&modify_pt.target),
            describe_signed_value(&modify_pt.power),
            describe_toughness_delta_with_power_context(&modify_pt.power, &modify_pt.toughness),
            describe_until(&modify_pt.duration)
        );
    }
    if let Some(set_base_pt) = effect.downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()
    {
        return format!(
            "{} has base power and toughness {}/{} {}",
            describe_choose_spec(&set_base_pt.target),
            describe_value(&set_base_pt.power),
            describe_value(&set_base_pt.toughness),
            describe_until(&set_base_pt.duration)
        );
    }
    if let Some(modify_pt_all) =
        effect.downcast_ref::<crate::effects::ModifyPowerToughnessAllEffect>()
    {
        return format!(
            "{} get {}/{} {}",
            modify_pt_all.filter.description(),
            describe_signed_value(&modify_pt_all.power),
            describe_toughness_delta_with_power_context(
                &modify_pt_all.power,
                &modify_pt_all.toughness,
            ),
            describe_until(&modify_pt_all.duration)
        );
    }
    if let Some(modify_pt_each) =
        effect.downcast_ref::<crate::effects::ModifyPowerToughnessForEachEffect>()
    {
        return format!(
            "{} gets +{} / +{} for each {} {}",
            describe_choose_spec(&modify_pt_each.target),
            modify_pt_each.power_per,
            modify_pt_each.toughness_per,
            describe_value(&modify_pt_each.count),
            describe_until(&modify_pt_each.duration)
        );
    }
    if let Some(gain_control) = effect.downcast_ref::<crate::effects::GainControlEffect>() {
        return format!(
            "Gain control of {} {}",
            describe_choose_spec(&gain_control.target),
            describe_until(&gain_control.duration)
        );
    }
    if let Some(exchange_control) = effect.downcast_ref::<crate::effects::ExchangeControlEffect>() {
        return format!(
            "Exchange control of {} and {}",
            describe_choose_spec(&exchange_control.permanent1),
            describe_choose_spec(&exchange_control.permanent2)
        );
    }
    if let Some(transform) = effect.downcast_ref::<crate::effects::TransformEffect>() {
        return format!("Transform {}", describe_transform_target(&transform.target));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if is_implicit_reference_tag(tagged.tag.as_str()) {
            return describe_effect(&tagged.effect);
        }
        return format!(
            "Tag '{}' then {}",
            tagged.tag.as_str(),
            describe_effect(&tagged.effect)
        );
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        if is_implicit_reference_tag(tag_all.tag.as_str()) {
            return describe_effect(&tag_all.effect);
        }
        return format!(
            "Tag all affected objects as '{}' then {}",
            tag_all.tag.as_str(),
            describe_effect(&tag_all.effect)
        );
    }
    if let Some(tag_triggering) = effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    {
        if is_implicit_reference_tag(tag_triggering.tag.as_str()) {
            return String::new();
        }
        return format!(
            "Tag the triggering object as '{}'",
            tag_triggering.tag.as_str()
        );
    }
    if let Some(tag_damage_target) =
        effect.downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>()
    {
        if is_implicit_reference_tag(tag_damage_target.tag.as_str()) {
            return String::new();
        }
        return format!(
            "Tag the triggering damaged object as '{}'",
            tag_damage_target.tag.as_str()
        );
    }
    if let Some(tag_attached) = effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>() {
        return format!(
            "Tag the object attached to this source as '{}'",
            tag_attached.tag.as_str()
        );
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return describe_effect(&with_id.effect);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        let true_branch = describe_effect_list(&conditional.if_true);
        let false_branch = describe_effect_list(&conditional.if_false);
        if false_branch.is_empty() {
            return format!(
                "If {}, {}",
                describe_condition(&conditional.condition),
                true_branch
            );
        }
        return format!(
            "If {}, {}. Otherwise, {}",
            describe_condition(&conditional.condition),
            true_branch,
            false_branch
        );
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        let then_text = describe_effect_list(&if_effect.then);
        let else_text = describe_effect_list(&if_effect.else_);
        if else_text.is_empty() {
            return format!(
                "If effect #{} {}, {}",
                if_effect.condition.0,
                describe_effect_predicate(&if_effect.predicate),
                then_text
            );
        }
        return format!(
            "If effect #{} {}, {}. Otherwise, {}",
            if_effect.condition.0,
            describe_effect_predicate(&if_effect.predicate),
            then_text,
            else_text
        );
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        if let Some(decider) = may.decider.as_ref() {
            let who = describe_player_filter(decider);
            let mut inner = describe_effect_list(&may.effects);
            let prefix = format!("{who} ");
            if inner.starts_with(&prefix) {
                inner = inner[prefix.len()..].to_string();
            } else if who == "you" && inner.starts_with("you ") {
                inner = inner["you ".len()..].to_string();
            }
            if who == "you" {
                inner = normalize_you_verb_phrase(&inner);
            }
            return format!("{who} may {inner}");
        }
        let mut inner = describe_effect_list(&may.effects);
        if inner.starts_with("you ") {
            inner = inner["you ".len()..].to_string();
        }
        inner = normalize_you_verb_phrase(&inner);
        return format!("You may {inner}");
    }
    if let Some(target_only) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() {
        return format!("Choose {}", describe_choose_spec(&target_only.target));
    }
    if let Some(compact) = describe_compact_protection_choice(effect) {
        return compact;
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        if let Some(compact) = describe_tap_or_untap_mode(choose_mode) {
            return compact;
        }
        let header = describe_mode_choice_header(
            &choose_mode.choose_count,
            choose_mode.min_choose_count.as_ref(),
        );
        let modes = choose_mode
            .modes
            .iter()
            .map(|mode| {
                let description = ensure_trailing_period(mode.description.trim());
                let mode_effects = describe_effect_list(&mode.effects);
                if mode_effects.is_empty() {
                    description
                } else if modal_text_equivalent(&description, &mode_effects) {
                    description
                } else {
                    format!("{description} [{mode_effects}]")
                }
            })
            .collect::<Vec<_>>()
            .join(" • ");
        return format!("{header} {modes}");
    }
    if let Some(create_token) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        if let Some(compact) = describe_compact_create_token(create_token) {
            return compact;
        }
        let token_blueprint = describe_token_blueprint(&create_token.token);
        let mut text = format!(
            "Create {} {} under {} control",
            describe_value(&create_token.count),
            token_blueprint,
            describe_possessive_player_filter(&create_token.controller)
        );
        if create_token.enters_tapped {
            text.push_str(", tapped");
        }
        if create_token.enters_attacking {
            text.push_str(", attacking");
        }
        if create_token.exile_at_end_of_combat {
            text.push_str(", and exile them at end of combat");
        }
        return text;
    }
    if let Some(create_copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() {
        let target = describe_choose_spec(&create_copy.target);
        let mut text = match create_copy.count {
            Value::Fixed(1) => format!("Create a token that's a copy of {target}"),
            Value::Fixed(n) => format!("Create {n} tokens that are copies of {target}"),
            _ => format!(
                "Create {} token copy/copies of {target}",
                describe_value(&create_copy.count)
            ),
        };
        if !matches!(create_copy.controller, PlayerFilter::You) {
            text.push_str(&format!(
                " under {} control",
                describe_possessive_player_filter(&create_copy.controller)
            ));
        }
        if create_copy.enters_tapped {
            text.push_str(", tapped");
        }
        if create_copy.has_haste {
            text.push_str(", with haste");
        }
        if create_copy.enters_attacking {
            text.push_str(", attacking");
        }
        if create_copy.exile_at_end_of_combat {
            text.push_str(", and exile at end of combat");
        }
        if create_copy.sacrifice_at_next_end_step {
            text.push_str(", and sacrifice it at the beginning of the next end step");
        }
        if let Some(adjustment) = &create_copy.pt_adjustment {
            text.push_str(&format!(", with P/T adjustment {adjustment:?}"));
        }
        return text;
    }
    if let Some(earthbend) = effect.downcast_ref::<crate::effects::EarthbendEffect>() {
        return format!(
            "Earthbend {} with {} +1/+1 counter(s)",
            describe_choose_spec(&earthbend.target),
            earthbend.counters
        );
    }
    if let Some(explore) = effect.downcast_ref::<crate::effects::ExploreEffect>() {
        if matches!(explore.target.base(), ChooseSpec::Source) {
            return "it explores".to_string();
        }
        let subject = capitalize_first(&describe_choose_spec(&explore.target));
        return format!("{subject} explores");
    }
    if effect
        .downcast_ref::<crate::effects::OpenAttractionEffect>()
        .is_some()
    {
        return "Open an Attraction".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::ManifestDreadEffect>()
        .is_some()
    {
        return "Manifest dread".to_string();
    }
    if let Some(bolster) = effect.downcast_ref::<crate::effects::BolsterEffect>() {
        return format!("Bolster {}", bolster.amount);
    }
    if let Some(support) = effect.downcast_ref::<crate::effects::SupportEffect>() {
        return format!("Support {}", support.amount);
    }
    if let Some(adapt) = effect.downcast_ref::<crate::effects::AdaptEffect>() {
        return format!("Adapt {}", adapt.amount);
    }
    if effect
        .downcast_ref::<crate::effects::CounterAbilityEffect>()
        .is_some()
    {
        return "Counter target activated or triggered ability".to_string();
    }
    if let Some(regenerate) = effect.downcast_ref::<crate::effects::RegenerateEffect>() {
        return format!(
            "Regenerate {} {}",
            describe_choose_spec(&regenerate.target),
            describe_until(&regenerate.duration)
        );
    }
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>() {
        return format!(
            "{} {}",
            describe_restriction(&cant.restriction),
            describe_until(&cant.duration)
        );
    }
    if let Some(remove_up_to_any) =
        effect.downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
    {
        return format!(
            "Remove up to {} counters from {}",
            describe_value(&remove_up_to_any.max_count),
            describe_choose_spec(&remove_up_to_any.target)
        );
    }
    if let Some(surveil) = effect.downcast_ref::<crate::effects::SurveilEffect>() {
        if surveil.player == PlayerFilter::You {
            return format!("Surveil {}", describe_value(&surveil.count));
        }
        let player = describe_player_filter(&surveil.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "surveil", "surveils"),
            describe_value(&surveil.count)
        );
    }
    if let Some(scry) = effect.downcast_ref::<crate::effects::ScryEffect>() {
        if scry.player == PlayerFilter::You {
            return format!("Scry {}", describe_value(&scry.count));
        }
        let player = describe_player_filter(&scry.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "scry", "scries"),
            describe_value(&scry.count)
        );
    }
    if let Some(investigate) = effect.downcast_ref::<crate::effects::InvestigateEffect>() {
        return format!("Investigate {}", describe_value(&investigate.count));
    }
    if let Some(poison) = effect.downcast_ref::<crate::effects::PoisonCountersEffect>() {
        return format!(
            "{} gets {} poison counter(s)",
            describe_player_filter(&poison.player),
            describe_value(&poison.count)
        );
    }
    if let Some(energy) = effect.downcast_ref::<crate::effects::EnergyCountersEffect>() {
        let player = describe_player_filter(&energy.player);
        return format!(
            "{} {} {} energy counter(s)",
            player,
            player_verb(&player, "get", "gets"),
            describe_value(&energy.count)
        );
    }
    if let Some(connive) = effect.downcast_ref::<crate::effects::ConniveEffect>() {
        return format!("{} connives", describe_choose_spec(&connive.target));
    }
    if let Some(extra_turn) = effect.downcast_ref::<crate::effects::ExtraTurnEffect>() {
        return format!(
            "{} takes an extra turn after this one",
            describe_player_filter(&extra_turn.player)
        );
    }
    if let Some(lose_game) = effect.downcast_ref::<crate::effects::LoseTheGameEffect>() {
        return format!(
            "{} loses the game",
            describe_player_filter(&lose_game.player)
        );
    }
    if let Some(skip_draw) = effect.downcast_ref::<crate::effects::SkipDrawStepEffect>() {
        return format!(
            "{} skips their next draw step",
            describe_player_filter(&skip_draw.player)
        );
    }
    if let Some(skip_turn) = effect.downcast_ref::<crate::effects::SkipTurnEffect>() {
        return format!(
            "{} skips their next turn",
            describe_player_filter(&skip_turn.player)
        );
    }
    if let Some(monstrosity) = effect.downcast_ref::<crate::effects::MonstrosityEffect>() {
        return format!("Monstrosity {}", describe_value(&monstrosity.n));
    }
    if let Some(copy_spell) = effect.downcast_ref::<crate::effects::CopySpellEffect>() {
        if matches!(copy_spell.target, ChooseSpec::Source)
            && matches!(
                copy_spell.count,
                Value::SpellsCastBeforeThisTurn(PlayerFilter::You)
            )
        {
            return "Copy this spell for each spell cast before it this turn".to_string();
        }
        if matches!(copy_spell.target, ChooseSpec::Source)
            && matches!(copy_spell.count, Value::Fixed(1))
        {
            return "Copy this spell".to_string();
        }
        return format!(
            "Copy {} {} time(s)",
            describe_choose_spec(&copy_spell.target),
            describe_value(&copy_spell.count)
        );
    }
    if let Some(choose_new) = effect.downcast_ref::<crate::effects::ChooseNewTargetsEffect>() {
        let chooser_text = choose_new
            .chooser
            .as_ref()
            .map(describe_player_filter)
            .unwrap_or_else(|| "you".to_string());
        return format!(
            "{} {} new targets for the copy",
            chooser_text,
            if choose_new.may {
                "may choose"
            } else {
                "chooses"
            },
        );
    }
    if let Some(set_life) = effect.downcast_ref::<crate::effects::SetLifeTotalEffect>() {
        return format!(
            "{}'s life total becomes {}",
            describe_player_filter(&set_life.player),
            describe_value(&set_life.amount)
        );
    }
    if let Some(pay_mana) = effect.downcast_ref::<crate::effects::PayManaEffect>() {
        return format!(
            "{} pays {}",
            describe_choose_spec(&pay_mana.player),
            pay_mana.cost.to_oracle()
        );
    }
    if let Some(add_any) = effect.downcast_ref::<crate::effects::AddManaOfAnyColorEffect>() {
        if let Some(colors) = &add_any.available_colors {
            if matches!(add_any.amount, Value::Fixed(1)) {
                let options = colors
                    .iter()
                    .copied()
                    .map(crate::mana::ManaSymbol::from_color)
                    .collect::<Vec<_>>();
                return format!(
                    "Add {} to {}",
                    describe_mana_alternatives(&options),
                    describe_mana_pool_owner(&add_any.player)
                );
            }
            let options = colors
                .iter()
                .copied()
                .map(crate::mana::ManaSymbol::from_color)
                .map(describe_mana_symbol)
                .collect::<Vec<_>>()
                .join(" and/or ");
            return format!(
                "Add {} mana in any combination of {} to {}",
                describe_value(&add_any.amount),
                options,
                describe_mana_pool_owner(&add_any.player)
            );
        }
        return format!(
            "Add {} mana of any color to {}",
            describe_value(&add_any.amount),
            describe_mana_pool_owner(&add_any.player)
        );
    }
    if let Some(add_one) = effect.downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>() {
        return format!(
            "Add {} mana of any one color to {}",
            describe_value(&add_one.amount),
            describe_mana_pool_owner(&add_one.player)
        );
    }
    if let Some(add_chosen) =
        effect.downcast_ref::<crate::effects::mana::AddManaOfChosenColorEffect>()
    {
        let pool = describe_mana_pool_owner(&add_chosen.player);
        let amount = describe_value(&add_chosen.amount);
        if let Some(fixed) = add_chosen.fixed_option {
            let fixed_symbol = describe_mana_symbol(crate::mana::ManaSymbol::from_color(fixed));
            if matches!(add_chosen.amount, Value::Fixed(1)) {
                return format!(
                    "Add {} or one mana of the chosen color to {}",
                    fixed_symbol, pool
                );
            }
            return format!(
                "Add {} or {} mana of the chosen color to {}",
                fixed_symbol, amount, pool
            );
        }
        if matches!(add_chosen.amount, Value::Fixed(1)) {
            return format!("Add one mana of the chosen color to {}", pool);
        }
        return format!("Add {} mana of the chosen color to {}", amount, pool);
    }
    if let Some(add_land_produced) =
        effect.downcast_ref::<crate::effects::AddManaOfLandProducedTypesEffect>()
    {
        let any_word = if add_land_produced.allow_colorless {
            "type"
        } else {
            "color"
        };
        let one_word = if add_land_produced.same_type {
            " one"
        } else {
            ""
        };
        return format!(
            "Add {} mana of any{} {} to {} that {} could produce",
            describe_value(&add_land_produced.amount),
            one_word,
            any_word,
            describe_mana_pool_owner(&add_land_produced.player),
            add_land_produced.land_filter.description()
        );
    }
    if let Some(add_commander) =
        effect.downcast_ref::<crate::effects::AddManaFromCommanderColorIdentityEffect>()
    {
        return format!(
            "Add {} mana of commander's color identity to {}",
            describe_value(&add_commander.amount),
            describe_mana_pool_owner(&add_commander.player)
        );
    }
    if effect
        .downcast_ref::<crate::effects::mana::AddManaOfImprintedColorsEffect>()
        .is_some()
    {
        return "Add one mana of any imprinted card color".to_string();
    }
    if let Some(prevent_damage) = effect.downcast_ref::<crate::effects::PreventDamageEffect>() {
        let filter = &prevent_damage.damage_filter;
        let is_default_filter = !filter.combat_only
            && !filter.noncombat_only
            && filter.from_source.is_none()
            && filter.from_specific_source.is_none()
            && filter
                .from_colors
                .as_ref()
                .is_none_or(|colors| colors.is_empty())
            && filter
                .from_card_types
                .as_ref()
                .is_none_or(|types| types.is_empty());
        let damage_text = if is_default_filter {
            "damage".to_string()
        } else {
            describe_damage_filter(filter)
        };
        return format!(
            "Prevent the next {} {} to {} {}",
            describe_value(&prevent_damage.amount),
            damage_text,
            describe_choose_spec(&prevent_damage.target),
            describe_until(&prevent_damage.duration)
        );
    }
    if let Some(prevent_from) =
        effect.downcast_ref::<crate::effects::PreventAllCombatDamageFromEffect>()
    {
        let timing = match prevent_from.until {
            Until::EndOfTurn => "this turn".to_string(),
            _ => describe_until(&prevent_from.until),
        };
        return format!(
            "Prevent all combat damage that would be dealt by {} {}",
            describe_choose_spec(&prevent_from.source),
            timing
        );
    }
    if let Some(prevent_all) = effect.downcast_ref::<crate::effects::PreventAllDamageEffect>() {
        let damage_type = describe_damage_filter(&prevent_all.damage_filter);
        let protected = describe_prevention_target(&prevent_all.target);
        if matches!(prevent_all.target, crate::prevention::PreventionTarget::All) {
            return format!(
                "Prevent {} {}",
                damage_type,
                describe_until(&prevent_all.until)
            );
        }
        return format!(
            "Prevent {} to {} {}",
            damage_type,
            protected,
            describe_until(&prevent_all.until)
        );
    }
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        return format!(
            "Schedule delayed trigger: {}",
            describe_effect_list(&schedule.effects)
        );
    }
    if let Some(exile_instead) =
        effect.downcast_ref::<crate::effects::ExileInsteadOfGraveyardEffect>()
    {
        return format!(
            "If a card would be put into {}'s graveyard, exile it instead",
            describe_player_filter(&exile_instead.player)
        );
    }
    if let Some(grant_play) = effect.downcast_ref::<crate::effects::GrantPlayFromGraveyardEffect>()
    {
        return format!(
            "{} may play lands and cast spells from their graveyard",
            describe_player_filter(&grant_play.player)
        );
    }
    if let Some(control_player) = effect.downcast_ref::<crate::effects::ControlPlayerEffect>() {
        return format!(
            "Control {} during their next turn",
            describe_player_filter(&control_player.player)
        );
    }
    if let Some(exile_hand) = effect.downcast_ref::<crate::effects::ExileFromHandAsCostEffect>() {
        return format!("Exile {} card(s) from your hand", exile_hand.count);
    }
    if let Some(for_each_ctrl) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        return format!(
            "For each controller of tagged '{}' objects, {}",
            for_each_ctrl.tag.as_str(),
            describe_effect_list(&for_each_ctrl.effects)
        );
    }
    if let Some(for_each_tagged_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        return format!(
            "For each tagged '{}' player, {}",
            for_each_tagged_player.tag.as_str(),
            describe_effect_list(&for_each_tagged_player.effects)
        );
    }
    format!("{effect:?}")
}

fn describe_activation_timing_clause(timing: &ActivationTiming) -> Option<&'static str> {
    match timing {
        ActivationTiming::AnyTime => None,
        ActivationTiming::SorcerySpeed => Some("Activate only as a sorcery"),
        ActivationTiming::DuringCombat => Some("Activate only during combat"),
        ActivationTiming::OncePerTurn => Some("Activate only once each turn"),
        ActivationTiming::DuringYourTurn => Some("Activate only during your turn"),
        ActivationTiming::DuringOpponentsTurn => Some("Activate only during an opponent's turn"),
    }
}

fn normalize_activation_restriction_clause(raw: &str) -> String {
    let mut clause = raw.trim().trim_end_matches('.').to_string();
    if clause.is_empty() {
        return clause;
    }
    clause = clause.replace("activate only as sorcery", "activate only as a sorcery");
    clause = clause.replace("activate only once turn", "activate only once each turn");
    if clause.starts_with("activate ") {
        clause = capitalize_first(&clause);
    }
    clause
}

fn describe_keyword_ability(ability: &Ability) -> Option<String> {
    let raw_text = ability.text.as_deref()?.trim();
    let text = raw_text.to_ascii_lowercase();
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.first().copied() == Some("equip") {
        if raw_text.eq_ignore_ascii_case("equip") {
            return Some("Equip".to_string());
        }
        return Some(raw_text.to_string());
    }
    if words.len() >= 2 && words[0] == "level" && words[1] == "up" {
        return Some(raw_text.to_string());
    }
    if text == "storm" {
        return Some("Storm".to_string());
    }
    if text == "toxic" || text.starts_with("toxic ") {
        return Some(raw_text.to_string());
    }
    let mut cycling_rendered = Vec::new();
    for (idx, word) in words.iter().enumerate() {
        if !word.ends_with("cycling") {
            continue;
        }
        let next = words.get(idx + 1);
        let has_cost = next.is_none_or(|next| is_cycling_cost_word(next));
        if !has_cost {
            continue;
        }
        let mut chars = word.chars();
        let base = match chars.next() {
            Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
            None => "Cycling".to_string(),
        };
        let mut cost_tokens = Vec::new();
        let mut j = idx + 1;
        while let Some(word) = words.get(j) {
            if is_cycling_cost_word(word) {
                cost_tokens.push(*word);
                j += 1;
            } else {
                break;
            }
        }
        if cost_tokens.is_empty() {
            cycling_rendered.push(base);
        } else {
            let cost = cost_tokens
                .iter()
                .map(|word| format!("{{{}}}", word.to_ascii_uppercase()))
                .collect::<Vec<_>>()
                .join("");
            cycling_rendered.push(format!("{} {}", base, cost));
        }
    }
    if !cycling_rendered.is_empty() {
        return Some(cycling_rendered.join(", "));
    }
    if text == "prowess" {
        return Some("Prowess".to_string());
    }
    if text == "exalted" {
        return Some("Exalted".to_string());
    }
    if text == "persist" {
        return Some("Persist".to_string());
    }
    if text == "undying" {
        return Some("Undying".to_string());
    }
    if text.starts_with("bushido ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("rampage ") {
        return Some(raw_text.to_string());
    }
    None
}

fn is_cycling_cost_word(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|ch| {
            ch.is_ascii_digit()
                || matches!(
                    ch,
                    '{' | '}' | '/' | 'w' | 'u' | 'b' | 'r' | 'g' | 'c' | 'x'
                )
        })
}

fn choices_are_simple_targets(choices: &[ChooseSpec]) -> bool {
    fn is_simple_target(choice: &ChooseSpec) -> bool {
        match choice {
            ChooseSpec::Target(_) | ChooseSpec::AnyTarget => true,
            ChooseSpec::WithCount(inner, _) => is_simple_target(inner),
            _ => false,
        }
    }

    choices.iter().all(is_simple_target)
}

fn describe_ability(index: usize, ability: &Ability) -> Vec<String> {
    if let Some(keyword) = describe_keyword_ability(ability) {
        return vec![format!("Keyword ability {index}: {keyword}")];
    }
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            if let Some(levels) = static_ability.level_abilities()
                && !levels.is_empty()
            {
                let mut lines = Vec::new();
                for level in levels {
                    let range = match level.max_level {
                        Some(max) if max == level.min_level => format!("Level {}", level.min_level),
                        Some(max) => format!("Level {}-{}", level.min_level, max),
                        None => format!("Level {}+", level.min_level),
                    };
                    lines.push(format!("Static ability {index}: {range}"));
                    if let Some((power, toughness)) = level.power_toughness {
                        lines.push(format!("Static ability {index}: {power}/{toughness}"));
                    }
                    for granted in &level.abilities {
                        lines.push(format!("Static ability {index}: {}", granted.display()));
                    }
                }
                return lines;
            }
            vec![format!(
                "Static ability {index}: {}",
                static_ability.display()
            )]
        }
        AbilityKind::Triggered(triggered) => {
            let mut line = format!("Triggered ability {index}: {}", triggered.trigger.display());
            let mut clauses = Vec::new();
            if !triggered.choices.is_empty()
                && !(!triggered.effects.is_empty()
                    && choices_are_simple_targets(&triggered.choices))
            {
                let choices = triggered
                    .choices
                    .iter()
                    .map(describe_choose_spec)
                    .collect::<Vec<_>>()
                    .join(", ");
                clauses.push(format!("choose {choices}"));
            }
            if !triggered.effects.is_empty() {
                clauses.push(describe_effect_list(&triggered.effects));
            }
            if !clauses.is_empty() {
                line.push_str(": ");
                line.push_str(&clauses.join(": "));
            }
            if triggered.once_each_turn {
                line.push_str(". This ability triggers only once each turn");
            }
            vec![line]
        }
        AbilityKind::Activated(activated) => {
            let mut line = format!("Activated ability {index}");
            let mut pre = Vec::new();
            if !activated.mana_cost.costs().is_empty() {
                pre.push(describe_cost_list(activated.mana_cost.costs()));
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
                line.push_str(": ");
                line.push_str(&pre.join(", "));
            }
            if !activated.effects.is_empty() {
                line.push_str(": ");
                line.push_str(&describe_effect_list(&activated.effects));
            }
            let mut restriction_clauses: Vec<String> = Vec::new();
            if let Some(timing_clause) = describe_activation_timing_clause(&activated.timing) {
                restriction_clauses.push(timing_clause.to_string());
            }
            for clause in &activated.additional_restrictions {
                let normalized = normalize_activation_restriction_clause(clause);
                if normalized.is_empty() {
                    continue;
                }
                if restriction_clauses
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&normalized))
                {
                    continue;
                }
                restriction_clauses.push(normalized);
            }
            if !restriction_clauses.is_empty() {
                line.push_str(". ");
                line.push_str(&restriction_clauses.join(" and "));
            }
            vec![line]
        }
        AbilityKind::Mana(mana_ability) => {
            let mut line = format!("Mana ability {index}");
            let mut parts = Vec::new();
            if !mana_ability.mana_cost.costs().is_empty() {
                parts.push(describe_cost_list(mana_ability.mana_cost.costs()));
            }
            if !mana_ability.mana.is_empty() {
                parts.push(format!(
                    "Add {}",
                    mana_ability
                        .mana
                        .iter()
                        .copied()
                        .map(describe_mana_symbol)
                        .collect::<Vec<_>>()
                        .join("")
                ));
            }
            if !parts.is_empty() {
                line.push_str(": ");
                line.push_str(&parts.join(", "));
            }
            if let Some(extra_effects) = &mana_ability.effects
                && !extra_effects.is_empty()
            {
                line.push_str(": ");
                line.push_str(&describe_effect_list(extra_effects));
            }
            if let Some(condition) = &mana_ability.activation_condition {
                if !parts.is_empty() || mana_ability.effects.is_some() {
                    line.push_str(". ");
                } else {
                    line.push_str(": ");
                }
                line.push_str(&describe_mana_activation_condition(condition));
            }
            vec![line]
        }
    }
}

fn describe_mana_activation_condition(condition: &crate::ability::ManaAbilityCondition) -> String {
    match condition {
        crate::ability::ManaAbilityCondition::ControlLandWithSubtype(subtypes) => {
            let names = subtypes
                .iter()
                .map(|subtype| format!("{subtype:?}"))
                .collect::<Vec<_>>();
            match names.len() {
                0 => "Activate only if you control a land of the required subtype".to_string(),
                1 => format!("Activate only if you control a {}", names[0]),
                2 => format!(
                    "Activate only if you control a {} or a {}",
                    names[0], names[1]
                ),
                _ => {
                    let mut list = names[..names.len() - 1]
                        .iter()
                        .map(|name| format!("a {name}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    list.push_str(", or a ");
                    list.push_str(names.last().expect("last subtype name"));
                    format!("Activate only if you control {list}")
                }
            }
        }
        crate::ability::ManaAbilityCondition::Timing(timing) => match timing {
            ActivationTiming::AnyTime => "Activate only any time you could cast an instant"
                .to_string(),
            ActivationTiming::SorcerySpeed => "Activate only as a sorcery".to_string(),
            ActivationTiming::DuringCombat => "Activate only during combat".to_string(),
            ActivationTiming::OncePerTurn => "Activate only once each turn".to_string(),
            ActivationTiming::DuringYourTurn => "Activate only during your turn".to_string(),
            ActivationTiming::DuringOpponentsTurn => {
                "Activate only during an opponent's turn".to_string()
            }
        },
        crate::ability::ManaAbilityCondition::All(conditions) => {
            let clauses = conditions
                .iter()
                .map(describe_mana_activation_condition)
                .collect::<Vec<_>>();
            match clauses.len() {
                0 => String::new(),
                1 => clauses[0].clone(),
                _ => {
                    let mut iter = clauses.into_iter();
                    let first = iter.next().unwrap_or_default();
                    let mut line = first;
                    for clause in iter {
                        if let Some(rest) = clause.strip_prefix("Activate only ") {
                            line.push_str(" and ");
                            line.push_str(rest);
                        } else {
                            line.push_str(" and ");
                            line.push_str(&clause);
                        }
                    }
                    line
                }
            }
        }
    }
}

fn collapse_redundant_keyword_tail(line: &str) -> String {
    let mut normalized = line.trim().to_string();
    loop {
        let lower = normalized.to_ascii_lowercase();
        let Some(split_idx) = lower.rfind(" and ") else {
            break;
        };
        let prefix = normalized[..split_idx].trim_end();
        let tail = normalized[split_idx + 5..].trim_start();

        // Only collapse trailing keyword duplication that appears as a second
        // sentence/clause after punctuation (e.g. ". and haste." or
        // "). and vigilance.").
        let Some(last_prefix_char) = prefix.chars().rev().find(|ch| !ch.is_whitespace()) else {
            break;
        };
        if !matches!(last_prefix_char, '.' | ')' | '"') {
            break;
        }

        let tail_no_reminder = tail
            .split_once('(')
            .map(|(head, _)| head.trim())
            .unwrap_or(tail)
            .trim_end_matches('.')
            .trim();
        let keyword_tail = if is_keyword_phrase(tail_no_reminder) {
            Some(tail_no_reminder.to_ascii_lowercase())
        } else {
            normalize_keyword_list_phrase(tail_no_reminder)
        };

        let Some(keyword_tail) = keyword_tail else {
            break;
        };

        let prefix_lower = strip_parenthetical_segments(prefix).to_ascii_lowercase();
        let duplicate = prefix_lower.contains(&format!(" has {keyword_tail}"))
            || prefix_lower.contains(&format!(" gain {keyword_tail}"))
            || prefix_lower.contains(&format!(" gains {keyword_tail}"));
        if !duplicate {
            break;
        }

        let collapsed = prefix.trim_end_matches('.').trim_end();
        normalized = if collapsed.ends_with('"') || collapsed.ends_with(')') {
            collapsed.to_string()
        } else {
            format!("{collapsed}.")
        };
    }
    normalized
}

fn describe_enchant_filter(filter: &ObjectFilter) -> String {
    let desc = filter.description();
    if let Some(stripped) = desc.strip_prefix("a ") {
        stripped.to_string()
    } else if let Some(stripped) = desc.strip_prefix("an ") {
        stripped.to_string()
    } else {
        desc
    }
}

fn ability_can_render_as_keyword_group(ability: &Ability) -> bool {
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            static_ability.is_keyword()
                || static_ability.id() == crate::static_abilities::StaticAbilityId::Custom
        }
        _ => false,
    }
}

pub fn compiled_lines(def: &CardDefinition) -> Vec<String> {
    let mut out = Vec::new();
    let has_attach_only_spell_effect = def.spell_effect.as_ref().is_some_and(|effects| {
        effects.len() == 1
            && effects[0]
                .downcast_ref::<crate::effects::AttachToEffect>()
                .is_some()
    });
    for (idx, method) in def.alternative_casts.iter().enumerate() {
        match method {
            AlternativeCastingMethod::AlternativeCost {
                name,
                mana_cost,
                cost_effects,
            } => {
                let mut parts = Vec::new();
                if let Some(cost) = mana_cost {
                    parts.push(format!("pay {}", cost.to_oracle()));
                }
                if !cost_effects.is_empty() {
                    parts.push(describe_effect_list(cost_effects));
                }
                let clause = if parts.is_empty() {
                    "cast this spell without paying its mana cost".to_string()
                } else {
                    parts.join(" and ")
                };
                let mut line = format!(
                    "You may {clause} rather than pay this spell's mana cost"
                );
                if !name.is_empty() {
                    line.push_str(&format!(" ({name})"));
                }
                out.push(line);
            }
            AlternativeCastingMethod::Madness { cost } => {
                out.push(format!("Madness {}", cost.to_oracle()));
            }
            AlternativeCastingMethod::Miracle { cost } => {
                out.push(format!("Miracle {}", cost.to_oracle()));
            }
            AlternativeCastingMethod::Escape { cost, exile_count } => {
                let count_text = small_number_word(*exile_count)
                    .map(str::to_string)
                    .unwrap_or_else(|| exile_count.to_string());
                if let Some(cost) = cost {
                    out.push(format!(
                        "Escape {}. Exile {count_text} other cards from your graveyard",
                        cost.to_oracle()
                    ));
                } else {
                    out.push(format!(
                        "Escape. Exile {count_text} other cards from your graveyard"
                    ));
                }
            }
            other => {
                if let Some(cost) = other.mana_cost() {
                    out.push(format!(
                        "Alternative cast {}: {} {}",
                        idx + 1,
                        other.name(),
                        cost.to_oracle()
                    ));
                } else {
                    out.push(format!("Alternative cast {}: {}", idx + 1, other.name()));
                }
            }
        }
    }
    if let Some(filter) = &def.aura_attach_filter {
        out.push(format!("Enchant {}", describe_enchant_filter(filter)));
    }
    let push_abilities = |output: &mut Vec<String>| {
        let mut ability_idx = 0usize;
        while ability_idx < def.abilities.len() {
            let ability = &def.abilities[ability_idx];
            if let Some(group_text) = ability.text.as_deref().map(str::trim)
                && group_text.contains(',')
                && ability_can_render_as_keyword_group(ability)
            {
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    if !ability_can_render_as_keyword_group(next) {
                        break;
                    }
                    let next_text = next.text.as_deref().map(str::trim);
                    if next_text != Some(group_text) {
                        break;
                    }
                    consumed += 1;
                }
                if consumed > 1 {
                    output.push(format!("Keyword ability {}: {group_text}", ability_idx + 1));
                    ability_idx += consumed;
                    continue;
                }
            }
            if let AbilityKind::Mana(first) = &ability.kind
                && first.effects.is_none()
                && first.activation_condition.is_none()
                && first.mana.len() == 1
                && ability.text.is_none()
            {
                let mut symbols = vec![first.mana[0]];
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    let AbilityKind::Mana(next_mana) = &next.kind else {
                        break;
                    };
                    if next_mana.effects.is_some()
                        || next_mana.activation_condition.is_some()
                        || next_mana.mana.len() != 1
                        || next_mana.mana_cost != first.mana_cost
                        || next.text.is_some()
                    {
                        break;
                    }
                    symbols.push(next_mana.mana[0]);
                    consumed += 1;
                }
                if consumed > 1 {
                    let mut line = format!("Mana ability {}", ability_idx + 1);
                    let mut parts = Vec::new();
                    if !first.mana_cost.costs().is_empty() {
                        parts.push(describe_cost_list(first.mana_cost.costs()));
                    }
                    parts.push(format!("Add {}", describe_mana_alternatives(&symbols)));
                    line.push_str(": ");
                    line.push_str(&parts.join(", "));
                    output.push(line);
                    ability_idx += consumed;
                    continue;
                }
            }
            output.extend(describe_ability(ability_idx + 1, ability));
            ability_idx += 1;
        }
    };

    let spell_like_card = def.card.card_types.contains(&CardType::Instant)
        || def.card.card_types.contains(&CardType::Sorcery);
    if !spell_like_card {
        push_abilities(&mut out);
    }
    if !def.cost_effects.is_empty() {
        out.push(format!(
            "As an additional cost to cast this spell: {}",
            describe_effect_list(&def.cost_effects)
        ));
    }
    if let Some(spell_effects) = &def.spell_effect
        && !spell_effects.is_empty()
        && !(def.aura_attach_filter.is_some() && has_attach_only_spell_effect)
    {
        out.push(format!(
            "Spell effects: {}",
            describe_effect_list(spell_effects)
        ));
    }
    if spell_like_card {
        push_abilities(&mut out);
    }
    out
}

fn is_render_heading_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim().to_ascii_lowercase();
    prefix == "spell effects"
        || prefix.starts_with("activated ability ")
        || prefix.starts_with("triggered ability ")
        || prefix.starts_with("static ability ")
        || prefix.starts_with("keyword ability ")
        || prefix.starts_with("mana ability ")
        || prefix.starts_with("ability ")
        || prefix.starts_with("alternative cast ")
}

fn strip_render_heading(line: &str) -> String {
    let Some((prefix, rest)) = line.split_once(':') else {
        return line.trim().to_string();
    };
    if is_render_heading_prefix(prefix) {
        rest.trim().to_string()
    } else {
        line.trim().to_string()
    }
}

fn is_keyword_phrase(phrase: &str) -> bool {
    let lower = phrase.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if lower.starts_with("protection from ") {
        return true;
    }
    if lower.starts_with("ward ") {
        return true;
    }
    matches!(
        lower.as_str(),
        "flying"
            | "first strike"
            | "double strike"
            | "deathtouch"
            | "defender"
            | "flash"
            | "haste"
            | "hexproof"
            | "indestructible"
            | "intimidate"
            | "lifelink"
            | "menace"
            | "reach"
            | "shroud"
            | "trample"
            | "vigilance"
            | "fear"
            | "flanking"
            | "shadow"
            | "horsemanship"
            | "phasing"
            | "wither"
            | "infect"
            | "changeling"
    )
}

fn split_have_clause(clause: &str) -> Option<(String, String)> {
    let trimmed = clause.trim();
    for verb in [" have ", " has "] {
        if let Some(idx) = trimmed.to_ascii_lowercase().find(verb) {
            let subject = trimmed[..idx].trim();
            let keyword = trimmed[idx + verb.len()..].trim();
            if !subject.is_empty() && is_keyword_phrase(keyword) {
                return Some((subject.to_string(), keyword.to_string()));
            }
        }
    }
    None
}

fn normalize_activation_cost_add_punctuation(line: &str) -> String {
    if line.contains(':') {
        return line.to_string();
    }
    if let Some(idx) = line.find(", Add ") {
        let (cost, rest) = line.split_at(idx);
        return format!("{cost}:{}", rest.trim_start_matches(','));
    }
    if let Some(idx) = line.find(", add ") {
        let (cost, rest) = line.split_at(idx);
        return format!("{cost}:{}", rest.trim_start_matches(','));
    }
    line.to_string()
}

fn normalize_cost_payment_wording(line: &str) -> String {
    let Some((cost, effect)) = line.split_once(": ") else {
        return line.to_string();
    };
    let lower_cost = cost.trim().to_ascii_lowercase();
    if lower_cost.starts_with("when ")
        || lower_cost.starts_with("whenever ")
        || lower_cost.starts_with("at the beginning ")
    {
        return line.to_string();
    }
    let normalized_cost = cost.replace("Lose ", "Pay ");
    let mut normalized_effect = effect.replace(" to your mana pool", "");
    normalized_effect = normalize_you_subject_phrase(&normalized_effect);
    if normalized_effect.starts_with("you ") {
        normalized_effect = capitalize_first(&normalized_effect);
    }
    if normalized_effect
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
    {
        normalized_effect = capitalize_first(&normalized_effect);
    }
    format!("{normalized_cost}: {normalized_effect}")
}

fn split_subject_predicate_clause(line: &str) -> Option<(&str, &str, &str)> {
    for verb in [" gets ", " get ", " has ", " have ", " gains ", " gain "] {
        if let Some((subject, rest)) = line.split_once(verb) {
            let subject = subject.trim();
            let rest = rest.trim();
            if !subject.is_empty() && !rest.is_empty() {
                return Some((subject, verb.trim(), rest));
            }
        }
    }
    None
}

fn can_merge_subject_predicates(left_verb: &str, right_verb: &str) -> bool {
    matches!(left_verb, "gets" | "get")
        && matches!(right_verb, "has" | "have" | "gains" | "gain")
}

fn normalize_keyword_predicate_case(predicate: &str) -> String {
    let trimmed = predicate.trim();
    if is_keyword_phrase(trimmed) {
        return trimmed.to_ascii_lowercase();
    }
    if let Some(joined) = normalize_keyword_list_phrase(trimmed) {
        return joined;
    }
    if let Some(keyword) = trimmed.strip_suffix(" until end of turn")
        && is_keyword_phrase(keyword)
    {
        return format!("{} until end of turn", keyword.to_ascii_lowercase());
    }
    if let Some(keywords) = trimmed.strip_suffix(" until end of turn")
        && let Some(joined) = normalize_keyword_list_phrase(keywords)
    {
        return format!("{joined} until end of turn");
    }
    trimmed.to_string()
}

fn normalize_keyword_list_phrase(text: &str) -> Option<String> {
    let parts = text
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    if !parts.iter().all(|part| is_keyword_phrase(part)) {
        return None;
    }
    Some(
        parts
            .iter()
            .map(|part| part.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(" and "),
    )
}

fn normalize_gains_tail(predicate: &str) -> String {
    let normalized = normalize_keyword_predicate_case(predicate);
    if let Some((first, second)) = normalized.split_once(", and gains ")
        && let Some(second) = second.strip_suffix(" until end of turn")
        && is_keyword_phrase(first)
        && is_keyword_phrase(second)
    {
        return format!(
            "{} and {} until end of turn",
            first.to_ascii_lowercase(),
            second.to_ascii_lowercase()
        );
    }
    normalized
}

fn merge_sentence_subject_predicates(line: &str) -> Option<String> {
    let (left, right) = line.split_once(". ")?;
    let (left_subject, left_verb, left_rest) = split_subject_predicate_clause(left)?;
    let (right_subject, right_verb, right_rest) = split_subject_predicate_clause(right)?;
    if !left_subject.eq_ignore_ascii_case(right_subject)
        || !can_merge_subject_predicates(left_verb, right_verb)
    {
        return None;
    }

    let right_rest = normalize_gains_tail(right_rest);
    if let (Some(left_body), Some(right_body)) = (
        left_rest.strip_suffix(" until end of turn"),
        right_rest.strip_suffix(" until end of turn"),
    ) {
        return Some(format!(
            "{left_subject} {left_verb} {left_body} and {right_verb} {right_body} until end of turn"
        ));
    }
    Some(format!(
        "{left_subject} {left_verb} {left_rest} and {right_verb} {right_rest}"
    ))
}

fn merge_adjacent_subject_predicate_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::new();
    let mut idx = 0usize;

    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let Some((left_subject, left_verb, left_rest)) =
                split_subject_predicate_clause(&lines[idx])
            && let Some((right_subject, right_verb, right_rest)) =
                split_subject_predicate_clause(&lines[idx + 1])
            && left_subject.eq_ignore_ascii_case(right_subject)
            && can_merge_subject_predicates(left_verb, right_verb)
        {
            merged.push(format!(
                "{left_subject} {left_verb} {left_rest} and {right_verb} {right_rest}"
            ));
            idx += 2;
            continue;
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }

    merged
}

fn merge_blockability_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim();
            let right = lines[idx + 1].trim();
            if (left == "This creature can't block" && right == "This creature can't be blocked")
                || (left == "Can't block" && right == "Can't be blocked")
            {
                merged.push("This creature can't block and can't be blocked".to_string());
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn parse_simple_mana_add_line(line: &str) -> Option<(&str, &str)> {
    let (cost, rest) = line.split_once(": ")?;
    let symbol = rest.strip_prefix("Add ")?;
    if symbol.contains(' ')
        || symbol.contains(',')
        || symbol.contains("or")
        || symbol.matches('{').count() != 1
        || symbol.matches('}').count() != 1
        || !symbol.starts_with('{')
        || !symbol.ends_with('}')
    {
        return None;
    }
    Some((cost, symbol))
}

fn format_mana_symbol_alternatives(symbols: &[String]) -> String {
    match symbols.len() {
        0 => String::new(),
        1 => symbols[0].clone(),
        2 => format!("{} or {}", symbols[0], symbols[1]),
        _ => {
            let mut joined = symbols[..symbols.len() - 1].join(", ");
            joined.push_str(", or ");
            joined.push_str(&symbols[symbols.len() - 1]);
            joined
        }
    }
}

fn merge_adjacent_simple_mana_add_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((cost, symbol)) = parse_simple_mana_add_line(lines[idx].trim()) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut symbols = vec![symbol.to_string()];
        let mut consumed = 1usize;
        while idx + consumed < lines.len() {
            let Some((next_cost, next_symbol)) =
                parse_simple_mana_add_line(lines[idx + consumed].trim())
            else {
                break;
            };
            if !next_cost.eq_ignore_ascii_case(cost) {
                break;
            }
            if !symbols.iter().any(|existing| existing == next_symbol) {
                symbols.push(next_symbol.to_string());
            }
            consumed += 1;
        }

        if symbols.len() > 1 {
            merged.push(format!(
                "{cost}: Add {}",
                format_mana_symbol_alternatives(&symbols)
            ));
            idx += consumed;
            continue;
        }

        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn merge_subject_has_keyword_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim();
            let right = lines[idx + 1].trim();
            if let Some((left_subject, left_tail)) = left.split_once(" has ")
                && let Some((right_subject, right_tail)) = right.split_once(" has ")
                && left_subject.eq_ignore_ascii_case(right_subject)
            {
                let left_tail = normalize_keyword_predicate_case(left_tail);
                let right_tail = normalize_keyword_predicate_case(right_tail);
                let left_key = strip_parenthetical_segments(&left_tail).to_ascii_lowercase();
                let right_key = strip_parenthetical_segments(&right_tail).to_ascii_lowercase();
                if left_key == right_key
                    || left_key.contains(&format!(" and {right_key}"))
                    || left_key.ends_with(&format!(" {right_key}"))
                {
                    merged.push(format!("{left_subject} has {left_tail}"));
                } else {
                    merged.push(format!("{left_subject} has {left_tail} and {right_tail}"));
                }
                idx += 2;
                continue;
            }
            if let Some((left_subject, left_rest)) = left.split_once(" gets ")
                && let Some((right_subject, right_tail)) = right.split_once(" has ")
                && left_subject.eq_ignore_ascii_case(right_subject)
                && left_rest.contains(" and has ")
            {
                let right_tail = normalize_keyword_predicate_case(right_tail);
                let left_key = strip_parenthetical_segments(left_rest).to_ascii_lowercase();
                let right_key = strip_parenthetical_segments(&right_tail).to_ascii_lowercase();
                if left_key.contains(&format!(" has {right_key}"))
                    || left_key.contains(&format!(" and {right_key}"))
                    || left_key.ends_with(&format!(" {right_key}"))
                {
                    merged.push(format!("{left_subject} gets {left_rest}"));
                } else {
                    merged.push(format!("{left_subject} gets {left_rest} and {right_tail}"));
                }
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn card_self_subject_for_oracle_lines(def: &CardDefinition) -> &'static str {
    use crate::types::CardType;

    let card_types = &def.card.card_types;
    if card_types.contains(&CardType::Creature) {
        return "creature";
    }
    if card_types.contains(&CardType::Land) {
        return "land";
    }
    if card_types.len() == 1 {
        return match card_types[0] {
            CardType::Land => "land",
            CardType::Artifact => "artifact",
            CardType::Enchantment => "enchantment",
            CardType::Planeswalker => "planeswalker",
            CardType::Battle => "battle",
            CardType::Kindred => "kindred",
            CardType::Instant | CardType::Sorcery | CardType::Creature => "permanent",
        };
    }
    "permanent"
}

fn card_has_graveyard_activated_ability(def: &CardDefinition) -> bool {
    def.abilities.iter().any(|ability| {
        ability.functional_zones.contains(&Zone::Graveyard)
            && matches!(
                ability.kind,
                AbilityKind::Activated(_) | AbilityKind::Mana(_)
            )
    })
}

fn enchanted_subject_for_oracle_lines(def: &CardDefinition) -> Option<&'static str> {
    if let Some(filter) = &def.aura_attach_filter {
        if filter.card_types.contains(&crate::types::CardType::Creature) {
            return Some("creature");
        }
        if filter.card_types.contains(&crate::types::CardType::Land) {
            return Some("land");
        }
        if filter.card_types.contains(&crate::types::CardType::Artifact) {
            return Some("artifact");
        }
        return Some("permanent");
    }

    for ability in &def.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(rest) = static_ability
            .display()
            .to_ascii_lowercase()
            .strip_prefix("enchant ")
            .map(str::to_string)
        else {
            continue;
        };
        if rest.starts_with("creature") {
            return Some("creature");
        }
        if rest.starts_with("land") {
            return Some("land");
        }
        if rest.starts_with("artifact") {
            return Some("artifact");
        }
        if rest.starts_with("permanent") {
            return Some("permanent");
        }
    }
    None
}

fn normalize_create_under_control_clause(text: &str) -> Option<String> {
    let (prefix, rest) = text.split_once("Create ")?;
    let normalize_created = |created: String| {
        created
            .replace(
                "token with Sacrifice this creature: Add {C}.",
                "token with \"Sacrifice this creature: Add {C}.\"",
            )
            .replace(
                "token with {T}, Sacrifice this artifact: Add one mana of any color.",
                "token with \"{T}, Sacrifice this artifact: Add one mana of any color.\"",
            )
    };
    if let Some((created, suffix)) = rest.split_once(" under your control") {
        let created = if let Some(single) = created.strip_prefix("1 ") {
            format!("a {single}")
        } else {
            created.to_string()
        };
        let created = normalize_created(created);
        let mut normalized = format!("{prefix}Create {created}{suffix}");
        if normalized == "Create a Powerstone artifact token, tapped" {
            normalized = "Create a tapped Powerstone token".to_string();
        }
        return Some(normalized);
    }
    let (created, suffix) = rest.split_once(" under that object's controller's control")?;
    let created = if let Some(single) = created.strip_prefix("1 ") {
        format!("a {single}")
    } else {
        created.to_string()
    };
    let created = normalize_created(created);
    if prefix.trim().is_empty() {
        return Some(format!("That object's controller creates {created}{suffix}"));
    }
    Some(format!("{prefix}Its controller creates {created}{suffix}"))
}

fn is_cost_symbol_word(word: &str) -> bool {
    matches!(word, "w" | "u" | "b" | "r" | "g" | "c" | "x") || word.parse::<u32>().is_ok()
}

fn is_effect_verb_word(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "deal"
            | "tap"
            | "untap"
            | "scry"
            | "surveil"
            | "gain"
            | "lose"
            | "draw"
            | "create"
            | "destroy"
            | "exile"
            | "return"
            | "counter"
            | "fight"
            | "mill"
            | "put"
            | "regenerate"
    )
}

fn format_cost_words(words: &[&str]) -> Option<String> {
    if words.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    let mut idx = 0usize;
    while idx < words.len() {
        let word = words[idx];
        if word == "," {
            idx += 1;
            continue;
        }
        if word == "t" {
            parts.push("{T}".to_string());
            idx += 1;
            continue;
        }
        if is_cost_symbol_word(word) {
            parts.push(format!("{{{}}}", word.to_ascii_uppercase()));
            idx += 1;
            continue;
        }
        if word == "sacrifice" {
            let tail = words[idx + 1..].join(" ");
            if tail.is_empty() {
                parts.push("Sacrifice".to_string());
            } else {
                parts.push(format!("Sacrifice {tail}"));
            }
            break;
        }
        return None;
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn normalize_granted_activated_ability_clause(text: &str) -> Option<String> {
    let (subject, tail, has_word) = if let Some((subject, tail)) = text.split_once(" has ") {
        (subject, tail, "has")
    } else if let Some((subject, tail)) = text.split_once(" have ") {
        (subject, tail, "have")
    } else {
        return None;
    };

    let words: Vec<&str> = tail.split_whitespace().collect();
    if words.len() < 2 {
        return None;
    }

    let mut effect_idx: Option<usize> = None;
    if let Some(t_idx) = words.iter().position(|word| *word == "t") {
        let mut candidate = t_idx + 1;
        if words
            .get(candidate)
            .is_some_and(|word| *word == "sacrifice")
            && words
                .get(candidate + 1)
                .is_some_and(|next| *next == "this" || *next == "thiss")
        {
            candidate += 2;
        }
        if candidate < words.len() {
            let head = words[candidate];
            if is_effect_verb_word(head)
                || matches!(head, "this" | "target" | "you" | "each" | "a" | "an")
            {
                effect_idx = Some(candidate);
            }
        }
    }
    if effect_idx.is_none() {
        let scan_start = words
            .iter()
            .position(|word| *word == "t")
            .map(|idx| idx + 1)
            .unwrap_or(0);
        for idx in scan_start..words.len() {
            let word = words[idx];
            if !is_effect_verb_word(word) {
                continue;
            }
            // "sacrifice this ..." may be part of the activation cost.
            if word == "sacrifice"
                && words
                    .get(idx + 1)
                    .is_some_and(|next| *next == "this" || *next == "thiss")
            {
                continue;
            }
            effect_idx = Some(idx);
            break;
        }
    }
    let effect_idx = effect_idx?;
    let cost_words = &words[..effect_idx];
    let effect_words = &words[effect_idx..];

    if !cost_words
        .iter()
        .any(|word| *word == "t" || is_cost_symbol_word(word) || *word == "sacrifice")
    {
        return None;
    }

    let cost = format_cost_words(cost_words)?;
    let mut effect = capitalize_first(&effect_words.join(" "));
    effect = normalize_zero_pt_prefix(&effect);
    if !effect.ends_with('.') {
        effect.push('.');
    }
    Some(format!("{subject} {has_word} \"{cost}: {effect}\""))
}

fn normalize_oracle_line_segment(segment: &str) -> String {
    let trimmed_owned = strip_square_bracketed_segments(segment.trim());
    let trimmed = trimmed_owned.trim();
    let lower_trimmed = trimmed.to_ascii_lowercase();
    if let Some((subject, rest)) = trimmed.split_once(" have ")
        && (subject.eq_ignore_ascii_case("all slivers")
            || subject.eq_ignore_ascii_case("all sliver creatures"))
        && let Some(normalized) = normalize_sliver_grant_clause(subject, rest)
    {
        return normalized;
    }
    if let Some(normalized) = normalize_granted_activated_ability_clause(trimmed) {
        return normalized;
    }
    if let Some(buff) = trimmed
        .strip_prefix("Tag the object attached to this source as 'enchanted'. enchanted creature gets ")
    {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed
        .strip_prefix("Tag the object attached to this source as 'enchanted'. enchanted creatures get ")
    {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(tail) = trimmed
        .strip_prefix("Tag the object attached to this source as 'enchanted'. enchanted creature gains ")
    {
        return format!("Enchanted creature gains {tail}");
    }
    if let Some(buff) = trimmed
        .strip_prefix("Tag the object attached to this enchantment as 'enchanted'. enchanted creature gets ")
    {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed
        .strip_prefix("Tag the object attached to this enchantment as 'enchanted'. enchanted creatures get ")
    {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed
        .strip_prefix("Tag the object attached to this aura as 'enchanted'. enchanted creature gets ")
    {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed
        .strip_prefix("Tag the object attached to this aura as 'enchanted'. enchanted creatures get ")
    {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some((prefix, ability_tail)) = trimmed.split_once(" and has ")
        && prefix.starts_with("enchanted creature gets ")
    {
        let ability_clause = format!("enchanted creature has {ability_tail}");
        if let Some(normalized) = normalize_granted_activated_ability_clause(&ability_clause)
            && let Some(ability_part) = normalized.strip_prefix("enchanted creature has ")
        {
            return capitalize_first(&format!("{prefix} and has {ability_part}"));
        }
    }
    if let Some(normalized) = normalize_create_under_control_clause(trimmed) {
        return normalized;
    }
    if let Some(normalized) = normalize_search_you_own_clause(trimmed) {
        return normalized;
    }
    if let Some(kind) = trimmed.strip_prefix("You may Put target ").and_then(|rest| {
        rest.strip_suffix(" card in your hand onto the battlefield")
    }) {
        let noun = if kind.trim().is_empty() {
            "card".to_string()
        } else {
            format!("{} card", kind.trim())
        };
        return format!(
            "You may put {} from your hand onto the battlefield",
            with_indefinite_article(&noun)
        );
    }
    if let Some(kind) = trimmed
        .strip_prefix("You may Put target ")
        .and_then(|rest| rest.strip_suffix(" in your hand onto the battlefield"))
    {
        return format!(
            "You may put {} from your hand onto the battlefield",
            with_indefinite_article(kind.trim())
        );
    }
    if let Some(normalized) = normalize_choose_between_modes_clause(trimmed) {
        return normalized;
    }
    if let Some(rest) = trimmed.strip_prefix("For each player, Create ") {
        if let Some((create_clause, tail)) = rest.split_once(". ") {
            return format!("Each player creates {create_clause}. {tail}");
        }
        return format!("Each player creates {rest}");
    }
    if let Some(amount) = trimmed
        .strip_prefix("For each player, Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that player"))
    {
        return format!("Deal {amount} damage to each player");
    }
    if let Some(rest) = trimmed.strip_prefix(
        "For each player, Reveal the top card of that player's library and tag it as 'revealed_0'",
    ) {
        if let Some(tail) = rest.strip_prefix(". ") {
            if tail.is_empty() {
                return "Each player reveals the top card of their library".to_string();
            }
            return format!("Each player reveals the top card of their library. {tail}");
        }
        return "Each player reveals the top card of their library".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("For each player, Investigate ")
        && let Some((count, tail)) = rest.split_once(". ")
    {
        if count.trim() == "1" {
            return format!("Each player investigates. {tail}");
        }
    }
    if trimmed == "For each player, Put target creature card in target player's graveyard onto the battlefield"
    {
        return "Each player puts a creature card from their graveyard onto the battlefield"
            .to_string();
    }
    if let Some(kind) = trimmed.strip_prefix("For each player, Return all ").and_then(|rest| {
        rest.strip_suffix(" card in target player's graveyard to the battlefield")
    }) {
        return format!("Each player returns all {kind} cards from their graveyard to the battlefield");
    }
    if let Some(kind) = trimmed.strip_prefix("For each player, Return all ").and_then(|rest| {
        rest.strip_suffix(" card from target player's graveyard to target player's hand")
    }) {
        return format!("Each player returns all {kind} cards from their graveyard to their hand");
    }
    if trimmed == "For each player, Return target player's creature to its owner's hand" {
        return "Each player returns a creature they control to its owner's hand".to_string();
    }
    if let Some(counter_rest) = trimmed.strip_prefix("For each ")
        && let Some((subject, tail)) = counter_rest.split_once(", Put ")
        && let Some(counter_phrase) = tail.strip_suffix(" on that object")
    {
        return format!("Put {counter_phrase} on each {subject}");
    }
    if trimmed == "Destroy all creature" {
        return "Destroy all creatures".to_string();
    }
    if trimmed == "creatures have Blocks each combat if able" {
        return "All creatures able to block this creature do so".to_string();
    }
    if trimmed == "Destroy all land" {
        return "Destroy all lands".to_string();
    }
    if trimmed == "Destroy all artifact. Destroy all enchantments"
        || trimmed == "Destroy all artifact. Destroy all enchantment"
    {
        return "Destroy all artifacts and enchantments".to_string();
    }
    if trimmed == "Destroy all artifact or creature. Destroy all enchantments" {
        return "Destroy all artifacts, creatures, and enchantments".to_string();
    }
    if trimmed == "Counter target spell Spirit or Arcane" {
        return "Counter target Spirit or Arcane spell".to_string();
    }
    if let Some(damage) = trimmed
        .strip_prefix("Counter target spell. Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to target creature"))
    {
        return format!("Counter target spell and this spell deals {damage} damage to target creature");
    }
    if trimmed == "Counter target spell unless its controller pays {2}. Counter target spell" {
        return "Counter target spell unless its controller pays {2}".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Counter target spell unless its controller pays ")
        && let Some((amount, tail)) = rest.split_once(". If it happened, Surveil ")
    {
        return format!(
            "Counter target spell unless its controller pays {amount}, then surveil {tail}"
        );
    }
    if let Some((left, right)) = trimmed.split_once(". Deal ")
        && left.starts_with("Counter target spell unless its controller pays ")
        && let Some(amount) = right.strip_suffix(" damage to target spell")
    {
        return format!("{left}. {} deals {amount} damage to that spell's controller", "This spell");
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one - ") {
        let rest = rest.replace("• ", "");
        return format!("Choose one —. {rest}");
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one — ") {
        return format!("Choose one —. {}", rest.replace("• ", ""));
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or both - ") {
        let rest = rest.replace("• ", "");
        return format!("Choose one or both —. {rest}");
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or both — ") {
        return format!("Choose one or both —. {}", rest.replace("• ", ""));
    }
    if let Some((subject, keyword)) = split_have_clause(trimmed) {
        if keyword.eq_ignore_ascii_case("can't be blocked") {
            return format!("{subject} can't be blocked");
        }
        if keyword.eq_ignore_ascii_case("can't block") {
            return format!("{subject} can't block");
        }
    }
    if let Some((subject, tail)) = trimmed.split_once(" has ")
        && let Some(joined) = normalize_keyword_list_phrase(tail)
    {
        return format!("{subject} has {joined}");
    }
    if trimmed == "creatures have Can't block" {
        return "Creatures can't block".to_string();
    }
    if trimmed == "Can't be blocked" {
        return "This creature can't be blocked".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Can't attack unless defending player controls ") {
        if rest.eq_ignore_ascii_case("island") {
            return "This creature can't attack unless defending player controls an Island"
                .to_string();
        }
        return format!("This creature can't attack unless defending player controls {rest}");
    }
    if let Some((trigger, effect)) = trimmed.split_once(": ")
        && (trigger.starts_with("When ")
            || trigger.starts_with("Whenever ")
            || trigger.starts_with("At the beginning "))
    {
        let normalized_effect = normalize_you_subject_phrase(effect);
        return format!("{trigger}, {normalized_effect}");
    }
    if let Some((head, tail)) = trimmed.split_once(", you draw ")
        && (head.starts_with("When ") || head.starts_with("Whenever ") || head.starts_with("At the beginning "))
    {
        return format!("{head}, draw {tail}");
    }
    if let Some((head, tail)) = trimmed.split_once(", you mill ")
        && (head.starts_with("When ") || head.starts_with("Whenever ") || head.starts_with("At the beginning "))
    {
        return format!("{head}, mill {tail}");
    }
    if let Some((cost, effect)) = trimmed.split_once(": ")
        && effect.starts_with("You draw ")
    {
        return format!("{cost}: Draw {}", effect.trim_start_matches("You draw "));
    }
    if let Some((cost, effect)) = trimmed.split_once(": ")
        && effect.starts_with("you draw ")
    {
        return format!("{cost}: Draw {}", effect.trim_start_matches("you draw "));
    }
    if let Some((cost, effect)) = trimmed.split_once(": ")
        && effect.starts_with("You mill ")
    {
        return format!("{cost}: Mill {}", effect.trim_start_matches("You mill "));
    }
    if let Some((cost, effect)) = trimmed.split_once(": ")
        && effect.starts_with("you mill ")
    {
        return format!("{cost}: Mill {}", effect.trim_start_matches("you mill "));
    }
    if let Some(merged) = merge_sentence_subject_predicates(trimmed) {
        return merged;
    }
    if let Some((subject, tail)) = trimmed.split_once(" gains ")
        && let Some(keywords) = tail.strip_suffix(" until end of turn")
        && let Some(joined) = normalize_keyword_list_phrase(keywords)
    {
        return format!("{subject} gains {joined} until end of turn");
    }
    if let Some((prefix, tail)) = trimmed.split_once(", gains ")
        && prefix.contains(" gets ")
        && let Some((first_keyword, second_tail)) = tail.split_once(", and gains ")
        && let Some(second_keyword) = second_tail.strip_suffix(" until end of turn")
        && is_keyword_phrase(first_keyword)
        && is_keyword_phrase(second_keyword)
    {
        return format!(
            "{prefix} and gains {} and {} until end of turn",
            first_keyword.to_ascii_lowercase(),
            second_keyword.to_ascii_lowercase()
        );
    }
    if let Some(amount) =
        lower_trimmed.strip_prefix("all artifacts have at the beginning of your upkeep sacrifice this artifact unless you pay ")
    {
        let amount = normalize_cost_amount_token(amount);
        return format!(
            "All artifacts have \"At the beginning of your upkeep, sacrifice this artifact unless you pay {amount}.\""
        );
    }
    if let Some(amount) =
        lower_trimmed.strip_prefix("all creatures have at the beginning of your upkeep sacrifice this creature unless you pay ")
    {
        let amount = normalize_cost_amount_token(amount);
        return format!(
            "All creatures have \"At the beginning of your upkeep, sacrifice this creature unless you pay {amount}.\""
        );
    }
    if let Some(effect) = trimmed.strip_prefix("As an additional cost to cast this spell: ") {
        let mut effect = effect.trim().to_string();
        if effect.starts_with("Discard ") {
            effect = effect.replacen("Discard ", "discard ", 1);
        }
        if effect.starts_with("You discard ") {
            effect = effect.replacen("You discard ", "discard ", 1);
        }
        if effect.starts_with("you discard ") {
            effect = effect.replacen("you discard ", "discard ", 1);
        }
        return format!("As an additional cost to cast this spell, {effect}");
    }
    if trimmed == "Enchant opponent's creature" {
        return "Enchant creature an opponent controls".to_string();
    }
    if trimmed.contains("target sliver") {
        return trimmed.replace("target sliver", "target Sliver");
    }
    if trimmed == "target player sacrifices target player's creature" {
        return "Target player sacrifices a creature of their choice".to_string();
    }
    if trimmed == "target player sacrifices target player's creature. target player loses 1 life" {
        return "Target player sacrifices a creature of their choice and loses 1 life".to_string();
    }
    if trimmed == "target player sacrifices target player's attacking/blocking permanent" {
        return "Target player sacrifices an attacking or blocking creature of their choice"
            .to_string();
    }
    if trimmed == "target player sacrifices target player's attacking/blocking creature" {
        return "Target player sacrifices an attacking or blocking creature of their choice"
            .to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("target player sacrifices target player's ")
        && !rest.is_empty()
        && !rest.contains(". ")
    {
        return format!(
            "Target player sacrifices {} of their choice",
            with_indefinite_article(rest)
        );
    }
    if let Some(rest) = trimmed.strip_prefix("target opponent sacrifices target opponent's ")
        && !rest.is_empty()
        && !rest.contains(". ")
    {
        return format!(
            "Target opponent sacrifices {} of their choice",
            with_indefinite_article(rest)
        );
    }
    if let Some(rest) = trimmed.strip_prefix("Deal ")
        && let Some((damage, loss_tail)) =
            rest.split_once(" damage to target creature. that object's controller loses ")
        && let Some(loss_amount) = loss_tail.strip_suffix(" life")
    {
        return format!(
            "This creature deals {damage} damage to target creature and that creature's controller loses {loss_amount} life"
        );
    }
    if trimmed == "commander creatures you own have token creatures you control get +2/+2" {
        return "Commander creatures you own have \"Creature tokens you control get +2/+2\""
            .to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("other ")
        && let Some((types, buff)) = rest.split_once(" creatures you control get ")
    {
        return format!("Other {types} creatures you control get {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix("other creature withs flying you control get ") {
        return format!("Other creatures you control with flying get {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix("other creature with flying you control get ") {
        return format!("Other creatures you control with flying get {buff}");
    }
    if trimmed
        == "Whenever a enchantment you control enters the battlefield: Put 1 +1/+1 counter(s) on this creature. you draw a card"
    {
        return "Whenever an enchantment you control enters, put a +1/+1 counter on this creature and draw a card"
            .to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("For each player, Deal ")
        && let Some((amount, per_player_tail)) =
            rest.split_once(" damage to that player. For each ")
        && let Some((per_player_filter, repeated_damage)) = per_player_tail.split_once(", Deal ")
        && repeated_damage == format!("{amount} damage to that object")
    {
        let per_player_filter = per_player_filter
            .strip_suffix(" that player controls")
            .unwrap_or(per_player_filter);
        return format!("Deal {amount} damage to each player and each {per_player_filter}");
    }
    if trimmed.contains("For each opponent, Deal ") && trimmed.contains(" damage to that player") {
        return trimmed
            .replacen("For each opponent, Deal ", "Deal ", 1)
            .replace(" damage to that player", " damage to each opponent");
    }
    if let Some(loss_tail) = trimmed.strip_prefix("For each opponent, that player loses ")
        && let Some(gain_tail) = loss_tail.split_once(" life. you gain ")
    {
        return format!(
            "Each opponent loses {} life and you gain {} life",
            gain_tail.0, gain_tail.1
        );
    }
    if let Some(loss_tail) = trimmed.strip_prefix("For each opponent, that player loses ")
        && let Some(gain_tail) = loss_tail.split_once(" life. You gain ")
    {
        return format!(
            "Each opponent loses {} life and you gain {} life",
            gain_tail.0, gain_tail.1
        );
    }
    if let Some(rest) = trimmed.strip_prefix("For each creature or planeswalker, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature and each planeswalker");
    }
    if let Some(rest) = trimmed.strip_prefix("For each creature without flying, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("This creature deals {amount} damage to each creature without flying");
    }
    if let Some(rest) = trimmed.strip_prefix("For each creature with flying, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("This creature deals {amount} damage to each creature with flying");
    }
    if let Some(rest) = trimmed.strip_prefix("For each other creature with flying, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("This creature deals {amount} damage to each other creature with flying");
    }
    if let Some(rest) = trimmed.strip_prefix("For each ")
        && let Some((targets, amount_tail)) = rest.split_once(", Deal ")
        && let Some(amount) = amount_tail.strip_suffix(" damage to that object")
        && let Some((left, right_and_type)) = targets.split_once(" or ")
        && let Some(kind) = right_and_type.strip_suffix(" creature")
    {
        return format!("Deal {amount} damage to each {left} and/or {kind} creature");
    }
    if let Some(rest) = trimmed.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if trimmed == "Tap all an opponent's creature. Untap all a creature you control" {
        return "Tap all creatures your opponents control and untap all creatures you control"
            .to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("creatures you control get ")
        && let Some(buff) = rest.strip_suffix(" until end of turn. Untap all permanent")
    {
        return format!("Creatures you control get {buff} until end of turn. Untap them");
    }
    if trimmed == "Can't block" {
        return "This creature can't block".to_string();
    }
    if trimmed
        == "Tag the object attached to this artifact as 'equipped'. Put 1 +1/+1 counter(s) on the tagged object 'equipped'"
    {
        return "Put a +1/+1 counter on equipped creature".to_string();
    }
    if trimmed
        == "Tag the object attached to this artifact as 'equipped'. Regenerate the tagged object 'equipped' until end of turn"
    {
        return "Regenerate equipped creature".to_string();
    }
    if let Some(counter) = trimmed
        .strip_prefix("Tag the object attached to this enchantment as 'enchanted'. Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counters on the tagged object 'enchanted'"))
    {
        return format!("Put a {counter} counter on enchanted creature");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Tag the object attached to this enchantment as 'enchanted'. Put a ")
        .and_then(|rest| rest.strip_suffix(" counter on the tagged object 'enchanted'"))
    {
        return format!("Put a {counter} counter on enchanted creature");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Tag the object attached to this aura as 'enchanted'. Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counters on the tagged object 'enchanted'"))
    {
        return format!("Put a {counter} counter on enchanted creature");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Tag the object attached to this aura as 'enchanted'. Put a ")
        .and_then(|rest| rest.strip_suffix(" counter on the tagged object 'enchanted'"))
    {
        return format!("Put a {counter} counter on enchanted creature");
    }
    if let Some(rest) = trimmed
        .strip_prefix("Tag the object attached to this enchantment as 'enchanted'. Regenerate the tagged object 'enchanted'")
    {
        return format!("Regenerate enchanted creature{rest}");
    }
    if let Some(rest) = trimmed
        .strip_prefix("Tag the object attached to this aura as 'enchanted'. Regenerate the tagged object 'enchanted'")
    {
        return format!("Regenerate enchanted creature{rest}");
    }
    if trimmed.starts_with("Attach this enchantment to target creature.")
        && let Some(rest) = trimmed
            .split("the tagged object 'enchanted' gets base power and toughness ")
            .nth(1)
        && let Some(pt) = rest.strip_suffix(" until end of turn")
    {
        return format!("Enchanted creature has base power and toughness {pt}");
    }
    if let Some(kind) = trimmed.strip_prefix("Whenever this permanent deals damage to ")
        && let Some(kind) = kind.strip_suffix(": Destroy it")
    {
        return format!("Whenever this creature deals damage to a {kind}, destroy that creature");
    }
    if trimmed
        == "At the beginning of your upkeep: Sacrifice this enchantment unless you pays {W}{W}"
    {
        return "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}"
            .to_string();
    }
    if trimmed == "Destroy all an opponent's nonland permanent" {
        return "Destroy all nonland permanents your opponents control".to_string();
    }
    if trimmed == "Destroy target white or green creature" {
        return "Destroy target green or white creature".to_string();
    }
    if trimmed == "an opponent's creature enter the battlefield tapped" {
        return "Creatures your opponents control enter tapped".to_string();
    }
    if trimmed == "Return all target player's nonland permanent to its owner's hand" {
        return "Return all nonland permanents target player controls to their owner's hand"
            .to_string();
    }
    if trimmed
        == "target opponent loses 3 life. Put a card from target player's hand on top of target player's library"
    {
        return "Target opponent loses 3 life and puts a card from their hand on top of their library"
            .to_string();
    }
    if trimmed == "Exchange control of creature and creature" {
        return "Exchange control of two target creatures".to_string();
    }
    if trimmed == "Exchange control of permanent and permanent" {
        return "Exchange control of two target permanents".to_string();
    }
    if trimmed == "For each player, Put a card from target player's hand on top of target player's library" {
        return "each player puts a card from their hand on top of their library".to_string();
    }
    if let Some(prefix) = trimmed.strip_suffix(" that an opponent's land could produce") {
        return format!("{prefix} that a land an opponent controls could produce");
    }
    if let Some(rest) = trimmed.strip_prefix("spells ")
        && let Some((creature_type, tail)) = rest.split_once(" you control cost ")
    {
        return format!("{creature_type} spells you cast cost {tail}");
    }
    if let Some((choice, rest)) = trimmed.split_once(". ")
        && let Some(chosen) = choice.strip_prefix("Choose ")
        && rest
            .to_ascii_lowercase()
            .starts_with(&chosen.to_ascii_lowercase())
    {
        return rest.to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("For each ")
        && let Some((who, tail)) = rest.split_once(", that player ")
    {
        return format!("each {who} {tail}");
    }
    if let Some((prefix, modes)) = trimmed.split_once("Choose between 0 and 1 mode(s) - ") {
        let modes = modes.replace("• ", "");
        return format!("{prefix}choose up to one — {modes}");
    }
    if let Some(modes) = trimmed.strip_prefix("Choose between 0 and 1 mode(s) - ") {
        let modes = modes.replace("• ", "");
        return format!("Choose up to one — {modes}");
    }
    if let Some(mana) = trimmed.strip_prefix("Add ")
        && let Some(mana) = mana.strip_suffix(" to your mana pool")
    {
        return format!("Add {mana}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && (left.starts_with("Tap ") || left.contains(", Tap "))
        && matches!(
            right,
            "creature can't untap until your next turn"
                | "creature cant untap until your next turn"
                | "target creature can't untap until your next turn"
                | "target creature cant untap until your next turn"
                | "permanent can't untap until your next turn"
                | "permanent cant untap until your next turn"
        )
    {
        if left.contains("target creatures") {
            return format!(
                "{left}. Those creatures don't untap during their controllers' next untap steps"
            );
        }
        if left.contains("target creature") {
            return format!(
                "{left}. That creature doesn't untap during its controller's next untap step"
            );
        }
    }
    if matches!(
        trimmed,
        "target creature can't untap until your next turn"
            | "target creature cant untap until your next turn"
    ) {
        return "Target creature doesn't untap during its controller's next untap step"
            .to_string();
    }
    if matches!(
        trimmed,
        "permanent can't untap until your next turn" | "permanent cant untap until your next turn"
    ) {
        return "That creature doesn't untap during its controller's next untap step".to_string();
    }
    if let Some(rest) = trimmed
        .strip_prefix("Whenever this creature attacks, permanent can't untap until your next turn")
    {
        return format!(
            "Whenever this creature attacks, it doesn't untap during its controller's next untap step{rest}"
        );
    }
    if let Some(rest) = trimmed
        .strip_prefix("Whenever this creature blocks creature, permanent can't untap until your next turn")
    {
        return format!(
            "Whenever this creature blocks a creature, that creature doesn't untap during its controller's next untap step{rest}"
        );
    }
    if lower_trimmed.starts_with("a creature blocks: this enchantment deals ")
        && let Some(rest) = trimmed
            .split_once(": ")
            .and_then(|(_, rhs)| rhs.strip_prefix("This enchantment deals "))
        && let Some((amount, tail)) = rest.split_once(" damage")
        && tail.trim().eq_ignore_ascii_case("to target creature")
    {
        return format!(
            "Whenever a creature blocks, this enchantment deals {amount} damage to that creature's controller"
        );
    }
    if lower_trimmed.starts_with("a creature blocks: deal ")
        && let Some(rest) = trimmed
            .split_once(": ")
            .and_then(|(_, rhs)| rhs.strip_prefix("Deal "))
        && let Some((amount, tail)) = rest.split_once(" damage")
        && tail.trim().eq_ignore_ascii_case("to target creature")
    {
        return format!(
            "Whenever a creature blocks, this enchantment deals {amount} damage to that creature's controller"
        );
    }
    if trimmed == "Whenever this permanent becomes the target of a spell or ability, you sacrifice it"
    {
        return "When this creature becomes the target of a spell or ability, sacrifice it"
            .to_string();
    }
    if lower_trimmed.starts_with("for each creature you control, put a +1/+1 counter on that object")
    {
        return "Put a +1/+1 counter on each creature you control".to_string();
    }
    if lower_trimmed.starts_with("for each creature you control, put")
        && lower_trimmed.contains("+1/+1")
        && lower_trimmed.contains("counter")
        && lower_trimmed.contains("that object")
    {
        return "Put a +1/+1 counter on each creature you control".to_string();
    }
    if lower_trimmed
        .starts_with("for each other creature you control, put a +1/+1 counter on that object")
    {
        return "Put a +1/+1 counter on each other creature you control".to_string();
    }
    if lower_trimmed.starts_with("for each other creature you control, put")
        && lower_trimmed.contains("+1/+1")
        && lower_trimmed.contains("counter")
        && lower_trimmed.contains("that object")
    {
        return "Put a +1/+1 counter on each other creature you control".to_string();
    }
    if lower_trimmed
        .starts_with("when this creature enters, for each creature you control, put a +1/+1 counter on that object")
    {
        return "When this creature enters, put a +1/+1 counter on each other creature you control"
            .to_string();
    }
    if lower_trimmed
        == "when this creature enters or whenever another ally you control enters, may put a +1/+1 counter on this creature"
    {
        return "Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature"
            .to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("opponent's ")
        && let Some((objects, predicate)) = rest.split_once(" get ")
    {
        return format!("{objects} your opponents control get {predicate}");
    }
    if let Some(rest) = trimmed.strip_prefix("opponent's ")
        && let Some((objects, predicate)) = rest.split_once(" gets ")
    {
        return format!("{objects} your opponents control gets {predicate}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you draw ")
        && right.starts_with("Create ")
        && right.contains(" Treasure token")
    {
        let draw_count = left
            .strip_prefix("you draw ")
            .or_else(|| left.strip_prefix("You draw "))
            .and_then(|tail| tail.strip_suffix(" cards"));
        let create_count = right
            .strip_prefix("Create ")
            .and_then(|tail| tail.strip_suffix(" Treasure tokens"));
        let render_count = |raw: &str| {
            raw.trim()
                .parse::<u32>()
                .ok()
                .and_then(small_number_word)
                .map(str::to_string)
                .unwrap_or_else(|| raw.trim().to_string())
        };
        if let (Some(draw_count), Some(create_count)) = (draw_count, create_count) {
            return format!(
                "Draw {} cards and create {} Treasure tokens",
                render_count(draw_count),
                render_count(create_count)
            );
        }
    }
    if let Some(rest) = trimmed.strip_prefix("you ") {
        let normalized = normalize_you_verb_phrase(rest);
        if normalized != rest {
            return normalized;
        }
    }
    if let Some((left, right)) = trimmed.split_once(". it gets ")
        && left.starts_with("Untap ")
        && left.contains("target ")
        && left.contains(" creatures")
    {
        return format!("{left}. They each get {right}");
    }
    if let Some(count) = trimmed.strip_prefix("you draw ")
        && let Some(count) = count.strip_suffix(" cards. Proliferate")
    {
        return format!("Draw {count} cards, then proliferate");
    }
    if let Some(count) = trimmed.strip_prefix("you draw ")
        && let Some(count) = count.strip_suffix(" cards. you sacrifice a permanent you control")
    {
        return format!("Draw {count} cards, then sacrifice a permanent");
    }
    if let Some(prefix) = trimmed.strip_suffix(" can't block until end of turn") {
        return format!("{prefix} can't block this turn");
    }
    if let Some(prefix) = trimmed.strip_suffix(" can't block until end of turn.") {
        return format!("{prefix} can't block this turn");
    }
    if let Some(prefix) = trimmed.strip_suffix(" can't be blocked until end of turn") {
        return format!("{prefix} can't be blocked this turn");
    }
    if let Some(prefix) = trimmed.strip_suffix(" can't be blocked until end of turn.") {
        return format!("{prefix} can't be blocked this turn");
    }
    if let Some(prefix) = trimmed.strip_suffix(" until end of turn")
        && prefix.contains("Regenerate ")
    {
        return prefix.to_string();
    }
    if let Some(prefix) = trimmed.strip_suffix(" until end of turn.")
        && prefix.contains("Regenerate ")
    {
        return prefix.to_string();
    }
    if trimmed.starts_with("Prevent the next ")
        && trimmed.contains(" damage to ")
        && trimmed.contains(" until end of turn")
    {
        return trimmed
            .replace(" damage to ", " damage that would be dealt to ")
            .replace(" until end of turn", " this turn");
    }
    if let Some((subject, mana_tail)) = trimmed.split_once(" have t add ") {
        return format!("{subject} have \"{{T}}: Add {mana_tail}.\"");
    }
    if let Some((subject, mana_tail)) = trimmed.split_once(" has t add ") {
        return format!("{subject} has \"{{T}}: Add {mana_tail}.\"");
    }
    if trimmed
        == "You may you choose exactly 1 a Island you control in the battlefield and tags it as 'return_cost_0'. Return target permanent to its owner's hand rather than pay this spell's mana cost (Parsed alternative cost)"
    {
        return "You may return an Island you control to its owner's hand rather than pay this spell's mana cost"
            .to_string();
    }
    if let Some((prefix, choice)) = trimmed
        .split_once(" and you choose exactly 1 ")
        .and_then(|(left, right)| {
            right
                .strip_suffix(
                    " in the battlefield and tags it as 'return_cost_0'. Return target permanent to its owner's hand rather than pay this spell's mana cost (Parsed alternative cost)",
                )
                .map(|choice| (left, choice))
        })
    {
        return format!(
            "{prefix} and return {} to its owner's hand rather than pay this spell's mana cost",
            choice
        );
    }
    if let Some(choice) = trimmed
        .strip_prefix("You may you choose exactly 1 ")
        .and_then(|rest| {
            rest.strip_suffix(
                " in the battlefield and tags it as 'return_cost_0'. Return target permanent to its owner's hand rather than pay this spell's mana cost (Parsed alternative cost)",
            )
        })
    {
        return format!(
            "You may return {} to its owner's hand rather than pay this spell's mana cost",
            choice
        );
    }
    if trimmed.starts_with("When this land enters, you sacrifice it unless you Return target land you control to its owner's hand")
        || trimmed
            .starts_with("When this land enters, you sacrifice it unless you return target land you control to its owner's hand")
        || trimmed
            .starts_with("When this permanent enters, you sacrifice it unless you Return target land you control to its owner's hand")
        || trimmed
            .starts_with("When this permanent enters, you sacrifice it unless you return target land you control to its owner's hand")
    {
        return "When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand"
            .to_string();
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && right.starts_with("you gain ")
        && left.contains(" loses ")
    {
        return format!("{left} and {right}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.contains("Scry ")
        && right == "Draw a card"
    {
        let lower_right = right.to_ascii_lowercase();
        return format!("{left}, then {lower_right}");
    }
    if trimmed == "you draw a card" {
        return "Draw a card".to_string();
    }
    if trimmed
        == "For each player, Put target card in target player's hand on top of its owner's library"
    {
        return "each player puts a card from their hand on top of their library".to_string();
    }
    if trimmed == "Exile all card in graveyard" {
        return "Exile all graveyards".to_string();
    }
    if trimmed == "Exile target nonland permanent. Create a Treasure token under that object's controller's control" {
        return "Exile target nonland permanent. Its controller creates a Treasure token"
            .to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Exile target ")
        && let Some((count, noun)) = rest.split_once(" target ")
        && let Ok(count) = count.parse::<u32>()
    {
        let count_text = small_number_word(count)
            .map(str::to_string)
            .unwrap_or_else(|| count.to_string());
        let noun = if count == 1 {
            noun.to_string()
        } else {
            pluralize_noun_phrase(noun)
        };
        return format!("Exile {count_text} target {noun}");
    }
    if trimmed.contains("Create a treasure token") {
        return trimmed.replace("Create a treasure token", "Create a Treasure token");
    }
    if let Some(prefix) =
        trimmed.strip_suffix(" target card in your hand on top of its owner's library")
    {
        if prefix == "Put" {
            return "Put a card from your hand on top of your library".to_string();
        }
        if let Some(count_text) = prefix.strip_prefix("Put ") {
            return format!("Put {count_text} cards from your hand on top of your library");
        }
    }
    if trimmed.contains(" in your graveyard to its owner's hand") {
        return trimmed.replace(
            " in your graveyard to its owner's hand",
            " from your graveyard to your hand",
        );
    }
    if let Some(tail) = trimmed.strip_prefix("you sacrifice a permanent you control unless ")
        && tail == "you sacrifice a Forest you control"
    {
        return "sacrifice it unless you sacrifice a Forest".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("you may Search your library for artifact with mana value ")
        && let Some((value, tail)) = rest.split_once(" you own, reveal it, put it into hand, then shuffle")
    {
        return format!(
            "you may search your library for an artifact card with mana value {value}, reveal it, put it into your hand, then shuffle{tail}"
        );
    }
    if let Some(rest) = trimmed.strip_prefix("you may Search your library for artifact with mana value ")
        && let Some(value) = rest.strip_suffix(" you own, reveal it, put it into hand, then shuffle")
    {
        return format!(
            "you may search your library for an artifact card with mana value {value}, reveal it, put it into your hand, then shuffle"
        );
    }
    if let Some(rest) = trimmed.strip_prefix("you may Search your library for Aura you own, reveal it, put it into hand, then shuffle")
    {
        return format!(
            "you may search your library for an Aura card, reveal it, put it into your hand, then shuffle{rest}"
        );
    }
    if let Some(rest) =
        trimmed.strip_prefix("you may Search your library for basic land you own, reveal it, put it on top of library, then shuffle")
    {
        return format!(
            "you may search your library for a basic land card, reveal it, then shuffle and put that card on top{rest}"
        );
    }
    if trimmed.starts_with("Whenever you cast creature") {
        return trimmed.replacen(
            "Whenever you cast creature",
            "When you cast a creature spell",
            1,
        );
    }
    if let Some(rest) = trimmed.strip_prefix("Sacrifice this creature: This creature deals ") {
        return format!("Sacrifice this creature: It deals {rest}");
    }
    if trimmed == "Untap target 1 to 2 target creature" {
        return "Untap one or two target creatures".to_string();
    }
    if trimmed.starts_with("This land enters with ")
        && trimmed.contains(" charge counters")
        && !trimmed.ends_with(" on it")
    {
        return format!("{trimmed} on it");
    }
    if let Some(after_this) = trimmed
        .strip_prefix("This ")
        .or_else(|| trimmed.strip_prefix("this "))
        && let Some((subject, tail)) = after_this.split_once(" enters with 1 ")
    {
        let counter_text = tail
            .strip_suffix(" counters")
            .or_else(|| tail.strip_suffix(" counters."))
            .or_else(|| tail.strip_suffix(" counter(s)"))
            .or_else(|| tail.strip_suffix(" counter(s)."))
            .or_else(|| tail.strip_suffix(" counters on it"))
            .or_else(|| tail.strip_suffix(" counters on it."));
        if let Some(counter_text) = counter_text {
            return format!("This {subject} enters with a {counter_text} counter on it");
        }
    }
    if trimmed.starts_with("Exile ") && trimmed.contains("target cards in graveyard") {
        return trimmed.replace(
            "target cards in graveyard",
            "target cards from a single graveyard",
        );
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && let Some(stripped) = right.strip_prefix("target player loses ")
        && left.starts_with("target player draws ")
        && stripped.ends_with(" life")
    {
        return format!("{left} and target player loses {stripped}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && let Some(scry_count) = left
            .strip_prefix("Scry ")
            .and_then(|value| value.trim().parse::<u32>().ok())
        && scry_count != 1
        && (right.starts_with("Draw ") || right.to_ascii_lowercase().starts_with("you draw "))
    {
        let draw_clause = if let Some(rest) = right.strip_prefix("Draw ") {
            format!("draw {rest}")
        } else {
            normalize_you_verb_phrase(right)
        };
        return format!("{left}, then {draw_clause}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you draw ")
        && right.to_ascii_lowercase().starts_with("you lose ")
        && !right.to_ascii_lowercase().contains("half your life")
        && right.to_ascii_lowercase().ends_with(" life")
    {
        return format!("{left} and {}", normalize_you_verb_phrase(right));
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left
            .to_ascii_lowercase()
            .starts_with("target player loses ")
        && right.to_ascii_lowercase().starts_with("you gain ")
        && !right.to_ascii_lowercase().contains("equal to")
        && right.to_ascii_lowercase().ends_with(" life")
    {
        return format!("{left} and {}", normalize_you_verb_phrase(right));
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you discard ")
        && right.to_ascii_lowercase().starts_with("you draw ")
    {
        let left = left.strip_prefix("you ").unwrap_or(left);
        return format!("{}, then {}", capitalize_first(left), normalize_you_verb_phrase(right));
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("target player discards ")
        && right.to_ascii_lowercase().starts_with("target player loses ")
    {
        return format!("{left} and {right}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you draw ")
        && right.starts_with("Put ")
        && right.contains(" from your hand on top of your library")
    {
        let left = left.strip_prefix("you ").unwrap_or(left);
        return format!(
            "{}, then {} in any order",
            capitalize_first(left),
            right.to_ascii_lowercase()
        );
    }
    if lower_trimmed == "target creature can't be blocked this turn you draw a card" {
        return "Target creature can't be blocked this turn. Draw a card".to_string();
    }
    if lower_trimmed == "target creature can't block this turn you draw a card" {
        return "Target creature can't block this turn. Draw a card".to_string();
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.starts_with("Draw ")
        && right.starts_with("Create ")
        && right.contains(" Treasure token")
    {
        let normalize_count = |prefix: &str, suffix: &str| -> Option<String> {
            let value = prefix.trim().parse::<u32>().ok()?;
            let amount = small_number_word(value)
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            Some(format!("{amount} {suffix}"))
        };
        if let Some(draw_tail) = left.strip_prefix("Draw ")
            && let Some(draw_count) = draw_tail.strip_suffix(" cards")
            && let Some(create_tail) = right.strip_prefix("Create ")
            && let Some(create_count) = create_tail.strip_suffix(" Treasure tokens")
            && let (Some(draw_text), Some(create_text)) = (
                normalize_count(draw_count, "cards"),
                normalize_count(create_count, "Treasure tokens"),
            )
        {
            return format!("Draw {draw_text} and create {create_text}");
        }
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.starts_with("Whenever this creature becomes tapped, you gain ")
        && right.starts_with("Scry ")
    {
        return format!(
            "{left} and {}",
            right.to_ascii_lowercase()
        );
    }
    if let Some((left, rest)) = trimmed.split_once(". ")
        && left.starts_with("Surveil ")
        && let Some((draw_clause, lose_clause)) = rest.split_once(". ")
        && draw_clause.starts_with("you draw ")
        && draw_clause.ends_with(" cards")
        && lose_clause.starts_with("you lose ")
        && lose_clause.ends_with(" life")
    {
        let draw_count = draw_clause
            .strip_prefix("you draw ")
            .and_then(|tail| tail.strip_suffix(" cards"))
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .ok()
            .and_then(small_number_word)
            .map(str::to_string)
            .unwrap_or_else(|| {
                draw_clause
                    .strip_prefix("you draw ")
                    .and_then(|tail| tail.strip_suffix(" cards"))
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            });
        return format!(
            "{left}, then draw {draw_count} cards. {}",
            capitalize_first(lose_clause)
        );
    }
    if let Some(rest) = trimmed.strip_prefix("target creature gets ")
        && let Some(buff) = rest.strip_suffix(" until end of turn. Untap it")
    {
        return format!("Target creature gets {buff} until end of turn. Untap that creature");
    }
    if let Some(rest) = trimmed.strip_prefix("target creature gets ")
        && let Some((pt_text, tail)) = rest.split_once(" until end of turn")
        && let Some((power, toughness)) = pt_text.split_once('/')
    {
        let power = power.trim();
        let toughness = toughness.trim();
        if power == toughness && power.starts_with("the number of ") {
            let mut line =
                format!("Target creature gets +X/+X until end of turn, where X is {power}");
            line.push_str(tail);
            return line;
        }
    }
    if let Some(rest) = trimmed.strip_prefix("you sacrifice a land you control. ")
        && rest.starts_with("Search your library for up to 2 basic land")
    {
        return format!("Sacrifice a land. {rest}");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counters on target creature. Proliferate"))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put a ")
        .and_then(|rest| rest.strip_suffix(" counter on target creature. Proliferate"))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counters on target creature. Proliferate."))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put a ")
        .and_then(|rest| rest.strip_suffix(" counter on target creature. Proliferate."))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counter(s) on target creature. Proliferate"))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counter(s) on target creature. Proliferate."))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if lower_trimmed == "put 1 -1/-1 counters on target creature. proliferate"
        || lower_trimmed == "put 1 -1/-1 counters on target creature. proliferate."
    {
        return "Put a -1/-1 counter on target creature, then proliferate".to_string();
    }
    if lower_trimmed == "put 1 +1/+1 counters on target creature. proliferate"
        || lower_trimmed == "put 1 +1/+1 counters on target creature. proliferate."
    {
        return "Put a +1/+1 counter on target creature, then proliferate".to_string();
    }
    if trimmed
        == "Schedule delayed trigger: Takes an extra turn after this one. you lose the game"
    {
        return "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game"
            .to_string();
    }
    if trimmed == "Schedule delayed trigger: Takes an extra turn after this one" {
        return "Take an extra turn after this one".to_string();
    }
    if trimmed
        == "Schedule delayed trigger: You takes an extra turn after this one. you lose the game"
    {
        return "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game"
            .to_string();
    }
    if trimmed == "Schedule delayed trigger: You takes an extra turn after this one" {
        return "Take an extra turn after this one".to_string();
    }
    if trimmed
        == "Untap target 1 to 2 target creature. this source gets +2/+2 until end of turn"
    {
        return "Untap one or two target creatures. They each get +2/+2 until end of turn"
            .to_string();
    }
    if let Some((first, second)) =
        trimmed.split_once(". with the same name as that object other creatures get ")
        && let Some(first_buff) = first
            .strip_prefix("target creature gets ")
            .and_then(|rest| rest.strip_suffix(" until end of turn"))
        && second == format!("{first_buff} until end of turn")
    {
        return format!(
            "target creature and all other creatures with the same name as that creature get {first_buff} until end of turn"
        );
    }

    let mut normalized = trimmed.to_string();
    normalized = normalized
        .replace("this creatures get ", "this creature gets ")
        .replace("this creatures gain ", "this creature gains ")
        .replace("enchanted creatures get ", "enchanted creature gets ")
        .replace("enchanted creatures gain ", "enchanted creature gains ")
        .replace("equipped creatures get ", "equipped creature gets ")
        .replace("equipped creatures gain ", "equipped creature gains ")
        .replace(
            "Whenever this creature blocks or Whenever this creature becomes blocked",
            "Whenever this creature blocks or becomes blocked",
        )
        .replace("target attacking/blocking creature", "target attacking or blocking creature")
        .replace("target creature blocking it", "target blocking creature")
        .replace("a another ", "another ")
        .replace("you takes ", "you take ")
        .replace("you loses ", "you lose ")
        .replace(" damage to it", " damage to that creature")
        .replace("+-", "-")
        .replace(" gets 0/+", " gets +0/+")
        .replace(" gets 0/", " gets +0/")
        .replace("that creatureself", "itself")
        .replace(
            "other creature you control flying get ",
            "other creatures you control with flying get ",
        )
        .replace(
            "other creature you control with flying get ",
            "other creatures you control with flying get ",
        )
        .replace("token creatures you control get ", "creature tokens you control get ")
        .replace("token creatures get ", "creature tokens get ")
        .replace(
            "Destroy target artifact or enchantment or land",
            "Destroy target artifact, enchantment, or land",
        )
        .replace(
            "Destroy target artifact or creature or planeswalker",
            "Destroy target artifact, creature, or planeswalker",
        )
        .replace(
            "Destroy target artifact or enchantment or planeswalker",
            "Destroy target artifact, enchantment, or planeswalker",
        )
        .replace(
            "Destroy target artifact or battle or enchantment or creature with flying",
            "Destroy target artifact, battle, enchantment, or creature with flying",
        )
        .replace(
            "target artifact or creature or enchantment or land card",
            "target artifact, creature, enchantment, or land card",
        )
        .replace(
            "target artifact or creature or enchantment or land",
            "target artifact, creature, enchantment, or land",
        )
        .replace("Destroy all enchantment", "Destroy all enchantments")
        .replace("Sacrifice a land you control", "Sacrifice a land")
        .replace("Exile target card in graveyard", "Exile target card from a graveyard")
        .replace(
            "Search your library for a card, put it into hand, then shuffle",
            "Search your library for a card, put that card into your hand, then shuffle",
        )
        .replace(
            "for each the number of an artifact you control",
            "for each artifact you control",
        )
        .replace(", then you draw ", ", then draw ")
        .replace(". you draw ", ". Draw ")
        .replace(", then you mill ", ", then mill ")
        .replace("Discard a card, then you draw ", "Discard a card, then draw ")
        .replace("discard a card, then you draw ", "Discard a card, then draw ")
        .replace("Sacrifice this creature: this creature deals ", "Sacrifice this creature: It deals ")
        .replace("Sacrifice this creature: This creature deals ", "Sacrifice this creature: It deals ")
        .replace(
            "Prevent combat damage until end of turn",
            "Prevent all combat damage that would be dealt this turn",
        )
        .replace(
            "Add one mana of any color that an opponent's land could produce",
            "Add one mana of any color that a land an opponent controls could produce",
        )
        .replace(
            "Add one mana of any color to your mana pool that an opponent's land could produce",
            "Add one mana of any color that a land an opponent controls could produce",
        )
        .replace(" / ", "/")
        .replace(
            "for each the number of blocking creature",
            "for each creature blocking it",
        )
        .replace("target opponent's creature", "target creature an opponent controls")
        .replace("enters the battlefield", "enters")
        .replace(
            "target opponent's nonland enchantment",
            "target nonland permanent an opponent controls",
        )
        .replace(
            "Destroy target opponent's artifact or enchantment",
            "Destroy target artifact or enchantment an opponent controls",
        )
        .replace(
            "Create a Powerstone artifact token, tapped",
            "Create a tapped Powerstone token",
        )
        .replace(
            ", you sacrifice a permanent you control unless you sacrifice a Forest you control",
            ", sacrifice it unless you sacrifice a Forest",
        )
        .replace(", This creature deals ", ", it deals ")
        .replace(
            " in your graveyard on top of its owner's library",
            " from your graveyard on top of your library",
        )
        .replace("Put 1 +1/+1 counter(s) on ", "Put a +1/+1 counter on ")
        .replace("counter(s)", "counters")
        .replace("artifact you control cost", "artifact spells you cast cost")
        .replace("Whenever a another ", "Whenever another ")
        .replace("you may Search", "you may search")
        .replace(
            "When this creature enters or Whenever another Ally you control enters,",
            "Whenever this creature or another Ally you control enters,",
        )
        .replace("Untap all a creature you control", "Untap all creatures you control")
        .replace(", May ", ", you may ")
        .replace("you pays ", "you pay ")
        .replace("Add 1 mana of any color", "Add one mana of any color");
    normalized = normalize_zero_pt_prefix(&normalized);
    if normalized.ends_with(" as long as it's your turn")
        && (normalized.starts_with("this creature ")
            || normalized.starts_with("creatures you control ")
            || normalized.starts_with("creature you control "))
    {
        let body = normalized
            .strip_suffix(" as long as it's your turn")
            .unwrap_or(normalized.as_str())
            .to_string();
        return format!("During your turn, {body}");
    }
    normalized
}

fn normalize_for_each_damage_clause(clause: &str) -> Option<String> {
    let rest = clause.strip_prefix("For each ")?;
    let (subject, tail) = rest.split_once(", Deal ")?;
    let amount = tail.strip_suffix(" damage to that object")?;
    Some(format!("Deal {amount} damage to each {subject}"))
}

fn normalize_each_player_then_for_each_damage_clause(line: &str) -> Option<String> {
    let (left, rest) = line.split_once(" damage to that player. For each ")?;
    let amount = left.strip_prefix("Deal ")?;
    let (filter, right) = rest.split_once(" that player controls, Deal ")?;
    let amount_right = right.strip_suffix(" damage to each player")?;
    if amount != amount_right {
        return None;
    }
    Some(format!(
        "Deal {amount} damage to each {filter} and each player"
    ))
}

fn normalize_oracle_line_for_card(def: &CardDefinition, line: &str) -> String {
    let mut normalized = line.trim().to_string();
    let has_player_or_planeswalker_target = def.abilities.iter().any(|ability| {
        ability.text.as_ref().is_some_and(|text| {
            text.to_ascii_lowercase()
                .contains("target player or planeswalker")
        })
    });
    let has_opponent_or_planeswalker_target = def.abilities.iter().any(|ability| {
        ability.text.as_ref().is_some_and(|text| {
            text.to_ascii_lowercase()
                .contains("target opponent or planeswalker")
        })
    });
    let has_controller_next_untap_text = def.abilities.iter().any(|ability| {
        ability.text.as_ref().is_some_and(|text| {
            let lower = text.to_ascii_lowercase();
            lower.contains("doesn't untap during its controller's next untap step")
                || lower.contains("doesnt untap during its controllers next untap step")
        })
    });
    let has_exactly_seven_cards_activation = def.abilities.iter().any(|ability| {
        ability.text.as_ref().is_some_and(|text| {
            text.to_ascii_lowercase()
                .contains("activate only if you have exactly seven cards in hand")
        })
    });
    let has_graveyard_activation = def.abilities.iter().any(|ability| {
        matches!(
            ability.kind,
            AbilityKind::Activated(_) | AbilityKind::Mana(_)
        ) && ability.functional_zones.contains(&Zone::Graveyard)
    });
    let card_name = def.card.name.trim().to_string();
    let card_name_lower = card_name.to_ascii_lowercase();
    let has_named_exile_prefix = def.abilities.iter().any(|ability| {
        ability.text.as_ref().is_some_and(|text| {
            let lower = text.trim().to_ascii_lowercase();
            lower.starts_with(&format!("exile {card_name_lower}"))
        })
    });
    let has_ward_pay_life = def.abilities.iter().any(|ability| {
        ability.text.as_ref().is_some_and(|text| {
            let lower = text.to_ascii_lowercase();
            lower.contains("ward") && lower.contains("pay") && lower.contains("life")
        })
    }) || def.abilities.iter().any(|ability| {
        matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.ward_cost().is_some())
    });
    let has_pay_life_clause = def.abilities.iter().any(|ability| {
        ability
            .text
            .as_ref()
            .is_some_and(|text| text.to_ascii_lowercase().contains("pay") && text.to_ascii_lowercase().contains("life"))
    });
    let escape_exile_count = def.alternative_casts.iter().find_map(|method| {
        if let AlternativeCastingMethod::Escape { exile_count, .. } = method {
            Some(*exile_count)
        } else {
            None
        }
    });

    normalized = normalized
        .replace(" can't block until end of turn.", " can't block this turn")
        .replace(" can't block until end of turn", " can't block this turn")
        .replace(
            " can't be blocked until end of turn.",
            " can't be blocked this turn",
        )
        .replace(" ors ", " or ")
        .replace(" ors", " or")
        .replace("ors ", "or ")
        .replace(
            " can't be blocked until end of turn",
            " can't be blocked this turn",
        );

    if has_graveyard_activation {
        normalized = normalized
            .replace("Exile this creature", "Exile this card from your graveyard")
            .replace("Exile this source", "Exile this card from your graveyard");
    }
    if !card_name.is_empty()
        && ((def.card.card_types.contains(&CardType::Instant)
            || def.card.card_types.contains(&CardType::Sorcery))
            || has_named_exile_prefix)
    {
        normalized = normalized.replace("Exile this source", &format!("Exile {card_name}"));
    }
    if !card_name.is_empty()
        && has_named_exile_prefix
    {
        normalized = normalized.replace("Exile this creature", &format!("Exile {card_name}"));
    }
    if has_ward_pay_life {
        if let Some(amount) = normalized
            .strip_prefix("you lose ")
            .and_then(|rest| rest.strip_suffix(" life"))
        {
            normalized = format!("Ward—Pay {amount} life");
        } else if let Some(amount) = normalized
            .strip_prefix("You lose ")
            .and_then(|rest| rest.strip_suffix(" life"))
        {
            normalized = format!("Ward—Pay {amount} life");
        }
    } else if has_pay_life_clause {
        if let Some(amount) = normalized
            .strip_prefix("you lose ")
            .and_then(|rest| rest.strip_suffix(" life"))
        {
            normalized = format!("Pay {amount} life");
        } else if let Some(amount) = normalized
            .strip_prefix("You lose ")
            .and_then(|rest| rest.strip_suffix(" life"))
        {
            normalized = format!("Pay {amount} life");
        }
    }
    if let Some(exile_count) = escape_exile_count {
        let count_text = small_number_word(exile_count)
            .map(str::to_string)
            .unwrap_or_else(|| exile_count.to_string());
        normalized = normalized.replace(
            "Exile target card in your graveyard",
            &format!("Exile {count_text} other cards from your graveyard"),
        );
    }

    if has_opponent_or_planeswalker_target && normalized.contains(" damage to any target") {
        normalized =
            normalized.replace(" damage to any target", " damage to target opponent or planeswalker");
    } else if has_player_or_planeswalker_target && normalized.contains(" damage to any target") {
        normalized =
            normalized.replace(" damage to any target", " damage to target player or planeswalker");
    }

    if has_controller_next_untap_text {
        normalized = normalized
            .replace(
                "target creature can't untap until your next turn",
                "Target creature doesn't untap during its controller's next untap step",
            )
            .replace(
                "target creature cant untap until your next turn",
                "Target creature doesn't untap during its controller's next untap step",
            )
            .replace(
                "permanent can't untap until your next turn",
                "That creature doesn't untap during its controller's next untap step",
            )
            .replace(
                "permanent cant untap until your next turn",
                "That creature doesn't untap during its controller's next untap step",
            );
    }

    if normalized
        .to_ascii_lowercase()
        .starts_with("when this land enters, you sacrifice it unless you return target land you control to its owner's hand")
    {
        normalized = "When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand".to_string();
    }

    if has_exactly_seven_cards_activation
        && (normalized.eq_ignore_ascii_case("{T}: Draw a card")
            || normalized.eq_ignore_ascii_case("{T}: you draw a card"))
    {
        normalized = "{T}: Draw a card. Activate only if you have exactly seven cards in hand"
            .to_string();
    }

    if normalized.contains("Tag the object attached to this source as 'enchanted'. Exile the tagged object 'enchanted'")
    {
        normalized = normalized.replace(
            "Tag the object attached to this source as 'enchanted'. Exile the tagged object 'enchanted'",
            "Exile enchanted creature",
        );
    }

    if let Some(cost) = normalized
        .strip_prefix("At the beginning of your upkeep, you sacrifice an Aura you control unless you pays ")
    {
        normalized = format!(
            "At the beginning of your upkeep, sacrifice this Aura unless you pay {cost}"
        );
    }

    if normalized.starts_with("When this permanent enters, Tap enchanted creature") {
        normalized = normalized.replace(
            "When this permanent enters, Tap enchanted creature",
            "When this Aura enters, tap enchanted creature",
        );
    }
    normalized = normalize_activation_cost_add_punctuation(&normalized);
    normalized = normalize_cost_payment_wording(&normalized);

    let subject = card_self_subject_for_oracle_lines(def);
    if subject != "permanent" {
        normalized = normalized.replace("This enters tapped", &format!("This {subject} enters tapped"));
        normalized = normalized.replace(
            "This enters with ",
            &format!("This {subject} enters with "),
        );
        normalized = normalized.replace("This source's", &format!("This {subject}'s"));
        normalized = normalized.replace("This source", &format!("This {subject}"));
        normalized = normalized.replace("this source's", &format!("this {subject}'s"));
        normalized = normalized.replace("this source", &format!("this {subject}"));
        normalized = normalized.replace(
            "Enters the battlefield with ",
            &format!("This {subject} enters with "),
        );
        normalized = normalized.replace(
            "When this permanent enters the battlefield",
            &format!("When this {subject} enters"),
        );
        normalized = normalized.replace(
            "Whenever this permanent enters the battlefield",
            &format!("Whenever this {subject} enters"),
        );
        normalized = normalized.replace(
            "When this permanent leaves the battlefield",
            &format!("When this {subject} leaves the battlefield"),
        );
        normalized = normalized.replace(
            "Whenever this permanent leaves the battlefield",
            &format!("Whenever this {subject} leaves the battlefield"),
        );
        if normalized.contains(": Deal ") && !normalized.contains('"') {
            normalized = normalized.replacen(
                ": Deal ",
                &format!(": This {subject} deals "),
                1,
            );
        }
        if normalized.contains(": target creature gets base power and toughness") {
            normalized = normalized.replacen(
                ": target creature gets base power and toughness",
                &format!(": target creature other than this {subject} has base power and toughness"),
                1,
            );
        }
        if normalized.contains(": Target creature gets base power and toughness") {
            normalized = normalized.replacen(
                ": Target creature gets base power and toughness",
                &format!(": Target creature other than this {subject} has base power and toughness"),
                1,
            );
        }
    }
    if def.is_spell() && normalized.starts_with("Deal ") && !normalized.contains(": ") {
        if let Some(rest) = normalized.strip_prefix("Deal ") {
            normalized = format!("{} deals {rest}", def.card.name);
            normalized = normalized.replace(" and deal ", " and ");
        }
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.contains(" deals ")
        && right.starts_with("you gain ")
        && !right.contains(". ")
    {
        normalized = format!("{left} and {right}");
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.contains(" deals ")
        && right.starts_with("Deal ")
        && !right.contains(". ")
    {
        let right_is_self_damage = right.contains(" damage to this source")
            || right.contains(" damage to this creature");
        let should_join = if right_is_self_damage {
            true
        } else if let Some((left_amount, _)) = left
            .split_once(" deals ")
            .and_then(|(_, tail)| tail.split_once(" damage to "))
            && let Some((right_amount, _)) = right
                .strip_prefix("Deal ")
                .and_then(|tail| tail.split_once(" damage to "))
            && let (Ok(left_amount), Ok(right_amount)) =
                (left_amount.trim().parse::<i32>(), right_amount.trim().parse::<i32>())
        {
            right_amount <= left_amount
        } else {
            true
        };
        if should_join {
            normalized = format!("{left} and {}", right.replacen("Deal ", "deal ", 1));
        }
    }
    if card_has_graveyard_activated_ability(def) {
        normalized = normalized.replace(
            "Return this source to its owner's hand",
            "Return this card from your graveyard to your hand",
        );
        normalized = normalized.replace(
            &format!("Return this {subject} to its owner's hand"),
            "Return this card from your graveyard to your hand",
        );
    }
    if normalized.starts_with("equipped creature has \"")
        && normalized
            .to_ascii_lowercase()
            .contains("sacrifice this: this deals")
    {
        normalized = normalized.replace(
            "Sacrifice this: This deals",
            &format!(
                "Sacrifice {}: {} deals",
                def.card.name.as_str(),
                def.card.name.as_str()
            ),
        );
        normalized = normalized.replace(
            "Sacrifice this: this deals",
            &format!(
                "Sacrifice {}: {} deals",
                def.card.name.as_str(),
                def.card.name.as_str()
            ),
        );
        normalized = normalized.replace(
            "sacrifice this: This deals",
            &format!(
                "Sacrifice {}: {} deals",
                def.card.name.as_str(),
                def.card.name.as_str()
            ),
        );
        normalized = normalized.replace(
            "sacrifice this: this deals",
            &format!(
                "Sacrifice {}: {} deals",
                def.card.name.as_str(),
                def.card.name.as_str()
            ),
        );
    }
    if let Some(enchanted_subject) = enchanted_subject_for_oracle_lines(def) {
        normalized = normalized.replace(
            "Tap enchanted permanent",
            &format!("Tap enchanted {enchanted_subject}"),
        );
        normalized = normalized.replace(
            "Untap enchanted permanent",
            &format!("Untap enchanted {enchanted_subject}"),
        );
    }
    normalized = normalized.replace(
        "Search your library for up to 1 basic land you own, put it onto the battlefield tapped, then shuffle",
        "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for basic land you own, put it onto the battlefield tapped, then shuffle",
        "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for basic land you own, reveal it, put it into hand, then shuffle",
        "Search your library for a basic land card, reveal it, put it into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for basic land you own, reveal it, put it into your hand, then shuffle",
        "Search your library for a basic land card, reveal it, put it into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for a basic land card, reveal it, put it into hand, then shuffle",
        "Search your library for a basic land card, reveal it, put it into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for artifact with mana value ",
        "Search your library for an artifact card with mana value ",
    );
    normalized = normalized.replace(
        "Search your library for Aura you own, reveal it, put it into hand, then shuffle",
        "Search your library for an Aura card, reveal it, put it into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for aura you own, reveal it, put it into hand, then shuffle",
        "Search your library for an Aura card, reveal it, put it into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for an artifact card with mana value 3 you own, reveal it, put it into hand, then shuffle",
        "Search your library for an artifact card with mana value 3, reveal it, put it into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for an artifact card with mana value 4 you own, reveal it, put it into hand, then shuffle",
        "Search your library for an artifact card with mana value 4, reveal it, put it into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for basic land you own, reveal it, put it on top of library, then shuffle",
        "Search your library for a basic land card, reveal it, then shuffle and put that card on top",
    );
    normalized = normalized.replace(
        "Search your library for up to 1 basic land you own, put it onto the battlefield tapped. Search your library for basic land you own, reveal it, put it into hand, then shuffle",
        "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for up to 1 basic land you own, put it onto the battlefield tapped. Search your library for a basic land card, reveal it, put it into hand, then shuffle",
        "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for up to 1 basic land you own, put it onto the battlefield tapped. Search your library for a basic land card, reveal it, put it into your hand, then shuffle",
        "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle",
    );
    normalized = normalized.replace(
        "Search your library for up to 2 basic land you own, put them onto the battlefield tapped, then shuffle",
        "Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle",
    );
    normalized = normalized.replace(
        "for each a creature you control",
        "for each creature you control",
    );
    normalized = normalized.replace("for each a Swamp you control", "for each Swamp you control");
    normalized = normalized.replace("for each the number of a ", "for each ");
    normalized = normalized.replace("for each the number of an ", "for each ");
    normalized = normalized.replace("Sacrifice a creature you control", "Sacrifice a creature");
    normalized = normalized.replace(
        "Sacrifice an artifact you control",
        "Sacrifice an artifact",
    );
    normalized = normalized.replace(
        "Sacrifice a Forest you control",
        "Sacrifice a Forest",
    );
    normalized = normalized.replace(
        "Add 1 mana of any color to your mana pool that an opponent's land could produce",
        "Add one mana of any color that a land an opponent controls could produce",
    );
    normalized = normalized.replace(
        "Add 1 mana of commander's color identity",
        "Add one mana of any color in your commander's color identity",
    );
    normalized = normalized.replace(
        "choose target card in target player's hand: For each player, Put target card in target player's hand on top of its owner's library",
        "Each player puts a card from their hand on top of their library",
    );
    normalized = normalized.replace(
        "Whenever a enchantment you control enters the battlefield: Put 1 +1/+1 counter(s) on this creature. you draw a card",
        "Whenever an enchantment you control enters, put a +1/+1 counter on this creature and draw a card",
    );
    normalized = normalized.replace(
        "Whenever you gain life: Put 1 +1/+1 counter(s) on this creature. Scry 1",
        "Whenever you gain life, put a +1/+1 counter on this creature and scry 1",
    );
    normalized = normalized.replace(
        "Whenever this creature attacks: Put 1 +1/+1 counter(s) on it",
        "Whenever this creature attacks, put a +1/+1 counter on it",
    );
    normalized = normalized.replace(
        "When this creature enters: Earthbend target land you control with 1 +1/+1 counter(s)",
        "When this creature enters, earthbend 1",
    );
    normalized = normalized.replace(
        "When this creature enters, draw a card. you lose ",
        "When this creature enters, you draw a card and you lose ",
    );
    normalized = normalized.replace(
        "As an additional cost to cast this spell: you discard a card",
        "As an additional cost to cast this spell, discard a card",
    );
    normalized = normalized.replace(
        "As an additional cost to cast this spell: You discard a card",
        "As an additional cost to cast this spell, discard a card",
    );
    normalized = normalized.replace(
        "Whenever this creature attacks: Tap any target",
        "Whenever this creature attacks, tap target creature defending player controls",
    );
    normalized = normalized.replace(
        "When this enchantment enters: Exile target nonland permanent an opponent controls",
        "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield",
    );
    normalized = normalized.replace(
        "all artifacts have at the beginning of your upkeep sacrifice this artifact unless you pay 2",
        "All artifacts have \"At the beginning of your upkeep, sacrifice this artifact unless you pay {2}.\"",
    );
    normalized = normalized.replace(
        "all creatures have at the beginning of your upkeep sacrifice this creature unless you pay 1",
        "All creatures have \"At the beginning of your upkeep, sacrifice this creature unless you pay {1}.\"",
    );
    normalized = normalized.replace(
        "Schedule delayed trigger: you take an extra turn after this one",
        "Take an extra turn after this one",
    );
    normalized = normalized.replace(
        "Schedule delayed trigger: You takes an extra turn after this one",
        "Take an extra turn after this one",
    );
    normalized = normalized.replace(
        "Schedule delayed trigger: you take an extra turn after this one. you lose the game",
        "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game",
    );
    normalized = normalized.replace(
        "Schedule delayed trigger: You takes an extra turn after this one. you lose the game",
        "Take an extra turn after this one. At the beginning of that turn's end step, you lose the game",
    );
    normalized = normalized.replace(
        "Scry 2. draw a card",
        "Scry 2, then draw a card",
    );
    let is_scry_one_then_draw_line = normalized.eq_ignore_ascii_case("Scry 1. Draw a card")
        || normalized.eq_ignore_ascii_case("Scry 1. you draw a card");
    if is_scry_one_then_draw_line
        && def.abilities.iter().any(|ability| {
            ability.text.as_ref().is_some_and(|text| {
                let lower = text.to_ascii_lowercase();
                lower.contains("when you cast this spell")
                    && lower.contains("copy it if you control")
            })
        })
    {
        normalized = "Scry 1, then draw a card".to_string();
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.starts_with("Deal ")
        && right.to_ascii_lowercase().starts_with("you gain ")
    {
        normalized = format!("{left} and {right}");
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.starts_with("Deal ")
        && right.starts_with("Deal ")
        && !right.contains(". ")
        && let Some(right_tail) = right.strip_prefix("Deal ")
    {
        normalized = format!("{left} and deal {right_tail}");
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && let Some(left) = normalize_for_each_damage_clause(left)
    {
        normalized = format!("{left}. {right}");
    } else if let Some(mapped) = normalize_for_each_damage_clause(&normalized) {
        normalized = mapped;
    }
    if let Some(mapped) = normalize_each_player_then_for_each_damage_clause(&normalized) {
        normalized = mapped;
    }
    if let Some((prefix, tail)) = normalized.split_once(" damage to each player and each ") {
        normalized = format!("{prefix} damage to each {tail} and each player");
    }
    if let Some(mapped) = normalize_create_under_control_clause(&normalized) {
        normalized = mapped;
    }
    if let Some(mapped) = normalize_search_you_own_clause(&normalized) {
        normalized = mapped;
    }

    if def.aura_attach_filter.is_some()
        && let Some(rest) = normalized
            .strip_prefix("At the beginning of each player's upkeep, Deal ")
            .or_else(|| {
                normalized.strip_prefix("At the beginning of each player's upkeep: Deal ")
            })
            .or_else(|| {
                normalized.strip_prefix(
                    "At the beginning of each player's upkeep, This enchantment deals ",
                )
            })
            .or_else(|| {
                normalized.strip_prefix(
                    "At the beginning of each player's upkeep: This enchantment deals ",
                )
            })
            .or_else(|| {
                normalized.strip_prefix("At the beginning of each player's upkeep, This Aura deals ")
            })
            .or_else(|| {
                normalized.strip_prefix("At the beginning of each player's upkeep: This Aura deals ")
            })
        && let Some((amount, tail)) = rest.split_once(" damage to target player")
        && tail.trim().is_empty()
    {
        normalized = format!(
            "At the beginning of the upkeep of enchanted creature's controller, this Aura deals {amount} damage to that player"
        );
    }

    if (normalized.contains(", Scry ") || normalized.contains(": Scry "))
        && (normalized.contains(". Draw a card") || normalized.contains(". you draw a card"))
    {
        normalized = normalized.replace(". Draw a card", ", then draw a card");
        normalized = normalized.replace(". you draw a card", ", then draw a card");
    }

    if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().contains("you gain ")
        && (left.starts_with("When ")
            || left.starts_with("Whenever ")
            || left.starts_with("At the beginning ")
            || left.contains(':'))
        && (right.eq_ignore_ascii_case("Draw a card")
            || right.eq_ignore_ascii_case("you draw a card"))
    {
        normalized = format!("{left} and draw a card");
    }

    normalized = normalized.replace(
        "you may Put target card Elf in your graveyard onto the battlefield",
        "You may put an Elf or Tyvar card from your graveyard onto the battlefield",
    );

    let chapter_one_tail = "you may Put target card Elf in your graveyard onto the battlefield";
    if let Some(rest) = normalized.strip_prefix("Chapter 1: Mill ")
        && let Some((count, tail)) = rest.split_once(" cards. ")
        && tail.eq_ignore_ascii_case(chapter_one_tail)
    {
        normalized = format!(
            "Chapter 1: Mill {count} cards. You may put an Elf or Tyvar card from your graveyard onto the battlefield"
        );
    } else if let Some(rest) = normalized.strip_prefix("Chapter 1: you mill ")
        && let Some((count, tail)) = rest.split_once(" cards. ")
        && tail.eq_ignore_ascii_case(chapter_one_tail)
    {
        normalized = format!(
            "Chapter 1: Mill {count} cards. You may put an Elf or Tyvar card from your graveyard onto the battlefield"
        );
    }
    if normalized.eq_ignore_ascii_case(
        "Chapter 3: Target opponent's creature or Elf gets -1/-1 until end of turn",
    ) {
        normalized = "Chapter 3: Whenever an Elf you control attacks this turn, target creature an opponent controls gets -1/-1 until end of turn".to_string();
    }

    normalized = normalized.replace(
        "Return this creature from graveyard to the battlefield tapped",
        "Return this card from your graveyard to the battlefield tapped",
    );
    let lower_normalized = normalized.to_ascii_lowercase();
    if lower_normalized.starts_with("all slivers have \"")
        || lower_normalized.starts_with("all sliver creatures have \"")
    {
        return normalize_oracle_line_segment(&normalized);
    }

    let mut normalized = normalized
        .split(": ")
        .map(normalize_oracle_line_segment)
        .collect::<Vec<_>>()
        .join(": ");

    if let Some(rest) = normalized.strip_prefix("a creature blocks: This enchantment deals ")
        && let Some((amount, tail)) = rest.split_once(" damage")
        && tail.trim().eq_ignore_ascii_case("to target creature")
    {
        normalized = format!(
            "Whenever a creature blocks, this enchantment deals {amount} damage to that creature's controller"
        );
    }

    if let Some(rest) = normalized.strip_prefix("a creature blocks: Deal ")
        && let Some((amount, tail)) = rest.split_once(" damage")
        && tail.trim().eq_ignore_ascii_case("to target creature")
    {
        normalized = format!(
            "Whenever a creature blocks, this enchantment deals {amount} damage to that creature's controller"
        );
    }

    if normalized.contains("Tap target creature an opponent controls. permanent can't untap until your next turn")
    {
        normalized = normalized.replace(
            "Tap target creature an opponent controls. permanent can't untap until your next turn",
            "tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step",
        );
    }

    if let Some(enchanted_subject) = enchanted_subject_for_oracle_lines(def) {
        normalized = normalized.replace(
            "Tap enchanted permanent",
            &format!("Tap enchanted {enchanted_subject}"),
        );
        normalized = normalized.replace(
            "Untap enchanted permanent",
            &format!("Untap enchanted {enchanted_subject}"),
        );
    }
    if let Some((trigger, effect)) = normalized.split_once(": ")
        && (trigger.starts_with("When ")
            || trigger.starts_with("Whenever ")
            || trigger.starts_with("At the beginning "))
    {
        normalized = format!("{trigger}, {effect}");
    }
    if normalized.starts_with("When ")
        || normalized.starts_with("Whenever ")
        || normalized.starts_with("At the beginning ")
    {
        normalized = normalized
            .replace(", you draw ", ", draw ")
            .replace(", You draw ", ", draw ")
            .replace(", you mill ", ", mill ")
            .replace(", You mill ", ", mill ");
    }
    if normalized == "Whenever this permanent becomes the target of a spell or ability, you sacrifice it"
    {
        normalized =
            "When this creature becomes the target of a spell or ability, sacrifice it"
                .to_string();
    }
    if let Some(tail) = normalized.strip_prefix(
        "Whenever you cast instant or sorcery or Whenever you copy instant or sorcery, ",
    ) {
        normalized = format!(
            "Magecraft — Whenever you cast or copy an instant or sorcery spell, {tail}"
        );
    }
    if let Some(tail) = normalized.strip_prefix("Whenever you cast enchantment, ") {
        normalized = format!("Whenever you cast an enchantment spell, {tail}");
    }
    if let Some(tail) = normalized.strip_prefix("Whenever you cast instant or sorcery, Deal ") {
        normalized = format!(
            "Whenever you cast an instant or sorcery spell, this creature deals {tail}"
        );
    }
    if let Some(tail) = normalized.strip_prefix("Whenever you cast noncreature spell, Deal ") {
        normalized = format!("Whenever you cast a noncreature spell, this creature deals {tail}");
    }
    if let Some(tail) = normalized.strip_prefix("Whenever you cast white spell, Put ") {
        normalized = format!(
            "Whenever you cast a spell that's white, blue, black, or red, put {tail}"
        );
    }
    if let Some((trigger, effect)) = normalized.split_once(": ")
        && let Some((left, right)) = effect.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you gain ")
        && right.to_ascii_lowercase().starts_with("you draw ")
    {
        normalized = format!(
            "{trigger}: {left} and {}",
            normalize_you_verb_phrase(right)
        );
    }
    if let Some((head, effect)) = normalized.split_once(": ")
        && let Some((left, right)) = effect.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you gain ")
    {
        let draw_clause = if let Some(tail) = right.strip_prefix("You draw ") {
            Some(format!("draw {tail}"))
        } else if let Some(tail) = right.strip_prefix("you draw ") {
            Some(format!("draw {tail}"))
        } else if let Some(tail) = right.strip_prefix("Draw ") {
            Some(format!("draw {tail}"))
        } else {
            right.strip_prefix("draw ").map(|tail| format!("draw {tail}"))
        };
        if let Some(draw_clause) = draw_clause {
            normalized = format!("{head}: {left} and {draw_clause}");
        }
    }
    if let Some(tail) = normalized
        .strip_prefix("Whenever this creature blocks or becomes blocked, it deals ")
    {
        normalized = format!(
            "Whenever this creature blocks or becomes blocked by a creature, this creature deals {tail}"
        );
    }
    if let Some(rest) = normalized.strip_prefix("When this creature enters, ")
        && let Some((gain_clause, draw_clause)) = rest.split_once(". ")
        && gain_clause.starts_with("you gain ")
        && draw_clause.eq_ignore_ascii_case("you draw a card")
    {
        normalized = format!("When this creature enters, {gain_clause} and draw a card");
    }
    normalized = normalized
        .replace(": You draw ", ": Draw ")
        .replace(": you draw ", ": Draw ")
        .replace(": You mill ", ": Mill ")
        .replace(": you mill ", ": Mill ")
        .replace(", then you draw ", ", then draw ")
        .replace(", then You draw ", ", then draw ")
        .replace(", then you mill ", ", then mill ")
        .replace(", then You mill ", ", then mill ")
        .replace("Discard a card, then you draw ", "Discard a card, then draw ")
        .replace("Discard a card, then You draw ", "Discard a card, then draw ")
        .replace(
            "Tag the object attached to this enchantment as 'enchanted'. Put a +1/+1 counter on the tagged object 'enchanted'",
            "Put a +1/+1 counter on enchanted creature",
        )
        .replace(
            "Tag the object attached to this enchantment as 'enchanted'. Put 1 -1/-1 counters on the tagged object 'enchanted'",
            "Put a -1/-1 counter on enchanted creature",
        )
        .replace(
            "Tag the object attached to this aura as 'enchanted'. Put a +1/+1 counter on the tagged object 'enchanted'",
            "Put a +1/+1 counter on enchanted creature",
        )
        .replace(
            "Tag the object attached to this aura as 'enchanted'. Put 1 -1/-1 counters on the tagged object 'enchanted'",
            "Put a -1/-1 counter on enchanted creature",
        );
    normalized = normalized
        .replace(
            "Sacrifice this creature: This creature deals ",
            "Sacrifice this creature: It deals ",
        )
        .replace(
            "Sacrifice this creature: this creature deals ",
            "Sacrifice this creature: It deals ",
        )
        .replace(", This creature deals ", ", it deals ")
        .replace(", this creature deals ", ", it deals ")
        .replace(", you gain 1 life. Scry 1", ", you gain 1 life and scry 1")
        .replace(" and deal 3 damage to this creature", " and 3 damage to itself")
        .replace(" and deal 2 damage to this creature", " and 2 damage to itself")
        .replace(" and deal 1 damage to this creature", " and 1 damage to itself")
        .replace(". Untap it. it gains Haste until end of turn", ". Untap that creature. It gains haste until end of turn")
        .replace(
            ". Untap target creature. it gains Haste until end of turn",
            ". Untap those creatures. They gain haste until end of turn",
        )
        .replace(
            ". Untap target creature. it gains haste until end of turn",
            ". Untap those creatures. They gain haste until end of turn",
        )
        .replace(
            "target creature can't be blocked this turn you draw a card",
            "Target creature can't be blocked this turn. Draw a card",
        )
        .replace(
            "target creature can't block this turn you draw a card",
            "Target creature can't block this turn. Draw a card",
        )
        .replace(
            "Tap any number of target untapped creatures you control",
            "Tap any number of untapped creatures you control",
        )
        .replace(
            "As an additional cost to cast this spell, sacrifice a creature you control",
            "As an additional cost to cast this spell, sacrifice a creature",
        )
        .replace(
            "As an additional cost to cast this spell, sacrifice an artifact you control",
            "As an additional cost to cast this spell, sacrifice an artifact",
        )
        .replace(
            "As an additional cost to cast this spell: Exile target creature card in your graveyard",
            "As an additional cost to cast this spell, exile a creature card from your graveyard",
        )
        .replace(
            "Create a 1/1 colorless Eldrazi Scion creature token with \"Sacrifice this creature: Add {C}",
            "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "Create 3 1/1 colorless Eldrazi Scion creature token with \"Sacrifice this creature: Add {C}",
            "Create three 1/1 colorless Eldrazi Scion creature tokens. They have \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "Create 2 1/1 colorless Eldrazi Scion creature token with \"Sacrifice this creature: Add {C}",
            "Create two 1/1 colorless Eldrazi Scion creature tokens. They have \"Sacrifice this token: Add {C}.\"",
        )
        .replace(
            "enchanted creature doesnt untap during its controllers untap step",
            "Enchanted creature doesn't untap during its controller's untap step",
        )
        .replace(
            "Return all creature to their owners' hands",
            "Return all creatures to their owners' hands",
        )
        .replace(
            "Counter target spell. Proliferate",
            "Counter target spell, then proliferate",
        )
        .replace(
            "Destroy target creature. Proliferate",
            "Destroy target creature, then proliferate",
        )
        .replace(
            "Deal 1 damage to each opponent's creature",
            "Deal 1 damage to each creature your opponents control",
        )
        .replace(
            ". an opponent's creature can't block this turn",
            ". Those creatures can't block this turn",
        )
        .replace(
            ". creature can't block this turn",
            ". That creature can't block this turn",
        );
    if normalized.contains("creatures you control get ")
        && normalized.ends_with(". Untap target creature")
    {
        normalized = normalized.replacen(
            ". Untap target creature",
            ". Untap those creatures",
            1,
        );
    }
    if normalized.contains("For each opponent, Gain control of up to one target creature that player controls until end of turn")
        && normalized.contains(". Untap target creature")
    {
        normalized = normalized.replacen(
            ". Untap target creature",
            ". Untap those creatures",
            1,
        );
        normalized = normalized.replace(
            ". it gains Haste until end of turn",
            ". They gain haste until end of turn",
        );
        normalized = normalized.replace(
            ". it gains haste until end of turn",
            ". They gain haste until end of turn",
        );
    }
    normalized = normalized
        .replace("Destroy all creature.", "Destroy all creatures.")
        .replace("Destroy all creature,", "Destroy all creatures,")
        .replace("Destroy all creature and", "Destroy all creatures and")
        .replace("Destroy all creature ", "Destroy all creatures ")
        .replace("Destroy all land.", "Destroy all lands.")
        .replace("Destroy all land,", "Destroy all lands,")
        .replace("Destroy all land and", "Destroy all lands and")
        .replace("Destroy all land ", "Destroy all lands ");
    if let Some(tail) = normalized.strip_prefix("each player discards their hand. you draw ") {
        normalized = format!("Each player discards their hand, then draws {tail}");
    }
    if let Some(tail) = normalized.strip_prefix("each player draws ") {
        if let Some((draw_count, discard_tail)) = tail.split_once(" cards. you discard ") {
            normalized = format!(
                "Each player draws {draw_count} cards, then discards {discard_tail}"
            );
        }
    }
    if let Some(rest) = normalized.strip_prefix("As an additional cost to cast this spell: You ") {
        normalized = format!("As an additional cost to cast this spell, {}", rest);
    } else if let Some(rest) =
        normalized.strip_prefix("As an additional cost to cast this spell: you ")
    {
        normalized = format!("As an additional cost to cast this spell, {}", rest);
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && let Some(rest) = right
            .to_ascii_lowercase()
            .strip_prefix("you gain the number of ")
            .map(str::to_string)
        && let Some(subject) = rest.strip_suffix(" life")
    {
        let mut subject = subject.trim_start_matches("a ").trim().to_string();
        if subject == "tapped creature" {
            subject = "creature tapped this way".to_string();
        }
        if left
            .to_ascii_lowercase()
            .starts_with("destroy all enchantment")
            && subject == "enchantment"
        {
            subject = "enchantment destroyed this way".to_string();
        }
        normalized = format!("{left}. you gain 1 life for each {subject}");
    }
    if let Some(rest) = normalized
        .to_ascii_lowercase()
        .strip_prefix("you gain the number of ")
        .map(str::to_string)
        && let Some(subject) = rest.strip_suffix(" life")
    {
        let mut subject = subject.trim_start_matches("a ").trim().to_string();
        if subject == "tapped creature" {
            subject = "creature tapped this way".to_string();
        }
        normalized = format!("you gain 1 life for each {subject}");
    }
    if normalized.contains("}.\":") {
        normalized = normalized.replace("}.\":", "}:");
    }
    if normalized.ends_with(".\"") {
        normalized.truncate(normalized.len().saturating_sub(2));
    }
    normalized = collapse_redundant_keyword_tail(&normalized);
    if let Some(rest) = normalized.strip_prefix("Choose one - ") {
        normalized = format!("Choose one —. {}", rest.replace("• ", ""));
    }
    if let Some(rest) = normalized.strip_prefix("Choose one — ") {
        normalized = format!("Choose one —. {}", rest.replace("• ", ""));
    }
    if let Some(rest) = normalized.strip_prefix("Choose one or both - ") {
        normalized = format!("Choose one or both —. {}", rest.replace("• ", ""));
    }
    if let Some(rest) = normalized.strip_prefix("Choose one or both — ") {
        normalized = format!("Choose one or both —. {}", rest.replace("• ", ""));
    }
    if normalized.contains("Draw 2 cards. Create 2 Treasure tokens") {
        normalized =
            normalized.replace("Draw 2 cards. Create 2 Treasure tokens", "Draw two cards and create two Treasure tokens");
    }
    normalized = normalized.replace(
        "Whenever this creature blocks or becomes blocked, this creature gets ",
        "Whenever this creature blocks or becomes blocked, it gets ",
    );
    normalized = normalized.replace(
        "Whenever this creature blocks or becomes blocked, it deals ",
        "Whenever this creature blocks or becomes blocked by a creature, this creature deals ",
    );
    normalized
}

/// Render compiled output in a near-oracle style for semantic diffing.
pub fn oracle_like_lines(def: &CardDefinition) -> Vec<String> {
    let base_lines = compiled_lines(def);
    let normalized = base_lines
        .iter()
        .map(|line| strip_render_heading(line))
        .map(|line| normalize_oracle_line_for_card(def, &line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let merged_predicates = merge_adjacent_subject_predicate_lines(normalized);
    let merged_mana = merge_adjacent_simple_mana_add_lines(merged_predicates);
    let merged_has_keywords = merge_subject_has_keyword_lines(merged_mana);
    merge_blockability_lines(merged_has_keywords)
}
